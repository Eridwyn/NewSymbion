//! Plugin Registry - Session 2
//!
//! Minimal registry for plugin state tracking and capability validation.
//!
//! # Design Principles
//! - No magic routing
//! - No orchestration
//! - Simple state machine: available → degraded → offline
//! - Explicit capability checking before dispatch

use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;

use super::contract::{Capability, PluginManifest, PluginStatus};

/// Plugin operational state in the registry
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PluginState {
    /// Plugin is healthy and accepting actions
    Available,
    /// Plugin responded but reported degraded status
    Degraded,
    /// Plugin is not responding or explicitly offline
    Offline,
}

impl From<PluginStatus> for PluginState {
    fn from(status: PluginStatus) -> Self {
        match status {
            PluginStatus::Healthy => PluginState::Available,
            PluginStatus::Degraded => PluginState::Degraded,
            PluginStatus::Unhealthy | PluginStatus::Stopping => PluginState::Offline,
        }
    }
}

/// Registered plugin entry with state and capabilities
#[derive(Debug, Clone)]
pub struct RegisteredPlugin {
    /// Plugin manifest (immutable after registration)
    pub manifest: PluginManifest,
    /// Current operational state
    pub state: PluginState,
    /// Last health check timestamp (Unix epoch seconds)
    pub last_health_at: u64,
    /// Consecutive health check failures
    pub health_failures: u32,
}

impl RegisteredPlugin {
    /// Create a new registered plugin from manifest
    pub fn new(manifest: PluginManifest) -> Self {
        Self {
            manifest,
            state: PluginState::Available,
            last_health_at: current_timestamp(),
            health_failures: 0,
        }
    }

    /// Check if plugin can handle a specific action type
    pub fn can_handle(&self, action_type: &str) -> bool {
        self.manifest
            .capabilities
            .iter()
            .any(|c| c.action_type == action_type)
    }

    /// Get capability for an action type
    pub fn get_capability(&self, action_type: &str) -> Option<&Capability> {
        self.manifest
            .capabilities
            .iter()
            .find(|c| c.action_type == action_type)
    }

    /// Check if plugin is available for dispatch
    pub fn is_available(&self) -> bool {
        self.state == PluginState::Available
    }

    /// Check if plugin can accept actions (available or degraded)
    pub fn accepts_actions(&self) -> bool {
        matches!(self.state, PluginState::Available | PluginState::Degraded)
    }
}

/// Health check failure threshold before marking offline
const HEALTH_FAILURE_THRESHOLD: u32 = 3;

/// Plugin Registry
///
/// Thread-safe registry for managing plugin lifecycle and capabilities.
#[derive(Debug)]
pub struct PluginRegistry {
    /// Registered plugins by ID
    plugins: HashMap<String, RegisteredPlugin>,
}

impl PluginRegistry {
    /// Create a new empty registry
    pub fn new() -> Self {
        Self {
            plugins: HashMap::new(),
        }
    }

    /// Register a plugin from its manifest
    ///
    /// Returns error if plugin ID is already registered.
    pub fn register(&mut self, manifest: PluginManifest) -> Result<(), RegistryError> {
        let plugin_id = manifest.plugin_id.clone();

        if self.plugins.contains_key(&plugin_id) {
            return Err(RegistryError::AlreadyRegistered(plugin_id));
        }

        println!(
            "[plugins] registered plugin '{}' with {} capabilities",
            plugin_id,
            manifest.capabilities.len()
        );

        self.plugins.insert(plugin_id, RegisteredPlugin::new(manifest));
        Ok(())
    }

    /// Unregister a plugin
    pub fn unregister(&mut self, plugin_id: &str) -> Option<RegisteredPlugin> {
        let removed = self.plugins.remove(plugin_id);
        if removed.is_some() {
            println!("[plugins] unregistered plugin '{}'", plugin_id);
        }
        removed
    }

    /// Get a registered plugin by ID
    pub fn get(&self, plugin_id: &str) -> Option<&RegisteredPlugin> {
        self.plugins.get(plugin_id)
    }

