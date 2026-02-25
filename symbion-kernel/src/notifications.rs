/**
 * SYMBION KERNEL - Notifications Manager (F4)
 *
 * RÔLE : Gestion centralisée des notifications push mobiles (Firebase FCM) et email (SMTP)
 * ARCHITECTURE : Priority-based avec retry automatique P0/P1, acknowledgment tracking
 * UTILITÉ : Permet notifications mobiles même app fermée, validation humaine temps réel
 */

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};
use tokio::time::sleep;
use uuid::Uuid;
use rumqttc::{AsyncClient, QoS};
use utoipa::ToSchema;

/// Notification complète avec métadonnées
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct Notification {
    pub id: String,
    pub priority: NotificationPriority,
    pub title: String,
    pub body: String,
    pub source: String,
    #[serde(with = "time::serde::timestamp")]
    #[schema(value_type = String)]
    pub timestamp: time::OffsetDateTime,
    pub acknowledged: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub acknowledged_at: Option<i64>,
    #[serde(skip_serializing_if = "Vec::is_empty", default)]
    pub actions: Vec<NotificationAction>,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[schema(value_type = Option<Object>)]
    pub data: Option<serde_json::Value>,
}

/// Priorités des notifications avec sémantique retry
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, ToSchema)]
pub enum NotificationPriority {
    /// Critical - Immédiat + retry 5min si pas vu + email escalation
    P0,
    /// High - Retry 15min une fois
    P1,
    /// Normal - Best effort, fire-and-forget
    P2,
}

/// Actions interactives dans notification (validation humaine)
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct NotificationAction {
    pub id: String,
    pub label: String,
    pub action_type: ActionType,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "snake_case")]
pub enum ActionType {
    Approve,
    Reject,
    Snooze,
    Custom(String),
}

/// Token FCM enregistré par l'app mobile
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct FcmToken {
    pub user_id: String,
    pub token: String,
    pub device_name: Option<String>,
    pub registered_at: i64,
}

/// Chemin de persistance des notifications (configurable via SYMBION_DATA_DIR)
fn notifications_file() -> String {
    let base = std::env::var("SYMBION_DATA_DIR").unwrap_or_else(|_| "/var/lib/symbion".to_string());
    format!("{}/notifications.json", base)
}

/// Manager de notifications avec Firebase FCM + Email SMTP + ntfy.sh + MQTT
pub struct NotificationManager {
    /// Tokens FCM enregistrés (user_id -> token)
    fcm_tokens: Arc<Mutex<HashMap<String, FcmToken>>>,
    /// Notifications actives (id -> notification)
    active_notifications: Arc<Mutex<HashMap<String, Notification>>>,
    /// Historique des notifications (limitées aux 1000 dernières)
    history: Arc<Mutex<Vec<Notification>>>,
    /// Configuration Firebase (API key)
    fcm_server_key: Option<String>,
    /// Configuration Email SMTP
    smtp_config: Option<SmtpConfig>,
    /// TEMPORAIRE: ntfy.sh topic en attendant l'app Symbion native
    ntfy_topic: Option<String>,
    /// URL externe Symbion pour les callbacks ntfy
    external_url: Option<String>,
    /// API key pour les callbacks ntfy
    api_key: Option<String>,
    /// Client MQTT pour publier vers PWA dashboard
    mqtt_client: Option<AsyncClient>,
    /// Rate limiting: (source -> (last_reset, count))
    rate_limits: Arc<Mutex<HashMap<String, (Instant, u32)>>>,
    /// Deduplication: recent content hashes with timestamp
    recent_hashes: Arc<Mutex<Vec<(u64, Instant)>>>,
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

impl NotificationManager {
    /// Crée un nouveau manager de notifications
    pub fn new(mqtt_client: Option<AsyncClient>) -> Self {
        let fcm_server_key = std::env::var("SYMBION_FCM_SERVER_KEY").ok();

        if fcm_server_key.is_none() {
            println!("[notifications] WARNING: SYMBION_FCM_SERVER_KEY not set - FCM disabled");
        } else {
            println!("[notifications] Firebase FCM enabled");
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
                port: port.parse().unwrap_or_else(|e| {
                    eprintln!("[notifications] Invalid SMTP port '{}': {} — using default 587", port, e);
                    587
                }),
                username: user,
                password: pass,
                from_email: from,
                to_email: to,
            })
        } else {
            println!("[notifications] WARNING: SMTP config incomplete - email escalation disabled");
            None
        };

