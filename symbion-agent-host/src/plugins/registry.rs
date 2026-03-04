//! Plugin registry for managing loaded agent plugins

use std::collections::HashMap;

use anyhow::Result;
use serde_json::Value;
use tracing::{error, info};

use super::trait_def::AgentPlugin;

/// Registry managing all loaded plugins
pub struct AgentPluginRegistry {
    plugins: HashMap<String, Box<dyn AgentPlugin>>,
}

impl AgentPluginRegistry {
    /// Create a new empty registry
    pub fn new() -> Self {
        Self {
            plugins: HashMap::new(),
        }
    }

    /// Register and initialize a plugin
    pub async fn register(&mut self, mut plugin: Box<dyn AgentPlugin>) -> Result<()> {
        let id = plugin.id().to_string();
        let name = plugin.name().to_string();

        match plugin.init().await {
            Ok(()) => {
                info!("[plugins] Loaded plugin '{}' ({})", name, id);
                self.plugins.insert(id, plugin);
                Ok(())
            }
            Err(e) => {
                error!("[plugins] Failed to initialize plugin '{}': {}", name, e);
                Err(e)
            }
        }
    }

    /// Tick all plugins and collect their data
    pub async fn tick_all(&self) -> HashMap<String, Value> {
        let mut data = HashMap::new();

        for (id, plugin) in &self.plugins {
            match plugin.tick().await {
                Ok(Some(value)) => {
                    data.insert(id.clone(), value);
                }
                Ok(None) => {}
                Err(e) => {
                    error!("[plugins] Tick failed for '{}': {}", id, e);
                }
            }
        }

        data
    }

    /// Handle a command for a specific plugin
    pub async fn handle_command(&self, plugin_id: &str, action: &str, params: Option<&Value>) -> Result<Value> {
        match self.plugins.get(plugin_id) {
            Some(plugin) => plugin.handle_command(action, params).await,
            None => anyhow::bail!("Plugin '{}' not found", plugin_id),
        }
    }

    /// Shutdown all plugins
    pub async fn shutdown_all(&self) {
        for (id, plugin) in &self.plugins {
            if let Err(e) = plugin.shutdown().await {
                error!("[plugins] Shutdown failed for '{}': {}", id, e);
            }
        }
    }

    /// List loaded plugin IDs
    pub fn list_ids(&self) -> Vec<&str> {
        self.plugins.keys().map(|s| s.as_str()).collect()
    }

    /// Get plugin count
    pub fn count(&self) -> usize {
        self.plugins.len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::plugins::trait_def::AgentPlugin;
    use async_trait::async_trait;

    struct MockPlugin {
        id: String,
    }

    #[async_trait]
    impl AgentPlugin for MockPlugin {
        fn id(&self) -> &str { &self.id }
        fn name(&self) -> &str { "Mock Plugin" }
        async fn init(&mut self) -> Result<()> { Ok(()) }
        async fn tick(&self) -> Result<Option<Value>> {
            Ok(Some(serde_json::json!({"mock": true})))
        }
        async fn handle_command(&self, action: &str, _params: Option<&Value>) -> Result<Value> {
            Ok(serde_json::json!({"action": action}))
        }
        async fn shutdown(&self) -> Result<()> { Ok(()) }
    }

    #[tokio::test]
    async fn test_register_plugin() {
        let mut registry = AgentPluginRegistry::new();
        registry.register(Box::new(MockPlugin { id: "mock".to_string() })).await.unwrap();
        assert_eq!(registry.count(), 1);
        assert!(registry.list_ids().contains(&"mock"));
    }

    #[tokio::test]
    async fn test_tick_all() {
        let mut registry = AgentPluginRegistry::new();
        registry.register(Box::new(MockPlugin { id: "mock".to_string() })).await.unwrap();
        let data = registry.tick_all().await;
        assert!(data.contains_key("mock"));
        assert_eq!(data["mock"]["mock"], true);
    }

    #[tokio::test]
    async fn test_handle_command() {
        let mut registry = AgentPluginRegistry::new();
        registry.register(Box::new(MockPlugin { id: "mock".to_string() })).await.unwrap();
        let result = registry.handle_command("mock", "test_action", None).await.unwrap();
        assert_eq!(result["action"], "test_action");
    }

    #[tokio::test]
    async fn test_handle_unknown_plugin() {
        let registry = AgentPluginRegistry::new();
        assert!(registry.handle_command("unknown", "test", None).await.is_err());
    }

    #[tokio::test]
    async fn test_shutdown_all() {
        let mut registry = AgentPluginRegistry::new();
        registry.register(Box::new(MockPlugin { id: "mock".to_string() })).await.unwrap();
        registry.shutdown_all().await; // Should not panic
    }
}
