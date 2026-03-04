//! Enhanced log collection for Symbion Agent Host
//!
//! Features over the inline pending_logs:
//! - Configurable minimum level (TRACE → CRITICAL)
//! - Immediate publish for ERROR/CRITICAL (no 30s wait)
//! - OS log capture via journalctl (Linux) / wevtutil (Windows)
//! - Source and service tracking per log entry

use std::sync::Arc;

use chrono::Utc;
use rumqttc::AsyncClient;
use serde::{Deserialize, Serialize};
use tokio::sync::Mutex;
use tracing::{debug, warn};

use crate::messages::{LogEntry, LogMessage};
use crate::mqtt_client;

/// Log level ordering for filtering
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "UPPERCASE")]
pub enum LogLevel {
    Trace,
    Debug,
    Info,
    Warn,
    Error,
    Critical,
}

impl LogLevel {
    /// Parse from string, case-insensitive
    pub fn from_str_loose(s: &str) -> Self {
        match s.to_uppercase().as_str() {
            "TRACE" => LogLevel::Trace,
            "DEBUG" => LogLevel::Debug,
            "INFO" => LogLevel::Info,
            "WARN" | "WARNING" => LogLevel::Warn,
            "ERROR" | "ERR" => LogLevel::Error,
            "CRITICAL" | "FATAL" | "CRIT" => LogLevel::Critical,
            _ => LogLevel::Info,
        }
    }
}

impl std::fmt::Display for LogLevel {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            LogLevel::Trace => write!(f, "TRACE"),
            LogLevel::Debug => write!(f, "DEBUG"),
            LogLevel::Info => write!(f, "INFO"),
            LogLevel::Warn => write!(f, "WARN"),
            LogLevel::Error => write!(f, "ERROR"),
            LogLevel::Critical => write!(f, "CRITICAL"),
        }
    }
}

/// Log collector configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LogConfig {
    /// Minimum level to collect (default: INFO)
    #[serde(default = "default_min_level")]
    pub min_level: LogLevel,
    /// Levels that trigger immediate publish (no waiting for heartbeat flush)
    #[serde(default = "default_immediate_levels")]
    pub immediate_publish_levels: Vec<LogLevel>,
    /// Maximum buffered entries before forced flush
    #[serde(default = "default_max_buffer")]
    pub max_buffer_size: usize,
    /// Capture OS-level logs (journalctl on Linux, wevtutil on Windows)
    #[serde(default)]
    pub capture_os_logs: bool,
    /// Services to monitor OS logs for (only if capture_os_logs=true)
    #[serde(default = "default_os_services")]
    pub os_log_services: Vec<String>,
}

fn default_min_level() -> LogLevel { LogLevel::Info }
fn default_immediate_levels() -> Vec<LogLevel> { vec![LogLevel::Error, LogLevel::Critical] }
fn default_max_buffer() -> usize { 200 }
fn default_os_services() -> Vec<String> { vec!["symbion-agent-host".to_string()] }

impl Default for LogConfig {
    fn default() -> Self {
        Self {
            min_level: default_min_level(),
            immediate_publish_levels: default_immediate_levels(),
            max_buffer_size: default_max_buffer(),
            capture_os_logs: false,
            os_log_services: default_os_services(),
        }
    }
}

/// Enhanced log collector with buffering and immediate publish
pub struct LogCollector {
    config: LogConfig,
    agent_id: String,
    buffer: Arc<Mutex<Vec<LogEntry>>>,
    mqtt_client: AsyncClient,
}

impl LogCollector {
    /// Create a new log collector
    pub fn new(config: LogConfig, agent_id: String, mqtt_client: AsyncClient) -> Self {
        Self {
            config,
            agent_id,
            buffer: Arc::new(Mutex::new(Vec::new())),
            mqtt_client,
        }
    }

    /// Push a log entry. If the level requires immediate publish, sends right away.
    pub async fn push(&self, level: &str, message: &str, module: Option<String>, source: Option<String>) {
        let parsed_level = LogLevel::from_str_loose(level);

        // Filter below minimum level
        if parsed_level < self.config.min_level {
            return;
        }

        let entry = LogEntry {
            timestamp: Utc::now(),
            level: level.to_string(),
            message: message.to_string(),
            module,
            source,
        };

        // Immediate publish for critical levels
        if self.config.immediate_publish_levels.contains(&parsed_level) {
            self.publish_immediate(entry.clone()).await;
        }

        // Always buffer for batch flush
        let mut buffer = self.buffer.lock().await;
        buffer.push(entry);

        // Cap buffer size
        if buffer.len() > self.config.max_buffer_size {
            let excess = buffer.len() - self.config.max_buffer_size;
            buffer.drain(..excess);
        }
    }

