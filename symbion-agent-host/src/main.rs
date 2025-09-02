//! Symbion Agent Host - Multi-OS system agent for network control
//!
//! This agent provides remote system control capabilities to the Symbion kernel:
//! - Auto-discovery and registration via MQTT
//! - System metrics monitoring and reporting  
//! - Remote command execution (shutdown, reboot, process control)
//! - Cross-platform support (Linux, Windows, Android)

mod discovery;
mod capabilities;
mod metrics;
mod execution;

use anyhow::{Result, Context};
use chrono::{DateTime, Utc};
use discovery::SystemInfo;
use rumqttc::{AsyncClient, Event, Incoming, MqttOptions, QoS};
use serde::{Deserialize, Serialize};
use std::time::Duration;
use tokio::time::interval;
use tokio::sync::mpsc;
use tracing::{info, error, debug, warn};
// use uuid::Uuid; // Not needed currently

/// Agent configuration
#[derive(Debug, Clone)]
struct AgentConfig {
    mqtt_broker: String,
    mqtt_port: u16,
    mqtt_client_id: String,
    heartbeat_interval_secs: u64,
    registration_retry_secs: u64,
}

impl Default for AgentConfig {
    fn default() -> Self {
        Self {
            mqtt_broker: "localhost".to_string(),
            mqtt_port: 1883,
            mqtt_client_id: "symbion-agent-unknown".to_string(),
            heartbeat_interval_secs: 30,
            registration_retry_secs: 10,
        }
    }
}

/// Agent registration message (matches agents.registration@v1 contract)
#[derive(Debug, Serialize)]
struct RegistrationMessage {
    agent_id: String,
    hostname: String,
    os: String,
    architecture: String,
    capabilities: Vec<String>,
    network: discovery::NetworkInfo,
    version: String,
    timestamp: DateTime<Utc>,
}

/// Agent heartbeat message (matches agents.heartbeat@v1 contract)
#[derive(Debug, Serialize)]
struct HeartbeatMessage {
    agent_id: String,
    status: String,
    system: metrics::SystemMetrics,
    processes: Option<metrics::ProcessInfo>,
    services: Option<Vec<metrics::ServiceStatus>>,
    last_command: Option<CommandInfo>,
    timestamp: DateTime<Utc>,
}

/// Command information for heartbeat
#[derive(Debug, Clone, Serialize)]
struct CommandInfo {
    command_id: String,
    command_type: String,
    status: String,
    timestamp: DateTime<Utc>,
}

/// Incoming command from kernel (matches agents.command@v1 contract)
#[derive(Debug, Deserialize)]
struct IncomingCommand {
    command_id: String,
    agent_id: String,
    command_type: String,
    parameters: Option<serde_json::Value>,
    timestamp: DateTime<Utc>,
    requester: Option<String>,
}

/// Command response to kernel (matches agents.response@v1 contract)
#[derive(Debug, Serialize)]
struct CommandResponse {
    command_id: String,
    agent_id: String,
    status: String,
    data: Option<serde_json::Value>,
    error: Option<ErrorInfo>,
    execution_time_ms: u128,
    timestamp: DateTime<Utc>,
}

/// Error information for failed commands
#[derive(Debug, Serialize)]
struct ErrorInfo {
    code: String,
    message: String,
}

/// Received command for internal processing
#[derive(Debug, Clone)]
struct ReceivedCommand {
    topic: String,
    payload: String,
}

/// Main agent state
struct Agent {
    config: AgentConfig,
    system_info: SystemInfo,
    mqtt_client: AsyncClient,
    last_command: Option<CommandInfo>,
    command_receiver: mpsc::Receiver<ReceivedCommand>,
}

