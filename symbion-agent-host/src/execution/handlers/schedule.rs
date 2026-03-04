//! Scheduled tasks command handler
//!
//! Handles: schedule_task, unschedule_task, list_scheduled_tasks

use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;

use chrono::Utc;
use serde_json::Value;

use crate::execution::handler::{CommandHandler, CommandResult};
use crate::scheduler::{ScheduledTask, Scheduler, TaskSchedule};

/// Handler for scheduled task management commands
pub struct ScheduleHandler {
    scheduler: Arc<Scheduler>,
}

impl ScheduleHandler {
    pub fn new(scheduler: Arc<Scheduler>) -> Self {
        Self { scheduler }
    }
}

impl CommandHandler for ScheduleHandler {
    fn command_types(&self) -> &[&str] {
        &["schedule_task", "unschedule_task", "list_scheduled_tasks"]
    }

    fn execute<'a>(
        &'a self,
        command_type: &'a str,
        params: Option<&'a Value>,
    ) -> Pin<Box<dyn Future<Output = CommandResult> + Send + 'a>> {
        Box::pin(async move {
            match command_type {
                "schedule_task" => self.handle_schedule(params).await,
                "unschedule_task" => self.handle_unschedule(params).await,
                "list_scheduled_tasks" => self.handle_list().await,
                _ => CommandResult::error("UNKNOWN_COMMAND", "Unknown schedule command"),
            }
        })
    }
}

impl ScheduleHandler {
    async fn handle_schedule(&self, params: Option<&Value>) -> CommandResult {
        let params = match params {
            Some(p) => p,
            None => return CommandResult::error("MISSING_PARAMS", "schedule_task requires parameters"),
        };

        let name = params.get("name")
            .and_then(|v| v.as_str())
            .unwrap_or("Unnamed task");

        let command_type = match params.get("command_type").and_then(|v| v.as_str()) {
            Some(ct) => ct,
            None => return CommandResult::error("MISSING_COMMAND_TYPE", "schedule_task requires 'command_type'"),
        };

        let schedule = match params.get("schedule") {
            Some(s) => {
                match serde_json::from_value::<TaskSchedule>(s.clone()) {
                    Ok(sched) => sched,
                    Err(e) => return CommandResult::error("INVALID_SCHEDULE", format!("Invalid schedule: {}", e)),
                }
            }
            None => return CommandResult::error("MISSING_SCHEDULE", "schedule_task requires 'schedule'"),
        };

        let command_params = params.get("parameters").cloned();

        let task_id = params.get("id")
            .and_then(|v| v.as_str())
            .map(String::from)
            .unwrap_or_else(|| uuid::Uuid::new_v4().to_string());

        let task = ScheduledTask {
            id: task_id.clone(),
            name: name.to_string(),
            schedule,
            command_type: command_type.to_string(),
            parameters: command_params,
            enabled: true,
            last_run: None,
            created_at: Utc::now(),
        };

        match self.scheduler.add_task(task).await {
            Ok(()) => CommandResult::success(serde_json::json!({
                "message": "Task scheduled",
                "task_id": task_id,
            })),
            Err(e) => CommandResult::error("SCHEDULE_FAILED", e.to_string()),
        }
    }

    async fn handle_unschedule(&self, params: Option<&Value>) -> CommandResult {
        let task_id = params
            .and_then(|p| p.get("task_id"))
            .and_then(|v| v.as_str());

        match task_id {
            Some(id) => {
                if self.scheduler.remove_task(id).await {
                    CommandResult::success(serde_json::json!({
                        "message": "Task removed",
                        "task_id": id,
                    }))
                } else {
                    CommandResult::error("NOT_FOUND", format!("Task '{}' not found", id))
                }
            }
            None => CommandResult::error("MISSING_TASK_ID", "unschedule_task requires 'task_id'"),
        }
    }

    async fn handle_list(&self) -> CommandResult {
        let tasks = self.scheduler.list_tasks().await;
        CommandResult::success(serde_json::json!({
            "tasks": tasks,
            "count": tasks.len(),
        }))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_handler() -> ScheduleHandler {
        ScheduleHandler::new(Arc::new(Scheduler::new_in_memory()))
    }

    #[tokio::test]
    async fn test_schedule_task() {
        let handler = make_handler();
        let params = serde_json::json!({
            "name": "Test Task",
            "command_type": "get_metrics",
            "schedule": {"type": "interval", "every_secs": 300}
        });
        let result = handler.execute("schedule_task", Some(&params)).await;
        assert_eq!(result.status, "success");
        assert!(result.data.unwrap()["task_id"].is_string());
    }

    #[tokio::test]
    async fn test_schedule_missing_command_type() {
        let handler = make_handler();
        let params = serde_json::json!({
            "name": "Bad Task",
            "schedule": {"type": "interval", "every_secs": 60}
        });
        let result = handler.execute("schedule_task", Some(&params)).await;
        assert_eq!(result.status, "error");
        assert_eq!(result.error.unwrap().code, "MISSING_COMMAND_TYPE");
    }

    #[tokio::test]
    async fn test_list_scheduled_tasks() {
        let handler = make_handler();
        // Add a task first
        let params = serde_json::json!({
            "name": "Test",
            "command_type": "get_metrics",
            "schedule": {"type": "interval", "every_secs": 60}
        });
        handler.execute("schedule_task", Some(&params)).await;

        let result = handler.execute("list_scheduled_tasks", None).await;
        assert_eq!(result.status, "success");
        assert_eq!(result.data.as_ref().unwrap()["count"], 1);
    }

    #[tokio::test]
    async fn test_unschedule_task() {
        let handler = make_handler();
        let params = serde_json::json!({
            "id": "my-task-1",
            "name": "Test",
            "command_type": "get_metrics",
            "schedule": {"type": "interval", "every_secs": 60}
        });
        handler.execute("schedule_task", Some(&params)).await;

        let result = handler.execute("unschedule_task", Some(&serde_json::json!({"task_id": "my-task-1"}))).await;
        assert_eq!(result.status, "success");

        // Verify removed
        let list = handler.execute("list_scheduled_tasks", None).await;
        assert_eq!(list.data.unwrap()["count"], 0);
    }

    #[tokio::test]
    async fn test_unschedule_not_found() {
        let handler = make_handler();
        let result = handler.execute("unschedule_task", Some(&serde_json::json!({"task_id": "nope"}))).await;
        assert_eq!(result.status, "error");
        assert_eq!(result.error.unwrap().code, "NOT_FOUND");
    }

    #[test]
    fn test_handler_command_types() {
        let handler = make_handler();
        let types = handler.command_types();
        assert!(types.contains(&"schedule_task"));
        assert!(types.contains(&"unschedule_task"));
        assert!(types.contains(&"list_scheduled_tasks"));
    }
}
