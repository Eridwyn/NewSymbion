//! Configuration loading for SSL plugin

use anyhow::{Context, Result};
use serde::Deserialize;
use std::collections::HashMap;

/// Main configuration structure
#[derive(Debug, Clone, Deserialize)]
pub struct Config {
    /// MQTT broker settings
    pub mqtt: MqttConfig,

    /// HTTP/Unix socket settings
    pub http: HttpConfig,

    /// Polling intervals
    pub polling: PollingConfig,

    /// Alert thresholds
    pub alerts: AlertConfig,

    /// Domains to monitor (key = feature_id suffix)
    pub domains: HashMap<String, DomainConfig>,
}

/// MQTT configuration
#[derive(Debug, Clone, Deserialize)]
pub struct MqttConfig {
    pub host: String,
    pub port: u16,
    #[serde(default = "default_client_id")]
    pub client_id: String,
}

fn default_client_id() -> String {
    "symbion-plugin-ssl".to_string()
}

/// HTTP server configuration
#[derive(Debug, Clone, Deserialize)]
pub struct HttpConfig {
    #[serde(default = "default_socket_path")]
    pub socket_path: String,
}

fn default_socket_path() -> String {
    "/run/symbion-plugins/ssl.sock".to_string()
}

/// Polling configuration
#[derive(Debug, Clone, Deserialize)]
pub struct PollingConfig {
    /// SSL check interval in seconds (default: 3600 = 1 hour)
    #[serde(default = "default_ssl_seconds")]
    pub ssl_seconds: u64,

    /// Online check interval in seconds (default: 60)
    #[serde(default = "default_online_seconds")]
    pub online_seconds: u64,
}

fn default_ssl_seconds() -> u64 { 3600 }
fn default_online_seconds() -> u64 { 60 }

/// Alert thresholds (also used per-domain)
#[derive(Debug, Clone, Copy, Deserialize, Default)]
pub struct AlertConfig {
    /// Days before expiry to trigger warning (yellow)
    #[serde(default = "default_warning_days")]
    pub warning_days: i64,

    /// Days before expiry to trigger critical (red)
    #[serde(default = "default_critical_days")]
    pub critical_days: i64,
}

fn default_warning_days() -> i64 { 30 }
fn default_critical_days() -> i64 { 14 }

/// Domain configuration
#[derive(Debug, Clone, Deserialize)]
pub struct DomainConfig {
    /// Domain hostname (e.g., "www.markcha.fr")
    pub hostname: String,

    /// Port to check (default: 443)
    #[serde(default = "default_port")]
    pub port: u16,

    /// Display label for PWA
    #[serde(default)]
    pub label: Option<String>,

    /// Whether to check HTTP health too
    #[serde(default = "default_check_http")]
    pub check_http: bool,
}

fn default_port() -> u16 { 443 }
fn default_check_http() -> bool { true }

impl Config {
    /// Load configuration from file
    pub fn load(path: &str) -> Result<Self> {
        let content = std::fs::read_to_string(path)
            .with_context(|| format!("Failed to read config file: {}", path))?;

        let config: Config = toml::from_str(&content)
            .with_context(|| "Failed to parse config TOML")?;

        // Validate
        if config.domains.is_empty() {
            anyhow::bail!("No domains configured");
        }

        Ok(config)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[allow(unused_imports)]
    use std::io::Write;

    #[test]
    fn test_default_warning_days() {
        assert_eq!(default_warning_days(), 30);
    }

    #[test]
    fn test_default_critical_days() {
        assert_eq!(default_critical_days(), 14);
    }

    #[test]
    fn test_alert_config_default() {
        // AlertConfig derives Default (zero-init), serde defaults only apply during deserialization
        let config = AlertConfig::default();
        assert_eq!(config.warning_days, 0);
        assert_eq!(config.critical_days, 0);
    }

    #[test]
    fn test_alert_config_serde_defaults() {
        // When deserialized from empty TOML, serde uses the default_* functions
        let config: AlertConfig = toml::from_str("").unwrap();
        assert_eq!(config.warning_days, 30);
        assert_eq!(config.critical_days, 14);
    }

    #[test]
    fn test_default_ssl_seconds() {
        assert_eq!(default_ssl_seconds(), 3600);
    }

    #[test]
    fn test_default_online_seconds() {
        assert_eq!(default_online_seconds(), 60);
    }

    #[test]
    fn test_config_load_valid_toml() {
        let toml_content = r#"
[mqtt]
host = "localhost"
port = 1883
client_id = "test-client"

[http]
socket_path = "/tmp/test.sock"

[polling]
ssl_seconds = 7200
online_seconds = 120

[alerts]
warning_days = 45
critical_days = 21

[domains.example]
hostname = "www.example.com"
port = 443
label = "Example Site"
check_http = true
"#;

        let dir = std::env::temp_dir().join("symbion-ssl-test-valid");
        let _ = std::fs::create_dir_all(&dir);
        let path = dir.join("config.toml");
        std::fs::write(&path, toml_content).unwrap();

        let config = Config::load(path.to_str().unwrap()).unwrap();
        let _ = std::fs::remove_dir_all(&dir);

        // Verify MQTT config
        assert_eq!(config.mqtt.host, "localhost");
        assert_eq!(config.mqtt.port, 1883);
        assert_eq!(config.mqtt.client_id, "test-client");

        // Verify HTTP config
        assert_eq!(config.http.socket_path, "/tmp/test.sock");

        // Verify polling config
        assert_eq!(config.polling.ssl_seconds, 7200);
        assert_eq!(config.polling.online_seconds, 120);

        // Verify alerts config
        assert_eq!(config.alerts.warning_days, 45);
        assert_eq!(config.alerts.critical_days, 21);

        // Verify domain config
        assert_eq!(config.domains.len(), 1);
        let domain = config.domains.get("example").unwrap();
        assert_eq!(domain.hostname, "www.example.com");
        assert_eq!(domain.port, 443);
        assert_eq!(domain.label, Some("Example Site".to_string()));
        assert!(domain.check_http);
    }

    #[test]
    fn test_config_load_empty_domains_fails() {
        let toml_content = r#"
[mqtt]
host = "localhost"
port = 1883

[http]
socket_path = "/tmp/test.sock"

[polling]
ssl_seconds = 3600
online_seconds = 60

[alerts]
warning_days = 30
critical_days = 14

[domains]
"#;

        let dir = std::env::temp_dir().join("symbion-ssl-test-empty");
        let _ = std::fs::create_dir_all(&dir);
        let path = dir.join("config.toml");
        std::fs::write(&path, toml_content).unwrap();

        let result = Config::load(path.to_str().unwrap());
        let _ = std::fs::remove_dir_all(&dir);
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("No domains configured"));
    }

    #[test]
    fn test_domain_config_default_port() {
        assert_eq!(default_port(), 443);
    }
}
