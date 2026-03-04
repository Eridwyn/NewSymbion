//! Internal Watchdog for Symbion Agent Host
//!
//! Monitors subsystem health and performs self-healing:
//! - MQTT liveness: force reconnect if disconnected > mqtt_reconnect_secs
//! - Metrics health: track consecutive failures, mark degraded
//! - Heartbeat liveness: alert if no heartbeat sent > heartbeat_timeout_secs
//! - Self-healing: restart subsystems, exit(1) after max_recovery_attempts

use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::{Duration, Instant};

use serde::{Deserialize, Serialize};
use tokio::sync::Mutex;
use tracing::{info, warn, error};

/// Watchdog configuration (all fields have serde defaults for backward compat)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WatchdogConfig {
    /// Seconds without heartbeat before alerting
    #[serde(default = "default_heartbeat_timeout")]
    pub heartbeat_timeout_secs: u64,
    /// Consecutive metrics failures before marking degraded
    #[serde(default = "default_metrics_failure_threshold")]
    pub metrics_failure_threshold: u32,
    /// Watchdog check interval in seconds
    #[serde(default = "default_check_interval")]
    pub check_interval_secs: u64,
    /// Seconds MQTT can be disconnected before forcing reconnect
    #[serde(default = "default_mqtt_reconnect")]
    pub mqtt_reconnect_secs: u64,
    /// Max recovery attempts before exit(1)
    #[serde(default = "default_max_recovery")]
    pub max_recovery_attempts: u32,
    /// Enable watchdog (can be disabled in config)
    #[serde(default = "default_enabled")]
    pub enabled: bool,
}

fn default_heartbeat_timeout() -> u64 { 120 }
fn default_metrics_failure_threshold() -> u32 { 5 }
fn default_check_interval() -> u64 { 15 }
fn default_mqtt_reconnect() -> u64 { 90 }
fn default_max_recovery() -> u32 { 3 }
fn default_enabled() -> bool { true }

impl Default for WatchdogConfig {
    fn default() -> Self {
        Self {
            heartbeat_timeout_secs: default_heartbeat_timeout(),
            metrics_failure_threshold: default_metrics_failure_threshold(),
            check_interval_secs: default_check_interval(),
            mqtt_reconnect_secs: default_mqtt_reconnect(),
            max_recovery_attempts: default_max_recovery(),
            enabled: default_enabled(),
        }
    }
}

/// Health status of a subsystem
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SubsystemStatus {
    Healthy,
    Degraded,
    Failed,
}

impl std::fmt::Display for SubsystemStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            SubsystemStatus::Healthy => write!(f, "healthy"),
            SubsystemStatus::Degraded => write!(f, "degraded"),
            SubsystemStatus::Failed => write!(f, "failed"),
        }
    }
}

/// Overall watchdog health report (included in heartbeat)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WatchdogReport {
    pub status: SubsystemStatus,
    pub mqtt_status: SubsystemStatus,
    pub metrics_status: SubsystemStatus,
    pub heartbeat_status: SubsystemStatus,
    pub recovery_attempts: u32,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub last_issue: Option<String>,
}

/// Internal mutable state for the watchdog
struct WatchdogState {
    metrics_consecutive_failures: u32,
    mqtt_disconnected_since: Option<Instant>,
    last_heartbeat_sent: Instant,
    recovery_attempts: u32,
    last_issue: Option<String>,
}

/// Watchdog monitoring agent subsystems
pub struct Watchdog {
    config: WatchdogConfig,
    mqtt_connected: Arc<AtomicBool>,
    state: Arc<Mutex<WatchdogState>>,
    shutdown_tx: Option<tokio::sync::mpsc::Sender<()>>,
}

impl Watchdog {
    /// Create a new watchdog instance
    pub fn new(
        config: WatchdogConfig,
        mqtt_connected: Arc<AtomicBool>,
        shutdown_tx: Option<tokio::sync::mpsc::Sender<()>>,
    ) -> Self {
        Self {
            config,
            mqtt_connected,
            state: Arc::new(Mutex::new(WatchdogState {
                metrics_consecutive_failures: 0,
                mqtt_disconnected_since: None,
                last_heartbeat_sent: Instant::now(),
                recovery_attempts: 0,
                last_issue: None,
            })),
            shutdown_tx,
        }
    }

    /// Notify the watchdog that a heartbeat was sent successfully
    pub async fn notify_heartbeat_sent(&self) {
        let mut state = self.state.lock().await;
        state.last_heartbeat_sent = Instant::now();
    }

