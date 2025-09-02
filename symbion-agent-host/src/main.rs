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

/// Main agent state
struct Agent {
    config: AgentConfig,
    system_info: SystemInfo,
    mqtt_client: AsyncClient,
    last_command: Option<CommandInfo>,
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
        
        // Start MQTT event loop in background
        tokio::spawn(async move {
            loop {
                match eventloop.poll().await {
                    Ok(Event::Incoming(Incoming::Publish(publish))) => {
                        debug!("Received MQTT message on topic: {}", publish.topic);
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
                
                // TODO: Handle incoming MQTT commands
                // This will be implemented when we add command processing
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