    /// Flush all buffered logs to the kernel via MQTT. Returns count of flushed entries.
    pub async fn flush(&self) -> usize {
        let entries: Vec<LogEntry> = {
            let mut buffer = self.buffer.lock().await;
            std::mem::take(&mut *buffer)
        };

        if entries.is_empty() {
            return 0;
        }

        let count = entries.len();
        let log_msg = LogMessage {
            agent_id: self.agent_id.clone(),
            entries,
            timestamp: Utc::now(),
        };

        if let Err(e) = mqtt_client::publish_json(&self.mqtt_client, mqtt_client::TOPIC_LOGS, &log_msg).await {
            warn!("Failed to flush logs to kernel: {}", e);
            // Re-buffer failed entries
            let mut buffer = self.buffer.lock().await;
            for entry in log_msg.entries {
                buffer.push(entry);
            }
            return 0;
        }

        debug!("Flushed {} log entries to kernel", count);
        count
    }

    /// Get current buffer size
    pub async fn buffer_len(&self) -> usize {
        self.buffer.lock().await.len()
    }

    /// Publish a single log entry immediately (for ERROR/CRITICAL)
    async fn publish_immediate(&self, entry: LogEntry) {
        let log_msg = LogMessage {
            agent_id: self.agent_id.clone(),
            entries: vec![entry],
            timestamp: Utc::now(),
        };

        if let Err(e) = mqtt_client::publish_json(&self.mqtt_client, mqtt_client::TOPIC_LOGS, &log_msg).await {
            warn!("Failed to publish immediate log: {}", e);
        } else {
            debug!("Immediate log published");
        }
    }
}

/// Spawn OS log capture as a background task (Linux only for now)
#[cfg(target_os = "linux")]
pub fn spawn_os_log_capture(
    collector: Arc<LogCollector>,
    services: Vec<String>,
) -> tokio::task::JoinHandle<()> {
    use tokio::io::{AsyncBufReadExt, BufReader};
    use tokio::process::Command;
    use tracing::error;

    tokio::spawn(async move {
        if services.is_empty() {
            return;
        }

        let mut args = vec!["-f", "--no-pager", "-o", "cat", "-n", "0"];
        for svc in &services {
            args.push("-u");
            args.push(svc);
        }

        let child = Command::new("journalctl")
            .args(&args)
            .stdout(std::process::Stdio::piped())
            .stderr(std::process::Stdio::null())
            .spawn();

        let mut child = match child {
            Ok(c) => c,
            Err(e) => {
                error!("[log_collector] Failed to spawn journalctl: {}", e);
                return;
            }
        };

        if let Some(stdout) = child.stdout.take() {
            let reader = BufReader::new(stdout);
            let mut lines = reader.lines();
            while let Ok(Some(line)) = lines.next_line().await {
                // Parse journalctl output (simple: treat as INFO from os_journal)
                let level = if line.contains("error") || line.contains("Error") || line.contains("ERROR") {
                    "ERROR"
                } else if line.contains("warn") || line.contains("Warn") || line.contains("WARNING") {
                    "WARN"
                } else {
                    "INFO"
                };
                collector.push(level, &line, None, Some("os_journal".to_string())).await;
            }
        }

        let _ = child.wait().await;
    })
}