impl Agent {
    /// Create new agent instance
    async fn new() -> Result<Self> {
        info!("Initializing Symbion Agent Host v1.0.0");
        
        // Discover system information
        let system_info = SystemInfo::discover().await
            .context("Failed to discover system information")?;
            
        // Configure MQTT client
        let mut config = AgentConfig::default();
        config.mqtt_client_id = format!("symbion-agent-{}", system_info.agent_id);
        
        let mut mqtt_options = MqttOptions::new(
            &config.mqtt_client_id,
            &config.mqtt_broker,
            config.mqtt_port
        );
        mqtt_options.set_keep_alive(Duration::from_secs(30));
        mqtt_options.set_clean_session(true);
        
        let (mqtt_client, mut eventloop) = AsyncClient::new(mqtt_options, 10);
        
        // Create command channel
        let (command_sender, command_receiver) = mpsc::channel::<ReceivedCommand>(100);
        
        // Start MQTT event loop in background
        tokio::spawn(async move {
            loop {
                match eventloop.poll().await {
                    Ok(Event::Incoming(Incoming::Publish(publish))) => {
                        debug!("Received MQTT message on topic: {}", publish.topic);
                        
                        // Forward command messages to main loop
                        if publish.topic.starts_with("symbion/agents/command@v1/") {
                            let payload = String::from_utf8_lossy(&publish.payload).to_string();
                            let command = ReceivedCommand {
                                topic: publish.topic.clone(),
                                payload,
                            };
                            
                            if let Err(e) = command_sender.send(command).await {
                                error!("Failed to forward command: {}", e);
                            }
                        }
                    }
                    Ok(_) => {}
                    Err(e) => {
                        error!("MQTT connection error: {}", e);
                        tokio::time::sleep(Duration::from_secs(5)).await;
                    }
                }
            }
        });
        
        info!("Agent initialized - ID: {}, Hostname: {}", 
              system_info.agent_id, system_info.hostname);
        
        Ok(Agent {
            config,
            system_info,
            mqtt_client,
            last_command: None,
            command_receiver,
        })
    }
    
    /// Start agent main loop
    async fn run(&mut self) -> Result<()> {
        info!("Starting agent main loop...");
        
        // Subscribe to command topic for this agent
        let command_topic = format!("symbion/agents/command@v1/{}", self.system_info.agent_id);
        self.mqtt_client.subscribe(&command_topic, QoS::AtLeastOnce).await
            .context("Failed to subscribe to command topic")?;
            
        info!("Subscribed to commands on: {}", command_topic);
        
        // Initial registration
        self.register().await?;
        
        // Set up periodic tasks
        let mut heartbeat_timer = interval(Duration::from_secs(self.config.heartbeat_interval_secs));
        let mut registration_timer = interval(Duration::from_secs(self.config.registration_retry_secs * 6)); // Re-register every minute
        
        loop {
            tokio::select! {
                _ = heartbeat_timer.tick() => {
                    if let Err(e) = self.send_heartbeat().await {
                        error!("Failed to send heartbeat: {}", e);
                    }
                }
                
                _ = registration_timer.tick() => {
                    if let Err(e) = self.register().await {
                        error!("Failed to re-register: {}", e);
                    }
                }
                
                command = self.command_receiver.recv() => {
                    match command {
                        Some(cmd) => {
                            info!("Processing command from topic: {}", cmd.topic);
                            if let Err(e) = self.process_command(cmd).await {
                                error!("Failed to process command: {}", e);
                            }
                        }
                        None => {
                            warn!("Command channel closed");
                            break Ok(());
                        }
                    }
                }
            }
        }
    }
    
    /// Register agent with kernel
    async fn register(&self) -> Result<()> {
        let capabilities = self.get_capabilities();
        
        let registration = RegistrationMessage {
            agent_id: self.system_info.agent_id.clone(),
            hostname: self.system_info.hostname.clone(),
            os: self.system_info.os.clone(),
            architecture: self.system_info.architecture.clone(),
            capabilities,
            network: self.system_info.network.clone(),
            version: "1.0.0".to_string(),
            timestamp: Utc::now(),
        };
        
        let payload = serde_json::to_string(&registration)
            .context("Failed to serialize registration message")?;
            
        self.mqtt_client
            .publish("symbion/agents/registration@v1", QoS::AtLeastOnce, false, payload)
            .await
            .context("Failed to publish registration")?;
            
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
        
        let payload = serde_json::to_string(&heartbeat)
            .context("Failed to serialize heartbeat message")?;
            
        self.mqtt_client
            .publish("symbion/agents/heartbeat@v1", QoS::AtLeastOnce, false, payload)
            .await
            .context("Failed to publish heartbeat")?;
            
        debug!("Heartbeat sent");
        Ok(())
    }
    
