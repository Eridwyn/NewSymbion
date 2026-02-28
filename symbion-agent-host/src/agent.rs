//! Symbion Agent core logic
//!
//! Contains the Agent struct, event loop, and command dispatch.
//! Command execution is delegated to the `execution` module (no duplication).

use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Duration;

use anyhow::{Result, Context};
use chrono::Utc;
use rumqttc::AsyncClient;
use tokio::sync::mpsc;
use tokio::time::interval;
use tracing::{info, error, debug, warn};

use crate::capabilities;
use crate::discovery::SystemInfo;
use crate::execution::handler::CommandRegistry;
use crate::execution::handlers;
use crate::local_api;
use crate::messages::*;
use crate::metrics;
use crate::mqtt_client;
use crate::system_tray;

/// Runtime MQTT configuration (derived from user config)
#[derive(Debug, Clone)]
pub struct AgentConfig {
    pub mqtt_broker: String,
    pub mqtt_port: u16,
    pub mqtt_client_id: String,
    pub heartbeat_interval_secs: u64,
}

impl Default for AgentConfig {
    fn default() -> Self {
        Self {
            mqtt_broker: "localhost".to_string(),
            mqtt_port: 1883,
            mqtt_client_id: "symbion-agent-unknown".to_string(),
            heartbeat_interval_secs: 30,
        }
    }
}

/// Main agent state
pub struct Agent {
    config: AgentConfig,
    system_info: SystemInfo,
    mqtt_client: AsyncClient,
    last_command: Option<CommandInfo>,
    command_receiver: mpsc::Receiver<ReceivedCommand>,
    command_registry: CommandRegistry,
    local_api: Option<Arc<local_api::LocalApiServer>>,
    system_tray: Option<system_tray::SystemTray>,
    mqtt_connected: Arc<AtomicBool>,
    reconnect_rx: Option<mpsc::Receiver<()>>,
}

impl Agent {
    /// Create new agent instance with loaded configuration
    pub async fn new_with_config(agent_config: crate::config::AgentConfig) -> Result<Self> {
        info!("Initializing Symbion Agent Host v{}", env!("CARGO_PKG_VERSION"));

        let system_info = SystemInfo::discover().await
            .context("Failed to discover system information")?;

        // Build MQTT config from user config
        let config = AgentConfig {
            mqtt_broker: agent_config.mqtt.broker_host.clone(),
            mqtt_port: agent_config.mqtt.broker_port,
            mqtt_client_id: agent_config.mqtt.client_id
                .unwrap_or_else(|| format!("symbion-agent-{}", system_info.agent_id)),
            heartbeat_interval_secs: 30,
        };

        // Create MQTT client with background event loop
        let mqtt_handle = mqtt_client::create_and_spawn(&mqtt_client::MqttConfig {
            broker_host: config.mqtt_broker.clone(),
            broker_port: config.mqtt_port,
            client_id: config.mqtt_client_id.clone(),
            keep_alive_secs: 30,
        });

        // Build command registry with all standard handlers
        let command_registry = handlers::build_default_registry();

        info!("Agent initialized — ID: {}, Hostname: {}, commands: {:?}",
              system_info.agent_id, system_info.hostname, command_registry.command_types());

        Ok(Agent {
            config,
            system_info,
            mqtt_client: mqtt_handle.client,
            last_command: None,
            command_receiver: mqtt_handle.command_rx,
            command_registry,
            local_api: None,
            system_tray: None,
            mqtt_connected: mqtt_handle.connected,
            reconnect_rx: None,
        })
    }

    /// Set local API server for status updates
    pub fn set_local_api(&mut self, local_api: Arc<local_api::LocalApiServer>) {
        self.local_api = Some(local_api);
    }

    /// Set reconnect receiver (from local API /reconnect endpoint)
    pub fn set_reconnect_rx(&mut self, rx: mpsc::Receiver<()>) {
        self.reconnect_rx = Some(rx);
    }

    /// Initialize system tray (optional)
    pub fn init_system_tray(&mut self) -> Result<()> {
        let mut tray = system_tray::SystemTray::new();
        if let Err(e) = tray.initialize(&self.system_info.agent_id, &self.system_info.hostname) {
            warn!("Failed to initialize system tray: {}", e);
            warn!("Agent will continue without system tray — use http://localhost:9899 for dashboard");
        } else {
            info!("System tray initialized");
            self.system_tray = Some(tray);
        }
        Ok(())
    }

