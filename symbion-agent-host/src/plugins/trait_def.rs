//! Plugin trait definition for Symbion Agent Host
//!
//! All agent plugins implement this trait to provide:
//! - Initialization and shutdown lifecycle
//! - Periodic tick data (included in heartbeat)
//! - Command handling for plugin-specific actions

use anyhow::Result;
use async_trait::async_trait;
use serde_json::Value;

/// Trait for agent plugins (in-process, trait objects)
#[async_trait]
pub trait AgentPlugin: Send + Sync {
    /// Unique plugin identifier
    fn id(&self) -> &str;

    /// Human-readable plugin name
    fn name(&self) -> &str;

    /// Initialize the plugin
    async fn init(&mut self) -> Result<()>;

    /// Periodic tick called each heartbeat cycle.
    /// Returns optional JSON data to include in heartbeat's plugin_data.
    async fn tick(&self) -> Result<Option<Value>>;

    /// Handle a plugin-specific command
    async fn handle_command(&self, action: &str, params: Option<&Value>) -> Result<Value>;

    /// Shutdown the plugin gracefully
    async fn shutdown(&self) -> Result<()>;
}
