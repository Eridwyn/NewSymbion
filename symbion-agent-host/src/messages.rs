//! MQTT message contracts for Symbion Agent Host
//!
//! All serializable/deserializable structs for MQTT communication
//! matching the kernel's agents.* topic contracts.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use crate::discovery;
use crate::metrics;

/// Agent registration message (matches agents.registration@v1 contract)
#[derive(Debug, Serialize)]
pub struct RegistrationMessage {
    pub agent_id: String,
    pub hostname: String,
    pub os: String,
    pub architecture: String,
    pub capabilities: Vec<String>,
    pub network: discovery::NetworkInfo,
    pub version: String,
    pub timestamp: DateTime<Utc>,
}

/// Agent heartbeat message (matches agents.heartbeat@v1 contract)
#[derive(Debug, Serialize)]
pub struct HeartbeatMessage {
    pub agent_id: String,
    pub status: String,
    pub system: metrics::SystemMetrics,
    pub processes: Option<metrics::ProcessInfo>,
    pub services: Option<Vec<metrics::ServiceStatus>>,
    pub last_command: Option<CommandInfo>,
    pub timestamp: DateTime<Utc>,
}

/// Command information for heartbeat
#[derive(Debug, Clone, Serialize)]
pub struct CommandInfo {
    pub command_id: String,
    pub command_type: String,
    pub status: String,
    pub timestamp: DateTime<Utc>,
}

/// Incoming command from kernel (matches agents.command@v1 contract)
#[derive(Debug, Deserialize)]
pub struct IncomingCommand {
    pub command_id: String,
    pub agent_id: String,
    pub command_type: String,
    pub parameters: Option<serde_json::Value>,
    pub timestamp: DateTime<Utc>,
    pub requester: Option<String>,
}

/// Command response to kernel (matches agents.response@v1 contract)
#[derive(Debug, Serialize)]
pub struct CommandResponse {
    pub command_id: String,
    pub agent_id: String,
    pub status: String,
    pub output: Option<serde_json::Value>,
    pub error: Option<ErrorInfo>,
    pub execution_time_ms: u128,
    pub timestamp: DateTime<Utc>,
}

/// Error information for failed commands
#[derive(Debug, Serialize, Clone)]
pub struct ErrorInfo {
    pub code: String,
    pub message: String,
}

/// Received command for internal processing (from MQTT event loop to agent)
#[derive(Debug, Clone)]
pub struct ReceivedCommand {
    pub topic: String,
    pub payload: String,
}