    /// Start agent main loop with graceful shutdown on SIGTERM/SIGINT
    pub async fn run(&mut self) -> Result<()> {
        info!("Starting agent main loop...");

        // Initial registration
        self.register().await?;

        let mut heartbeat_timer = interval(Duration::from_secs(self.config.heartbeat_interval_secs));
        let mut registration_timer = interval(Duration::from_secs(300));
        let mut reconnect_rx = self.reconnect_rx.take();

        // Graceful shutdown signal
        let mut shutdown = std::pin::pin!(shutdown_signal());

        loop {
            tokio::select! {
                _ = &mut shutdown => {
                    info!("Shutdown signal received, stopping agent gracefully...");
                    self.send_offline_heartbeat().await;
                    info!("Agent stopped.");
                    break Ok(());
                }

                _ = heartbeat_timer.tick() => {
                    if let Err(e) = self.send_heartbeat().await {
                        error!("Failed to send heartbeat: {}", e);
                    }
                    let connected = self.mqtt_connected.load(Ordering::Relaxed);
                    self.update_local_api_status(connected).await;
                }

                _ = registration_timer.tick() => {
                    if let Err(e) = self.register().await {
                        error!("Failed to re-register: {}", e);
                    }
                }

                command = self.command_receiver.recv() => {
                    match command {
                        Some(cmd) => {
                            if let Err(e) = self.process_command(cmd).await {
                                error!("Failed to process command: {}", e);
                            }
                        }
                        None => {
                            warn!("Command channel closed");
                            self.send_offline_heartbeat().await;
                            break Ok(());
                        }
                    }
                }

                _ = async {
                    if let Some(ref mut rx) = reconnect_rx {
                        rx.recv().await
                    } else {
                        std::future::pending::<Option<()>>().await
                    }
                } => {
                    info!("Reconnect signal received, forcing re-registration...");
                    if let Err(e) = self.register().await {
                        error!("Failed to re-register after reconnect signal: {}", e);
                    }
                }
            }
        }
    }

    /// Send a final heartbeat with status=offline before shutting down
    async fn send_offline_heartbeat(&self) {
        let heartbeat = HeartbeatMessage {
            agent_id: self.system_info.agent_id.clone(),
            status: "offline".to_string(),
            system: match metrics::SystemMetrics::collect().await {
                Ok(m) => m,
                Err(_) => return,
            },
            processes: None,
            services: None,
            last_command: self.last_command.clone(),
            timestamp: Utc::now(),
        };

        if let Err(e) = mqtt_client::publish_json(
            &self.mqtt_client, mqtt_client::TOPIC_HEARTBEAT, &heartbeat
        ).await {
            warn!("Failed to send offline heartbeat: {}", e);
        } else {
            info!("Offline heartbeat sent");
        }
    }

    // ========================================================================
    // MQTT Publishing
    // ========================================================================

    /// Register agent with kernel
    async fn register(&self) -> Result<()> {
        let capabilities = self.get_capabilities().await;

        let registration = RegistrationMessage {
            agent_id: self.system_info.agent_id.clone(),
            hostname: self.system_info.hostname.clone(),
            os: self.system_info.os.clone(),
            architecture: self.system_info.architecture.clone(),
            capabilities,
            network: self.system_info.network.clone(),
            version: env!("CARGO_PKG_VERSION").to_string(),
            timestamp: Utc::now(),
        };

        mqtt_client::publish_json(&self.mqtt_client, mqtt_client::TOPIC_REGISTRATION, &registration).await?;
        info!("Agent registered successfully");
        Ok(())
    }

    /// Send heartbeat with system metrics
    async fn send_heartbeat(&self) -> Result<()> {
        let system_metrics = metrics::SystemMetrics::collect().await
            .context("Failed to collect system metrics")?;
        let process_info = metrics::ProcessInfo::collect().await.ok();
        let services = metrics::ServiceStatus::collect_critical().await.ok();

        let heartbeat = HeartbeatMessage {
            agent_id: self.system_info.agent_id.clone(),
            status: "online".to_string(),
            system: system_metrics,
            processes: process_info,
            services,
            last_command: self.last_command.clone(),
            timestamp: Utc::now(),
        };

        mqtt_client::publish_json(&self.mqtt_client, mqtt_client::TOPIC_HEARTBEAT, &heartbeat).await?;
        debug!("Heartbeat sent");
        Ok(())
    }

    /// Get agent capabilities
    async fn get_capabilities(&self) -> Vec<String> {
        capabilities::CapabilityDetector::get_available_capabilities().await
    }

    // ========================================================================
    // Command Processing — delegates to CommandRegistry
    // ========================================================================

