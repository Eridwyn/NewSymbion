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

/// Manager de notifications
pub struct NotificationManager {
    fcm_tokens: Arc<Mutex<HashMap<String, FcmToken>>>,
    active_notifications: Arc<Mutex<HashMap<String, Notification>>>,
    history: Arc<Mutex<Vec<Notification>>>,
    fcm_server_key: Option<String>,
    smtp_config: Option<SmtpConfig>,
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

        Self {
            fcm_tokens: Arc::new(Mutex::new(HashMap::new())),
            active_notifications: Arc::new(Mutex::new(HashMap::new())),
            history: Arc::new(Mutex::new(Vec::new())),
            fcm_server_key,
            smtp_config,
            mqtt_client,
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

        println!("[notifications-plugin] sending: {} (priority: {:?})", notif.title, notif.priority);

        // Envoyer via FCM
        if let Err(e) = self.send_fcm(&notif).await {
            eprintln!("[notifications-plugin] FCM failed: {}", e);
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
            // TODO: intégration lettre crate
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
        let mut active = self.active_notifications.lock().unwrap();
        if let Some(notif) = active.get_mut(notification_id) {
            notif.acknowledged = true;
            notif.acknowledged_at = Some(time::OffsetDateTime::now_utc().unix_timestamp());
            println!("[notifications-plugin] acknowledged: {}", notification_id);
            Ok(())
        } else {
            Err("Notification not found".to_string())
        }
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

#[tokio::main]
async fn main() {
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
    client.subscribe("symbion/notifications/send@v1", QoS::AtLeastOnce).await.unwrap();
    client.subscribe("symbion/notifications/acknowledge@v1", QoS::AtLeastOnce).await.unwrap();
    client.subscribe("symbion/notifications/register_fcm@v1", QoS::AtLeastOnce).await.unwrap();
    client.subscribe("symbion/notifications/list@v1", QoS::AtLeastOnce).await.unwrap();

    println!("[notifications-plugin] subscribed to MQTT topics");

    let manager = Arc::new(NotificationManager::new(client.clone()));

    loop {
        match eventloop.poll().await {
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
                sleep(Duration::from_secs(1)).await;
            }
            _ => {}
        }
    }
}