        // ntfy.sh configuration (temporary solution for mobile push)
        let ntfy_topic = std::env::var("SYMBION_NTFY_TOPIC").ok();
        if let Some(ref topic) = ntfy_topic {
            println!("[notifications] ntfy.sh enabled - topic: {}", topic);
        }

        let external_url = std::env::var("SYMBION_EXTERNAL_URL").ok();
        let api_key = std::env::var("SYMBION_API_KEY").ok();
        if external_url.is_some() && api_key.is_some() {
            println!("[notifications] ntfy.sh action buttons enabled");
        }

        // Load persisted notifications
        let history = Self::load_from_file();
        let history_count = history.len();

        if mqtt_client.is_some() {
            println!("[notifications] MQTT publishing enabled for PWA toasts");
        }

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
            rate_limits: Arc::new(Mutex::new(HashMap::new())),
            recent_hashes: Arc::new(Mutex::new(Vec::new())),
        };

        if history_count > 0 {
            println!("[notifications] Loaded {} notifications from disk", history_count);
        }

        manager
    }

    /// Charge les notifications depuis le fichier JSON
    fn load_from_file() -> Vec<Notification> {
        match std::fs::read_to_string(&notifications_file()) {
            Ok(content) => serde_json::from_str(&content).unwrap_or_else(|e| {
                eprintln!("[notifications] Failed to parse {}: {}", &notifications_file(), e);
                Vec::new()
            }),
            Err(_) => Vec::new(),
        }
    }

    /// Sauvegarde les notifications dans le fichier JSON
    fn save_to_file(&self) {
        let history = self.history.lock().unwrap();

        // Create directory if needed
        if let Some(parent) = std::path::Path::new(&notifications_file()).parent() {
            let _ = std::fs::create_dir_all(parent);
        }

        match serde_json::to_string_pretty(&*history) {
            Ok(json) => {
                if let Err(e) = std::fs::write(&notifications_file(), json) {
                    eprintln!("[notifications] Failed to save notifications: {}", e);
                }
            }
            Err(e) => eprintln!("[notifications] Failed to serialize notifications: {}", e),
        }
    }

    /// Enregistre un token FCM pour un utilisateur
    pub fn register_fcm_token(&self, user_id: String, token: String, device_name: Option<String>) {
        let fcm_token = FcmToken {
            user_id: user_id.clone(),
            token,
            device_name,
            registered_at: time::OffsetDateTime::now_utc().unix_timestamp(),
        };

        println!("[notifications] registered FCM token for user: {}", user_id);
        self.fcm_tokens.lock().unwrap().insert(user_id, fcm_token);
    }

    /// Envoie une notification à tous les tokens FCM enregistrés
    pub async fn send(self: &Arc<Self>, mut notif: Notification) -> Result<(), Box<dyn std::error::Error>> {
        // Assign ID si pas déjà défini
        if notif.id.is_empty() {
            notif.id = Uuid::new_v4().to_string();
        }

        // Rate limiting: max 10 notifications per source per minute
        {
            let mut limits = self.rate_limits.lock().unwrap();
            let source = notif.source.clone();
            let now = Instant::now();

            let (last_reset, count) = limits.entry(source.clone()).or_insert((now, 0));
            if now.duration_since(*last_reset) > Duration::from_secs(60) {
                *last_reset = now;
                *count = 0;
            }
            if *count >= 10 {
                eprintln!("[notifications] Rate limit exceeded for source: {}", source);
                return Err("Rate limit exceeded (max 10/min per source)".into());
            }
            *count += 1;
        }

        // Deduplication: skip if same content hash seen in last 60 seconds
        {
            let content_hash = {
                let mut hasher = DefaultHasher::new();
                notif.title.hash(&mut hasher);
                notif.body.hash(&mut hasher);
                notif.source.hash(&mut hasher);
                hasher.finish()
            };

            let mut hashes = self.recent_hashes.lock().unwrap();
            let now = Instant::now();

            // Clean old entries (older than 60 seconds)
            hashes.retain(|(_, ts)| now.duration_since(*ts) < Duration::from_secs(60));

            // Check for duplicate
            if hashes.iter().any(|(h, _)| *h == content_hash) {
                println!("[notifications] Duplicate notification skipped: {}", notif.title);
                return Ok(()); // Silently skip duplicate
            }

            hashes.push((content_hash, now));
        }

        // Store dans active + history
        {
            let mut active = self.active_notifications.lock().unwrap();
            active.insert(notif.id.clone(), notif.clone());
        }

        {
            let mut history = self.history.lock().unwrap();
            history.push(notif.clone());
            // Limiter historique à 1000 dernières notifications
            let history_len = history.len();
            if history_len > 1000 {
                history.drain(0..(history_len - 1000));
            }
        }

        // Persist to disk
        self.save_to_file();

        println!("[notifications] sending notification: {} (priority: {:?})", notif.title, notif.priority);

        // Publier sur MQTT pour PWA toasts temps réel
        if let Some(ref client) = self.mqtt_client {
            let payload = serde_json::json!({
                "notification": notif,
                "timestamp": time::OffsetDateTime::now_utc().unix_timestamp()
            });
            if let Ok(json) = serde_json::to_string(&payload) {
                let client = client.clone();
                tokio::spawn(async move {
                    if let Err(e) = client.publish(
                        "symbion/notifications/sent@v1",
                        QoS::AtLeastOnce,
                        false,
                        json
                    ).await {
                        eprintln!("[notifications] MQTT publish failed: {}", e);
                    } else {
                        println!("[notifications] published to MQTT for PWA");
                    }
                });
            }
        }

        // Envoyer via FCM
        if let Err(e) = self.send_fcm(&notif).await {
            eprintln!("[notifications] FCM send failed: {}", e);
        }

        // TEMPORAIRE: Envoyer P0/P1 vers ntfy.sh en attendant l'app Symbion native
        if notif.priority == NotificationPriority::P0 || notif.priority == NotificationPriority::P1 {
            if let Err(e) = self.send_ntfy(&notif).await {
                eprintln!("[notifications] ntfy.sh failed: {}", e);
            }
        }

        // P0 = email immédiat (alerte critique)
        if notif.priority == NotificationPriority::P0 {
            if let Err(e) = self.send_email(&notif).await {
                eprintln!("[notifications] email failed: {}", e);
            }
        }

        // Retry logic selon priorité
        match notif.priority {
            NotificationPriority::P0 => {
                // P0: retry après 5min si pas acknowledged + email escalation
                let notif_id = notif.id.clone();
                let self_clone = Arc::clone(self);
                tokio::spawn(async move {
                    sleep(Duration::from_secs(300)).await;
                    if !self_clone.is_acknowledged(&notif_id) {
                        println!("[notifications] P0 retry after 5min: {}", notif_id);
                        let _ = self_clone.send_fcm(&notif).await;
                        let _ = self_clone.send_ntfy(&notif).await;
                        let _ = self_clone.send_email(&notif).await;
                    }
                });
            }
            NotificationPriority::P1 => {
                // P1: retry après 15min une fois
                let notif_id = notif.id.clone();
                let self_clone = Arc::clone(self);
                tokio::spawn(async move {
                    sleep(Duration::from_secs(900)).await;
                    if !self_clone.is_acknowledged(&notif_id) {
                        println!("[notifications] P1 retry after 15min: {}", notif_id);
                        let _ = self_clone.send_fcm(&notif).await;
                        let _ = self_clone.send_ntfy(&notif).await;
                    }
                });
            }
            NotificationPriority::P2 => {
                // P2: best effort, pas de retry
            }
        }

        Ok(())
    }

    /// Sanitize header value to prevent HTTP header injection
    fn sanitize_header(value: &str) -> String {
        value
            .replace('\n', " ")
            .replace('\r', " ")
            .replace('\0', "")
            .trim()
            .to_string()
    }

    /// Envoie notification via ntfy.sh (solution temporaire)
    async fn send_ntfy(&self, notif: &Notification) -> Result<(), Box<dyn std::error::Error>> {
        let topic = match &self.ntfy_topic {
            Some(t) => t.clone(),
            None => return Ok(()),
        };

        println!("[notifications] sending ntfy.sh to topic: {}", topic);

        // Priority mapping pour ntfy
        let priority = match notif.priority {
            NotificationPriority::P0 => "5", // max
            NotificationPriority::P1 => "4", // high
            NotificationPriority::P2 => "3", // default
        };

        let url = format!("https://ntfy.sh/{}", topic);
        let client = reqwest::Client::new();

        // Sanitize user-controlled values for HTTP headers
        let safe_title = Self::sanitize_header(&notif.title);

        let mut request = client
            .post(&url)
            .header("Title", &safe_title)
            .header("Priority", priority)
            .header("Tags", match notif.priority {
                NotificationPriority::P0 => "rotating_light,warning",
                NotificationPriority::P1 => "bell,warning",
                NotificationPriority::P2 => "bell",
            });

        // Add action buttons if external URL and API key are configured
        if let (Some(ext_url), Some(api_key)) = (&self.external_url, &self.api_key) {
            // Sanitize notification ID in URL to prevent injection
            let safe_id = Self::sanitize_header(&notif.id);
            let ack_url = format!("{}/v1/notifications/{}/acknowledge?api_key={}", ext_url, safe_id, api_key);
            request = request.header("Actions", format!("http, Acquitter, {}, clear=true", ack_url));
        }

        // Sanitize body as well
        let safe_body = Self::sanitize_header(&notif.body);
        let response = request
            .body(safe_body)
            .timeout(Duration::from_secs(10))
            .send()
            .await?;

        if !response.status().is_success() {
            let status = response.status();
            let body = response.text().await?;
            eprintln!("[notifications] ntfy.sh failed: {} - {}", status, body);
        } else {
            println!("[notifications] ntfy.sh sent successfully");
        }

        Ok(())
    }

    /// Envoie notification via Firebase FCM
    async fn send_fcm(&self, notif: &Notification) -> Result<(), Box<dyn std::error::Error>> {
        let fcm_key = match &self.fcm_server_key {
            Some(key) => key.clone(),
            None => {
                println!("[notifications] FCM not configured, skipping");
                return Ok(());
            }
        };

        // Clone tokens to avoid holding lock across await
        let tokens_list: Vec<FcmToken> = {
            let tokens = self.fcm_tokens.lock().unwrap();
            if tokens.is_empty() {
                println!("[notifications] no FCM tokens registered, skipping");
                return Ok(());
            }
            tokens.values().cloned().collect()
        };

        for fcm_token in tokens_list.iter() {
            let user_id = &fcm_token.user_id;
            println!("[notifications] sending FCM to user: {} (device: {:?})", user_id, fcm_token.device_name);

            // Payload FCM selon format Firebase Cloud Messaging HTTP v1 API
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
                    "actions": serde_json::to_string(&notif.actions)?,
                    "custom_data": notif.data.as_ref().unwrap_or(&serde_json::json!({})),
                }
            });

            // Envoyer HTTP POST vers FCM
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
                let status = response.status();
                let body = response.text().await?;
                eprintln!("[notifications] FCM failed for user {}: {} - {}", user_id, status, body);
            } else {
                println!("[notifications] FCM sent successfully to user: {}", user_id);
            }
        }

        Ok(())
    }

    /// Envoie notification via Email (escalation P0) - utilise msmtp
    async fn send_email(&self, notif: &Notification) -> Result<(), Box<dyn std::error::Error>> {
        // Utiliser msmtp qui est déjà configuré sur le système
        let to_email = std::env::var("SYMBION_EMAIL_TO")
            .unwrap_or_else(|_| "Markchavatte@gmail.com".to_string());

        println!("[notifications] sending email escalation for P0: {}", notif.title);

        let subject = format!("[Symbion P0] {}", notif.title);
        let timestamp = time::OffsetDateTime::now_utc()
            .format(&time::format_description::well_known::Rfc3339)
            .unwrap_or_else(|_| "unknown".to_string());

        let body = format!(
            "From: Symbion System <Markchavatte@gmail.com>\n\
             To: {}\n\
             Subject: {}\n\
             Content-Type: text/plain; charset=utf-8\n\n\
             {}\n\n\
             ---\n\
             Priority: {:?}\n\
             Source: {}\n\
             Notification ID: {}\n\
             Timestamp: {}\n\n\
             ---\n\
             Sent from Symbion Notification System",
            to_email, subject, notif.body, notif.priority, notif.source, notif.id, timestamp
        );

        // Exécuter msmtp en async
        let output = tokio::process::Command::new("msmtp")
            .arg("-t")
            .stdin(std::process::Stdio::piped())
            .stdout(std::process::Stdio::piped())
            .stderr(std::process::Stdio::piped())
            .spawn();

        match output {
            Ok(mut child) => {
                if let Some(mut stdin) = child.stdin.take() {
                    use tokio::io::AsyncWriteExt;
                    if let Err(e) = stdin.write_all(body.as_bytes()).await {
                        eprintln!("[notifications] failed to write to msmtp stdin: {}", e);
                        return Err(e.into());
                    }
                }

                let result = child.wait_with_output().await?;
                if result.status.success() {
                    println!("[notifications] email sent successfully via msmtp to {}", to_email);
                } else {
                    let stderr = String::from_utf8_lossy(&result.stderr);
                    eprintln!("[notifications] msmtp failed: {}", stderr);
                    return Err(format!("msmtp failed: {}", stderr).into());
                }
            }
            Err(e) => {
                eprintln!("[notifications] failed to spawn msmtp: {}", e);
                return Err(e.into());
            }
        }

        Ok(())
    }

    /// Marque une notification comme acquittée
    pub fn acknowledge(&self, notification_id: &str) -> Result<(), Box<dyn std::error::Error>> {
        let now = time::OffsetDateTime::now_utc().unix_timestamp();

        // Update active_notifications
        {
            let mut active = self.active_notifications.lock().unwrap();
            if let Some(notif) = active.get_mut(notification_id) {
                notif.acknowledged = true;
                notif.acknowledged_at = Some(now);
            }
        }

        // Update history (source of truth for persistence)
        {
            let mut history = self.history.lock().unwrap();
            if let Some(notif) = history.iter_mut().find(|n| n.id == notification_id) {
                notif.acknowledged = true;
                notif.acknowledged_at = Some(now);
                println!("[notifications] acknowledged: {}", notification_id);
            } else {
                return Err("Notification not found".into());
            }
        }

        self.save_to_file();
        Ok(())
    }

    /// Supprime une notification
    pub fn delete(&self, notification_id: &str) -> Result<(), Box<dyn std::error::Error>> {
        // Remove from active_notifications
        {
            let mut active = self.active_notifications.lock().unwrap();
            active.remove(notification_id);
        }

        // Remove from history (source of truth for persistence)
        {
            let mut history = self.history.lock().unwrap();
            let len_before = history.len();
            history.retain(|n| n.id != notification_id);
            if history.len() == len_before {
                return Err("Notification not found".into());
            }
            println!("[notifications] deleted: {}", notification_id);
        }

        self.save_to_file();
        Ok(())
    }

    /// Supprime toutes les notifications
    pub fn delete_all(&self) -> usize {
        let count;
        {
            let mut active = self.active_notifications.lock().unwrap();
            active.clear();
        }
        {
            let mut history = self.history.lock().unwrap();
            count = history.len();
            history.clear();
        }
        self.save_to_file();
        println!("[notifications] deleted all ({} notifications)", count);
        count
    }

    /// Vérifie si une notification est acquittée
    pub fn is_acknowledged(&self, notification_id: &str) -> bool {
        self.active_notifications
            .lock()
            .unwrap()
            .get(notification_id)
            .map(|n| n.acknowledged)
            .unwrap_or(false)
    }

    /// Liste toutes les notifications actives (non acquittées)
    pub fn list_active(&self) -> Vec<Notification> {
        self.active_notifications
            .lock()
            .unwrap()
            .values()
            .filter(|n| !n.acknowledged)
            .cloned()
            .collect()
    }

    /// Liste toutes les notifications (actives + historique)
    pub fn list_all(&self) -> Vec<Notification> {
        let mut all = self.history.lock().unwrap().clone();
        // Trier par timestamp décroissant (plus récentes en premier)
        all.sort_by(|a, b| b.timestamp.cmp(&a.timestamp));
        all
    }

    /// Nettoie les notifications acquittées anciennes (> 7 jours)
    pub fn cleanup_old_notifications(&self) {
        let cutoff = time::OffsetDateTime::now_utc() - time::Duration::days(7);

        let mut active = self.active_notifications.lock().unwrap();
        active.retain(|_, notif| {
            !notif.acknowledged || notif.timestamp > cutoff
        });

        println!("[notifications] cleanup: {} active notifications remaining", active.len());
    }

    /// Liste tous les tokens FCM enregistrés
    pub fn list_fcm_tokens(&self) -> Vec<FcmToken> {
        self.fcm_tokens.lock().unwrap().values().cloned().collect()
    }
}

