/**
 * SYMBION KERNEL - Notifications Manager (F4)
 *
 * RÔLE : Gestion centralisée des notifications push mobiles (Firebase FCM) et email (SMTP)
 * ARCHITECTURE : Priority-based avec retry automatique P0/P1, acknowledgment tracking
 * UTILITÉ : Permet notifications mobiles même app fermée, validation humaine temps réel
 */

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

/// Actions interactives dans notification (validation humaine)
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

/// Token FCM enregistré par l'app mobile
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FcmToken {
    pub user_id: String,
    pub token: String,
    pub device_name: Option<String>,
    pub registered_at: i64,
}

/// Manager de notifications avec Firebase FCM + Email SMTP
pub struct NotificationManager {
    /// Tokens FCM enregistrés (user_id -> token)
    fcm_tokens: Arc<Mutex<HashMap<String, FcmToken>>>,
    /// Notifications actives (id -> notification)
    active_notifications: Arc<Mutex<HashMap<String, Notification>>>,
    /// Historique des notifications (limitées aux 1000 dernières)
    history: Arc<Mutex<Vec<Notification>>>,
    /// Configuration Firebase (API key)
    fcm_server_key: Option<String>,
    /// Configuration Email SMTP (TODO: intégration lettre crate)
    smtp_config: Option<SmtpConfig>,
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
    pub fn new() -> Self {
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
                port: port.parse().unwrap_or(587),
                username: user,
                password: pass,
                from_email: from,
                to_email: to,
            })
        } else {
            println!("[notifications] WARNING: SMTP config incomplete - email escalation disabled");
            None
        };

        Self {
            fcm_tokens: Arc::new(Mutex::new(HashMap::new())),
            active_notifications: Arc::new(Mutex::new(HashMap::new())),
            history: Arc::new(Mutex::new(Vec::new())),
            fcm_server_key,
            smtp_config,
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
    pub async fn send(&self, mut notif: Notification) -> Result<(), Box<dyn std::error::Error>> {
        // Assign ID si pas déjà défini
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
            // Limiter historique à 1000 dernières notifications
            let history_len = history.len();
            if history_len > 1000 {
                history.drain(0..(history_len - 1000));
            }
        }

        println!("[notifications] sending notification: {} (priority: {:?})", notif.title, notif.priority);

        // Envoyer via FCM
        if let Err(e) = self.send_fcm(&notif).await {
            eprintln!("[notifications] FCM send failed: {}", e);
        }

        // Retry logic selon priorité
        match notif.priority {
            NotificationPriority::P0 => {
                // P0: retry après 5min si pas acknowledged + email escalation
                let notif_id = notif.id.clone();
                let self_clone = self.clone_arc();
                tokio::spawn(async move {
                    sleep(Duration::from_secs(300)).await;
                    if !self_clone.is_acknowledged(&notif_id) {
                        println!("[notifications] P0 retry after 5min: {}", notif_id);
                        let _ = self_clone.send_fcm(&notif).await;
                        let _ = self_clone.send_email(&notif).await;
                    }
                });
            }
            NotificationPriority::P1 => {
                // P1: retry après 15min une fois
                let notif_id = notif.id.clone();
                let self_clone = self.clone_arc();
                tokio::spawn(async move {
                    sleep(Duration::from_secs(900)).await;
                    if !self_clone.is_acknowledged(&notif_id) {
                        println!("[notifications] P1 retry after 15min: {}", notif_id);
                        let _ = self_clone.send_fcm(&notif).await;
                    }
                });
            }
            NotificationPriority::P2 => {
                // P2: best effort, pas de retry
            }
        }

        Ok(())
    }

    /// Helper pour cloner Arc<Self>
    fn clone_arc(&self) -> Arc<Self> {
        Arc::new(Self {
            fcm_tokens: self.fcm_tokens.clone(),
            active_notifications: self.active_notifications.clone(),
            history: self.history.clone(),
            fcm_server_key: self.fcm_server_key.clone(),
            smtp_config: self.smtp_config.clone(),
        })
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

    /// Envoie notification via Email (escalation P0)
    async fn send_email(&self, notif: &Notification) -> Result<(), Box<dyn std::error::Error>> {
        let smtp = match &self.smtp_config {
            Some(cfg) => cfg,
            None => {
                println!("[notifications] SMTP not configured, skipping email");
                return Ok(());
            }
        };

        println!("[notifications] sending email escalation for P0: {}", notif.title);

        // TODO: intégration lettre crate pour envoi email SMTP
        // Pour l'instant, log uniquement
        println!("[notifications] EMAIL ESCALATION:");
        println!("  From: {}", smtp.from_email);
        println!("  To: {}", smtp.to_email);
        println!("  Subject: [Symbion P0] {}", notif.title);
        println!("  Body: {}", notif.body);

        Ok(())
    }

    /// Marque une notification comme acquittée
    pub fn acknowledge(&self, notification_id: &str) -> Result<(), Box<dyn std::error::Error>> {
        let mut active = self.active_notifications.lock().unwrap();

        if let Some(notif) = active.get_mut(notification_id) {
            notif.acknowledged = true;
            notif.acknowledged_at = Some(time::OffsetDateTime::now_utc().unix_timestamp());
            println!("[notifications] acknowledged: {}", notification_id);
            Ok(())
        } else {
            Err("Notification not found".into())
        }
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
        Self::new()
    }
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
        let manager = NotificationManager::new();
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
        let manager = NotificationManager::new();
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

        manager.active_notifications.lock().unwrap().insert(notif.id.clone(), notif);

        assert!(!manager.is_acknowledged("test-ack"));
        manager.acknowledge("test-ack").unwrap();
        assert!(manager.is_acknowledged("test-ack"));
    }
}
