//! MQTT transport abstraction for testability
//!
//! Provides a `MqttTransport` trait that decouples the agent from
//! the concrete rumqttc `AsyncClient`, enabling mock transports in tests.

use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};

use anyhow::{Result, Context};
use async_trait::async_trait;
use rumqttc::{AsyncClient, QoS};
use serde::Serialize;

/// Abstract MQTT transport for publishing messages
#[allow(dead_code)] // Trait available for future agent.rs migration
#[async_trait]
pub trait MqttTransport: Send + Sync {
    /// Publish a JSON-serialized payload to the given topic
    async fn publish_json<T: Serialize + Send + Sync>(&self, topic: &str, payload: &T) -> Result<()>;

    /// Check if the transport is currently connected
    fn is_connected(&self) -> bool;
}

/// Production transport wrapping rumqttc AsyncClient
#[allow(dead_code)] // Available for future agent.rs migration
pub struct RumqttcTransport {
    client: AsyncClient,
    connected: Arc<AtomicBool>,
}

impl RumqttcTransport {
    #[allow(dead_code)] // Available for future agent.rs migration
    pub fn new(client: AsyncClient, connected: Arc<AtomicBool>) -> Self {
        Self { client, connected }
    }
}

#[async_trait]
impl MqttTransport for RumqttcTransport {
    async fn publish_json<T: Serialize + Send + Sync>(&self, topic: &str, payload: &T) -> Result<()> {
        let json = serde_json::to_string(payload)
            .context("Failed to serialize MQTT message")?;
        self.client
            .publish(topic, QoS::AtLeastOnce, false, json)
            .await
            .with_context(|| format!("Failed to publish to {}", topic))?;
        Ok(())
    }

    fn is_connected(&self) -> bool {
        self.connected.load(Ordering::Relaxed)
    }
}

#[cfg(test)]
pub mod mock {
    use super::*;
    use std::sync::Mutex;

    /// Mock transport that records published messages for test assertions
    pub struct MockTransport {
        connected: AtomicBool,
        published: Mutex<Vec<(String, String)>>, // (topic, json_payload)
    }

    impl MockTransport {
        pub fn new() -> Self {
            Self {
                connected: AtomicBool::new(true),
                published: Mutex::new(Vec::new()),
            }
        }

        pub fn set_connected(&self, connected: bool) {
            self.connected.store(connected, Ordering::Relaxed);
        }

        pub fn published_messages(&self) -> Vec<(String, String)> {
            self.published.lock().unwrap().clone()
        }
    }

    #[async_trait]
    impl MqttTransport for MockTransport {
        async fn publish_json<T: Serialize + Send + Sync>(&self, topic: &str, payload: &T) -> Result<()> {
            let json = serde_json::to_string(payload)?;
            self.published.lock().unwrap().push((topic.to_string(), json));
            Ok(())
        }

        fn is_connected(&self) -> bool {
            self.connected.load(Ordering::Relaxed)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use mock::MockTransport;

    #[tokio::test]
    async fn test_mock_transport_publish() {
        let transport = MockTransport::new();
        assert!(transport.is_connected());

        transport.publish_json("test/topic", &serde_json::json!({"key": "value"}))
            .await
            .unwrap();

        let messages = transport.published_messages();
        assert_eq!(messages.len(), 1);
        assert_eq!(messages[0].0, "test/topic");
        assert!(messages[0].1.contains("key"));
    }

    #[test]
    fn test_mock_transport_connection_state() {
        let transport = MockTransport::new();
        assert!(transport.is_connected());

        transport.set_connected(false);
        assert!(!transport.is_connected());

        transport.set_connected(true);
        assert!(transport.is_connected());
    }
}