    /// Get mutable reference to a registered plugin
    pub fn get_mut(&mut self, plugin_id: &str) -> Option<&mut RegisteredPlugin> {
        self.plugins.get_mut(plugin_id)
    }

    /// List all registered plugins
    pub fn list(&self) -> Vec<&RegisteredPlugin> {
        self.plugins.values().collect()
    }

    /// List plugins by state
    pub fn list_by_state(&self, state: PluginState) -> Vec<&RegisteredPlugin> {
        self.plugins
            .values()
            .filter(|p| p.state == state)
            .collect()
    }

    /// Find plugin that can handle an action type
    ///
    /// Returns None if no plugin can handle the action or plugin is offline.
    pub fn find_handler(&self, action_type: &str) -> Option<&RegisteredPlugin> {
        self.plugins
            .values()
            .find(|p| p.accepts_actions() && p.can_handle(action_type))
    }

    /// Validate dispatch: check if action can be sent to plugin
    ///
    /// Returns error if:
    /// - Plugin not registered
    /// - Plugin is offline
    /// - Plugin doesn't have capability for this action
    pub fn validate_dispatch(
        &self,
        plugin_id: &str,
        action_type: &str,
    ) -> Result<&Capability, DispatchError> {
        let plugin = self
            .plugins
            .get(plugin_id)
            .ok_or_else(|| DispatchError::PluginNotFound(plugin_id.to_string()))?;

        if !plugin.accepts_actions() {
            return Err(DispatchError::PluginOffline(plugin_id.to_string()));
        }

        plugin
            .get_capability(action_type)
            .ok_or_else(|| DispatchError::CapabilityNotFound {
                plugin_id: plugin_id.to_string(),
                action_type: action_type.to_string(),
            })
    }

    /// Update plugin health status
    ///
    /// Called when health check response is received.
    pub fn update_health(&mut self, plugin_id: &str, status: PluginStatus) {
        if let Some(plugin) = self.plugins.get_mut(plugin_id) {
            let new_state = PluginState::from(status);
            let old_state = plugin.state;

            plugin.state = new_state;
            plugin.last_health_at = current_timestamp();

            if new_state == PluginState::Available {
                plugin.health_failures = 0;
            }

            if old_state != new_state {
                println!(
                    "[plugins] plugin '{}' state changed: {:?} -> {:?}",
                    plugin_id, old_state, new_state
                );
            }
        }
    }

    /// Record health check failure
    ///
    /// Marks plugin as offline after HEALTH_FAILURE_THRESHOLD consecutive failures.
    pub fn record_health_failure(&mut self, plugin_id: &str) {
        if let Some(plugin) = self.plugins.get_mut(plugin_id) {
            plugin.health_failures += 1;

            if plugin.health_failures >= HEALTH_FAILURE_THRESHOLD {
                if plugin.state != PluginState::Offline {
                    eprintln!(
                        "[plugins] plugin '{}' marked offline after {} health failures",
                        plugin_id, plugin.health_failures
                    );
                    plugin.state = PluginState::Offline;
                }
            }
        }
    }

    /// Get summary statistics
    pub fn stats(&self) -> RegistryStats {
        let total = self.plugins.len();
        let available = self.list_by_state(PluginState::Available).len();
        let degraded = self.list_by_state(PluginState::Degraded).len();
        let offline = self.list_by_state(PluginState::Offline).len();

        RegistryStats {
            total,
            available,
            degraded,
            offline,
        }
    }
}

impl Default for PluginRegistry {
    fn default() -> Self {
        Self::new()
    }
}

/// Shared plugin registry for async access
pub type SharedPluginRegistry = Arc<RwLock<PluginRegistry>>;

/// Create a new shared registry
pub fn new_shared_registry() -> SharedPluginRegistry {
    Arc::new(RwLock::new(PluginRegistry::new()))
}

/// Registry statistics
#[derive(Debug, Clone, Copy)]
pub struct RegistryStats {
    pub total: usize,
    pub available: usize,
    pub degraded: usize,
    pub offline: usize,
}

