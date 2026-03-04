//! MQTT message contracts for Symbion Agent Host
//!
//! All serializable/deserializable structs for MQTT communication
//! matching the kernel's agents.* topic contracts.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use crate::discovery;
use crate::metrics;
use crate::watchdog;

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
    #[serde(skip_serializing_if = "Option::is_none")]
    pub watchdog: Option<watchdog::WatchdogReport>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub plugin_data: Option<std::collections::HashMap<String, serde_json::Value>>,
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
#[allow(dead_code)] // Fields deserialized from kernel JSON
pub struct IncomingCommand {
    pub command_id: String,
    pub agent_id: String,
    pub command_type: String,
    pub parameters: Option<serde_json::Value>,
    /// Accept any timestamp format (kernel uses `time` crate ISO 8601, agent uses chrono)
    pub timestamp: String,
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
#[allow(dead_code)] // topic used for routing in future extensions
pub struct ReceivedCommand {
    pub topic: String,
    pub payload: String,
}

/// Log entry forwarded from agent to kernel
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LogEntry {
    pub timestamp: DateTime<Utc>,
    pub level: String,     // WARN, ERROR, CRITICAL, etc.
    pub message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub module: Option<String>,
    /// Source of the log: "agent" (default), "os_journal", "event_viewer"
    #[serde(skip_serializing_if = "Option::is_none")]
    pub source: Option<String>,
}

/// Batch of log entries sent to kernel via MQTT
#[derive(Debug, Serialize, Deserialize)]
pub struct LogMessage {
    pub agent_id: String,
    pub entries: Vec<LogEntry>,
    pub timestamp: DateTime<Utc>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_incoming_command_deserialize() {
        let json = r#"{
            "command_id": "cmd-001",
            "agent_id": "agent-abc",
            "command_type": "get_metrics",
            "parameters": null,
            "timestamp": "2026-02-28T12:00:00Z"
        }"#;
        let cmd: IncomingCommand = serde_json::from_str(json).unwrap();
        assert_eq!(cmd.command_id, "cmd-001");
        assert_eq!(cmd.command_type, "get_metrics");
        assert!(cmd.parameters.is_none());
        assert!(cmd.requester.is_none());
    }

    #[test]
    fn test_incoming_command_with_parameters() {
        let json = r#"{
            "command_id": "cmd-002",
            "agent_id": "agent-abc",
            "command_type": "run_command",
            "parameters": {"command": "ls -la", "timeout": 30},
            "timestamp": "2026-02-28T12:00:00Z",
            "requester": "admin"
        }"#;
        let cmd: IncomingCommand = serde_json::from_str(json).unwrap();
        assert_eq!(cmd.command_type, "run_command");
        let params = cmd.parameters.unwrap();
        assert_eq!(params["command"].as_str().unwrap(), "ls -la");
        assert_eq!(params["timeout"].as_u64().unwrap(), 30);
        assert_eq!(cmd.requester.unwrap(), "admin");
    }

    #[test]
    fn test_command_response_serialize() {
        let response = CommandResponse {
            command_id: "cmd-001".to_string(),
            agent_id: "agent-abc".to_string(),
            status: "success".to_string(),
            output: Some(serde_json::json!({"message": "done"})),
            error: None,
            execution_time_ms: 42,
            timestamp: Utc::now(),
        };
        let json = serde_json::to_string(&response).unwrap();
        assert!(json.contains("\"status\":\"success\""));
        assert!(json.contains("\"execution_time_ms\":42"));
    }

    #[test]
    fn test_command_response_with_error() {
        let response = CommandResponse {
            command_id: "cmd-002".to_string(),
            agent_id: "agent-abc".to_string(),
            status: "error".to_string(),
            output: None,
            error: Some(ErrorInfo {
                code: "TIMEOUT".to_string(),
                message: "Command timed out".to_string(),
            }),
            execution_time_ms: 30000,
            timestamp: Utc::now(),
        };
        let json = serde_json::to_string(&response).unwrap();
        assert!(json.contains("TIMEOUT"));
        assert!(json.contains("Command timed out"));
    }

    #[test]
    fn test_error_info_serialize() {
        let err = ErrorInfo {
            code: "NOT_FOUND".to_string(),
            message: "Resource not found".to_string(),
        };
        let json = serde_json::to_value(&err).unwrap();
        assert_eq!(json["code"], "NOT_FOUND");
        assert_eq!(json["message"], "Resource not found");
    }

    #[test]
    fn test_command_info_serialize() {
        let info = CommandInfo {
            command_id: "cmd-123".to_string(),
            command_type: "shutdown".to_string(),
            status: "success".to_string(),
            timestamp: Utc::now(),
        };
        let json = serde_json::to_value(&info).unwrap();
        assert_eq!(json["command_type"], "shutdown");
        assert!(json["timestamp"].is_string());
    }

    #[test]
    fn test_log_entry_serialization() {
        let entry = LogEntry {
            timestamp: Utc::now(),
            level: "ERROR".to_string(),
            message: "disk full".to_string(),
            module: Some("metrics".to_string()),
            source: None,
        };
        let json = serde_json::to_string(&entry).unwrap();
        assert!(json.contains("ERROR"));
        assert!(json.contains("disk full"));
        let parsed: LogEntry = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed.level, "ERROR");
    }

    #[test]
    fn test_log_message_serialization() {
        let msg = LogMessage {
            agent_id: "test-agent".to_string(),
            entries: vec![
                LogEntry {
                    timestamp: Utc::now(),
                    level: "WARN".to_string(),
                    message: "high cpu".to_string(),
                    module: None,
                    source: None,
                },
            ],
            timestamp: Utc::now(),
        };
        let json = serde_json::to_string(&msg).unwrap();
        assert!(json.contains("test-agent"));
        assert!(json.contains("high cpu"));
    }

    #[test]
    fn test_log_message_empty_entries() {
        let msg = LogMessage {
            agent_id: "agent-x".to_string(),
            entries: vec![],
            timestamp: Utc::now(),
        };
        let json = serde_json::to_string(&msg).unwrap();
        let parsed: LogMessage = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed.entries.len(), 0);
    }
}
