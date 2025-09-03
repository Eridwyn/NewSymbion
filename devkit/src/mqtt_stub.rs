/*!
Mock MQTT Client pour développement sans broker

Permet de développer et tester des plugins sans démarrer un broker MQTT réel.
Enregistre tous les messages publics et permet de simuler la réception.
*/

use rumqttc::QoS;
use serde_json::Value;
use std::sync::{Arc, Mutex};
use tokio::sync::mpsc;
use anyhow::Result;

#[derive(Debug, Clone)]
pub struct MockMessage {
    pub topic: String,
    pub payload: Vec<u8>,
    pub qos: QoS,
    pub retain: bool,
}

/// Mock MQTT Client qui simule rumqttc::AsyncClient
#[derive(Clone)]
pub struct MockMqttClient {
    published_messages: Arc<Mutex<Vec<MockMessage>>>,
    subscriptions: Arc<Mutex<Vec<String>>>,
    message_sender: Arc<Mutex<Option<mpsc::UnboundedSender<MockMessage>>>>,
}

impl MockMqttClient {
    pub fn new() -> Self {
        Self {
            published_messages: Arc::new(Mutex::new(Vec::new())),
            subscriptions: Arc::new(Mutex::new(Vec::new())),
            message_sender: Arc::new(Mutex::new(None)),
        }
    }

    /// Configuration d'un channel pour recevoir les messages simulés
    pub fn setup_receiver(&self) -> mpsc::UnboundedReceiver<MockMessage> {
        let (sender, receiver) = mpsc::unbounded_channel();
        *self.message_sender.lock().unwrap() = Some(sender);
        receiver
    }

    /// Simule la publication d'un message (compatible avec AsyncClient)
    pub async fn publish<S, V>(&self, topic: S, qos: QoS, retain: bool, payload: V) -> Result<()>
    where
        S: Into<String>,
        V: Into<Vec<u8>>,
    {
        let message = MockMessage {
            topic: topic.into(),
            payload: payload.into(),
            qos,
            retain,
        };

        // Enregistrer le message
        self.published_messages.lock().unwrap().push(message.clone());

        log::info!("📤 [MOCK] Published to {}: {} bytes", message.topic, message.payload.len());
        Ok(())
    }

    /// Simule l'abonnement à un topic (compatible avec AsyncClient)
    pub async fn subscribe<S: Into<String>>(&self, topic: S, _qos: QoS) -> Result<()> {
        let topic = topic.into();
        self.subscriptions.lock().unwrap().push(topic.clone());
        log::info!("📥 [MOCK] Subscribed to {}", topic);
        Ok(())
    }

    /// Simule la réception d'un message (pour tests)
    pub async fn simulate_incoming<S, V>(&self, topic: S, payload: V) -> Result<()>
    where
        S: Into<String>,
        V: Into<Vec<u8>>,
    {
        let message = MockMessage {
            topic: topic.into(),
            payload: payload.into(),
            qos: QoS::AtLeastOnce,
            retain: false,
        };

        if let Some(sender) = self.message_sender.lock().unwrap().as_ref() {
            sender.send(message.clone()).map_err(|e| anyhow::anyhow!("Send error: {}", e))?;
        }

        log::info!("📨 [MOCK] Simulated incoming: {}", message.topic);
        Ok(())
    }

    /// Récupère tous les messages publiés (pour assertions de tests)
    pub fn get_published_messages(&self) -> Vec<MockMessage> {
        self.published_messages.lock().unwrap().clone()
    }

    /// Récupère les abonnements (pour assertions de tests)
    pub fn get_subscriptions(&self) -> Vec<String> {
        self.subscriptions.lock().unwrap().clone()
    }

    /// Trouve les messages publiés sur un topic donné
    pub fn find_messages_by_topic(&self, topic: &str) -> Vec<MockMessage> {
        self.published_messages
            .lock()
            .unwrap()
            .iter()
            .filter(|msg| msg.topic == topic)
            .cloned()
            .collect()
    }

