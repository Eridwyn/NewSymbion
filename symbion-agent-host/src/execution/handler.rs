//! Trait-based command handler system for Symbion Agent
//!
//! Provides a `CommandHandler` trait and `CommandRegistry` for extensible
//! command dispatch without modifying the agent core loop.

use std::collections::HashMap;
use std::future::Future;
use std::pin::Pin;

use serde_json::Value;

use crate::messages::ErrorInfo;

/// Result of executing a command handler
pub struct CommandResult {
    pub status: String,
    pub data: Option<Value>,
    pub error: Option<ErrorInfo>,
}

impl CommandResult {
    /// Create a success result with data
    pub fn success(data: Value) -> Self {
        Self {
            status: "success".to_string(),
            data: Some(data),
            error: None,
        }
    }

    /// Create an error result
    pub fn error(code: &str, message: impl Into<String>) -> Self {
        Self {
            status: "error".to_string(),
            data: None,
            error: Some(ErrorInfo {
                code: code.to_string(),
                message: message.into(),
            }),
        }
    }

    /// Create an error result with partial data (e.g. failed command output)
    pub fn error_with_data(code: &str, message: impl Into<String>, data: Value) -> Self {
        Self {
            status: "error".to_string(),
            data: Some(data),
            error: Some(ErrorInfo {
                code: code.to_string(),
                message: message.into(),
            }),
        }
    }
}

/// Trait for command handlers.
///
/// Each handler declares which command types it supports and implements
/// the async execute logic. Handlers are registered in the `CommandRegistry`.
pub trait CommandHandler: Send + Sync {
    /// Command types this handler supports (e.g. ["shutdown", "reboot", "hibernate"])
    fn command_types(&self) -> &[&str];

    /// Execute the command with the given parameters.
    /// The `command_type` is provided for handlers that support multiple types.
    fn execute<'a>(
        &'a self,
        command_type: &'a str,
        params: Option<&'a Value>,
    ) -> Pin<Box<dyn Future<Output = CommandResult> + Send + 'a>>;
}

/// Registry of command handlers, keyed by command type string.
pub struct CommandRegistry {
    handlers: HashMap<String, Box<dyn CommandHandler>>,
}

impl CommandRegistry {
    /// Create a new empty registry
    pub fn new() -> Self {
        Self {
            handlers: HashMap::new(),
        }
    }

    /// Register a handler for all its declared command types
    pub fn register(&mut self, handler: Box<dyn CommandHandler>) {
        let types: Vec<String> = handler.command_types().iter().map(|s| s.to_string()).collect();
        // For multi-type handlers, we need shared ownership
        let handler: std::sync::Arc<Box<dyn CommandHandler>> = std::sync::Arc::from(handler);
        for cmd_type in types {
            self.handlers.insert(
                cmd_type,
                Box::new(ArcHandler(handler.clone())),
            );
        }
    }

    /// Execute a command by type, returning None if no handler is registered
    pub async fn execute(
        &self,
        command_type: &str,
        params: Option<&Value>,
    ) -> Option<CommandResult> {
        let handler = self.handlers.get(command_type)?;
        Some(handler.execute(command_type, params).await)
    }

    /// List all registered command types
    pub fn command_types(&self) -> Vec<&str> {
        self.handlers.keys().map(|s| s.as_str()).collect()
    }
}

/// Internal wrapper to allow shared ownership of multi-type handlers
struct ArcHandler(std::sync::Arc<Box<dyn CommandHandler>>);

impl CommandHandler for ArcHandler {
    fn command_types(&self) -> &[&str] {
        self.0.command_types()
    }

    fn execute<'a>(
        &'a self,
        command_type: &'a str,
        params: Option<&'a Value>,
    ) -> Pin<Box<dyn Future<Output = CommandResult> + Send + 'a>> {
        self.0.execute(command_type, params)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    struct EchoHandler;

    impl CommandHandler for EchoHandler {
        fn command_types(&self) -> &[&str] {
            &["echo"]
        }

        fn execute<'a>(
            &'a self,
            _command_type: &'a str,
            params: Option<&'a Value>,
        ) -> Pin<Box<dyn Future<Output = CommandResult> + Send + 'a>> {
            Box::pin(async move {
                let msg = params
                    .and_then(|p| p.get("message"))
                    .and_then(|m| m.as_str())
                    .unwrap_or("no message");
                CommandResult::success(serde_json::json!({ "echo": msg }))
            })
        }
    }

    #[tokio::test]
    async fn test_registry_dispatch() {
        let mut registry = CommandRegistry::new();
        registry.register(Box::new(EchoHandler));

        let result = registry
            .execute("echo", Some(&serde_json::json!({ "message": "hello" })))
            .await;

        assert!(result.is_some());
        let result = result.unwrap();
        assert_eq!(result.status, "success");
        assert_eq!(
            result.data.unwrap().get("echo").unwrap().as_str().unwrap(),
            "hello"
        );
    }

    #[tokio::test]
    async fn test_registry_unknown_command() {
        let registry = CommandRegistry::new();
        let result = registry.execute("unknown", None).await;
        assert!(result.is_none());
    }

    #[test]
    fn test_command_result_helpers() {
        let ok = CommandResult::success(serde_json::json!("ok"));
        assert_eq!(ok.status, "success");
        assert!(ok.error.is_none());

        let err = CommandResult::error("FAIL", "something broke");
        assert_eq!(err.status, "error");
        assert_eq!(err.error.unwrap().code, "FAIL");
    }
}