    /// Notify the watchdog that metrics collection succeeded
    pub async fn notify_metrics_ok(&self) {
        let mut state = self.state.lock().await;
        state.metrics_consecutive_failures = 0;
    }

    /// Notify the watchdog that metrics collection failed
    pub async fn notify_metrics_failed(&self) {
        let mut state = self.state.lock().await;
        state.metrics_consecutive_failures += 1;
        if state.metrics_consecutive_failures >= self.config.metrics_failure_threshold {
            warn!("[watchdog] Metrics failure threshold reached: {} consecutive failures",
                  state.metrics_consecutive_failures);
            state.last_issue = Some(format!(
                "Metrics failed {} consecutive times",
                state.metrics_consecutive_failures
            ));
        }
    }

    /// Run a single watchdog check cycle. Returns true if agent should continue, false to exit.
    pub async fn check(&self) -> bool {
        let mqtt_connected = self.mqtt_connected.load(Ordering::Relaxed);
        let mut state = self.state.lock().await;

        // --- MQTT liveness ---
        if !mqtt_connected {
            match state.mqtt_disconnected_since {
                None => {
                    state.mqtt_disconnected_since = Some(Instant::now());
                }
                Some(since) => {
                    let elapsed = since.elapsed().as_secs();
                    if elapsed >= self.config.mqtt_reconnect_secs {
                        warn!("[watchdog] MQTT disconnected for {}s (threshold: {}s), requesting reconnect",
                              elapsed, self.config.mqtt_reconnect_secs);
                        state.last_issue = Some(format!("MQTT disconnected for {}s", elapsed));
                        state.recovery_attempts += 1;

                        // Request MQTT reconnect via shutdown channel
                        if let Some(ref tx) = self.shutdown_tx {
                            let _ = tx.try_send(());
                        }

                        // Reset the timer so we don't spam reconnects
                        state.mqtt_disconnected_since = Some(Instant::now());
                    }
                }
            }
        } else {
            state.mqtt_disconnected_since = None;
        }

        // --- Heartbeat liveness ---
        let heartbeat_age = state.last_heartbeat_sent.elapsed().as_secs();
        if heartbeat_age >= self.config.heartbeat_timeout_secs {
            warn!("[watchdog] No heartbeat sent for {}s (threshold: {}s)",
                  heartbeat_age, self.config.heartbeat_timeout_secs);
            state.last_issue = Some(format!("No heartbeat for {}s", heartbeat_age));
            state.recovery_attempts += 1;
        }

        // --- Fatal threshold ---
        if state.recovery_attempts >= self.config.max_recovery_attempts {
            error!("[watchdog] Max recovery attempts ({}) reached — exiting for systemd restart",
                   self.config.max_recovery_attempts);
            return false;
        }

        true
    }

    /// Generate a health report for inclusion in heartbeat
    pub async fn report(&self) -> WatchdogReport {
        let mqtt_connected = self.mqtt_connected.load(Ordering::Relaxed);
        let state = self.state.lock().await;

        let mqtt_status = if mqtt_connected {
            SubsystemStatus::Healthy
        } else if state.mqtt_disconnected_since
            .map(|s| s.elapsed().as_secs() >= self.config.mqtt_reconnect_secs)
            .unwrap_or(false)
        {
            SubsystemStatus::Failed
        } else if state.mqtt_disconnected_since.is_some() {
            SubsystemStatus::Degraded
        } else {
            SubsystemStatus::Healthy
        };

        let metrics_status = if state.metrics_consecutive_failures == 0 {
            SubsystemStatus::Healthy
        } else if state.metrics_consecutive_failures >= self.config.metrics_failure_threshold {
            SubsystemStatus::Failed
        } else {
            SubsystemStatus::Degraded
        };

        let heartbeat_status = {
            let age = state.last_heartbeat_sent.elapsed().as_secs();
            if age <= self.config.heartbeat_timeout_secs / 2 {
                SubsystemStatus::Healthy
            } else if age <= self.config.heartbeat_timeout_secs {
                SubsystemStatus::Degraded
            } else {
                SubsystemStatus::Failed
            }
        };

        let overall = if mqtt_status == SubsystemStatus::Failed
            || metrics_status == SubsystemStatus::Failed
            || heartbeat_status == SubsystemStatus::Failed
        {
            SubsystemStatus::Failed
        } else if mqtt_status == SubsystemStatus::Degraded
            || metrics_status == SubsystemStatus::Degraded
            || heartbeat_status == SubsystemStatus::Degraded
        {
            SubsystemStatus::Degraded
        } else {
            SubsystemStatus::Healthy
        };

        WatchdogReport {
            status: overall,
            mqtt_status,
            metrics_status,
            heartbeat_status,
            recovery_attempts: state.recovery_attempts,
            last_issue: state.last_issue.clone(),
        }
    }