    /// Process incoming command from MQTT
    async fn process_command(&mut self, cmd: ReceivedCommand) -> Result<()> {
        let start_time = std::time::Instant::now();
        
        // Parse the incoming command
        let incoming: IncomingCommand = serde_json::from_str(&cmd.payload)
            .context("Failed to parse incoming command")?;
        
        info!("Executing command: {} ({})", incoming.command_type, incoming.command_id);
        
        // Execute the command based on type
        let (status, data, error) = match incoming.command_type.as_str() {
            "shutdown" => self.execute_shutdown(&incoming).await,
            "reboot" => self.execute_reboot(&incoming).await,
            "hibernate" => self.execute_hibernate(&incoming).await,
            "kill_process" => self.execute_kill_process(&incoming).await,
            "run_command" => self.execute_shell_command(&incoming).await,
            "get_metrics" => self.execute_get_metrics(&incoming).await,
            _ => {
                let err = ErrorInfo {
                    code: "UNKNOWN_COMMAND".to_string(),
                    message: format!("Unknown command type: {}", incoming.command_type),
                };
                ("error".to_string(), None, Some(err))
            }
        };
        
        // Update last command info
        self.last_command = Some(CommandInfo {
            command_id: incoming.command_id.clone(),
            command_type: incoming.command_type.clone(),
            status: status.clone(),
            timestamp: Utc::now(),
        });
        
        // Send response back to kernel
        let execution_time = start_time.elapsed().as_millis();
        let response = CommandResponse {
            command_id: incoming.command_id,
            agent_id: self.system_info.agent_id.clone(),
            status,
            data,
            error,
            execution_time_ms: execution_time,
            timestamp: Utc::now(),
        };
        
        let payload = serde_json::to_string(&response)
            .context("Failed to serialize command response")?;
            
        self.mqtt_client
            .publish("symbion/agents/response@v1", QoS::AtLeastOnce, false, payload)
            .await
            .context("Failed to publish command response")?;
            
        Ok(())
    }
    
    /// Execute shutdown command
    async fn execute_shutdown(&self, _cmd: &IncomingCommand) -> (String, Option<serde_json::Value>, Option<ErrorInfo>) {
        info!("Executing shutdown command...");
        
        match self.system_info.os.as_str() {
            "windows" => {
                match tokio::process::Command::new("shutdown")
                    .args(&["/s", "/t", "3", "/f", "/c", "Shutdown by Symbion Agent"])
                    .output()
                    .await
                {
                    Ok(output) => {
                        if output.status.success() {
                            info!("Shutdown command executed successfully");
                            ("success".to_string(), Some(serde_json::json!({"message": "Shutdown initiated"})), None)
                        } else {
                            let stderr = String::from_utf8_lossy(&output.stderr);
                            error!("Shutdown failed: {}", stderr);
                            let err = ErrorInfo {
                                code: "SHUTDOWN_FAILED".to_string(),
                                message: format!("Command failed: {}", stderr),
                            };
                            ("error".to_string(), None, Some(err))
                        }
                    }
                    Err(e) => {
                        error!("Failed to execute shutdown: {}", e);
                        let err = ErrorInfo {
                            code: "EXECUTION_ERROR".to_string(),
                            message: format!("Failed to execute shutdown: {}", e),
                        };
                        ("error".to_string(), None, Some(err))
                    }
                }
            }
            "linux" => {
                match tokio::process::Command::new("sudo")
                    .args(&["shutdown", "-h", "+1", "Shutdown initiated by Symbion"])
                    .output()
                    .await
                {
                    Ok(output) => {
                        if output.status.success() {
                            info!("Shutdown command executed successfully");
                            ("success".to_string(), Some(serde_json::json!({"message": "Shutdown initiated"})), None)
                        } else {
                            let stderr = String::from_utf8_lossy(&output.stderr);
                            error!("Shutdown failed: {}", stderr);
                            let err = ErrorInfo {
                                code: "SHUTDOWN_FAILED".to_string(),
                                message: format!("Command failed: {}", stderr),
                            };
                            ("error".to_string(), None, Some(err))
                        }
                    }
                    Err(e) => {
                        error!("Failed to execute shutdown: {}", e);
                        let err = ErrorInfo {
                            code: "EXECUTION_ERROR".to_string(),
                            message: format!("Failed to execute shutdown: {}", e),
                        };
                        ("error".to_string(), None, Some(err))
                    }
                }
            }
            _ => {
                let err = ErrorInfo {
                    code: "UNSUPPORTED_OS".to_string(),
                    message: format!("Shutdown not supported on OS: {}", self.system_info.os),
                };
                ("error".to_string(), None, Some(err))
            }
        }
    }
    
