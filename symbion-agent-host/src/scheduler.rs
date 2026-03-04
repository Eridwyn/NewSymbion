//! Lightweight task scheduler for Symbion Agent Host
//!
//! Supports one-time, interval, daily, and weekly schedules.
//! Persistence via JSON file in config directory.
//! Dispatches via the existing CommandRegistry.

use std::collections::HashMap;
use std::sync::Arc;

use anyhow::Result;
use chrono::{DateTime, Datelike, NaiveTime, Timelike, Utc};
use serde::{Deserialize, Serialize};
use tokio::sync::Mutex;
use tracing::{debug, error, info, warn};

use crate::execution::handler::CommandRegistry;

/// Schedule type for a task
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum TaskSchedule {
    /// Run once at a specific time
    Once { at: DateTime<Utc> },
    /// Run every N seconds
    Interval { every_secs: u64 },
    /// Run daily at a specific time (UTC)
    Daily { hour: u32, minute: u32 },
    /// Run weekly on a specific day (0=Mon..6=Sun) and time
    Weekly { day: u32, hour: u32, minute: u32 },
}

/// A scheduled task definition
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScheduledTask {
    pub id: String,
    pub name: String,
    pub schedule: TaskSchedule,
    pub command_type: String,
    pub parameters: Option<serde_json::Value>,
    pub enabled: bool,
    #[serde(default)]
    pub last_run: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
}

/// Persisted state
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
struct SchedulerState {
    tasks: Vec<ScheduledTask>,
}

/// The scheduler manages scheduled tasks and dispatches them
pub struct Scheduler {
    tasks: Arc<Mutex<Vec<ScheduledTask>>>,
    persistence_path: Option<std::path::PathBuf>,
}

impl Scheduler {
    /// Create a new scheduler, loading persisted tasks from disk
    pub async fn new() -> Self {
        let persistence_path = Self::state_path();
        let tasks = if let Some(ref path) = persistence_path {
            Self::load_from_disk(path).await.unwrap_or_default()
        } else {
            Vec::new()
        };

        info!("[scheduler] Loaded {} scheduled tasks", tasks.len());

        Self {
            tasks: Arc::new(Mutex::new(tasks)),
            persistence_path,
        }
    }

    /// Create a scheduler without persistence (for testing)
    pub fn new_in_memory() -> Self {
        Self {
            tasks: Arc::new(Mutex::new(Vec::new())),
            persistence_path: None,
        }
    }

    /// Add a new scheduled task
    pub async fn add_task(&self, task: ScheduledTask) -> Result<()> {
        let mut tasks = self.tasks.lock().await;
        // Check for duplicate IDs
        if tasks.iter().any(|t| t.id == task.id) {
            anyhow::bail!("Task with id '{}' already exists", task.id);
        }
        info!("[scheduler] Added task '{}' ({})", task.name, task.id);
        tasks.push(task);
        drop(tasks);
        self.persist().await;
        Ok(())
    }

    /// Remove a scheduled task by ID
    pub async fn remove_task(&self, task_id: &str) -> bool {
        let mut tasks = self.tasks.lock().await;
        let len_before = tasks.len();
        tasks.retain(|t| t.id != task_id);
        let removed = tasks.len() < len_before;
        if removed {
            info!("[scheduler] Removed task '{}'", task_id);
            drop(tasks);
            self.persist().await;
        }
        removed
    }

    /// List all scheduled tasks
    pub async fn list_tasks(&self) -> Vec<ScheduledTask> {
        self.tasks.lock().await.clone()
    }