    /// Process incoming command from MQTT
    async fn process_command(&mut self, cmd: ReceivedCommand) -> Result<()> {
        let start_time = std::time::Instant::now();

        let incoming: IncomingCommand = serde_json::from_str(&cmd.payload)
            .context("Failed to parse incoming command")?;

        // Only process commands for this agent
        if incoming.agent_id != self.system_info.agent_id {
            debug!("Ignoring command {} for agent {}", incoming.command_id, incoming.agent_id);
            return Ok(());
        }

        info!("Executing command: {} ({})", incoming.command_type, incoming.command_id);

        // Dispatch via registry (with special-case for reconnect)
        let result = if incoming.command_type == "reconnect" {
            crate::execution::handler::CommandResult::success(serde_json::json!({
                "message": "Reconnect acknowledged — agent will re-register on next heartbeat"
            }))
        } else {
            match self.command_registry.execute(&incoming.command_type, incoming.parameters.as_ref()).await {
                Some(result) => result,
                None => crate::execution::handler::CommandResult::error(
                    "UNKNOWN_COMMAND",
                    format!("Unknown command type: {}", incoming.command_type),
                ),
            }
        };

        // Update last command info
        self.last_command = Some(CommandInfo {
            command_id: incoming.command_id.clone(),
            command_type: incoming.command_type.clone(),
            status: result.status.clone(),
            timestamp: Utc::now(),
        });

        // Build and send response
        let response = CommandResponse {
            command_id: incoming.command_id,
            agent_id: self.system_info.agent_id.clone(),
            status: result.status,
            output: result.data,
            error: result.error,
            execution_time_ms: start_time.elapsed().as_millis(),
            timestamp: Utc::now(),
        };

        self.send_response(response).await
    }

    // ========================================================================
    // Response sending with output truncation
    // ========================================================================

    /// Send command response, truncating large outputs for MQTT transport
    async fn send_response(&self, mut response: CommandResponse) -> Result<()> {
        truncate_output(&mut response);
        mqtt_client::publish_json(&self.mqtt_client, mqtt_client::TOPIC_RESPONSE, &response).await
    }

    // ========================================================================
    // Local API status update
    // ========================================================================

    async fn update_local_api_status(&self, mqtt_connected: bool) {
        if let Some(ref api) = self.local_api {
            let system_status = match metrics::SystemMetrics::collect().await {
                Ok(m) => {
                    let process_count = metrics::ProcessInfo::collect().await
                        .map(|p| p.total_count as u32)
                        .unwrap_or(0);
                    Some(local_api::SystemStatus {
                        cpu_percent: m.cpu.percent as f64,
                        memory_used_mb: m.memory.used_mb,
                        memory_total_mb: m.memory.total_mb,
                        process_count,
                        load_average: Some(m.cpu.load_avg[0]),
                    })
                }
                Err(_) => None,
            };
            api.update_status(mqtt_connected, system_status).await;
        }
    }
}

/// Maximum output size for MQTT transport (bytes)
const MAX_OUTPUT_SIZE: usize = 7000;

/// Truncate large string outputs in a CommandResponse for MQTT transport
fn truncate_output(response: &mut CommandResponse) {
    if let Some(serde_json::Value::String(ref output_str)) = response.output {
        if output_str.len() > MAX_OUTPUT_SIZE {
            let mut truncated: String = output_str.chars().take(MAX_OUTPUT_SIZE).collect();
            truncated.push_str("\n\n[OUTPUT TRUNCATED]");
            response.output = Some(serde_json::Value::String(truncated));
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Utc;

    fn make_response(output: Option<serde_json::Value>) -> CommandResponse {
        CommandResponse {
            command_id: "test".to_string(),
            agent_id: "agent".to_string(),
            status: "success".to_string(),
            output,
            error: None,
            execution_time_ms: 0,
            timestamp: Utc::now(),
        }
    }

    #[test]
    fn test_truncate_small_output() {
        let mut resp = make_response(Some(serde_json::Value::String("short".to_string())));
        truncate_output(&mut resp);
        assert_eq!(resp.output.unwrap().as_str().unwrap(), "short");
    }

    #[test]
    fn test_truncate_large_output() {
        let large = "x".repeat(10000);
        let mut resp = make_response(Some(serde_json::Value::String(large)));
        truncate_output(&mut resp);
        let output = resp.output.unwrap();
        let s = output.as_str().unwrap();
        assert!(s.len() < 10000);
        assert!(s.ends_with("[OUTPUT TRUNCATED]"));
    }

    #[test]
    fn test_truncate_no_output() {
        let mut resp = make_response(None);
        truncate_output(&mut resp);
        assert!(resp.output.is_none());
    }

    #[test]
    fn test_truncate_json_output_untouched() {
        // Non-string JSON values should not be truncated
        let mut resp = make_response(Some(serde_json::json!({"key": "value"})));
        truncate_output(&mut resp);
        assert_eq!(resp.output.unwrap()["key"], "value");
    }
}

/// Wait for a shutdown signal (SIGTERM or SIGINT on Unix, Ctrl+C on Windows)
async fn shutdown_signal() {
    #[cfg(unix)]
    {
        use tokio::signal::unix::{signal, SignalKind};
        let mut sigterm = signal(SignalKind::terminate())
            .expect("failed to register SIGTERM handler");
        let mut sigint = signal(SignalKind::interrupt())
            .expect("failed to register SIGINT handler");
        tokio::select! {
            _ = sigterm.recv() => info!("Received SIGTERM"),
            _ = sigint.recv() => info!("Received SIGINT"),
        }
    }
    #[cfg(not(unix))]
    {
        tokio::signal::ctrl_c().await.expect("failed to register Ctrl+C handler");
        info!("Received Ctrl+C");
    }
}