    /// Execute reboot command
    async fn execute_reboot(&self, _cmd: &IncomingCommand) -> (String, Option<serde_json::Value>, Option<ErrorInfo>) {
        info!("Executing reboot command...");
        
        match self.system_info.os.as_str() {
            "windows" => {
                match tokio::process::Command::new("shutdown")
                    .args(&["/r", "/t", "5", "/c", "Reboot initiated by Symbion"])
                    .output()
                    .await
                {
                    Ok(output) => {
                        if output.status.success() {
                            info!("Reboot command executed successfully");
                            ("success".to_string(), Some(serde_json::json!({"message": "Reboot initiated"})), None)
                        } else {
                            let stderr = String::from_utf8_lossy(&output.stderr);
                            error!("Reboot failed: {}", stderr);
                            let err = ErrorInfo {
                                code: "REBOOT_FAILED".to_string(),
                                message: format!("Command failed: {}", stderr),
                            };
                            ("error".to_string(), None, Some(err))
                        }
                    }
                    Err(e) => {
                        error!("Failed to execute reboot: {}", e);
                        let err = ErrorInfo {
                            code: "EXECUTION_ERROR".to_string(),
                            message: format!("Failed to execute reboot: {}", e),
                        };
                        ("error".to_string(), None, Some(err))
                    }
                }
            }
            "linux" => {
                match tokio::process::Command::new("sudo")
                    .args(&["reboot"])
                    .output()
                    .await
                {
                    Ok(output) => {
                        if output.status.success() {
                            info!("Reboot command executed successfully");
                            ("success".to_string(), Some(serde_json::json!({"message": "Reboot initiated"})), None)
                        } else {
                            let stderr = String::from_utf8_lossy(&output.stderr);
                            error!("Reboot failed: {}", stderr);
                            let err = ErrorInfo {
                                code: "REBOOT_FAILED".to_string(),
                                message: format!("Command failed: {}", stderr),
                            };
                            ("error".to_string(), None, Some(err))
                        }
                    }
                    Err(e) => {
                        error!("Failed to execute reboot: {}", e);
                        let err = ErrorInfo {
                            code: "EXECUTION_ERROR".to_string(),
                            message: format!("Failed to execute reboot: {}", e),
                        };
                        ("error".to_string(), None, Some(err))
                    }
                }
            }
            _ => {
                let err = ErrorInfo {
                    code: "UNSUPPORTED_OS".to_string(),
                    message: format!("Reboot not supported on OS: {}", self.system_info.os),
                };
                ("error".to_string(), None, Some(err))
            }
        }
    }
    