    /// Tick: check all tasks and execute those that are due.
    /// Returns the number of tasks executed.
    pub async fn tick(&self, registry: &CommandRegistry) -> usize {
        let now = Utc::now();
        let mut executed = 0;
        let mut tasks = self.tasks.lock().await;

        for task in tasks.iter_mut() {
            if !task.enabled {
                continue;
            }

            if Self::is_due(task, &now) {
                debug!("[scheduler] Executing task '{}' ({})", task.name, task.command_type);

                match registry.execute(&task.command_type, task.parameters.as_ref()).await {
                    Some(result) => {
                        if result.status == "success" {
                            debug!("[scheduler] Task '{}' completed successfully", task.name);
                        } else {
                            warn!("[scheduler] Task '{}' failed: {:?}", task.name, result.error);
                        }
                    }
                    None => {
                        warn!("[scheduler] Unknown command type '{}' for task '{}'",
                              task.command_type, task.name);
                    }
                }

                task.last_run = Some(now);
                executed += 1;

                // Remove one-time tasks after execution
                if matches!(task.schedule, TaskSchedule::Once { .. }) {
                    task.enabled = false;
                }
            }
        }

        if executed > 0 {
            drop(tasks);
            self.persist().await;
        }

        executed
    }

    /// Check if a task is due for execution
    fn is_due(task: &ScheduledTask, now: &DateTime<Utc>) -> bool {
        match &task.schedule {
            TaskSchedule::Once { at } => {
                task.last_run.is_none() && now >= at
            }
            TaskSchedule::Interval { every_secs } => {
                match task.last_run {
                    None => true,
                    Some(last) => {
                        let elapsed = (now.timestamp() - last.timestamp()) as u64;
                        elapsed >= *every_secs
                    }
                }
            }
            TaskSchedule::Daily { hour, minute } => {
                let target_time = NaiveTime::from_hms_opt(*hour, *minute, 0)
                    .unwrap_or(NaiveTime::from_hms_opt(0, 0, 0).unwrap());
                let now_time = now.time();

                // Check if we're within the tick window (15s)
                let now_secs = now_time.num_seconds_from_midnight();
                let target_secs = target_time.num_seconds_from_midnight();
                let within_window = now_secs >= target_secs && now_secs < target_secs + 20;

                if !within_window {
                    return false;
                }

                // Don't run if already ran today
                match task.last_run {
                    None => true,
                    Some(last) => last.date_naive() < now.date_naive(),
                }
            }
            TaskSchedule::Weekly { day, hour, minute } => {
                let current_day = now.weekday().num_days_from_monday();
                if current_day != *day {
                    return false;
                }

                let target_time = NaiveTime::from_hms_opt(*hour, *minute, 0)
                    .unwrap_or(NaiveTime::from_hms_opt(0, 0, 0).unwrap());
                let now_time = now.time();

                let now_secs = now_time.num_seconds_from_midnight();
                let target_secs = target_time.num_seconds_from_midnight();
                let within_window = now_secs >= target_secs && now_secs < target_secs + 20;

                if !within_window {
                    return false;
                }

                match task.last_run {
                    None => true,
                    Some(last) => last.date_naive() < now.date_naive(),
                }
            }
        }
    }

    /// Persist tasks to disk
    async fn persist(&self) {
        if let Some(ref path) = self.persistence_path {
            let tasks = self.tasks.lock().await;
            let state = SchedulerState { tasks: tasks.clone() };
            match serde_json::to_string_pretty(&state) {
                Ok(json) => {
                    if let Some(parent) = path.parent() {
                        let _ = tokio::fs::create_dir_all(parent).await;
                    }
                    if let Err(e) = tokio::fs::write(path, json).await {
                        error!("[scheduler] Failed to persist tasks: {}", e);
                    }
                }
                Err(e) => error!("[scheduler] Failed to serialize tasks: {}", e),
            }
        }
    }

    /// Load tasks from disk
    async fn load_from_disk(path: &std::path::Path) -> Result<Vec<ScheduledTask>> {
        if !path.exists() {
            return Ok(Vec::new());
        }
        let content = tokio::fs::read_to_string(path).await?;
        let state: SchedulerState = serde_json::from_str(&content)?;
        Ok(state.tasks)
    }