    /// Parse le dernier message d'un topic en JSON
    pub fn get_last_json_message<T>(&self, topic: &str) -> Result<Option<T>>
    where
        T: for<'de> serde::Deserialize<'de>,
    {
        let messages = self.find_messages_by_topic(topic);
        if let Some(last_msg) = messages.last() {
            let parsed: T = serde_json::from_slice(&last_msg.payload)?;
            Ok(Some(parsed))
        } else {
            Ok(None)
        }
    }

    /// Reset tous les messages enregistrés
    pub fn clear(&self) {
        self.published_messages.lock().unwrap().clear();
        self.subscriptions.lock().unwrap().clear();
    }
}

impl Default for MockMqttClient {
    fn default() -> Self {
        Self::new()
    }
}

/// Helper pour créer des messages de test formatés selon les contrats Symbion
pub struct SymbionMessageBuilder {
    base_topic: String,
}

impl SymbionMessageBuilder {
    pub fn new<S: Into<String>>(service: S) -> Self {
        Self {
            base_topic: format!("symbion/{}", service.into()),
        }
    }

    /// Crée un message heartbeat v2
    pub fn heartbeat_v2<S: Into<String>>(host_id: S, cpu: f32, ram: f32, ip: S) -> Value {
        serde_json::json!({
            "host_id": host_id.into(),
            "ts": chrono::Utc::now().to_rfc3339(),
            "metrics": {
                "cpu": cpu,
                "ram": ram
            },
            "net": {
                "ip": ip.into()
            }
        })
    }

    /// Crée un message wake v1
    pub fn wake_v1<S: Into<String>>(host_id: S, mac: S, broadcast: S) -> Value {
        serde_json::json!({
            "host_id": host_id.into(),
            "mac": mac.into(),
            "broadcast": broadcast.into()
        })
    }

    /// Crée un message notes.command v1
    pub fn notes_command_v1<S: Into<String>>(action: S, data: Value) -> Value {
        serde_json::json!({
            "action": action.into(),
            "data": data,
            "timestamp": chrono::Utc::now().to_rfc3339()
        })
    }

    /// Crée un message kernel.health v1
    pub fn kernel_health_v1(uptime: u64, memory_mb: u64, mqtt_connected: bool) -> Value {
        serde_json::json!({
            "uptime_seconds": uptime,
            "memory_mb": memory_mb,
            "mqtt_connected": mqtt_connected,
            "timestamp": chrono::Utc::now().to_rfc3339()
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tokio;

    #[tokio::test]
    async fn test_mock_client_publish_subscribe() {
        let client = MockMqttClient::new();
        
        // Test abonnement
        client.subscribe("test/topic", QoS::AtLeastOnce).await.unwrap();
        assert_eq!(client.get_subscriptions(), vec!["test/topic"]);

        // Test publication
        let payload = b"test message";
        client.publish("test/topic", QoS::AtLeastOnce, false, payload.to_vec()).await.unwrap();

        // Vérifier le message publié
        let messages = client.get_published_messages();
        assert_eq!(messages.len(), 1);
        assert_eq!(messages[0].topic, "test/topic");
        assert_eq!(messages[0].payload, payload);
    }

    #[tokio::test]
    async fn test_json_message_parsing() {
        let client = MockMqttClient::new();
        
        let test_data = serde_json::json!({
            "test_field": "test_value",
            "number": 42
        });
        
        let payload = serde_json::to_vec(&test_data).unwrap();
        client.publish("json/topic", QoS::AtLeastOnce, false, payload).await.unwrap();

        // Parse du JSON
        let parsed: Option<serde_json::Value> = client.get_last_json_message("json/topic").unwrap();
        assert!(parsed.is_some());
        assert_eq!(parsed.unwrap()["test_field"], "test_value");
    }

    #[test]
    fn test_message_builders() {
        let heartbeat = SymbionMessageBuilder::heartbeat_v2("host1", 25.5, 60.0, "192.168.1.10");
        assert_eq!(heartbeat["host_id"], "host1");
        assert_eq!(heartbeat["metrics"]["cpu"], 25.5);

        let wake = SymbionMessageBuilder::wake_v1("host2", "aa:bb:cc:dd:ee:ff", "192.168.1.255");
        assert_eq!(wake["host_id"], "host2");
        assert_eq!(wake["mac"], "aa:bb:cc:dd:ee:ff");
    }
}