    /// Execute hibernate command  
    async fn execute_hibernate(&self, _cmd: &IncomingCommand) -> (String, Option<serde_json::Value>, Option<ErrorInfo>) {
        info!("Executing hibernate command...");
        
        match self.system_info.os.as_str() {
            "windows" => {
                match tokio::process::Command::new("rundll32.exe")
                    .args(&["powrprof.dll,SetSuspendState", "Hibernate"])
                    .output()
                    .await
                {
                    Ok(output) => {
                        if output.status.success() {
                            info!("Hibernate command executed successfully");
                            ("success".to_string(), Some(serde_json::json!({"message": "Hibernate initiated"})), None)
                        } else {
                            let stderr = String::from_utf8_lossy(&output.stderr);
                            error!("Hibernate failed: {}", stderr);
                            let err = ErrorInfo {
                                code: "HIBERNATE_FAILED".to_string(),
                                message: format!("Command failed: {}", stderr),
                            };
                            ("error".to_string(), None, Some(err))
                        }
                    }
                    Err(e) => {
                        error!("Failed to execute hibernate: {}", e);
                        let err = ErrorInfo {
                            code: "EXECUTION_ERROR".to_string(),
                            message: format!("Failed to execute hibernate: {}", e),
                        };
                        ("error".to_string(), None, Some(err))
                    }
                }
            }
            "linux" => {
                match tokio::process::Command::new("systemctl")
                    .args(&["hibernate"])
                    .output()
                    .await
                {
                    Ok(output) => {
                        if output.status.success() {
                            info!("Hibernate command executed successfully");
                            ("success".to_string(), Some(serde_json::json!({"message": "Hibernate initiated"})), None)
                        } else {
                            let stderr = String::from_utf8_lossy(&output.stderr);
                            error!("Hibernate failed: {}", stderr);
                            let err = ErrorInfo {
                                code: "HIBERNATE_FAILED".to_string(),
                                message: format!("Command failed: {}", stderr),
                            };
                            ("error".to_string(), None, Some(err))
                        }
                    }
                    Err(e) => {
                        error!("Failed to execute hibernate: {}", e);
                        let err = ErrorInfo {
                            code: "EXECUTION_ERROR".to_string(),
                            message: format!("Failed to execute hibernate: {}", e),
                        };
                        ("error".to_string(), None, Some(err))
                    }
                }
            }
            _ => {
                let err = ErrorInfo {
                    code: "UNSUPPORTED_OS".to_string(),
                    message: format!("Hibernate not supported on OS: {}", self.system_info.os),
                };
                ("error".to_string(), None, Some(err))
            }
        }
    }
    
    /// Execute kill process command
    async fn execute_kill_process(&self, cmd: &IncomingCommand) -> (String, Option<serde_json::Value>, Option<ErrorInfo>) {
        info!("Executing kill process command...");
        
        let pid = match cmd.parameters.as_ref()
            .and_then(|p| p.get("pid"))
            .and_then(|p| p.as_u64()) {
            Some(pid) => pid,
            None => {
                let err = ErrorInfo {
                    code: "INVALID_PARAMETERS".to_string(),
                    message: "Missing 'pid' parameter".to_string(),
                };
                return ("error".to_string(), None, Some(err));
            }
        };
        
        match self.system_info.os.as_str() {
            "windows" => {
                match tokio::process::Command::new("taskkill")
                    .args(&["/PID", &pid.to_string(), "/F"])
                    .output()
                    .await
                {
                    Ok(output) => {
                        if output.status.success() {
                            info!("Process {} killed successfully", pid);
                            ("success".to_string(), Some(serde_json::json!({"message": format!("Process {} killed", pid)})), None)
                        } else {
                            let stderr = String::from_utf8_lossy(&output.stderr);
                            error!("Kill process failed: {}", stderr);
                            let err = ErrorInfo {
                                code: "KILL_FAILED".to_string(),
                                message: format!("Command failed: {}", stderr),
                            };
                            ("error".to_string(), None, Some(err))
                        }
                    }
                    Err(e) => {
                        error!("Failed to execute kill: {}", e);
                        let err = ErrorInfo {
                            code: "EXECUTION_ERROR".to_string(),
                            message: format!("Failed to execute kill: {}", e),
                        };
                        ("error".to_string(), None, Some(err))
                    }
                }
            }
            "linux" => {
                match tokio::process::Command::new("kill")
                    .args(&["-9", &pid.to_string()])
                    .output()
                    .await
                {
                    Ok(output) => {
                        if output.status.success() {
                            info!("Process {} killed successfully", pid);
                            ("success".to_string(), Some(serde_json::json!({"message": format!("Process {} killed", pid)})), None)
                        } else {
                            let stderr = String::from_utf8_lossy(&output.stderr);
                            error!("Kill process failed: {}", stderr);
                            let err = ErrorInfo {
                                code: "KILL_FAILED".to_string(),
                                message: format!("Command failed: {}", stderr),
                            };
                            ("error".to_string(), None, Some(err))
                        }
                    }
                    Err(e) => {
                        error!("Failed to execute kill: {}", e);
                        let err = ErrorInfo {
                            code: "EXECUTION_ERROR".to_string(),
                            message: format!("Failed to execute kill: {}", e),
                        };
                        ("error".to_string(), None, Some(err))
                    }
                }
            }
            _ => {
                let err = ErrorInfo {
                    code: "UNSUPPORTED_OS".to_string(),
                    message: format!("Kill process not supported on OS: {}", self.system_info.os),
                };
                ("error".to_string(), None, Some(err))
            }
        }
    }
    
