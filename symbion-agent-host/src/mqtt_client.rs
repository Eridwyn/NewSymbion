//! MQTT client wrapper for Symbion Agent Host
//!
//! Manages MQTT connection, event loop, reconnection with exponential backoff,
//! and command forwarding to the agent main loop.

use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Duration;

use anyhow::{Result, Context};
use rumqttc::{AsyncClient, Event, EventLoop, Incoming, MqttOptions, QoS};
use tokio::sync::mpsc;
use tracing::{info, error};

use tracing::warn;

use crate::messages::ReceivedCommand;

/// Maximum allowed command payload size (1 MB)
const MAX_COMMAND_PAYLOAD: usize = 1_048_576;

/// MQTT topic constants
pub const TOPIC_REGISTRATION: &str = "symbion/agents/registration@v1";
pub const TOPIC_HEARTBEAT: &str = "symbion/agents/heartbeat@v1";
pub const TOPIC_COMMAND: &str = "symbion/agents/command@v1";
pub const TOPIC_RESPONSE: &str = "symbion/agents/response@v1";
pub const TOPIC_LOGS: &str = "symbion/agents/logs@v1";

/// MQTT connection configuration
#[derive(Debug, Clone)]
pub struct MqttConfig {
    pub broker_host: String,
    pub broker_port: u16,
    pub client_id: String,
    pub keep_alive_secs: u64,
}

/// Result of creating an MQTT client: the client + a command receiver channel
pub struct MqttClientHandle {
    pub client: AsyncClient,
    pub command_rx: mpsc::Receiver<ReceivedCommand>,
    pub connected: Arc<AtomicBool>,
}

/// Create MQTT client and spawn background event loop.
///
/// Returns the client handle for publishing and a receiver for incoming commands.
/// The event loop runs in a background tokio task with automatic reconnection
/// and exponential backoff (2s → 32s).
pub fn create_and_spawn(config: &MqttConfig) -> MqttClientHandle {
    let mut mqtt_options = MqttOptions::new(
        &config.client_id,
        &config.broker_host,
        config.broker_port,
    );
    mqtt_options.set_keep_alive(Duration::from_secs(config.keep_alive_secs));
    mqtt_options.set_clean_session(true);
    mqtt_options.set_max_packet_size(5 * 1024 * 1024, 5 * 1024 * 1024); // 5MB for screenshot base64

    let (client, eventloop) = AsyncClient::new(mqtt_options, 10);
    let (command_tx, command_rx) = mpsc::channel::<ReceivedCommand>(100);

    let connected = Arc::new(AtomicBool::new(false));
    let client_for_loop = client.clone();
    let connected_clone = connected.clone();

    // Spawn background event loop
    tokio::spawn(run_event_loop(eventloop, client_for_loop, command_tx, connected_clone));

    MqttClientHandle {
        client,
        command_rx,
        connected,
    }
}

/// Background MQTT event loop with auto-reconnection and exponential backoff.
async fn run_event_loop(
    mut eventloop: EventLoop,
    client: AsyncClient,
    command_tx: mpsc::Sender<ReceivedCommand>,
    connected: Arc<AtomicBool>,
) {
    let mut retry_count: u32 = 0;

    loop {
        match eventloop.poll().await {
            Ok(Event::Incoming(Incoming::ConnAck(_))) => {
                retry_count = 0;
                connected.store(true, Ordering::Relaxed);
                info!("MQTT connected/reconnected — subscribing to command topic");
                if let Err(e) = client.subscribe(TOPIC_COMMAND, QoS::AtLeastOnce).await {
                    error!("Failed to subscribe to command topic: {}", e);
                } else {
                    info!("Subscribed to {}", TOPIC_COMMAND);
                }
            }
            Ok(Event::Incoming(Incoming::Publish(publish))) => {
                if publish.topic == TOPIC_COMMAND {
                    if publish.payload.len() > MAX_COMMAND_PAYLOAD {
                        warn!("Command payload too large: {} bytes (max {}), dropping",
                            publish.payload.len(), MAX_COMMAND_PAYLOAD);
                        continue;
                    }
                    let payload = String::from_utf8_lossy(&publish.payload).to_string();
                    let command = ReceivedCommand {
                        topic: publish.topic.clone(),
                        payload,
                    };
                    if let Err(e) = command_tx.send(command).await {
                        error!("Failed to forward command to agent: {}", e);
                    }
                }
            }
            Ok(_) => {}
            Err(e) => {
                connected.store(false, Ordering::Relaxed);
                retry_count = retry_count.saturating_add(1);
                let delay = backoff_secs(retry_count);
                error!("MQTT error (retry #{}, backoff {}s): {}", retry_count, delay, e);
                tokio::time::sleep(Duration::from_secs(delay)).await;
            }
        }
    }
}

/// Calculate exponential backoff delay in seconds for a given retry count.
/// Capped at 32 seconds (2^5).
pub fn backoff_secs(retry_count: u32) -> u64 {
    2u64.saturating_pow(retry_count.min(5))
}

/// Publish a JSON-serialized message to an MQTT topic.
pub async fn publish_json<T: serde::Serialize>(
    client: &AsyncClient,
    topic: &str,
    payload: &T,
) -> Result<()> {
    let json = serde_json::to_string(payload)
        .context("Failed to serialize MQTT message")?;
    client
        .publish(topic, QoS::AtLeastOnce, false, json)
        .await
        .with_context(|| format!("Failed to publish to {}", topic))?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_backoff_exponential() {
        assert_eq!(backoff_secs(1), 2);
        assert_eq!(backoff_secs(2), 4);
        assert_eq!(backoff_secs(3), 8);
        assert_eq!(backoff_secs(4), 16);
        assert_eq!(backoff_secs(5), 32);
    }

    #[test]
    fn test_backoff_capped_at_32() {
        assert_eq!(backoff_secs(6), 32);
        assert_eq!(backoff_secs(10), 32);
        assert_eq!(backoff_secs(100), 32);
    }

    #[test]
    fn test_topic_constants() {
        assert!(TOPIC_REGISTRATION.starts_with("symbion/agents/"));
        assert!(TOPIC_HEARTBEAT.starts_with("symbion/agents/"));
        assert!(TOPIC_COMMAND.starts_with("symbion/agents/"));
        assert!(TOPIC_RESPONSE.starts_with("symbion/agents/"));
        assert!(TOPIC_LOGS.starts_with("symbion/agents/"));
        // All should have @v1 suffix
        assert!(TOPIC_REGISTRATION.ends_with("@v1"));
        assert!(TOPIC_HEARTBEAT.ends_with("@v1"));
        assert!(TOPIC_LOGS.ends_with("@v1"));
    }

    #[test]
    fn test_max_payload_constant() {
        assert_eq!(MAX_COMMAND_PAYLOAD, 1_048_576); // 1 MB
    }

    #[test]
    fn test_mqtt_config_debug() {
        let config = MqttConfig {
            broker_host: "localhost".to_string(),
            broker_port: 1883,
            client_id: "test-agent".to_string(),
            keep_alive_secs: 30,
        };
        let debug = format!("{:?}", config);
        assert!(debug.contains("localhost"));
        assert!(debug.contains("1883"));
    }
}