impl Default for NotificationManager {
    fn default() -> Self {
        Self::new(None)
    }
}

/// Type partagé pour le manager
pub type SharedNotificationManager = std::sync::Arc<NotificationManager>;

/// Crée un manager partagé avec client MQTT pour PWA toasts
pub fn create_shared_manager(mqtt_client: Option<AsyncClient>) -> SharedNotificationManager {
    std::sync::Arc::new(NotificationManager::new(mqtt_client))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_notification_creation() {
        let notif = Notification {
            id: "test-1".to_string(),
            priority: NotificationPriority::P1,
            title: "Test Notification".to_string(),
            body: "This is a test".to_string(),
            source: "test-module".to_string(),
            timestamp: time::OffsetDateTime::now_utc(),
            acknowledged: false,
            acknowledged_at: None,
            actions: vec![],
            data: None,
        };

        assert_eq!(notif.priority, NotificationPriority::P1);
        assert!(!notif.acknowledged);
    }

    #[test]
    fn test_fcm_token_registration() {
        let manager = NotificationManager::new(None);
        manager.register_fcm_token(
            "user-1".to_string(),
            "fcm-token-123".to_string(),
            Some("iPhone 13".to_string()),
        );

        let tokens = manager.list_fcm_tokens();
        assert_eq!(tokens.len(), 1);
        assert_eq!(tokens[0].user_id, "user-1");
        assert_eq!(tokens[0].token, "fcm-token-123");
    }

    #[test]
    fn test_acknowledgment() {
        let manager = NotificationManager::new(None);
        let notif = Notification {
            id: "test-ack".to_string(),
            priority: NotificationPriority::P2,
            title: "Test".to_string(),
            body: "Test".to_string(),
            source: "test".to_string(),
            timestamp: time::OffsetDateTime::now_utc(),
            acknowledged: false,
            acknowledged_at: None,
            actions: vec![],
            data: None,
        };

        manager.active_notifications.lock().unwrap().insert(notif.id.clone(), notif.clone());
        manager.history.lock().unwrap().push(notif);

        assert!(!manager.is_acknowledged("test-ack"));
        manager.acknowledge("test-ack").unwrap();
        assert!(manager.is_acknowledged("test-ack"));
    }
}