    /// Execute shell command
    async fn execute_shell_command(&self, cmd: &IncomingCommand) -> (String, Option<serde_json::Value>, Option<ErrorInfo>) {
        info!("Executing shell command...");
        
        let command = match cmd.parameters.as_ref()
            .and_then(|p| p.get("command"))
            .and_then(|p| p.as_str()) {
            Some(command) => command,
            None => {
                let err = ErrorInfo {
                    code: "INVALID_PARAMETERS".to_string(),
                    message: "Missing 'command' parameter".to_string(),
                };
                return ("error".to_string(), None, Some(err));
            }
        };
        
        // Security check - only allow safe commands
        let safe_commands = ["dir", "ls", "whoami", "hostname", "date", "uptime", "ps", "tasklist"];
        let is_safe = safe_commands.iter().any(|&safe_cmd| command.starts_with(safe_cmd));
        
        if !is_safe {
            let err = ErrorInfo {
                code: "UNSAFE_COMMAND".to_string(),
                message: format!("Command not allowed: {}", command),
            };
            return ("error".to_string(), None, Some(err));
        }
        
        match self.system_info.os.as_str() {
            "windows" => {
                match tokio::process::Command::new("cmd")
                    .args(&["/C", command])
                    .output()
                    .await
                {
                    Ok(output) => {
                        let stdout = String::from_utf8_lossy(&output.stdout);
                        let stderr = String::from_utf8_lossy(&output.stderr);
                        
                        if output.status.success() {
                            info!("Shell command executed successfully");
                            ("success".to_string(), Some(serde_json::json!({
                                "stdout": stdout,
                                "stderr": stderr,
                                "exit_code": output.status.code()
                            })), None)
                        } else {
                            error!("Shell command failed: {}", stderr);
                            let err = ErrorInfo {
                                code: "COMMAND_FAILED".to_string(),
                                message: format!("Command failed with exit code: {:?}", output.status.code()),
                            };
                            ("error".to_string(), Some(serde_json::json!({
                                "stdout": stdout,
                                "stderr": stderr,
                                "exit_code": output.status.code()
                            })), Some(err))
                        }
                    }
                    Err(e) => {
                        error!("Failed to execute shell command: {}", e);
                        let err = ErrorInfo {
                            code: "EXECUTION_ERROR".to_string(),
                            message: format!("Failed to execute command: {}", e),
                        };
                        ("error".to_string(), None, Some(err))
                    }
                }
            }
            "linux" => {
                match tokio::process::Command::new("sh")
                    .args(&["-c", command])
                    .output()
                    .await
                {
                    Ok(output) => {
                        let stdout = String::from_utf8_lossy(&output.stdout);
                        let stderr = String::from_utf8_lossy(&output.stderr);
                        
                        if output.status.success() {
                            info!("Shell command executed successfully");
                            ("success".to_string(), Some(serde_json::json!({
                                "stdout": stdout,
                                "stderr": stderr,
                                "exit_code": output.status.code()
                            })), None)
                        } else {
                            error!("Shell command failed: {}", stderr);
                            let err = ErrorInfo {
                                code: "COMMAND_FAILED".to_string(),
                                message: format!("Command failed with exit code: {:?}", output.status.code()),
                            };
                            ("error".to_string(), Some(serde_json::json!({
                                "stdout": stdout,
                                "stderr": stderr,
                                "exit_code": output.status.code()
                            })), Some(err))
                        }
                    }
                    Err(e) => {
                        error!("Failed to execute shell command: {}", e);
                        let err = ErrorInfo {
                            code: "EXECUTION_ERROR".to_string(),
                            message: format!("Failed to execute command: {}", e),
                        };
                        ("error".to_string(), None, Some(err))
                    }
                }
            }
            _ => {
                let err = ErrorInfo {
                    code: "UNSUPPORTED_OS".to_string(),
                    message: format!("Shell commands not supported on OS: {}", self.system_info.os),
                };
                ("error".to_string(), None, Some(err))
            }
        }
    }
    
