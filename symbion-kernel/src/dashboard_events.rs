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
        self.publish_with_retain(topic, payload, false).await
    }

    /// Publie un événement sur MQTT avec option retain
    async fn publish_with_retain<T: Serialize>(&self, topic: &str, payload: &T, retain: bool) -> Result<(), String> {
        let json = serde_json::to_string(payload)
            .map_err(|e| format!("Failed to serialize payload: {}", e))?;

        self.mqtt_client
            .publish(topic, QoS::AtLeastOnce, retain, json.as_bytes())
            .await
            .map_err(|e| format!("Failed to publish to MQTT: {}", e))?;

        println!("[dashboard-events] Published to {} (retain={})", topic, retain);
        Ok(())
    }

    /// Événement: Changement de contexte (avec retain=true pour que les nouveaux clients reçoivent l'état actuel)
    pub async fn publish_context_change(&self, context: &crate::context::ContextState) -> Result<(), String> {
        self.publish_with_retain("symbion/dashboard/context@v1", context, true).await
    }

    /// Événement: Changement état agents
    /// Publie chaque agent sur son topic individuel pour réduire la taille des messages
    pub async fn publish_agents_update(&self, agents: &Vec<crate::agents::Agent>) -> Result<(), String> {
        // NOUVEAU: Publication individuelle par agent
        for agent in agents {
            let topic = format!("symbion/dashboard/agents/{}@v1", agent.agent_id);
            if let Err(e) = self.publish(&topic, agent).await {
                eprintln!("[dashboard-events] Failed to publish agent {}: {}", agent.agent_id, e);
            }
        }
        Ok(())

        // ANCIEN SYSTÈME (commenté - tous les agents dans un seul message)
        // self.publish("symbion/dashboard/agents@v1", agents).await
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

    // Note: publish_pattern_detected removed - patterns now managed by Intelligence Engine
}
