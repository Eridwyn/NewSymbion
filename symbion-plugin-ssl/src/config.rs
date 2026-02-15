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

/// Alert thresholds
#[derive(Debug, Clone, Deserialize)]
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