    /// Execute get metrics command
    async fn execute_get_metrics(&self, _cmd: &IncomingCommand) -> (String, Option<serde_json::Value>, Option<ErrorInfo>) {
        info!("Collecting system metrics...");
        
        match metrics::SystemMetrics::collect().await {
            Ok(system_metrics) => {
                let process_info = metrics::ProcessInfo::collect().await.ok();
                let services = metrics::ServiceStatus::collect_critical().await.ok();
                
                let metrics_data = serde_json::json!({
                    "system": system_metrics,
                    "processes": process_info,
                    "services": services,
                    "timestamp": Utc::now()
                });
                
                ("success".to_string(), Some(metrics_data), None)
            }
            Err(e) => {
                error!("Failed to collect metrics: {}", e);
                let err = ErrorInfo {
                    code: "METRICS_ERROR".to_string(),
                    message: format!("Failed to collect metrics: {}", e),
                };
                ("error".to_string(), None, Some(err))
            }
        }
    }
    
    /// Get agent capabilities based on OS and available features
    fn get_capabilities(&self) -> Vec<String> {
        let mut capabilities = vec![
            "system_metrics".to_string(),
        ];
        
        // Add OS-specific capabilities
        match self.system_info.os.as_str() {
            "linux" => {
                capabilities.extend_from_slice(&[
                    "power_management".to_string(),
                    "process_control".to_string(),
                    "command_execution".to_string(),
                    "service_management".to_string(),
                ]);
            }
            "windows" => {
                capabilities.extend_from_slice(&[
                    "power_management".to_string(),
                    "process_control".to_string(),
                    "command_execution".to_string(),
                    "service_management".to_string(),
                ]);
            }
            "android" => {
                capabilities.extend_from_slice(&[
                    "process_control".to_string(),
                    "command_execution".to_string(),
                ]);
            }
            _ => {
                warn!("Unknown OS: {}, limited capabilities", self.system_info.os);
            }
        }
        
        capabilities
    }
}

#[tokio::main]
async fn main() -> Result<()> {
    // Initialize logging
    tracing_subscriber::fmt()
        // .with_env_filter(
        //     std::env::var("RUST_LOG")
        //         .unwrap_or_else(|_| "symbion_agent_host=info".to_string())
        // )
        .init();
        
    info!("🤖 Symbion Agent Host starting...");
    
    // Create and run agent
    let mut agent = Agent::new().await
        .context("Failed to create agent")?;
        
    agent.run().await
        .context("Agent execution failed")?;
        
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[tokio::test]
    async fn test_system_discovery() {
        let system_info = SystemInfo::discover().await.unwrap();
        assert!(!system_info.agent_id.is_empty());
        assert!(!system_info.hostname.is_empty());
        assert!(!system_info.network.interfaces.is_empty());
    }
}