/// Registry operation errors
#[derive(Debug, Clone, thiserror::Error)]
pub enum RegistryError {
    #[error("Plugin '{0}' is already registered")]
    AlreadyRegistered(String),
}

/// Dispatch validation errors
#[derive(Debug, Clone, thiserror::Error)]
pub enum DispatchError {
    #[error("Plugin '{0}' not found in registry")]
    PluginNotFound(String),

    #[error("Plugin '{0}' is offline")]
    PluginOffline(String),

    #[error("Plugin '{plugin_id}' does not have capability for action '{action_type}'")]
    CapabilityNotFound {
        plugin_id: String,
        action_type: String,
    },
}

/// Get current Unix timestamp in seconds
fn current_timestamp() -> u64 {
    use std::time::{SystemTime, UNIX_EPOCH};
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0)
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use crate::plugins::contract::ImpactLevel;

    fn test_manifest(id: &str, actions: Vec<&str>) -> PluginManifest {
        PluginManifest {
            spec_version: "1.0".to_string(),
            plugin_id: id.to_string(),
            name: format!("Test Plugin {}", id),
            version: "1.0.0".to_string(),
            description: "Test".to_string(),
            capabilities: actions
                .into_iter()
                .map(|a| Capability {
                    action_type: a.to_string(),
                    description: String::new(),
                    impact_level: ImpactLevel::Low,
                    parameters: serde_json::Value::Null,
                })
                .collect(),
            events: vec![],
            health_endpoint: "/health".to_string(),
            socket_path: format!("/run/symbion-plugins/{}.sock", id),
        }
    }

    #[test]
    fn test_register_plugin() {
        let mut registry = PluginRegistry::new();
        let manifest = test_manifest("notes", vec!["create_note", "delete_note"]);

        assert!(registry.register(manifest.clone()).is_ok());
        assert!(registry.get("notes").is_some());

        // Duplicate registration fails
        assert!(matches!(
            registry.register(manifest),
            Err(RegistryError::AlreadyRegistered(_))
        ));
    }

    #[test]
    fn test_validate_dispatch() {
        let mut registry = PluginRegistry::new();
        registry
            .register(test_manifest("notes", vec!["create_note"]))
            .unwrap();

        // Valid dispatch
        assert!(registry.validate_dispatch("notes", "create_note").is_ok());

        // Unknown plugin
        assert!(matches!(
            registry.validate_dispatch("unknown", "create_note"),
            Err(DispatchError::PluginNotFound(_))
        ));

        // Unknown action
        assert!(matches!(
            registry.validate_dispatch("notes", "unknown_action"),
            Err(DispatchError::CapabilityNotFound { .. })
        ));
    }

    #[test]
    fn test_health_failures() {
        let mut registry = PluginRegistry::new();
        registry
            .register(test_manifest("notes", vec!["create_note"]))
            .unwrap();

        assert!(registry.get("notes").unwrap().is_available());

        // Failures below threshold
        registry.record_health_failure("notes");
        registry.record_health_failure("notes");
        assert!(registry.get("notes").unwrap().accepts_actions());

        // Third failure triggers offline
        registry.record_health_failure("notes");
        assert_eq!(registry.get("notes").unwrap().state, PluginState::Offline);

        // Recovery via health update
        registry.update_health("notes", PluginStatus::Healthy);
        assert!(registry.get("notes").unwrap().is_available());
    }

    #[test]
    fn test_find_handler() {
        let mut registry = PluginRegistry::new();
        registry
            .register(test_manifest("notes", vec!["create_note"]))
            .unwrap();
        registry
            .register(test_manifest("notifications", vec!["send_notification"]))
            .unwrap();

        assert!(registry.find_handler("create_note").is_some());
        assert!(registry.find_handler("send_notification").is_some());
        assert!(registry.find_handler("unknown").is_none());

        // Offline plugin not found
        registry.update_health("notes", PluginStatus::Unhealthy);
        assert!(registry.find_handler("create_note").is_none());
    }
}
