//! Power management command handler (shutdown, reboot, hibernate)

use std::future::Future;
use std::pin::Pin;

use serde_json::Value;

use crate::execution::handler::{CommandHandler, CommandResult};
use crate::execution::CommandExecutor;

pub struct PowerHandler;

impl PowerHandler {
    /// Extract delay parameter from command params
    pub(crate) fn parse_delay(params: Option<&Value>) -> Option<u32> {
        params
            .and_then(|p| p.get("delay"))
            .and_then(|d| d.as_u64())
            .map(|d| d as u32)
    }
}

impl CommandHandler for PowerHandler {
    fn command_types(&self) -> &[&str] {
        &["shutdown", "reboot", "hibernate"]
    }

    fn execute<'a>(
        &'a self,
        command_type: &'a str,
        params: Option<&'a Value>,
    ) -> Pin<Box<dyn Future<Output = CommandResult> + Send + 'a>> {
        Box::pin(async move {
            let delay = Self::parse_delay(params);

            match CommandExecutor::execute_power_command(command_type, delay).await {
                Ok(result) if result.success => {
                    CommandResult::success(serde_json::json!({
                        "message": format!("{} initiated", command_type)
                    }))
                }
                Ok(result) => CommandResult::error(
                    &format!("{}_FAILED", command_type.to_uppercase()),
                    result.error.unwrap_or_default(),
                ),
                Err(e) => CommandResult::error("EXECUTION_ERROR", e.to_string()),
            }
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::execution::handler::CommandHandler;

    #[test]
    fn test_parse_delay_with_valid_value() {
        let params = serde_json::json!({"delay": 60});
        assert_eq!(PowerHandler::parse_delay(Some(&params)), Some(60));
    }

    #[test]
    fn test_parse_delay_none_params() {
        assert_eq!(PowerHandler::parse_delay(None), None);
    }

    #[test]
    fn test_parse_delay_missing_field() {
        let params = serde_json::json!({"other": "value"});
        assert_eq!(PowerHandler::parse_delay(Some(&params)), None);
    }

    #[test]
    fn test_parse_delay_invalid_type() {
        let params = serde_json::json!({"delay": "not_a_number"});
        assert_eq!(PowerHandler::parse_delay(Some(&params)), None);
    }

    #[test]
    fn test_command_types() {
        let handler = PowerHandler;
        let types = handler.command_types();
        assert!(types.contains(&"shutdown"));
        assert!(types.contains(&"reboot"));
        assert!(types.contains(&"hibernate"));
        assert_eq!(types.len(), 3);
    }
}
