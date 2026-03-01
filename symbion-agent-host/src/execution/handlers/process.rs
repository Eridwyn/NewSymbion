//! Process management command handler (kill, list)

use std::future::Future;
use std::pin::Pin;

use chrono::Utc;
use serde_json::Value;

use crate::execution::handler::{CommandHandler, CommandResult};
use crate::execution::CommandExecutor;
use crate::metrics;

pub struct ProcessHandler;

impl CommandHandler for ProcessHandler {
    fn command_types(&self) -> &[&str] {
        &["kill_process", "list_processes"]
    }

    fn execute<'a>(
        &'a self,
        command_type: &'a str,
        params: Option<&'a Value>,
    ) -> Pin<Box<dyn Future<Output = CommandResult> + Send + 'a>> {
        Box::pin(async move {
            match command_type {
                "kill_process" => Self::handle_kill(params).await,
                "list_processes" => Self::handle_list().await,
                _ => CommandResult::error("UNKNOWN_COMMAND", format!("Unknown: {}", command_type)),
            }
        })
    }
}

impl ProcessHandler {
    async fn handle_kill(params: Option<&Value>) -> CommandResult {
        let pid = match params
            .and_then(|p| p.get("pid"))
            .and_then(|p| p.as_u64())
        {
            Some(pid) => pid as u32,
            None => return CommandResult::error("INVALID_PARAMETERS", "Missing 'pid' parameter"),
        };

        // Reject critical system PIDs (init, kernel threads)
        if pid <= 10 {
            return CommandResult::error(
                "FORBIDDEN_PID",
                format!("Cannot kill system-critical PID {} (PIDs 1-10 are protected)", pid),
            );
        }

        match CommandExecutor::kill_process(pid).await {
            Ok(result) if result.success => {
                CommandResult::success(serde_json::json!({
                    "message": format!("Process {} killed", pid)
                }))
            }
            Ok(result) => CommandResult::error("KILL_FAILED", result.error.unwrap_or_default()),
            Err(e) => CommandResult::error("EXECUTION_ERROR", e.to_string()),
        }
    }

    async fn handle_list() -> CommandResult {
        match metrics::ProcessInfo::collect().await {
            Ok(process_info) => CommandResult::success(serde_json::json!({
                "total_count": process_info.total_count,
                "running_count": process_info.running_count,
                "top_cpu": process_info.top_cpu,
                "top_memory": process_info.top_memory,
                "timestamp": Utc::now()
            })),
            Err(e) => CommandResult::error("PROCESSES_ERROR", e.to_string()),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_kill_missing_pid() {
        let handler = ProcessHandler;
        let result = handler.execute("kill_process", None).await;
        assert_eq!(result.status, "error");
        assert_eq!(result.error.unwrap().code, "INVALID_PARAMETERS");
    }

    #[tokio::test]
    async fn test_list_processes() {
        let handler = ProcessHandler;
        let result = handler.execute("list_processes", None).await;
        assert_eq!(result.status, "success");
        let data = result.data.unwrap();
        assert!(data.get("total_count").is_some());
    }
}