    /// Spawn the watchdog check loop as a background task
    pub fn spawn(self: Arc<Self>) -> tokio::task::JoinHandle<()> {
        let interval_secs = self.config.check_interval_secs;
        let enabled = self.config.enabled;

        tokio::spawn(async move {
            if !enabled {
                info!("[watchdog] Disabled by configuration");
                return;
            }

            info!("[watchdog] Started (check every {}s, heartbeat timeout {}s, mqtt reconnect {}s)",
                  interval_secs, self.config.heartbeat_timeout_secs, self.config.mqtt_reconnect_secs);

            let mut timer = tokio::time::interval(Duration::from_secs(interval_secs));
            loop {
                timer.tick().await;
                if !self.check().await {
                    error!("[watchdog] Fatal: exiting process for systemd restart");
                    std::process::exit(1);
                }
            }
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_config() -> WatchdogConfig {
        WatchdogConfig {
            heartbeat_timeout_secs: 5,
            metrics_failure_threshold: 3,
            check_interval_secs: 1,
            mqtt_reconnect_secs: 3,
            max_recovery_attempts: 3,
            enabled: true,
        }
    }

    #[tokio::test]
    async fn test_watchdog_healthy_state() {
        let connected = Arc::new(AtomicBool::new(true));
        let wd = Watchdog::new(make_config(), connected, None);
        wd.notify_heartbeat_sent().await;
        assert!(wd.check().await);
        let report = wd.report().await;
        assert_eq!(report.status, SubsystemStatus::Healthy);
        assert_eq!(report.mqtt_status, SubsystemStatus::Healthy);
    }

    #[tokio::test]
    async fn test_watchdog_mqtt_degraded() {
        let connected = Arc::new(AtomicBool::new(false));
        let wd = Watchdog::new(make_config(), connected, None);
        wd.notify_heartbeat_sent().await;
        // First check starts the disconnect timer
        assert!(wd.check().await);
        let report = wd.report().await;
        assert_eq!(report.mqtt_status, SubsystemStatus::Degraded);
    }

    #[tokio::test]
    async fn test_watchdog_mqtt_reconnect_after_timeout() {
        let connected = Arc::new(AtomicBool::new(false));
        let mut config = make_config();
        config.mqtt_reconnect_secs = 0; // trigger immediately
        let (tx, mut rx) = tokio::sync::mpsc::channel(1);
        let wd = Watchdog::new(config, connected, Some(tx));
        wd.notify_heartbeat_sent().await;

        // First check sets mqtt_disconnected_since
        assert!(wd.check().await);
        // Second check triggers reconnect (0s threshold)
        assert!(wd.check().await);
        // Should have sent reconnect signal
        assert!(rx.try_recv().is_ok());
    }

    #[tokio::test]
    async fn test_watchdog_metrics_degraded() {
        let connected = Arc::new(AtomicBool::new(true));
        let wd = Watchdog::new(make_config(), connected, None);
        wd.notify_heartbeat_sent().await;
        wd.notify_metrics_failed().await;
        wd.notify_metrics_failed().await;
        let report = wd.report().await;
        assert_eq!(report.metrics_status, SubsystemStatus::Degraded);
    }

    #[tokio::test]
    async fn test_watchdog_metrics_failed_threshold() {
        let connected = Arc::new(AtomicBool::new(true));
        let wd = Watchdog::new(make_config(), connected, None);
        wd.notify_heartbeat_sent().await;
        for _ in 0..3 {
            wd.notify_metrics_failed().await;
        }
        let report = wd.report().await;
        assert_eq!(report.metrics_status, SubsystemStatus::Failed);
    }

    #[tokio::test]
    async fn test_watchdog_metrics_reset_on_success() {
        let connected = Arc::new(AtomicBool::new(true));
        let wd = Watchdog::new(make_config(), connected, None);
        wd.notify_heartbeat_sent().await;
        wd.notify_metrics_failed().await;
        wd.notify_metrics_failed().await;
        wd.notify_metrics_ok().await;
        let report = wd.report().await;
        assert_eq!(report.metrics_status, SubsystemStatus::Healthy);
    }

    #[tokio::test]
    async fn test_watchdog_max_recovery_exits() {
        let connected = Arc::new(AtomicBool::new(false));
        let mut config = make_config();
        config.mqtt_reconnect_secs = 0;
        config.max_recovery_attempts = 2;
        config.heartbeat_timeout_secs = 999; // prevent heartbeat from adding recovery attempts
        let wd = Watchdog::new(config, connected, None);
        wd.notify_heartbeat_sent().await;

        // Check 1: sets mqtt_disconnected_since, recovery_attempts still 0
        assert!(wd.check().await);
        // Check 2: elapsed >= 0 → recovery_attempts = 1, resets timer
        assert!(wd.check().await);
        // Check 3: elapsed >= 0 → recovery_attempts = 2 >= max → false
        assert!(!wd.check().await);
    }

    #[tokio::test]
    async fn test_watchdog_report_overall_status() {
        let connected = Arc::new(AtomicBool::new(true));
        let wd = Watchdog::new(make_config(), connected, None);
        wd.notify_heartbeat_sent().await;
        let report = wd.report().await;
        assert_eq!(report.status, SubsystemStatus::Healthy);
        assert_eq!(report.recovery_attempts, 0);
        assert!(report.last_issue.is_none());
    }

    #[tokio::test]
    async fn test_watchdog_heartbeat_notification() {
        let connected = Arc::new(AtomicBool::new(true));
        let wd = Watchdog::new(make_config(), connected, None);
        wd.notify_heartbeat_sent().await;
        let report = wd.report().await;
        assert_eq!(report.heartbeat_status, SubsystemStatus::Healthy);
    }

    #[test]
    fn test_watchdog_config_defaults() {
        let config = WatchdogConfig::default();
        assert_eq!(config.heartbeat_timeout_secs, 120);
        assert_eq!(config.metrics_failure_threshold, 5);
        assert_eq!(config.check_interval_secs, 15);
        assert_eq!(config.mqtt_reconnect_secs, 90);
        assert_eq!(config.max_recovery_attempts, 3);
        assert!(config.enabled);
    }

    #[test]
    fn test_watchdog_config_serde_defaults() {
        // Empty TOML should deserialize with all defaults
        let config: WatchdogConfig = toml::from_str("").unwrap();
        assert_eq!(config.heartbeat_timeout_secs, 120);
        assert_eq!(config.check_interval_secs, 15);
        assert!(config.enabled);
    }

    #[test]
    fn test_watchdog_config_partial_override() {
        let toml_str = r#"heartbeat_timeout_secs = 60"#;
        let config: WatchdogConfig = toml::from_str(toml_str).unwrap();
        assert_eq!(config.heartbeat_timeout_secs, 60);
        // Others should be defaults
        assert_eq!(config.check_interval_secs, 15);
        assert_eq!(config.mqtt_reconnect_secs, 90);
    }

    #[test]
    fn test_subsystem_status_display() {
        assert_eq!(SubsystemStatus::Healthy.to_string(), "healthy");
        assert_eq!(SubsystemStatus::Degraded.to_string(), "degraded");
        assert_eq!(SubsystemStatus::Failed.to_string(), "failed");
    }

    #[test]
    fn test_watchdog_report_serialization() {
        let report = WatchdogReport {
            status: SubsystemStatus::Healthy,
            mqtt_status: SubsystemStatus::Healthy,
            metrics_status: SubsystemStatus::Healthy,
            heartbeat_status: SubsystemStatus::Healthy,
            recovery_attempts: 0,
            last_issue: None,
        };
        let json = serde_json::to_string(&report).unwrap();
        assert!(json.contains("\"status\":\"healthy\""));
        assert!(!json.contains("last_issue")); // skip_serializing_if
    }

    #[test]
    fn test_watchdog_report_with_issue() {
        let report = WatchdogReport {
            status: SubsystemStatus::Degraded,
            mqtt_status: SubsystemStatus::Degraded,
            metrics_status: SubsystemStatus::Healthy,
            heartbeat_status: SubsystemStatus::Healthy,
            recovery_attempts: 1,
            last_issue: Some("MQTT disconnected for 95s".to_string()),
        };
        let json = serde_json::to_string(&report).unwrap();
        assert!(json.contains("degraded"));
        assert!(json.contains("MQTT disconnected"));
    }
}