    /// Get the persistence file path
    fn state_path() -> Option<std::path::PathBuf> {
        dirs::config_dir().map(|mut p| {
            p.push("symbion-agent");
            p.push("scheduled-tasks.json");
            p
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Duration;

    fn make_task(id: &str, schedule: TaskSchedule, command: &str) -> ScheduledTask {
        ScheduledTask {
            id: id.to_string(),
            name: format!("Test task {}", id),
            schedule,
            command_type: command.to_string(),
            parameters: None,
            enabled: true,
            last_run: None,
            created_at: Utc::now(),
        }
    }

    #[tokio::test]
    async fn test_add_and_list_tasks() {
        let scheduler = Scheduler::new_in_memory();
        let task = make_task("t1", TaskSchedule::Interval { every_secs: 60 }, "get_metrics");
        scheduler.add_task(task).await.unwrap();
        let tasks = scheduler.list_tasks().await;
        assert_eq!(tasks.len(), 1);
        assert_eq!(tasks[0].id, "t1");
    }

    #[tokio::test]
    async fn test_add_duplicate_fails() {
        let scheduler = Scheduler::new_in_memory();
        let task1 = make_task("t1", TaskSchedule::Interval { every_secs: 60 }, "get_metrics");
        let task2 = make_task("t1", TaskSchedule::Interval { every_secs: 120 }, "get_metrics");
        scheduler.add_task(task1).await.unwrap();
        assert!(scheduler.add_task(task2).await.is_err());
    }

    #[tokio::test]
    async fn test_remove_task() {
        let scheduler = Scheduler::new_in_memory();
        let task = make_task("t1", TaskSchedule::Interval { every_secs: 60 }, "get_metrics");
        scheduler.add_task(task).await.unwrap();
        assert!(scheduler.remove_task("t1").await);
        assert!(!scheduler.remove_task("t1").await);
        assert_eq!(scheduler.list_tasks().await.len(), 0);
    }

    #[test]
    fn test_once_is_due() {
        let past = Utc::now() - Duration::seconds(10);
        let task = make_task("t1", TaskSchedule::Once { at: past }, "cmd");
        assert!(Scheduler::is_due(&task, &Utc::now()));
    }

    #[test]
    fn test_once_not_due_future() {
        let future = Utc::now() + Duration::hours(1);
        let task = make_task("t1", TaskSchedule::Once { at: future }, "cmd");
        assert!(!Scheduler::is_due(&task, &Utc::now()));
    }

    #[test]
    fn test_once_already_ran() {
        let past = Utc::now() - Duration::seconds(10);
        let mut task = make_task("t1", TaskSchedule::Once { at: past }, "cmd");
        task.last_run = Some(Utc::now());
        assert!(!Scheduler::is_due(&task, &Utc::now()));
    }

    #[test]
    fn test_interval_due_first_time() {
        let task = make_task("t1", TaskSchedule::Interval { every_secs: 60 }, "cmd");
        assert!(Scheduler::is_due(&task, &Utc::now()));
    }

    #[test]
    fn test_interval_not_due_yet() {
        let mut task = make_task("t1", TaskSchedule::Interval { every_secs: 60 }, "cmd");
        task.last_run = Some(Utc::now());
        assert!(!Scheduler::is_due(&task, &Utc::now()));
    }

    #[test]
    fn test_interval_due_after_elapsed() {
        let mut task = make_task("t1", TaskSchedule::Interval { every_secs: 60 }, "cmd");
        task.last_run = Some(Utc::now() - Duration::seconds(61));
        assert!(Scheduler::is_due(&task, &Utc::now()));
    }

    #[test]
    fn test_task_schedule_serialization() {
        let schedule = TaskSchedule::Daily { hour: 14, minute: 30 };
        let json = serde_json::to_string(&schedule).unwrap();
        assert!(json.contains("daily"));
        let parsed: TaskSchedule = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed, schedule);
    }

    #[test]
    fn test_scheduled_task_serialization() {
        let task = make_task("t1", TaskSchedule::Interval { every_secs: 300 }, "get_metrics");
        let json = serde_json::to_string(&task).unwrap();
        let parsed: ScheduledTask = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed.id, "t1");
        assert_eq!(parsed.command_type, "get_metrics");
        assert!(parsed.enabled);
    }
}
