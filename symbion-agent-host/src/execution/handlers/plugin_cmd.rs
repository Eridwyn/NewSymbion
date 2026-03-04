//! Plugin command handler
//!
//! Handles: plugin_command, list_plugins

use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;

use serde_json::Value;
use tokio::sync::Mutex;

use crate::execution::handler::{CommandHandler, CommandResult};
use crate::plugins::AgentPluginRegistry;

/// Handler for plugin management commands
pub struct PluginCommandHandler {
    registry: Arc<Mutex<AgentPluginRegistry>>,
}

impl PluginCommandHandler {
    pub fn new(registry: Arc<Mutex<AgentPluginRegistry>>) -> Self {
        Self { registry }
    }
}

impl CommandHandler for PluginCommandHandler {
    fn command_types(&self) -> &[&str] {
        &["plugin_command", "list_plugins"]
    }

    fn execute<'a>(
        &'a self,
        command_type: &'a str,
        params: Option<&'a Value>,
    ) -> Pin<Box<dyn Future<Output = CommandResult> + Send + 'a>> {
        Box::pin(async move {
            match command_type {
                "list_plugins" => {
                    let registry = self.registry.lock().await;
                    let ids = registry.list_ids();
                    CommandResult::success(serde_json::json!({
                        "plugins": ids,
                        "count": ids.len(),
                    }))
                }
                "plugin_command" => {
                    let params = match params {
                        Some(p) => p,
                        None => return CommandResult::error("MISSING_PARAMS", "plugin_command requires parameters"),
                    };

                    let plugin_id = match params.get("plugin_id").and_then(|v| v.as_str()) {
                        Some(id) => id,
                        None => return CommandResult::error("MISSING_PLUGIN_ID", "plugin_command requires 'plugin_id'"),
                    };

                    let action = match params.get("action").and_then(|v| v.as_str()) {
                        Some(a) => a,
                        None => return CommandResult::error("MISSING_ACTION", "plugin_command requires 'action'"),
                    };

                    let cmd_params = params.get("parameters");
                    let registry = self.registry.lock().await;

                    match registry.handle_command(plugin_id, action, cmd_params).await {
                        Ok(result) => CommandResult::success(result),
                        Err(e) => CommandResult::error("PLUGIN_ERROR", e.to_string()),
                    }
                }
                _ => CommandResult::error("UNKNOWN_COMMAND", "Unknown plugin command"),
            }
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_handler() -> PluginCommandHandler {
        PluginCommandHandler::new(Arc::new(Mutex::new(AgentPluginRegistry::new())))
    }

    #[tokio::test]
    async fn test_list_plugins_empty() {
        let handler = make_handler();
        let result = handler.execute("list_plugins", None).await;
        assert_eq!(result.status, "success");
        assert_eq!(result.data.as_ref().unwrap()["count"], 0);
    }

    #[tokio::test]
    async fn test_plugin_command_missing_params() {
        let handler = make_handler();
        let result = handler.execute("plugin_command", None).await;
        assert_eq!(result.status, "error");
    }

    #[tokio::test]
    async fn test_plugin_command_missing_plugin_id() {
        let handler = make_handler();
        let params = serde_json::json!({"action": "test"});
        let result = handler.execute("plugin_command", Some(&params)).await;
        assert_eq!(result.error.unwrap().code, "MISSING_PLUGIN_ID");
    }

    #[tokio::test]
    async fn test_plugin_command_unknown_plugin() {
        let handler = make_handler();
        let params = serde_json::json!({"plugin_id": "nope", "action": "test"});
        let result = handler.execute("plugin_command", Some(&params)).await;
        assert_eq!(result.status, "error");
    }

    #[test]
    fn test_handler_command_types() {
        let handler = make_handler();
        let types = handler.command_types();
        assert!(types.contains(&"plugin_command"));
        assert!(types.contains(&"list_plugins"));
    }
}