#[cfg(not(target_os = "linux"))]
pub fn spawn_os_log_capture(
    _collector: Arc<LogCollector>,
    _services: Vec<String>,
) -> tokio::task::JoinHandle<()> {
    tokio::spawn(async {
        // OS log capture not implemented for this platform
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use rumqttc::{MqttOptions, AsyncClient};

    fn make_test_client() -> AsyncClient {
        let mqtt_options = MqttOptions::new("test-log-collector", "localhost", 1883);
        let (client, _eventloop) = AsyncClient::new(mqtt_options, 10);
        client
    }

    fn make_collector() -> LogCollector {
        LogCollector::new(
            LogConfig::default(),
            "test-agent".to_string(),
            make_test_client(),
        )
    }

    #[test]
    fn test_log_config_defaults() {
        let config = LogConfig::default();
        assert_eq!(config.min_level, LogLevel::Info);
        assert!(config.immediate_publish_levels.contains(&LogLevel::Error));
        assert!(config.immediate_publish_levels.contains(&LogLevel::Critical));
        assert_eq!(config.max_buffer_size, 200);
        assert!(!config.capture_os_logs);
    }

    #[test]
    fn test_log_config_serde_defaults() {
        let config: LogConfig = toml::from_str("").unwrap();
        assert_eq!(config.min_level, LogLevel::Info);
        assert_eq!(config.max_buffer_size, 200);
    }

    #[test]
    fn test_log_config_partial_override() {
        let toml_str = r#"min_level = "INFO""#;
        let config: LogConfig = toml::from_str(toml_str).unwrap();
        assert_eq!(config.min_level, LogLevel::Info);
        assert_eq!(config.max_buffer_size, 200); // default
    }

    #[test]
    fn test_log_level_ordering() {
        assert!(LogLevel::Trace < LogLevel::Debug);
        assert!(LogLevel::Debug < LogLevel::Info);
        assert!(LogLevel::Info < LogLevel::Warn);
        assert!(LogLevel::Warn < LogLevel::Error);
        assert!(LogLevel::Error < LogLevel::Critical);
    }

    #[test]
    fn test_log_level_from_str() {
        assert_eq!(LogLevel::from_str_loose("ERROR"), LogLevel::Error);
        assert_eq!(LogLevel::from_str_loose("error"), LogLevel::Error);
        assert_eq!(LogLevel::from_str_loose("ERR"), LogLevel::Error);
        assert_eq!(LogLevel::from_str_loose("WARN"), LogLevel::Warn);
        assert_eq!(LogLevel::from_str_loose("WARNING"), LogLevel::Warn);
        assert_eq!(LogLevel::from_str_loose("CRITICAL"), LogLevel::Critical);
        assert_eq!(LogLevel::from_str_loose("FATAL"), LogLevel::Critical);
        assert_eq!(LogLevel::from_str_loose("unknown"), LogLevel::Info);
    }

    #[test]
    fn test_log_level_display() {
        assert_eq!(LogLevel::Error.to_string(), "ERROR");
        assert_eq!(LogLevel::Warn.to_string(), "WARN");
        assert_eq!(LogLevel::Critical.to_string(), "CRITICAL");
    }

    #[tokio::test]
    async fn test_collector_filters_below_min_level() {
        let collector = make_collector();
        // Default min_level is INFO, so DEBUG should be filtered out
        collector.push("DEBUG", "should be filtered", None, None).await;
        assert_eq!(collector.buffer_len().await, 0);
    }

    #[tokio::test]
    async fn test_collector_accepts_above_min_level() {
        let collector = make_collector();
        collector.push("WARN", "should be accepted", None, None).await;
        assert_eq!(collector.buffer_len().await, 1);
        collector.push("ERROR", "also accepted", None, None).await;
        assert_eq!(collector.buffer_len().await, 2);
    }

    #[tokio::test]
    async fn test_collector_buffer_cap() {
        let config = LogConfig {
            max_buffer_size: 5,
            ..Default::default()
        };
        let collector = LogCollector::new(config, "test".to_string(), make_test_client());
        for i in 0..10 {
            collector.push("WARN", &format!("msg {}", i), None, None).await;
        }
        assert_eq!(collector.buffer_len().await, 5);
    }

    #[tokio::test]
    async fn test_collector_source_tracking() {
        let collector = make_collector();
        collector.push("ERROR", "test msg", Some("metrics".to_string()), Some("os_journal".to_string())).await;
        let buffer = collector.buffer.lock().await;
        assert_eq!(buffer[0].source, Some("os_journal".to_string()));
        assert_eq!(buffer[0].module, Some("metrics".to_string()));
    }

    #[test]
    fn test_log_level_serialization() {
        let level = LogLevel::Error;
        let json = serde_json::to_string(&level).unwrap();
        assert_eq!(json, "\"ERROR\"");
        let parsed: LogLevel = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed, LogLevel::Error);
    }
}
