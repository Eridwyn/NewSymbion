/**
 * NOTIFICATION CLIENT - Interface sécurisée pour envoyer des notifications
 *
 * RÔLE : Permet aux modules du kernel d'envoyer des notifications
 *        SANS dépendre du plugin notifications (optionnel/désactivable)
 *
 * ARCHITECTURE : Vérifie si le plugin est dispo avant d'envoyer via MQTT
 * UTILITÉ : Découplage complet - le kernel fonctionne même sans le plugin
 */

use crate::plugin_proxy::PluginRegistry;
use rumqttc::AsyncClient;
use serde::{Deserialize, Serialize};
use std::sync::Arc;

/// Priorités des notifications
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum NotificationPriority {
    /// Critical - Immédiat + retry 5min si pas vu + email escalation
    P0,
    /// High - Retry 15min une fois
    P1,
    /// Normal - Best effort, fire-and-forget
    P2,
}

impl std::fmt::Display for NotificationPriority {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            NotificationPriority::P0 => write!(f, "P0"),
            NotificationPriority::P1 => write!(f, "P1"),
            NotificationPriority::P2 => write!(f, "P2"),
        }
    }
}

/// Payload de notification à envoyer
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NotificationPayload {
    /// ID unique de la notification (généré automatiquement)
    pub id: String,
    pub priority: NotificationPriority,
    pub title: String,
    pub body: String,
    pub source: String,
    pub timestamp: i64,
    #[serde(default)]
    pub acknowledged: bool,
    #[serde(default)]
    pub actions: Vec<NotificationAction>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NotificationAction {
    pub id: String,
    pub label: String,
    pub action_type: String,
}

impl NotificationPayload {
    /// Créer une notification simple
    pub fn new(priority: NotificationPriority, title: impl Into<String>, body: impl Into<String>, source: impl Into<String>) -> Self {
        let now = time::OffsetDateTime::now_utc();
        let timestamp = now.unix_timestamp();
        let source_str = source.into();
        // Générer un ID unique : source-timestamp-nanos
        let nanos = now.nanosecond();
        let id = format!("{}-{}-{}", source_str, timestamp, nanos);
        Self {
            id,
            priority,
            title: title.into(),
            body: body.into(),
            source: source_str,
            timestamp,
            acknowledged: false,
            actions: vec![],
        }
    }

    /// Ajouter une action à la notification
    pub fn with_action(mut self, id: impl Into<String>, label: impl Into<String>, action_type: impl Into<String>) -> Self {
        self.actions.push(NotificationAction {
            id: id.into(),
            label: label.into(),
            action_type: action_type.into(),
        });
        self
    }
}

/// Client de notification sécurisé
/// Vérifie si le plugin est disponible avant d'envoyer
#[derive(Clone)]
pub struct NotificationClient {
    mqtt_client: AsyncClient,
    plugin_registry: PluginRegistry,
}

impl NotificationClient {
    pub fn new(mqtt_client: AsyncClient, plugin_registry: PluginRegistry) -> Self {
        Self {
            mqtt_client,
            plugin_registry,
        }
    }

    /// Vérifie si le plugin notifications est disponible
    pub async fn is_available(&self) -> bool {
        let plugins = self.plugin_registry.list_plugins().await;
        plugins.iter().any(|p| {
            p.name == "notifications" && p.socket_path.exists()
        })
    }

    /// Envoie une notification si le plugin est disponible
    /// Retourne Ok(true) si envoyé, Ok(false) si plugin indisponible, Err si erreur
    pub async fn send(&self, notification: NotificationPayload) -> Result<bool, String> {
        // Vérifier si le plugin est dispo
        if !self.is_available().await {
            println!("[notification-client] Plugin notifications indisponible, notification ignorée: {}", notification.title);
            return Ok(false);
        }

        // Sérialiser et envoyer via MQTT
        let payload = serde_json::to_string(&notification)
            .map_err(|e| format!("Erreur sérialisation: {}", e))?;

        self.mqtt_client
            .publish(
                "symbion/notifications/send@v1",
                rumqttc::QoS::AtLeastOnce,
                false,
                payload,
            )
            .await
            .map_err(|e| format!("Erreur MQTT: {}", e))?;

        println!(
            "[notification-client] Notification envoyée: {} ({})",
            notification.title, notification.priority
        );

        Ok(true)
    }

    /// Envoie une notification P0 (critique)
    pub async fn send_critical(&self, title: impl Into<String>, body: impl Into<String>, source: impl Into<String>) -> Result<bool, String> {
        self.send(NotificationPayload::new(NotificationPriority::P0, title, body, source)).await
    }

    /// Envoie une notification P1 (importante)
    pub async fn send_important(&self, title: impl Into<String>, body: impl Into<String>, source: impl Into<String>) -> Result<bool, String> {
        self.send(NotificationPayload::new(NotificationPriority::P1, title, body, source)).await
    }

    /// Envoie une notification P2 (info)
    pub async fn send_info(&self, title: impl Into<String>, body: impl Into<String>, source: impl Into<String>) -> Result<bool, String> {
        self.send(NotificationPayload::new(NotificationPriority::P2, title, body, source)).await
    }
}

/// Helper pour créer des notifications depuis n'importe quel module
/// Usage:
/// ```
/// notify!(client, P0, "Alerte critique", "Détails...", "module-name");
/// notify!(client, P1, "Important", "Message", "source");
/// ```
#[macro_export]
macro_rules! notify {
    ($client:expr, P0, $title:expr, $body:expr, $source:expr) => {
        $client.send_critical($title, $body, $source).await
    };
    ($client:expr, P1, $title:expr, $body:expr, $source:expr) => {
        $client.send_important($title, $body, $source).await
    };
    ($client:expr, P2, $title:expr, $body:expr, $source:expr) => {
        $client.send_info($title, $body, $source).await
    };
}
