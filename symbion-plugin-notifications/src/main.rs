/**
 * SYMBION PLUGIN - Notifications Manager (F4)
 *
 * RÔLE : Plugin autonome pour gestion notifications push Firebase FCM + Email SMTP
 * ARCHITECTURE : Écoute MQTT symbion/notifications/send@v1, répond via symbion/notifications/sent@v1
 * UTILITÉ : Notifications mobiles même app fermée, validation humaine temps réel
 */

use rumqttc::{AsyncClient, Event, Incoming, MqttOptions, QoS};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use std::time::Duration;
use tokio::time::sleep;
use uuid::Uuid;
use axum::{
    extract::State,
    routing::{get, post},
    Json, Router,
};
use symbion_plugin_common::PluginHttpServer;
use tokio::signal::unix::{signal, SignalKind};
use tokio::sync::broadcast;
use lettre::{
    message::header::ContentType,
    transport::smtp::authentication::Credentials,
    AsyncSmtpTransport, AsyncTransport, Message, Tokio1Executor,
};

/// Notification complète avec métadonnées
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Notification {
    pub id: String,
    pub priority: NotificationPriority,
    pub title: String,
    pub body: String,
    pub source: String,
    #[serde(with = "time::serde::timestamp")]
    pub timestamp: time::OffsetDateTime,
    pub acknowledged: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub acknowledged_at: Option<i64>,
    #[serde(skip_serializing_if = "Vec::is_empty", default)]
    pub actions: Vec<NotificationAction>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub data: Option<serde_json::Value>,
}

/// Priorités des notifications avec sémantique retry
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum NotificationPriority {
    /// Critical - Immédiat + retry 5min si pas vu + email escalation
    P0,
    /// High - Retry 15min une fois
    P1,
    /// Normal - Best effort, fire-and-forget
    P2,
}

