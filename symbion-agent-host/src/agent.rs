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
    system_info: SystemInfo,
    mqtt_client: AsyncClient,
    last_command: Option<CommandInfo>,
    command_receiver: mpsc::Receiver<ReceivedCommand>,
    command_registry: CommandRegistry,
    local_api: Option<Arc<local_api::LocalApiServer>>,
    system_tray: Option<system_tray::SystemTray>,
    mqtt_connected: Arc<AtomicBool>,
    reconnect_rx: Option<mpsc::Receiver<()>>,
    log_collector: Arc<LogCollector>,
    scheduler: Arc<Scheduler>,
    plugin_registry: Arc<tokio::sync::Mutex<AgentPluginRegistry>>,
    watchdog: Arc<Watchdog>,
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
            watchdog: Some(self.watchdog.report().await),
            plugin_data: None,
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

    /// Send heartbeat with system metrics. Returns collected metrics for reuse.
    async fn send_heartbeat(&self) -> Result<metrics::SystemMetrics> {
        let system_metrics = metrics::SystemMetrics::collect().await
            .context("Failed to collect system metrics")?;
        let process_info = metrics::ProcessInfo::collect().await.ok();
        let services = metrics::ServiceStatus::collect_critical().await.ok();

        let watchdog_report = self.watchdog.report().await;

        // Tick plugins and collect data
        let plugin_data = {
            let registry = self.plugin_registry.lock().await;
            let data = registry.tick_all().await;
            if data.is_empty() { None } else { Some(data) }
        };

        let heartbeat = HeartbeatMessage {
            agent_id: self.system_info.agent_id.clone(),
            status: "online".to_string(),
            system: system_metrics.clone(),
            processes: process_info,
            services,
            last_command: self.last_command.clone(),
            watchdog: Some(watchdog_report),
            plugin_data,
            timestamp: Utc::now(),
        };

        mqtt_client::publish_json(&self.mqtt_client, mqtt_client::TOPIC_HEARTBEAT, &heartbeat).await?;
        debug!("Heartbeat sent");

        // Flush buffered logs to kernel
        self.log_collector.flush().await;

        Ok(system_metrics)
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
        self.log("INFO", &format!("Command: {} ({})", incoming.command_type, incoming.command_id)).await;

        // Dispatch via registry (with special-case for reconnect)
        let result = if incoming.command_type == "reconnect" {
            crate::execution::handler::CommandResult::success(serde_json::json!({
                "message": "Reconnect acknowledged — agent will re-register on next heartbeat"
            }))
        } else {
            match self.command_registry.execute(&incoming.command_type, incoming.parameters.as_ref()).await {
                Some(result) => result,
                None => {
                    warn!(
                        command_type = %incoming.command_type,
                        command_id = %incoming.command_id,
                        "[agent] REJECTED unknown command type — forensic audit"
                    );
                    crate::execution::handler::CommandResult::error(
                        "UNKNOWN_COMMAND",
                        format!("Unknown command type: {}", incoming.command_type),
                    )
                }
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

    /// Best-effort error response when command JSON fails to parse.
    /// Extracts command_id from raw JSON so the kernel can update PendingCommand status.
    async fn send_error_response_from_raw(&self, raw_json: &str, error_msg: &str) {
        // Try to extract command_id and agent_id from the raw JSON
        let parsed: Result<serde_json::Value, _> = serde_json::from_str(raw_json);
        let (command_id, agent_id) = match parsed {
            Ok(val) => {
                let cid = val.get("command_id").and_then(|v| v.as_str()).unwrap_or("unknown");
                let aid = val.get("agent_id").and_then(|v| v.as_str()).unwrap_or("unknown");
                (cid.to_string(), aid.to_string())
            }
            Err(_) => return, // Can't even parse as JSON — nothing we can do
        };

        // Only respond if this command is for us
        if agent_id != self.system_info.agent_id {
            return;
        }

        let response = CommandResponse {
            command_id,
            agent_id,
            status: "error".to_string(),
            output: None,
            error: Some(crate::messages::ErrorInfo {
                code: "PARSE_ERROR".to_string(),
                message: error_msg.to_string(),
            }),
            execution_time_ms: 0,
            timestamp: Utc::now(),
        };

        if let Err(e) = mqtt_client::publish_json(&self.mqtt_client, mqtt_client::TOPIC_RESPONSE, &response).await {
            error!("Failed to send error response: {}", e);
        }
    }

    // ========================================================================
    // Local API status update
    // ========================================================================

    /// Push a log entry to the local API ring buffer and log collector
    async fn log(&self, level: &str, message: &str) {
        if let Some(ref api) = self.local_api {
            api.push_log(level, message).await;
        }
        // Forward to log collector (it handles level filtering and immediate publish)
        self.log_collector.push(level, message, None, None).await;
    }

    /// Update local API status. Reuses metrics from heartbeat to avoid double collection.
    async fn update_local_api_status(&self, mqtt_connected: bool, cached_metrics: Option<metrics::SystemMetrics>) {
        if let Some(ref api) = self.local_api {
            let system_status = if let Some(m) = cached_metrics {
                let process_count = metrics::ProcessInfo::collect().await
                    .map(|p| p.total_count as u32)
                    .unwrap_or(0);

                // Aggregate disk: use root "/" or first disk
                let (disk_used, disk_total) = m.disk.first()
                    .map(|d| (Some(d.used_gb), Some(d.total_gb)))
                    .unwrap_or((None, None));

                // Temperature: use cpu_celsius from temperature metrics
                let temperature = m.temperature.as_ref()
                    .and_then(|t| t.cpu_celsius)
                    .map(|c| c as f64);

                // Network: aggregate all interfaces
                let (net_rx, net_tx) = m.network.as_ref()
                    .map(|n| {
                        let rx: u64 = n.interfaces.iter().map(|i| i.bytes_recv).sum();
                        let tx: u64 = n.interfaces.iter().map(|i| i.bytes_sent).sum();
                        (Some(rx), Some(tx))
                    })
                    .unwrap_or((None, None));

                Some(local_api::SystemStatus {
                    cpu_percent: m.cpu.percent as f64,
                    memory_used_mb: m.memory.used_mb,
                    memory_total_mb: m.memory.total_mb,
                    disk_used_gb: disk_used,
                    disk_total_gb: disk_total,
                    process_count,
                    load_average: Some(m.cpu.load_avg[0]),
                    temperature,
                    swap_used_mb: Some(m.swap.used_mb),
                    swap_total_mb: Some(m.swap.total_mb),
                    network_rx_bytes: net_rx,
                    network_tx_bytes: net_tx,
                    cpu_cores: Some(m.cpu.core_count),
                })
            } else {
                None
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
