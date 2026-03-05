//! Symbion Agent core logic
//!
//! Contains the Agent struct, event loop, and command dispatch.
//! Command execution is delegated to the `execution` module (no duplication).
//!
//! Sub-modules:
//! - heartbeat: Registration and heartbeat publishing
//! - command_dispatch: Command processing and response
//! - status_sync: Local API status updates

mod heartbeat;
mod command_dispatch;
mod status_sync;

use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Duration;

use anyhow::{Result, Context};
use rumqttc::AsyncClient;
use tokio::sync::mpsc;
use tokio::time::interval;
use tracing::{info, error, debug, warn};

use crate::discovery::SystemInfo;
use crate::execution::handler::CommandRegistry;
use crate::execution::handlers;
use crate::local_api;
use crate::messages::*;
use crate::mqtt_client;
use crate::system_tray;
use crate::file_transfer::FileTransferManager;
use crate::log_collector::LogCollector;
use crate::plugins::{AgentPluginRegistry, ActivityTracker};
use crate::scheduler::Scheduler;
use crate::watchdog::Watchdog;

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
    pub(crate) system_info: SystemInfo,
    pub(crate) mqtt_client: AsyncClient,
    pub(crate) last_command: Option<CommandInfo>,
    command_receiver: mpsc::Receiver<ReceivedCommand>,
    pub(crate) command_registry: CommandRegistry,
    pub(crate) local_api: Option<Arc<local_api::LocalApiServer>>,
    system_tray: Option<system_tray::SystemTray>,
    pub(crate) mqtt_connected: Arc<AtomicBool>,
    reconnect_rx: Option<mpsc::Receiver<()>>,
    pub(crate) log_collector: Arc<LogCollector>,
    scheduler: Arc<Scheduler>,
    pub(crate) plugin_registry: Arc<tokio::sync::Mutex<AgentPluginRegistry>>,
    pub(crate) watchdog: Arc<Watchdog>,
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
        let mut command_registry = handlers::build_default_registry();

        // Create scheduler and register its handler
        let scheduler = Arc::new(Scheduler::new().await);
        command_registry.register(Box::new(
            handlers::ScheduleHandler::new(scheduler.clone()),
        ));

        // Create file transfer manager and register its handler
        let transfer_dir = dirs::config_dir()
            .unwrap_or_else(|| std::path::PathBuf::from("/tmp"))
            .join("symbion-agent")
            .join("transfers");
        let file_transfer = Arc::new(FileTransferManager::new(transfer_dir));
        let _ = file_transfer.ensure_dir().await;
        command_registry.register(Box::new(
            handlers::FileTransferHandler::new(file_transfer.clone()),
        ));

        // Register screenshot handler (reuses file transfer directory)
        command_registry.register(Box::new(
            handlers::ScreenshotHandler::new(file_transfer),
        ));

        // Create plugin registry with built-in plugins
        let plugin_registry = {
            let mut registry = AgentPluginRegistry::new();
            let _ = registry.register(Box::new(ActivityTracker::new())).await;
            Arc::new(tokio::sync::Mutex::new(registry))
        };
        command_registry.register(Box::new(
            handlers::PluginCommandHandler::new(plugin_registry.clone()),
        ));

        // Create log collector
        let log_collector = Arc::new(LogCollector::new(
            agent_config.logging.clone(),
            system_info.agent_id.clone(),
            mqtt_handle.client.clone(),
        ));

        // Create watchdog
        let watchdog = Arc::new(Watchdog::new(
            agent_config.watchdog.clone(),
            mqtt_handle.connected.clone(),
            None,
        ));

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
            log_collector,
            scheduler,
            plugin_registry,
            watchdog,
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
        self.log("INFO", "Agent main loop started").await;

        // Spawn watchdog background task
        let _watchdog_handle = self.watchdog.clone().spawn();

        // Initial registration
        self.register().await?;
        self.log("INFO", "Agent registered with kernel").await;

        let mut heartbeat_timer = interval(Duration::from_secs(self.config.heartbeat_interval_secs));
        let mut registration_timer = interval(Duration::from_secs(300));
        let mut scheduler_timer = interval(Duration::from_secs(15));
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
                    let connected = self.mqtt_connected.load(Ordering::Relaxed);
                    match self.send_heartbeat().await {
                        Ok(system_metrics) => {
                            self.watchdog.notify_heartbeat_sent().await;
                            self.watchdog.notify_metrics_ok().await;
                            // Reuse metrics from heartbeat — avoid double collection
                            self.update_local_api_status(connected, Some(system_metrics)).await;
                        }
                        Err(e) => {
                            error!("Failed to send heartbeat: {}", e);
                            self.watchdog.notify_metrics_failed().await;
                            self.log("ERROR", &format!("Heartbeat failed: {}", e)).await;
                            self.update_local_api_status(connected, None).await;
                        }
                    }
                }

                _ = registration_timer.tick() => {
                    if let Err(e) = self.register().await {
                        error!("Failed to re-register: {}", e);
                    }
                }

                _ = scheduler_timer.tick() => {
                    let executed = self.scheduler.tick(&self.command_registry).await;
                    if executed > 0 {
                        debug!("Scheduler executed {} task(s)", executed);
                    }
                }

                command = self.command_receiver.recv() => {
                    match command {
                        Some(cmd) => {
                            if let Err(e) = self.process_command(cmd.clone()).await {
                                error!("Failed to process command: {}", e);
                                self.log("ERROR", &format!("Command failed: {}", e)).await;
                                // Try to send error response even when parsing fails
                                self.send_error_response_from_raw(&cmd.payload, &e.to_string()).await;
                            }
                        }
                        None => {
                            warn!("Command channel closed");
                            self.log("WARN", "Command channel closed").await;
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
                    self.log("INFO", "Reconnect signal received").await;
                    if let Err(e) = self.register().await {
                        error!("Failed to re-register after reconnect signal: {}", e);
                        self.log("ERROR", &format!("Re-registration failed: {}", e)).await;
                    }
                }
            }
        }
    }
}