/// Actions interactives dans notification
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NotificationAction {
    pub id: String,
    pub label: String,
    pub action_type: ActionType,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ActionType {
    Approve,
    Reject,
    Snooze,
    Custom(String),
}

/// Token FCM enregistré
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FcmToken {
    pub user_id: String,
    pub token: String,
    pub device_name: Option<String>,
    pub registered_at: i64,
}

#[derive(Debug, Clone)]
pub struct SmtpConfig {
    pub server: String,
    pub port: u16,
    pub username: String,
    pub password: String,
    pub from_email: String,
    pub to_email: String,
}

/// Chemin du fichier de persistance
const NOTIFICATIONS_FILE: &str = "/var/lib/symbion/notifications.json";

/// Manager de notifications
pub struct NotificationManager {
    fcm_tokens: Arc<Mutex<HashMap<String, FcmToken>>>,
    active_notifications: Arc<Mutex<HashMap<String, Notification>>>,
    history: Arc<Mutex<Vec<Notification>>>,
    fcm_server_key: Option<String>,
    smtp_config: Option<SmtpConfig>,
    /// TEMPORAIRE: ntfy.sh en attendant l'app Symbion native
    ntfy_topic: Option<String>,
    /// URL externe Symbion pour les callbacks ntfy (ex: https://symbion.local:8443)
    external_url: Option<String>,
    /// API key pour les callbacks ntfy
    api_key: Option<String>,
    mqtt_client: AsyncClient,
}

impl NotificationManager {
    pub fn new(mqtt_client: AsyncClient) -> Self {
        let fcm_server_key = std::env::var("SYMBION_FCM_SERVER_KEY").ok();

        if fcm_server_key.is_none() {
            println!("[notifications-plugin] WARNING: SYMBION_FCM_SERVER_KEY not set - FCM disabled");
        } else {
            println!("[notifications-plugin] Firebase FCM enabled");
        }

        let smtp_config = if let (Ok(server), Ok(port), Ok(user), Ok(pass), Ok(from), Ok(to)) = (
            std::env::var("SYMBION_SMTP_SERVER"),
            std::env::var("SYMBION_SMTP_PORT"),
            std::env::var("SYMBION_SMTP_USERNAME"),
            std::env::var("SYMBION_SMTP_PASSWORD"),
            std::env::var("SYMBION_SMTP_FROM"),
            std::env::var("SYMBION_SMTP_TO"),
        ) {
            Some(SmtpConfig {
                server,
                port: port.parse().unwrap_or(587),
                username: user,
                password: pass,
                from_email: from,
                to_email: to,
            })
        } else {
            println!("[notifications-plugin] WARNING: SMTP config incomplete - email disabled");
            None
        };

        // TEMPORAIRE: ntfy.sh en attendant l'app Symbion native
        let ntfy_topic = std::env::var("SYMBION_NTFY_TOPIC").ok();
        if let Some(ref topic) = ntfy_topic {
            println!("[notifications-plugin] ntfy.sh enabled - topic: {}", topic);
        }

        // URL externe et API key pour les boutons d'action ntfy
        let external_url = std::env::var("SYMBION_EXTERNAL_URL").ok();
        let api_key = std::env::var("SYMBION_API_KEY").ok();
        if external_url.is_some() && api_key.is_some() {
            println!("[notifications-plugin] ntfy.sh action buttons enabled");
        }

        // Charger les notifications persistées
        let history = Self::load_from_file();
        let history_count = history.len();

        let manager = Self {
            fcm_tokens: Arc::new(Mutex::new(HashMap::new())),
            active_notifications: Arc::new(Mutex::new(HashMap::new())),
            history: Arc::new(Mutex::new(history)),
            fcm_server_key,
            smtp_config,
            ntfy_topic,
            external_url,
            api_key,
            mqtt_client,
        };

        if history_count > 0 {
            println!("[notifications-plugin] Loaded {} notifications from disk", history_count);
        }

        manager
    }

    /// Charge les notifications depuis le fichier JSON
    fn load_from_file() -> Vec<Notification> {
        match std::fs::read_to_string(NOTIFICATIONS_FILE) {
            Ok(content) => {
                serde_json::from_str(&content).unwrap_or_else(|e| {
                    eprintln!("[notifications-plugin] Failed to parse {}: {}", NOTIFICATIONS_FILE, e);
                    Vec::new()
                })
            }
            Err(_) => Vec::new(), // Fichier n'existe pas encore
        }
    }

    /// Sauvegarde les notifications dans le fichier JSON
    fn save_to_file(&self) {
        let history = self.history.lock().unwrap();

        // Créer le répertoire si nécessaire
        if let Some(parent) = std::path::Path::new(NOTIFICATIONS_FILE).parent() {
            let _ = std::fs::create_dir_all(parent);
        }

        match serde_json::to_string_pretty(&*history) {
            Ok(json) => {
                if let Err(e) = std::fs::write(NOTIFICATIONS_FILE, json) {
                    eprintln!("[notifications-plugin] Failed to save notifications: {}", e);
                }
            }
            Err(e) => eprintln!("[notifications-plugin] Failed to serialize notifications: {}", e),
        }
    }

    pub fn register_fcm_token(&self, user_id: String, token: String, device_name: Option<String>) {
        let fcm_token = FcmToken {
            user_id: user_id.clone(),
            token,
            device_name,
            registered_at: time::OffsetDateTime::now_utc().unix_timestamp(),
        };

        println!("[notifications-plugin] registered FCM token for user: {}", user_id);
        self.fcm_tokens.lock().unwrap().insert(user_id, fcm_token);
    }

    pub async fn send(&self, mut notif: Notification) -> Result<(), Box<dyn std::error::Error>> {
        if notif.id.is_empty() {
            notif.id = Uuid::new_v4().to_string();
        }

        // Store dans active + history
        {
            let mut active = self.active_notifications.lock().unwrap();
            active.insert(notif.id.clone(), notif.clone());
        }

        {
            let mut history = self.history.lock().unwrap();
            history.push(notif.clone());
            let history_len = history.len();
            if history_len > 1000 {
                history.drain(0..(history_len - 1000));
            }
        }

        // Persister sur disque
        self.save_to_file();

        println!("[notifications-plugin] sending: {} (priority: {:?})", notif.title, notif.priority);

        // Envoyer via FCM
        if let Err(e) = self.send_fcm(&notif).await {
            eprintln!("[notifications-plugin] FCM failed: {}", e);
        }

        // TEMPORAIRE: Envoyer P0/P1 vers ntfy.sh en attendant l'app Symbion native
        if notif.priority == NotificationPriority::P0 || notif.priority == NotificationPriority::P1 {
            if let Err(e) = self.send_ntfy(&notif).await {
                eprintln!("[notifications-plugin] ntfy.sh failed: {}", e);
            }
        }

        // Publish confirmation MQTT
        let sent_payload = serde_json::json!({
            "notification_id": notif.id,
            "status": "sent",
            "timestamp": time::OffsetDateTime::now_utc().unix_timestamp()
        });

        let _ = self.mqtt_client.publish(
            "symbion/notifications/sent@v1",
            QoS::AtLeastOnce,
            false,
            serde_json::to_string(&sent_payload)?
        ).await;

        // Retry logic selon priorité
        match notif.priority {
            NotificationPriority::P0 => {
                // P0: retry 5min + email
                let notif_id = notif.id.clone();
                let manager = self.clone_for_retry();
                tokio::spawn(async move {
                    sleep(Duration::from_secs(300)).await;
                    if !manager.is_acknowledged(&notif_id) {
                        println!("[notifications-plugin] P0 retry: {}", notif_id);
                        let _ = manager.send_fcm(&notif).await;
                        let _ = manager.send_email(&notif).await;
                    }
                });
            }
            NotificationPriority::P1 => {
                // P1: retry 15min
                let notif_id = notif.id.clone();
                let manager = self.clone_for_retry();
                tokio::spawn(async move {
                    sleep(Duration::from_secs(900)).await;
                    if !manager.is_acknowledged(&notif_id) {
                        println!("[notifications-plugin] P1 retry: {}", notif_id);
                        let _ = manager.send_fcm(&notif).await;
                    }
                });
            }
            NotificationPriority::P2 => {}
        }

        Ok(())
    }

    fn clone_for_retry(&self) -> Arc<Self> {
        Arc::new(Self {
            fcm_tokens: self.fcm_tokens.clone(),
            active_notifications: self.active_notifications.clone(),
            history: self.history.clone(),
            fcm_server_key: self.fcm_server_key.clone(),
            smtp_config: self.smtp_config.clone(),
            ntfy_topic: self.ntfy_topic.clone(),
            external_url: self.external_url.clone(),
            api_key: self.api_key.clone(),
            mqtt_client: self.mqtt_client.clone(),
        })
    }

    async fn send_fcm(&self, notif: &Notification) -> Result<(), Box<dyn std::error::Error>> {
        let fcm_key = match &self.fcm_server_key {
            Some(key) => key.clone(),
            None => return Ok(()),
        };

        let tokens_list: Vec<FcmToken> = {
            let tokens = self.fcm_tokens.lock().unwrap();
            if tokens.is_empty() {
                println!("[notifications-plugin] no FCM tokens, skipping");
                return Ok(());
            }
            tokens.values().cloned().collect()
        };

        for fcm_token in tokens_list.iter() {
            println!("[notifications-plugin] sending FCM to: {}", fcm_token.user_id);

            let fcm_payload = serde_json::json!({
                "to": fcm_token.token,
                "priority": match notif.priority {
                    NotificationPriority::P0 | NotificationPriority::P1 => "high",
                    NotificationPriority::P2 => "normal",
                },
                "notification": {
                    "title": notif.title,
                    "body": notif.body,
                    "sound": "default",
                },
                "data": {
                    "id": notif.id,
                    "priority": format!("{:?}", notif.priority),
                    "source": notif.source,
                    "timestamp": notif.timestamp.unix_timestamp(),
                }
            });

            let client = reqwest::Client::new();
            let response = client
                .post("https://fcm.googleapis.com/fcm/send")
                .header("Authorization", format!("key={}", fcm_key))
                .header("Content-Type", "application/json")
                .json(&fcm_payload)
                .timeout(Duration::from_secs(10))
                .send()
                .await?;

            if !response.status().is_success() {
                eprintln!("[notifications-plugin] FCM error: {}", response.status());
            }
        }

        Ok(())
    }

    async fn send_email(&self, notif: &Notification) -> Result<(), Box<dyn std::error::Error>> {
        if let Some(smtp) = &self.smtp_config {
            println!("[notifications-plugin] EMAIL escalation P0: {}", notif.title);
            println!("  From: {} To: {}", smtp.from_email, smtp.to_email);

            // Construire le message email
            let email = Message::builder()
                .from(smtp.from_email.parse()?)
                .to(smtp.to_email.parse()?)
                .subject(format!("[SYMBION P0] {}", notif.title))
                .header(ContentType::TEXT_PLAIN)
                .body(format!(
                    "{}\n\n---\nPriorité: {:?}\nSource: {}\nID: {}\n\nEnvoyé par Symbion Notifications",
                    notif.body,
                    notif.priority,
                    notif.source,
                    notif.id
                ))?;

            // Configurer le transport SMTP
            let creds = Credentials::new(smtp.username.clone(), smtp.password.clone());
            let mailer: AsyncSmtpTransport<Tokio1Executor> =
                AsyncSmtpTransport::<Tokio1Executor>::starttls_relay(&smtp.server)?
                    .port(smtp.port)
                    .credentials(creds)
                    .build();

            // Envoyer l'email
            match mailer.send(email).await {
                Ok(_) => println!("[notifications-plugin] ✅ Email sent successfully"),
                Err(e) => eprintln!("[notifications-plugin] ❌ Email send failed: {}", e),
            }
        }
        Ok(())
    }

    /// TEMPORAIRE: Envoi vers ntfy.sh en attendant l'app Symbion native
    /// Simple HTTP POST vers https://ntfy.sh/{topic}
    async fn send_ntfy(&self, notif: &Notification) -> Result<(), Box<dyn std::error::Error>> {
        if let Some(topic) = &self.ntfy_topic {
            let priority_label = match notif.priority {
                NotificationPriority::P0 => "🔴 P0 CRITIQUE",
                NotificationPriority::P1 => "🟠 P1 Important",
                NotificationPriority::P2 => "🟢 P2 Normal",
            };

            let ntfy_priority = match notif.priority {
                NotificationPriority::P0 => "5", // urgent
                NotificationPriority::P1 => "4", // high
                NotificationPriority::P2 => "3", // default
            };

            let url = format!("https://ntfy.sh/{}", topic);
            println!("[notifications-plugin] NTFY push: {} → {}", notif.title, url);

            let client = reqwest::Client::new();
            let mut request = client
                .post(&url)
                .header("Title", format!("[SYMBION] {}", notif.title))
                .header("Priority", ntfy_priority)
                .header("Tags", format!("symbion,{}", priority_label));

            // Ajouter bouton d'action "Vu" si URL externe et API key configurés
            if let (Some(ext_url), Some(api_key)) = (&self.external_url, &self.api_key) {
                let ack_url = format!(
                    "{}/v1/plugin-api/notifications/notifications/acknowledge",
                    ext_url
                );
                // Format ntfy Actions: http, <label>, <url>, method=POST, headers.X=Y, body=json
                let action = format!(
                    "http, ✅ Vu, {}, method=POST, headers.Content-Type=application/json, headers.X-API-Key={}, body={{\"notification_id\":\"{}\"}}",
                    ack_url, api_key, notif.id
                );
                request = request.header("Actions", action);
            }

            let response = request
                .body(format!("{}\n\n---\n{} | Source: {}", notif.body, priority_label, notif.source))
                .timeout(Duration::from_secs(10))
                .send()
                .await?;

            if response.status().is_success() {
                println!("[notifications-plugin] ✅ ntfy.sh sent successfully");
            } else {
                eprintln!("[notifications-plugin] ❌ ntfy.sh failed: {}", response.status());
            }
        }
        Ok(())
    }

    pub fn is_acknowledged(&self, notification_id: &str) -> bool {
        self.active_notifications
            .lock()
            .unwrap()
            .get(notification_id)
            .map(|n| n.acknowledged)
            .unwrap_or(false)
    }

    pub fn acknowledge(&self, notification_id: &str) -> Result<(), String> {
        let ack_time = time::OffsetDateTime::now_utc().unix_timestamp();

        // Mettre à jour dans active_notifications
        {
            let mut active = self.active_notifications.lock().unwrap();
            if let Some(notif) = active.get_mut(notification_id) {
                notif.acknowledged = true;
                notif.acknowledged_at = Some(ack_time);
            }
        }

        // Mettre à jour aussi dans history
        {
            let mut history = self.history.lock().unwrap();
            if let Some(notif) = history.iter_mut().find(|n| n.id == notification_id) {
                notif.acknowledged = true;
                notif.acknowledged_at = Some(ack_time);
                println!("[notifications-plugin] acknowledged: {}", notification_id);
                drop(history); // Libérer le lock avant save
                self.save_to_file();
                return Ok(());
            }
        }

        Err("Notification not found".to_string())
    }

    /// Supprime une notification
    pub fn delete(&self, notification_id: &str) -> Result<(), String> {
        // Supprimer de active_notifications
        {
            let mut active = self.active_notifications.lock().unwrap();
            active.remove(notification_id);
        }

        // Supprimer de history
        {
            let mut history = self.history.lock().unwrap();
            let len_before = history.len();
            history.retain(|n| n.id != notification_id);
            if history.len() < len_before {
                println!("[notifications-plugin] deleted: {}", notification_id);
                drop(history);
                self.save_to_file();
                return Ok(());
            }
        }

        Err("Notification not found".to_string())
    }

    pub fn list_active(&self) -> Vec<Notification> {
        self.active_notifications
            .lock()
            .unwrap()
            .values()
            .filter(|n| !n.acknowledged)
            .cloned()
            .collect()
    }

    pub fn list_all(&self) -> Vec<Notification> {
        let mut all = self.history.lock().unwrap().clone();
        all.sort_by(|a, b| b.timestamp.cmp(&a.timestamp));
        all
    }
}

// ============================================================================
// HTTP API Handlers
// ============================================================================

/// GET /notifications - List all notifications
async fn list_notifications(State(manager): State<Arc<NotificationManager>>) -> Json<serde_json::Value> {
    let notifications = manager.list_all();
    Json(serde_json::json!({
        "notifications": notifications,
        "timestamp": time::OffsetDateTime::now_utc().unix_timestamp()
    }))
}

/// POST /notifications - Send a new notification
async fn send_notification(
    State(manager): State<Arc<NotificationManager>>,
    Json(notif): Json<Notification>,
) -> Json<serde_json::Value> {
    match manager.send(notif.clone()).await {
        Ok(_) => Json(serde_json::json!({
            "status": "sent",
            "notification_id": notif.id
        })),
        Err(e) => Json(serde_json::json!({
            "status": "error",
            "error": e.to_string()
        })),
    }
}

/// POST /notifications/acknowledge - Acknowledge a notification
#[derive(Deserialize)]
struct AcknowledgeRequest {
    notification_id: String,
}

/// DELETE /notifications/:id - Delete a notification
#[derive(Deserialize)]
struct DeleteRequest {
    notification_id: String,
}

async fn acknowledge_notification(
    State(manager): State<Arc<NotificationManager>>,
    Json(req): Json<AcknowledgeRequest>,
) -> Json<serde_json::Value> {
    match manager.acknowledge(&req.notification_id) {
        Ok(_) => Json(serde_json::json!({
            "status": "acknowledged",
            "notification_id": req.notification_id
        })),
        Err(e) => Json(serde_json::json!({
            "status": "error",
            "error": format!("{}", e)
        })),
    }
}

/// POST /notifications/delete - Delete a notification
async fn delete_notification(
    State(manager): State<Arc<NotificationManager>>,
    Json(req): Json<DeleteRequest>,
) -> Json<serde_json::Value> {
    match manager.delete(&req.notification_id) {
        Ok(_) => Json(serde_json::json!({
            "status": "deleted",
            "notification_id": req.notification_id
        })),
        Err(e) => Json(serde_json::json!({
            "status": "error",
            "error": format!("{}", e)
        })),
    }
}

/// POST /fcm/register - Register FCM token
#[derive(Deserialize)]
struct RegisterFcmRequest {
    user_id: String,
    token: String,
    device_name: Option<String>,
}

async fn register_fcm(
    State(manager): State<Arc<NotificationManager>>,
    Json(req): Json<RegisterFcmRequest>,
) -> Json<serde_json::Value> {
    manager.register_fcm_token(req.user_id.clone(), req.token, req.device_name);
    Json(serde_json::json!({
        "status": "registered",
        "user_id": req.user_id
    }))
}

/// Health check endpoint
async fn health_check() -> Json<serde_json::Value> {
    use std::sync::OnceLock;
    static START_TIME: OnceLock<std::time::Instant> = OnceLock::new();
    let start = START_TIME.get_or_init(std::time::Instant::now);
    let uptime_secs = start.elapsed().as_secs();

    Json(serde_json::json!({
        "status": "healthy",
        "plugin": "notifications",
        "version": "0.1.0",
        "uptime_seconds": uptime_secs
    }))
}

/// Build HTTP router for plugin API
fn build_router(manager: Arc<NotificationManager>) -> Router {
    Router::new()
        .route("/health", get(health_check))
        .route("/notifications", get(list_notifications))
        .route("/notifications", post(send_notification))
        .route("/notifications/acknowledge", post(acknowledge_notification))
        .route("/notifications/delete", post(delete_notification))
        .route("/fcm/register", post(register_fcm))
        .with_state(manager)
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    dotenvy::dotenv().ok();

    let mqtt_broker = std::env::var("SYMBION_MQTT_BROKER").unwrap_or("127.0.0.1:1883".to_string());
    println!("[notifications-plugin] MQTT broker env: {}", mqtt_broker);

    let (host, port) = if mqtt_broker.contains(':') {
        let parts: Vec<&str> = mqtt_broker.split(':').collect();
        (parts[0].to_string(), parts.get(1).and_then(|p| p.parse().ok()).unwrap_or(1883))
    } else {
        (mqtt_broker.clone(), 1883)
    };

    println!("[notifications-plugin] Connecting to host: {}, port: {}", host, port);
    let mut mqttoptions = MqttOptions::new("symbion-plugin-notifications", &host, port);
    mqttoptions.set_keep_alive(Duration::from_secs(30));

    let (client, mut eventloop) = AsyncClient::new(mqttoptions, 10);

    println!("[notifications-plugin] starting...");
    println!("[notifications-plugin] MQTT broker: {}", mqtt_broker);

    // Subscribe aux topics
    client.subscribe("symbion/notifications/send@v1", QoS::AtLeastOnce).await?;
    client.subscribe("symbion/notifications/acknowledge@v1", QoS::AtLeastOnce).await?;
    client.subscribe("symbion/notifications/register_fcm@v1", QoS::AtLeastOnce).await?;
    client.subscribe("symbion/notifications/list@v1", QoS::AtLeastOnce).await?;

    println!("[notifications-plugin] subscribed to MQTT topics");

    let manager = Arc::new(NotificationManager::new(client.clone()));

    // Build HTTP router and start Unix socket server
    let router = build_router(manager.clone());
    let socket_path = "/run/symbion-plugins/notifications.sock";

    // Cleanup old socket at startup (triple safety net)
    if std::path::Path::new(socket_path).exists() {
        eprintln!("[notifications] cleaning up old socket at startup");
        let _ = std::fs::remove_file(socket_path);
    }

    // Create shutdown channel for graceful termination
    let (shutdown_tx, mut shutdown_rx) = broadcast::channel::<()>(1);

    let http_server = PluginHttpServer::new(socket_path, router);

    println!("[notifications-plugin] starting HTTP server on Unix socket: {}", socket_path);

    // Wait for socket to be created before registering (server will create it in serve())
    // We'll register in parallel with server startup
    let socket_path_clone = socket_path.to_string();
    tokio::spawn(async move {
        // Give server time to create socket
        tokio::time::sleep(Duration::from_millis(500)).await;

        // Register with kernel using Service Discovery
        use symbion_plugin_common::PluginRegistrationBuilder;

        match PluginRegistrationBuilder::new("notifications", &socket_path_clone)
            .route("/notifications")
            .route("/notifications/send")
            .route("/notifications/acknowledge")
            .route("/notifications/delete")
            .route("/fcm/register")
            .route("/health")
            .version("1.0.0")
            .description("Push notifications plugin with FCM + Email + persistence support")
            .register()
            .await
        {
            Ok(_) => println!("[notifications-plugin] ✅ Registered with kernel via Service Discovery"),
            Err(e) => eprintln!("[notifications-plugin] ❌ Failed to register with kernel: {}", e),
        }
    });

    // Signal handlers for graceful shutdown (SIGTERM from systemd, SIGINT from Ctrl+C)
    let socket_path_for_cleanup = socket_path.to_string();
    let shutdown_tx_clone = shutdown_tx.clone();
    tokio::spawn(async move {
        let mut sigterm = signal(SignalKind::terminate()).expect("failed to install SIGTERM handler");
        let mut sigint = signal(SignalKind::interrupt()).expect("failed to install SIGINT handler");

        tokio::select! {
            _ = sigterm.recv() => {
                eprintln!("[notifications] received SIGTERM, shutting down gracefully...");
            }
            _ = sigint.recv() => {
                eprintln!("[notifications] received SIGINT (Ctrl+C), shutting down gracefully...");
            }
        }

        // Cleanup socket
        if std::path::Path::new(&socket_path_for_cleanup).exists() {
            eprintln!("[notifications] cleaning up socket: {}", socket_path_for_cleanup);
            let _ = std::fs::remove_file(&socket_path_for_cleanup);
        }

        // Signal main loop to exit
        let _ = shutdown_tx_clone.send(());
    });

    // Run MQTT event loop and HTTP server concurrently
    let mqtt_handle = tokio::spawn(async move {
        mqtt_event_loop(eventloop, manager, client, shutdown_rx).await;
    });

    let http_handle = tokio::spawn(async move {
        if let Err(e) = http_server.serve().await {
            eprintln!("[notifications-plugin] HTTP server error: {}", e);
        }
    });

    // Wait for both tasks (they should run forever)
    let _ = tokio::join!(mqtt_handle, http_handle);

    eprintln!("[notifications] exited main loop, performing final cleanup");
    Ok(())
}

/// MQTT event loop - separated for concurrency
async fn mqtt_event_loop(
    mut eventloop: rumqttc::EventLoop,
    manager: Arc<NotificationManager>,
    client: AsyncClient,
    mut shutdown_rx: broadcast::Receiver<()>,
) {
    loop {
        tokio::select! {
            // Check for shutdown signal
            _ = shutdown_rx.recv() => {
                eprintln!("[notifications] shutdown signal received, exiting MQTT loop");
                break;
            }
            // Process MQTT events
            event = eventloop.poll() => {
                match event {
                    Ok(Event::Incoming(Incoming::Publish(p))) => {
                        let topic = &p.topic;
                        let payload = String::from_utf8_lossy(&p.payload);

                        match topic.as_str() {
                            "symbion/notifications/send@v1" => {
                                if let Ok(notif) = serde_json::from_str::<Notification>(&payload) {
                                    let mgr = manager.clone();
                                    tokio::spawn(async move {
                                        if let Err(e) = mgr.send(notif).await {
                                            eprintln!("[notifications-plugin] send error: {}", e);
                                        }
                                    });
                                }
                            }
                            "symbion/notifications/acknowledge@v1" => {
                                if let Ok(req) = serde_json::from_str::<serde_json::Value>(&payload) {
                                    if let Some(id) = req.get("notification_id").and_then(|v| v.as_str()) {
                                        let _ = manager.acknowledge(id);
                                    }
                                }
                            }
                            "symbion/notifications/register_fcm@v1" => {
                                if let Ok(req) = serde_json::from_str::<serde_json::Value>(&payload) {
                                    let user_id = req.get("user_id").and_then(|v| v.as_str()).unwrap_or("default");
                                    let token = req.get("token").and_then(|v| v.as_str()).unwrap_or("");
                                    let device_name = req.get("device_name").and_then(|v| v.as_str()).map(String::from);
                                    manager.register_fcm_token(user_id.to_string(), token.to_string(), device_name);
                                }
                            }
                            "symbion/notifications/list@v1" => {
                                let notifications = manager.list_all();
                                let response = serde_json::json!({
                                    "notifications": notifications,
                                    "timestamp": time::OffsetDateTime::now_utc().unix_timestamp()
                                });
                                let _ = client.publish(
                                    "symbion/notifications/listed@v1",
                                    QoS::AtLeastOnce,
                                    false,
                                    serde_json::to_string(&response).unwrap()
                                ).await;
                            }
                            _ => {}
                        }
                    }
                    Err(e) => {
                        eprintln!("[notifications-plugin] MQTT error: {}", e);
                        eprintln!("[notifications-plugin] Fatal error - exiting to allow restart");
                        break;
                    }
                    _ => {}
                }
            }
        }
    }
}
