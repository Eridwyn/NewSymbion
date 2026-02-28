//! Service management command handler (status, start, stop, restart)
//!
//! Wires the previously-unused ServiceManager into the command system.

use std::future::Future;
use std::pin::Pin;

use serde_json::Value;

use crate::execution::handler::{CommandHandler, CommandResult};
use crate::execution::ServiceManager;

pub struct ServiceHandler;

impl CommandHandler for ServiceHandler {
    fn command_types(&self) -> &[&str] {
        &["service_status", "service_start", "service_stop", "service_restart"]
    }

    fn execute<'a>(
        &'a self,
        command_type: &'a str,
        params: Option<&'a Value>,
    ) -> Pin<Box<dyn Future<Output = CommandResult> + Send + 'a>> {
        Box::pin(async move {
            let service_name = match params
                .and_then(|p| p.get("service"))
                .and_then(|s| s.as_str())
            {
                Some(name) => name,
                None => return CommandResult::error(
                    "INVALID_PARAMETERS",
                    "Missing 'service' parameter",
                ),
            };

            // Validate service name (same rules as process names)
            if !is_safe_service_name(service_name) {
                return CommandResult::error(
                    "INVALID_PARAMETERS",
                    format!("Invalid service name: {}", service_name),
                );
            }

            match command_type {
                "service_status" => Self::handle_status(service_name).await,
                "service_start" => Self::handle_action(service_name, "start").await,
                "service_stop" => Self::handle_action(service_name, "stop").await,
                "service_restart" => Self::handle_action(service_name, "restart").await,
                _ => CommandResult::error("UNKNOWN_COMMAND", format!("Unknown: {}", command_type)),
            }
        })
    }
}

impl ServiceHandler {
    async fn handle_status(service_name: &str) -> CommandResult {
        match ServiceManager::status(service_name).await {
            Ok(status) => CommandResult::success(serde_json::json!({
                "service": status.name,
                "running": status.running,
                "status": status.status_text,
            })),
            Err(e) => CommandResult::error("SERVICE_ERROR", e.to_string()),
        }
    }

    async fn handle_action(service_name: &str, action: &str) -> CommandResult {
        let result = match action {
            "start" => ServiceManager::start(service_name).await,
            "stop" => ServiceManager::stop(service_name).await,
            "restart" => ServiceManager::restart(service_name).await,
            _ => return CommandResult::error("UNKNOWN_ACTION", format!("Unknown action: {}", action)),
        };

        match result {
            Ok(exec_result) if exec_result.success => {
                CommandResult::success(serde_json::json!({
                    "message": format!("Service '{}' {} successful", service_name, action),
                    "output": exec_result.output,
                }))
            }
            Ok(exec_result) => CommandResult::error(
                "SERVICE_FAILED",
                exec_result.error.unwrap_or_else(|| format!("Failed to {} service", action)),
            ),
            Err(e) => CommandResult::error("SERVICE_ERROR", e.to_string()),
        }
    }
}

/// Validate a service name contains only safe characters
fn is_safe_service_name(name: &str) -> bool {
    !name.is_empty()
        && name.len() <= 256
        && name.chars().all(|c| c.is_alphanumeric() || matches!(c, '-' | '_' | '.' | '@'))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_safe_service_names() {
        assert!(is_safe_service_name("nginx"));
        assert!(is_safe_service_name("symbion-kernel"));
        assert!(is_safe_service_name("mosquitto.service"));
        assert!(is_safe_service_name("user@1000"));
    }

    #[test]
    fn test_unsafe_service_names() {
        assert!(!is_safe_service_name(""));
        assert!(!is_safe_service_name("service; rm -rf /"));
        assert!(!is_safe_service_name("$(evil)"));
        assert!(!is_safe_service_name("a b c"));
    }

    #[tokio::test]
    async fn test_missing_service_param() {
        let handler = ServiceHandler;
        let result = handler.execute("service_status", None).await;
        assert_eq!(result.status, "error");
        assert_eq!(result.error.unwrap().code, "INVALID_PARAMETERS");
    }
}
