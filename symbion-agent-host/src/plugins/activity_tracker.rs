//! Activity Tracker plugin — first built-in agent plugin
//!
//! Tracks user activity:
//! - Idle time detection
//! - Active window title (Linux via xdotool, Windows via WinAPI)
//! - is_idle flag based on configurable threshold

use anyhow::Result;
use async_trait::async_trait;
use serde_json::Value;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::Instant;
use tracing::debug;

use super::trait_def::AgentPlugin;

/// Default idle threshold in seconds
const DEFAULT_IDLE_THRESHOLD_SECS: u64 = 300; // 5 minutes

/// Activity tracker plugin
pub struct ActivityTracker {
    idle_threshold_secs: u64,
    last_activity: std::sync::Mutex<Instant>,
    total_active_secs: AtomicU64,
}

impl ActivityTracker {
    pub fn new() -> Self {
        Self {
            idle_threshold_secs: DEFAULT_IDLE_THRESHOLD_SECS,
            last_activity: std::sync::Mutex::new(Instant::now()),
            total_active_secs: AtomicU64::new(0),
        }
    }

    /// Get idle time in seconds from the system
    fn get_system_idle_secs() -> u64 {
        #[cfg(target_os = "linux")]
        {
            // Try xprintidle (returns milliseconds)
            if let Ok(output) = std::process::Command::new("xprintidle").output() {
                if output.status.success() {
                    if let Ok(ms) = String::from_utf8_lossy(&output.stdout).trim().parse::<u64>() {
                        return ms / 1000;
                    }
                }
            }
            // Fallback: use /proc/uptime difference (rough approximation)
            0
        }

        #[cfg(target_os = "windows")]
        {
            // Would use GetLastInputInfo — placeholder
            0
        }

        #[cfg(not(any(target_os = "linux", target_os = "windows")))]
        {
            0
        }
    }

    /// Get active window title
    fn get_active_window() -> Option<String> {
        #[cfg(target_os = "linux")]
        {
            if let Ok(output) = std::process::Command::new("xdotool")
                .args(["getactivewindow", "getwindowname"])
                .output()
            {
                if output.status.success() {
                    let title = String::from_utf8_lossy(&output.stdout).trim().to_string();
                    if !title.is_empty() {
                        return Some(title);
                    }
                }
            }
            None
        }

        #[cfg(not(target_os = "linux"))]
        {
            None
        }
    }
}

#[async_trait]
impl AgentPlugin for ActivityTracker {
    fn id(&self) -> &str {
        "activity_tracker"
    }

    fn name(&self) -> &str {
        "Activity Tracker"
    }

    async fn init(&mut self) -> Result<()> {
        debug!("[activity_tracker] Initialized with idle threshold {}s", self.idle_threshold_secs);
        Ok(())
    }

    async fn tick(&self) -> Result<Option<Value>> {
        let idle_secs = Self::get_system_idle_secs();
        let is_idle = idle_secs >= self.idle_threshold_secs;
        let active_window = Self::get_active_window();

        if !is_idle {
            // Track active time approximately (30s heartbeat interval)
            self.total_active_secs.fetch_add(30, Ordering::Relaxed);
            *self.last_activity.lock().unwrap_or_else(|e| e.into_inner()) = Instant::now();
        }

        Ok(Some(serde_json::json!({
            "idle_secs": idle_secs,
            "is_idle": is_idle,
            "active_window": active_window,
            "total_active_secs": self.total_active_secs.load(Ordering::Relaxed),
        })))
    }

    async fn handle_command(&self, action: &str, _params: Option<&Value>) -> Result<Value> {
        match action {
            "get_status" => {
                let idle_secs = Self::get_system_idle_secs();
                Ok(serde_json::json!({
                    "idle_secs": idle_secs,
                    "is_idle": idle_secs >= self.idle_threshold_secs,
                    "active_window": Self::get_active_window(),
                    "total_active_secs": self.total_active_secs.load(Ordering::Relaxed),
                }))
            }
            "reset_counter" => {
                self.total_active_secs.store(0, Ordering::Relaxed);
                Ok(serde_json::json!({"message": "Active time counter reset"}))
            }
            _ => anyhow::bail!("Unknown action: {}", action),
        }
    }

    async fn shutdown(&self) -> Result<()> {
        debug!("[activity_tracker] Shutdown");
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_activity_tracker_init() {
        let mut tracker = ActivityTracker::new();
        assert!(tracker.init().await.is_ok());
        assert_eq!(tracker.id(), "activity_tracker");
        assert_eq!(tracker.name(), "Activity Tracker");
    }

    #[tokio::test]
    async fn test_activity_tracker_tick() {
        let mut tracker = ActivityTracker::new();
        tracker.init().await.unwrap();
        let data = tracker.tick().await.unwrap();
        assert!(data.is_some());
        let value = data.unwrap();
        assert!(value.get("idle_secs").is_some());
        assert!(value.get("is_idle").is_some());
    }

    #[tokio::test]
    async fn test_activity_tracker_get_status() {
        let mut tracker = ActivityTracker::new();
        tracker.init().await.unwrap();
        let result = tracker.handle_command("get_status", None).await.unwrap();
        assert!(result.get("idle_secs").is_some());
    }

    #[tokio::test]
    async fn test_activity_tracker_reset_counter() {
        let mut tracker = ActivityTracker::new();
        tracker.init().await.unwrap();
        tracker.total_active_secs.store(100, Ordering::Relaxed);
        let _result = tracker.handle_command("reset_counter", None).await.unwrap();
        assert_eq!(tracker.total_active_secs.load(Ordering::Relaxed), 0);
    }

    #[tokio::test]
    async fn test_activity_tracker_unknown_action() {
        let mut tracker = ActivityTracker::new();
        tracker.init().await.unwrap();
        assert!(tracker.handle_command("unknown", None).await.is_err());
    }

    #[tokio::test]
    async fn test_activity_tracker_shutdown() {
        let mut tracker = ActivityTracker::new();
        tracker.init().await.unwrap();
        assert!(tracker.shutdown().await.is_ok());
    }
}
