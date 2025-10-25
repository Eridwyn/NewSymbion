/**
 * Dashboard Events Publisher
 *
 * Publie les événements système sur MQTT pour le dashboard en temps réel
 * Remplace le polling HTTP par des événements push
 */

use rumqttc::{AsyncClient, QoS};
use serde::Serialize;
use std::sync::Arc;

/// Publisher d'événements dashboard centralisé
#[derive(Clone)]
pub struct DashboardEventPublisher {
    mqtt_client: Arc<AsyncClient>,
}

impl DashboardEventPublisher {
    pub fn new(mqtt_client: AsyncClient) -> Self {
        Self {
            mqtt_client: Arc::new(mqtt_client),
        }
    }

    /// Publie un événement générique sur MQTT
    async fn publish<T: Serialize>(&self, topic: &str, payload: &T) -> Result<(), String> {
        let json = serde_json::to_string(payload)
            .map_err(|e| format!("Failed to serialize payload: {}", e))?;

        self.mqtt_client
            .publish(topic, QoS::AtLeastOnce, false, json.as_bytes())
            .await
            .map_err(|e| format!("Failed to publish to MQTT: {}", e))?;

        println!("[dashboard-events] Published to {}", topic);
        Ok(())
    }

    /// Événement: Changement de contexte
    pub async fn publish_context_change(&self, context: &crate::context::ContextState) -> Result<(), String> {
        self.publish("symbion/dashboard/context@v1", context).await
    }

    /// Événement: Changement état agents
    pub async fn publish_agents_update(&self, agents: &Vec<crate::agents::Agent>) -> Result<(), String> {
        self.publish("symbion/dashboard/agents@v1", agents).await
    }

    /// Événement: Health système
    pub async fn publish_system_health(&self, health: &crate::health::KernelHealth) -> Result<(), String> {
        self.publish("symbion/dashboard/health@v1", health).await
    }

    /// Événement: Nouvelle note créée
    pub async fn publish_note_created(&self, note_id: &str) -> Result<(), String> {
        #[derive(Serialize)]
        struct NoteEvent {
            note_id: String,
            timestamp: String,
        }

        let event = NoteEvent {
            note_id: note_id.to_string(),
            timestamp: time::OffsetDateTime::now_utc().to_string(),
        };

        self.publish("symbion/dashboard/notes@v1", &event).await
    }

    /// Événement: Statistiques contextuelles mises à jour
    pub async fn publish_stats_update(&self, stats: &Vec<crate::context::ModeStats>) -> Result<(), String> {
        self.publish("symbion/dashboard/stats@v1", stats).await
    }

    /// Événement: Nouveau pattern détecté
    pub async fn publish_pattern_detected(&self, pattern: &crate::context::DetectedPattern) -> Result<(), String> {
        self.publish("symbion/dashboard/pattern@v1", pattern).await
    }
}