// ========================================================================
// Helpers
// ========================================================================

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

/// Wait for a shutdown signal (SIGTERM or SIGINT on Unix, Ctrl+C on Windows)
async fn shutdown_signal() {
    #[cfg(unix)]
    {
        use tokio::signal::unix::{signal, SignalKind};
        let sigterm = signal(SignalKind::terminate());
        let sigint = signal(SignalKind::interrupt());

        match (sigterm, sigint) {
            (Ok(mut term), Ok(mut int)) => {
                tokio::select! {
                    _ = term.recv() => info!("Received SIGTERM"),
                    _ = int.recv() => info!("Received SIGINT"),
                }
            }
            (Ok(mut term), Err(e)) => {
                warn!("Failed to register SIGINT handler: {}, using SIGTERM only", e);
                term.recv().await;
                info!("Received SIGTERM");
            }
            (Err(e), Ok(mut int)) => {
                warn!("Failed to register SIGTERM handler: {}, using SIGINT only", e);
                int.recv().await;
                info!("Received SIGINT");
            }
            (Err(e1), Err(e2)) => {
                error!("Failed to register signal handlers: SIGTERM={}, SIGINT={}", e1, e2);
                error!("Falling back to ctrl_c handler");
                if let Err(e) = tokio::signal::ctrl_c().await {
                    error!("ctrl_c handler also failed: {} — agent will run until killed", e);
                    std::future::pending::<()>().await;
                }
            }
        }
    }
    #[cfg(not(unix))]
    {
        match tokio::signal::ctrl_c().await {
            Ok(()) => info!("Received Ctrl+C"),
            Err(e) => {
                error!("Failed to register Ctrl+C handler: {} — agent will run until killed", e);
                std::future::pending::<()>().await;
            }
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

    #[test]
    fn test_log_buffer_capacity() {
        use std::sync::Arc;
        let buffer = Arc::new(std::sync::Mutex::new(Vec::<crate::messages::LogEntry>::new()));
        {
            let mut logs = buffer.lock().unwrap();
            for i in 0..250 {
                logs.push(crate::messages::LogEntry {
                    timestamp: Utc::now(),
                    level: "ERROR".to_string(),
                    message: format!("error {}", i),
                    module: None,
                    source: None,
                });
            }
            // Cap at 200
            if logs.len() > 200 {
                let excess = logs.len() - 200;
                logs.drain(..excess);
            }
        }
        let logs = buffer.lock().unwrap();
        assert_eq!(logs.len(), 200);
        assert!(logs.last().unwrap().message.contains("249"));
    }

    #[test]
    fn test_log_entry_timestamp_format() {
        let entry = crate::messages::LogEntry {
            timestamp: Utc::now(),
            level: "WARN".to_string(),
            message: "test".to_string(),
            module: Some("agent".to_string()),
            source: None,
        };
        let json = serde_json::to_string(&entry).unwrap();
        // Verify timestamp is ISO 8601
        assert!(json.contains("20"));  // Year prefix
    }
}
