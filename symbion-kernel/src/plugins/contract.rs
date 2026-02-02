//! Plugin Contract v1.0 - Symbion Plugin System
//!
//! This module defines the core contract between the Kernel and plugins.
//!
//! # Fundamental Rules
//!
//! ## Rule #1: Action vs Event
//! - **Action**: Kernel → Plugin, requires ACK, goes through Decision Engine
//! - **Event**: Plugin → Kernel, best-effort, informational only
//!
//! ## Rule #2: Plugin ≠ Decision Maker
//! A plugin can NEVER modify global state without going through an Action
//! evaluated by the Decision Engine.

use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// Current specification version
pub const SPEC_VERSION: &str = "1.0";

// ============================================================================
// Impact Levels
// ============================================================================

/// Impact level of an action, used by Decision Engine for evaluation
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ImpactLevel {
    /// Reversible action, minimal impact
    /// Auto-approve if trust > 0.5
    Low,

    /// Moderate impact, potentially reversible
    /// Requires trust > 0.7
    Medium,

    /// Significant impact, irreversible
    /// Requires explicit validation
    High,

    /// Critical system impact
    /// Always requires manual validation
    Critical,
}

impl Default for ImpactLevel {
    fn default() -> Self {
        Self::Low
    }
}

// ============================================================================
// Plugin Manifest
// ============================================================================

/// Plugin manifest declaring capabilities and metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PluginManifest {
    /// Contract version (must be "1.0")
    pub spec_version: String,

    /// Unique plugin identifier (kebab-case)
    pub plugin_id: String,

    /// Human-readable plugin name
    pub name: String,

    /// Plugin version (semver)
    pub version: String,

    /// Plugin description
    #[serde(default)]
    pub description: String,

    /// List of capabilities (actions this plugin can handle)
    #[serde(default)]
    pub capabilities: Vec<Capability>,

    /// List of events this plugin can emit
    #[serde(default)]
    pub events: Vec<EventDeclaration>,

    /// Health check endpoint path (default: "/health")
    #[serde(default = "default_health_endpoint")]
    pub health_endpoint: String,

    /// Unix socket path for HTTP communication
    pub socket_path: String,
}

fn default_health_endpoint() -> String {
    "/health".to_string()
}

/// A capability declaration (action type the plugin can handle)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Capability {
    /// Action type identifier (e.g., "create_note")
    pub action_type: String,

    /// Human-readable description
    #[serde(default)]
    pub description: String,

    /// Impact level for Decision Engine
    #[serde(default)]
    pub impact_level: ImpactLevel,

    /// Parameter schema (JSON Schema subset)
    #[serde(default)]
    pub parameters: serde_json::Value,
}

/// An event declaration (event type the plugin can emit)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EventDeclaration {
    /// Event type identifier (e.g., "note_created")
    pub event_type: String,

    /// Human-readable description
    #[serde(default)]
    pub description: String,
}

// ============================================================================
// Action Request (Kernel → Plugin)
// ============================================================================

/// Action request sent from Kernel to Plugin
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ActionRequest {
    /// Contract version
    pub spec_version: String,

    /// Unique action identifier
    pub action_id: Uuid,

    /// Action type (must match a capability)
    pub action_type: String,

    /// Action-specific payload
    pub payload: serde_json::Value,

    /// Execution metadata
    #[serde(default)]
    pub metadata: ActionMetadata,
}

/// Metadata about action execution context
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ActionMetadata {
    /// Automation that triggered this action (if any)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub automation_id: Option<Uuid>,

    /// How this action was triggered
    #[serde(default)]
    pub triggered_by: TriggerSource,

    /// Timestamp of request (ISO 8601)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub timestamp: Option<String>,
}

/// Source that triggered an action
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum TriggerSource {
    /// Triggered by a schedule
    Schedule,

    /// Triggered by an event
    Event,

    /// Triggered manually (API call, UI)
    #[default]
    Manual,
}

impl ActionRequest {
    /// Create a new action request with current spec version
    pub fn new(action_type: impl Into<String>, payload: serde_json::Value) -> Self {
        Self {
            spec_version: SPEC_VERSION.to_string(),
            action_id: Uuid::new_v4(),
            action_type: action_type.into(),
            payload,
            metadata: ActionMetadata::default(),
        }
    }

    /// Set the automation ID that triggered this action
    pub fn with_automation(mut self, automation_id: Uuid) -> Self {
        self.metadata.automation_id = Some(automation_id);
        self
    }

    /// Set the trigger source
    pub fn with_trigger(mut self, source: TriggerSource) -> Self {
        self.metadata.triggered_by = source;
        self
    }
}

// ============================================================================
// Action Response (Plugin → Kernel)
// ============================================================================

/// Action response sent from Plugin to Kernel (HTTP response = ACK)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ActionResponse {
    /// Contract version
    pub spec_version: String,

    /// Action ID being responded to
    pub action_id: Uuid,

    /// Execution status
    pub status: ActionStatus,

    /// Result data (on success)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub result: Option<serde_json::Value>,

    /// Error details (on error/rejected)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<ActionError>,

    /// Execution time in milliseconds
    #[serde(skip_serializing_if = "Option::is_none")]
    pub execution_time_ms: Option<u64>,
}

/// Action execution status
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ActionStatus {
    /// Action executed successfully
    Success,

    /// Technical error (retry may work)
    Error,

    /// Action rejected by plugin (no retry)
    Rejected,
}

/// Error details for failed actions
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ActionError {
    /// Error code (plugin-specific)
    pub code: String,

    /// Human-readable error message
    pub message: String,

    /// Whether this error is retryable
    #[serde(default)]
    pub retryable: bool,
}

