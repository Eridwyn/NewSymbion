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
use crate::execution::CommandExecutor;
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

        info!("Agent initialized — ID: {}, Hostname: {}",
              system_info.agent_id, system_info.hostname);

        Ok(Agent {
            config,
            system_info,
            mqtt_client: mqtt_handle.client,
            last_command: None,
            command_receiver: mqtt_handle.command_rx,
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
    // Command Processing — delegates to execution module
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

        let (status, data, err) = self.dispatch_command(&incoming).await;

        // Update last command info
        self.last_command = Some(CommandInfo {
            command_id: incoming.command_id.clone(),
            command_type: incoming.command_type.clone(),
            status: status.clone(),
            timestamp: Utc::now(),
        });

        // Build and send response
        let response = CommandResponse {
            command_id: incoming.command_id,
            agent_id: self.system_info.agent_id.clone(),
            status,
            output: data,
            error: err,
            execution_time_ms: start_time.elapsed().as_millis(),
            timestamp: Utc::now(),
        };

        self.send_response(response).await
    }

    /// Dispatch command to the appropriate handler via `execution` module.
    /// Returns (status, output_data, error_info).
    async fn dispatch_command(&self, cmd: &IncomingCommand) -> (String, Option<serde_json::Value>, Option<ErrorInfo>) {
        match cmd.command_type.as_str() {
            "shutdown" | "reboot" | "hibernate" => {
                self.handle_power_command(&cmd.command_type).await
            }
            "reconnect" => {
                ("success".to_string(), Some(serde_json::json!({
                    "message": "Reconnect acknowledged — agent will re-register on next heartbeat"
                })), None)
            }
            "kill_process" => {
                self.handle_kill_process(cmd).await
            }
            "run_command" => {
                self.handle_shell_command(cmd).await
            }
            "get_metrics" => {
                self.handle_get_metrics().await
            }
            "list_processes" => {
                self.handle_list_processes().await
            }
            _ => {
                let err = ErrorInfo {
                    code: "UNKNOWN_COMMAND".to_string(),
                    message: format!("Unknown command type: {}", cmd.command_type),
                };
                ("error".to_string(), None, Some(err))
            }
        }
    }

    /// Handle power commands (shutdown, reboot, hibernate) via execution module
    async fn handle_power_command(&self, command_type: &str) -> (String, Option<serde_json::Value>, Option<ErrorInfo>) {
        match CommandExecutor::execute_power_command(command_type, None).await {
            Ok(result) => {
                if result.success {
                    ("success".to_string(), Some(serde_json::json!({
                        "message": format!("{} initiated", command_type)
                    })), None)
                } else {
                    let err = ErrorInfo {
                        code: format!("{}_FAILED", command_type.to_uppercase()),
                        message: result.error.unwrap_or_default(),
                    };
                    ("error".to_string(), None, Some(err))
                }
            }
            Err(e) => {
                let err = ErrorInfo {
                    code: "EXECUTION_ERROR".to_string(),
                    message: e.to_string(),
                };
                ("error".to_string(), None, Some(err))
            }
        }
    }

    /// Handle kill_process via execution module
    async fn handle_kill_process(&self, cmd: &IncomingCommand) -> (String, Option<serde_json::Value>, Option<ErrorInfo>) {
        let pid = match cmd.parameters.as_ref()
            .and_then(|p| p.get("pid"))
            .and_then(|p| p.as_u64())
        {
            Some(pid) => pid as u32,
            None => {
                return ("error".to_string(), None, Some(ErrorInfo {
                    code: "INVALID_PARAMETERS".to_string(),
                    message: "Missing 'pid' parameter".to_string(),
                }));
            }
        };

        match CommandExecutor::kill_process(pid).await {
            Ok(result) => {
                if result.success {
                    ("success".to_string(), Some(serde_json::json!({
                        "message": format!("Process {} killed", pid)
                    })), None)
                } else {
                    let err = ErrorInfo {
                        code: "KILL_FAILED".to_string(),
                        message: result.error.unwrap_or_default(),
                    };
                    ("error".to_string(), None, Some(err))
                }
            }
            Err(e) => {
                let err = ErrorInfo {
                    code: "EXECUTION_ERROR".to_string(),
                    message: e.to_string(),
                };
                ("error".to_string(), None, Some(err))
            }
        }
    }

    /// Handle run_command via execution module with safety check
    async fn handle_shell_command(&self, cmd: &IncomingCommand) -> (String, Option<serde_json::Value>, Option<ErrorInfo>) {
        let command = match cmd.parameters.as_ref()
            .and_then(|p| p.get("command"))
            .and_then(|p| p.as_str())
        {
            Some(c) => c,
            None => {
                return ("error".to_string(), None, Some(ErrorInfo {
                    code: "INVALID_PARAMETERS".to_string(),
                    message: "Missing 'command' parameter".to_string(),
                }));
            }
        };

        // Security: validate command against allowlist
        if let Err(reason) = validate_shell_command(command) {
            return ("error".to_string(), None, Some(ErrorInfo {
                code: "UNSAFE_COMMAND".to_string(),
                message: reason,
            }));
        }

        let timeout_secs = cmd.parameters.as_ref()
            .and_then(|p| p.get("timeout"))
            .and_then(|t| t.as_u64())
            .unwrap_or(30) as u32;

        match CommandExecutor::execute_shell_command(command, timeout_secs).await {
            Ok(result) => {
                if result.success {
                    // Clean ANSI escape codes from output
                    let clean = clean_output(&result.output);
                    ("success".to_string(), Some(serde_json::Value::String(clean)), None)
                } else {
                    let err = ErrorInfo {
                        code: "COMMAND_FAILED".to_string(),
                        message: format!("Exit code: {:?}", result.exit_code),
                    };
                    let clean = clean_output(&result.output);
                    ("error".to_string(), Some(serde_json::Value::String(clean)), Some(err))
                }
            }
            Err(e) => {
                let err = ErrorInfo {
                    code: "EXECUTION_ERROR".to_string(),
                    message: e.to_string(),
                };
                ("error".to_string(), None, Some(err))
            }
        }
    }

    /// Handle get_metrics
    async fn handle_get_metrics(&self) -> (String, Option<serde_json::Value>, Option<ErrorInfo>) {
        match metrics::SystemMetrics::collect().await {
            Ok(system_metrics) => {
                let process_info = metrics::ProcessInfo::collect().await.ok();
                let services = metrics::ServiceStatus::collect_critical().await.ok();
                let data = serde_json::json!({
                    "system": system_metrics,
                    "processes": process_info,
                    "services": services,
                    "timestamp": Utc::now()
                });
                ("success".to_string(), Some(data), None)
            }
            Err(e) => {
                let err = ErrorInfo {
                    code: "METRICS_ERROR".to_string(),
                    message: e.to_string(),
                };
                ("error".to_string(), None, Some(err))
            }
        }
    }

    /// Handle list_processes
    async fn handle_list_processes(&self) -> (String, Option<serde_json::Value>, Option<ErrorInfo>) {
        match metrics::ProcessInfo::collect().await {
            Ok(process_info) => {
                let data = serde_json::json!({
                    "total_count": process_info.total_count,
                    "running_count": process_info.running_count,
                    "top_cpu": process_info.top_cpu,
                    "top_memory": process_info.top_memory,
                    "timestamp": Utc::now()
                });
                ("success".to_string(), Some(data), None)
            }
            Err(e) => {
                let err = ErrorInfo {
                    code: "PROCESSES_ERROR".to_string(),
                    message: e.to_string(),
                };
                ("error".to_string(), None, Some(err))
            }
        }
    }

    // ========================================================================
    // Response sending with output truncation
    // ========================================================================

    /// Send command response, truncating large outputs for MQTT transport
    async fn send_response(&self, mut response: CommandResponse) -> Result<()> {
        const MAX_OUTPUT_SIZE: usize = 7000;

        // Truncate large string outputs
        if let Some(serde_json::Value::String(ref output_str)) = response.output {
            if output_str.len() > MAX_OUTPUT_SIZE {
                info!("Output too large ({} chars), truncating to {}", output_str.len(), MAX_OUTPUT_SIZE);
                let mut truncated: String = output_str.chars().take(MAX_OUTPUT_SIZE).collect();
                truncated.push_str("\n\n[OUTPUT TRUNCATED]");
                response.output = Some(serde_json::Value::String(truncated));
            }
        }

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

/// Allowed commands for remote shell execution.
/// Only the first token (binary name) is checked — shell chaining is blocked separately.
const ALLOWED_COMMANDS: &[&str] = &[
    "cat", "date", "df", "dir", "echo", "free", "head", "hostname",
    "id", "ifconfig", "ip", "ipconfig", "ls", "netstat", "nslookup",
    "ping", "ps", "pwd", "systemctl", "tail", "tasklist", "tracert",
    "traceroute", "uname", "uptime", "wc", "who", "whoami",
];

/// Shell metacharacters that indicate command chaining or injection.
const DANGEROUS_PATTERNS: &[&str] = &[
    ";", "&&", "||", "|", "$(", "`", "<(", ">(", "\n", "\r",
];

/// Validate a shell command against the allowlist.
/// Returns Ok(()) if safe, Err(reason) if blocked.
fn validate_shell_command(command: &str) -> Result<(), String> {
    let trimmed = command.trim();
    if trimmed.is_empty() {
        return Err("Empty command".to_string());
    }

    // Block shell chaining / injection metacharacters
    for pattern in DANGEROUS_PATTERNS {
        if trimmed.contains(pattern) {
            return Err(format!(
                "Command contains blocked operator '{}': {}",
                pattern, trimmed
            ));
        }
    }

    // Block output redirection
    if trimmed.contains('>') {
        return Err(format!("Command contains output redirection: {}", trimmed));
    }

    // Extract the first token (binary name) and validate against allowlist
    let first_token = trimmed
        .split_whitespace()
        .next()
        .unwrap_or("");

    // Strip path prefix (e.g. /usr/bin/ls -> ls)
    let binary_name = first_token.rsplit('/').next().unwrap_or(first_token);
    // Also strip Windows path prefix (e.g. C:\Windows\System32\hostname.exe -> hostname.exe -> hostname)
    let binary_name = binary_name.rsplit('\\').next().unwrap_or(binary_name);
    let binary_name = binary_name.strip_suffix(".exe").unwrap_or(binary_name);

    if !ALLOWED_COMMANDS.contains(&binary_name) {
        return Err(format!("Command '{}' not in allowlist", binary_name));
    }

    Ok(())
}

/// Clean ANSI escape codes and control characters from command output
fn clean_output(s: &str) -> String {
    s.chars()
        .filter(|c| c.is_ascii_graphic() || *c == ' ' || *c == '\n' || *c == '\r' || *c == '\t')
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_allowed_command_passes() {
        assert!(validate_shell_command("ls -la").is_ok());
        assert!(validate_shell_command("whoami").is_ok());
        assert!(validate_shell_command("ping 8.8.8.8").is_ok());
        assert!(validate_shell_command("df -h").is_ok());
        assert!(validate_shell_command("systemctl status nginx").is_ok());
    }

    #[test]
    fn test_blocked_command_rejected() {
        assert!(validate_shell_command("rm -rf /").is_err());
        assert!(validate_shell_command("curl http://evil.com").is_err());
        assert!(validate_shell_command("wget http://evil.com").is_err());
        assert!(validate_shell_command("powershell -Command Get-Process").is_err());
        assert!(validate_shell_command("bash -c 'echo pwned'").is_err());
    }

    #[test]
    fn test_chaining_blocked() {
        assert!(validate_shell_command("ls; rm -rf /").is_err());
        assert!(validate_shell_command("ls && cat /etc/shadow").is_err());
        assert!(validate_shell_command("ls || wget evil.com").is_err());
        assert!(validate_shell_command("ls | xargs rm").is_err());
    }

    #[test]
    fn test_injection_blocked() {
        assert!(validate_shell_command("echo $(whoami)").is_err());
        assert!(validate_shell_command("echo `id`").is_err());
        assert!(validate_shell_command("ls > /tmp/output").is_err());
    }

    #[test]
    fn test_path_prefix_stripped() {
        assert!(validate_shell_command("/usr/bin/ls -la").is_ok());
        assert!(validate_shell_command("/bin/cat /etc/hostname").is_ok());
    }

    #[test]
    fn test_empty_command() {
        assert!(validate_shell_command("").is_err());
        assert!(validate_shell_command("   ").is_err());
    }

    #[test]
    fn test_clean_output() {
        // clean_output strips non-printable control chars (like ESC \x1b)
        // but keeps ASCII graphic chars like [, 3, 1, m
        assert_eq!(clean_output("Hello\x1b World"), "Hello World");
        assert_eq!(clean_output("line1\nline2"), "line1\nline2");
        assert_eq!(clean_output("ok\x00hidden"), "okhidden");
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
