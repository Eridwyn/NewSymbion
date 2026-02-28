//! Power management command handler (shutdown, reboot, hibernate)

use std::future::Future;
use std::pin::Pin;

use serde_json::Value;

use crate::execution::handler::{CommandHandler, CommandResult};
use crate::execution::CommandExecutor;

pub struct PowerHandler;

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
            let delay = params
                .and_then(|p| p.get("delay"))
                .and_then(|d| d.as_u64())
                .map(|d| d as u32);

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