impl ActionResponse {
    /// Create a success response
    pub fn success(action_id: Uuid, result: serde_json::Value) -> Self {
        Self {
            spec_version: SPEC_VERSION.to_string(),
            action_id,
            status: ActionStatus::Success,
            result: Some(result),
            error: None,
            execution_time_ms: None,
        }
    }

    /// Create an error response
    pub fn error(action_id: Uuid, code: impl Into<String>, message: impl Into<String>) -> Self {
        Self {
            spec_version: SPEC_VERSION.to_string(),
            action_id,
            status: ActionStatus::Error,
            result: None,
            error: Some(ActionError {
                code: code.into(),
                message: message.into(),
                retryable: true,
            }),
            execution_time_ms: None,
        }
    }

    /// Create a rejected response
    pub fn rejected(action_id: Uuid, code: impl Into<String>, message: impl Into<String>) -> Self {
        Self {
            spec_version: SPEC_VERSION.to_string(),
            action_id,
            status: ActionStatus::Rejected,
            result: None,
            error: Some(ActionError {
                code: code.into(),
                message: message.into(),
                retryable: false,
            }),
            execution_time_ms: None,
        }
    }

    /// Set execution time
    pub fn with_execution_time(mut self, ms: u64) -> Self {
        self.execution_time_ms = Some(ms);
        self
    }
}

// ============================================================================
// Event Message (Plugin → Kernel)
// ============================================================================

/// Event message sent from Plugin to Kernel via MQTT
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EventMessage {
    /// Contract version
    pub spec_version: String,

    /// Event type
    pub event_type: String,

    /// Plugin that emitted this event
    pub plugin_id: String,

    /// Event-specific payload
    pub payload: serde_json::Value,

    /// Timestamp (ISO 8601 UTC)
    pub timestamp: String,
}

impl EventMessage {
    /// Create a new event message
    pub fn new(
        plugin_id: impl Into<String>,
        event_type: impl Into<String>,
        payload: serde_json::Value,
    ) -> Self {
        use time::OffsetDateTime;
        use time::format_description::well_known::Rfc3339;

        Self {
            spec_version: SPEC_VERSION.to_string(),
            event_type: event_type.into(),
            plugin_id: plugin_id.into(),
            payload,
            timestamp: OffsetDateTime::now_utc()
                .format(&Rfc3339)
                .unwrap_or_else(|_| "unknown".to_string()),
        }
    }
}

// ============================================================================
// Health Status
// ============================================================================

/// Health status message published by plugins
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HealthStatus {
    /// Contract version
    pub spec_version: String,

    /// Plugin identifier
    pub plugin_id: String,

    /// Current status
    pub status: PluginStatus,

    /// Uptime in seconds
    #[serde(default)]
    pub uptime_seconds: u64,

    /// Last action processed timestamp
    #[serde(skip_serializing_if = "Option::is_none")]
    pub last_action_at: Option<String>,
}

/// Plugin operational status
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum PluginStatus {
    /// Plugin is healthy and accepting actions
    Healthy,

    /// Plugin is degraded but functional
    Degraded,

    /// Plugin is unhealthy
    Unhealthy,

    /// Plugin is shutting down
    Stopping,
}

// ============================================================================
// MQTT Topic Helpers
// ============================================================================

/// Generate MQTT topic paths for a plugin
pub mod topics {
    /// Actions topic (Kernel → Plugin commands)
    pub fn actions(plugin_id: &str) -> String {
        format!("symbion/plugins/{}/actions", plugin_id)
    }

    /// Events topic (Plugin → Kernel informational)
    pub fn events(plugin_id: &str) -> String {
        format!("symbion/plugins/{}/events", plugin_id)
    }

    /// Health topic (Plugin → Kernel heartbeat)
    pub fn health(plugin_id: &str) -> String {
        format!("symbion/plugins/{}/health", plugin_id)
    }

    /// Manifest topic (Plugin → Kernel at startup)
    pub fn manifest(plugin_id: &str) -> String {
        format!("symbion/plugins/{}/manifest", plugin_id)
    }
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_action_request_serialization() {
        let request = ActionRequest::new(
            "create_note",
            serde_json::json!({
                "title": "Test Note",
                "content": "Hello World"
            }),
        );

        let json = serde_json::to_string(&request).unwrap();
        assert!(json.contains("\"spec_version\":\"1.0\""));
        assert!(json.contains("\"action_type\":\"create_note\""));
    }

    #[test]
    fn test_action_response_success() {
        let response = ActionResponse::success(
            Uuid::new_v4(),
            serde_json::json!({"note_id": "123"}),
        );

        assert_eq!(response.status, ActionStatus::Success);
        assert!(response.error.is_none());
    }

    #[test]
    fn test_action_response_error() {
        let response = ActionResponse::error(
            Uuid::new_v4(),
            "STORAGE_ERROR",
            "Failed to save note",
        );

        assert_eq!(response.status, ActionStatus::Error);
        assert!(response.error.as_ref().unwrap().retryable);
    }

    #[test]
    fn test_impact_level_default() {
        assert_eq!(ImpactLevel::default(), ImpactLevel::Low);
    }

    #[test]
    fn test_topics() {
        assert_eq!(topics::actions("notes"), "symbion/plugins/notes/actions");
        assert_eq!(topics::events("notes"), "symbion/plugins/notes/events");
        assert_eq!(topics::health("hue-lights"), "symbion/plugins/hue-lights/health");
    }
}
