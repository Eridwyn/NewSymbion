//! Configuration for the Freebox plugin

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Main plugin configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    /// Freebox API configuration
    pub freebox: FreeboxConfig,

    /// MQTT broker configuration
    pub mqtt: MqttConfig,

    /// HTTP health endpoint configuration
    pub http: HttpConfig,

    /// Devices to track for presence
    #[serde(default)]
    pub devices: HashMap<String, DeviceConfig>,

    /// Polling intervals
    #[serde(default)]
    pub polling: PollingConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FreeboxConfig {
    /// Freebox API URL (default: http://mafreebox.freebox.fr)
    #[serde(default = "default_freebox_url")]
    pub api_url: String,

    /// App ID registered with Freebox
    #[serde(default = "default_app_id")]
    pub app_id: String,

    /// App token obtained during authorization
    pub app_token: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MqttConfig {
    /// MQTT broker host
    #[serde(default = "default_mqtt_host")]
    pub host: String,

    /// MQTT broker port
    #[serde(default = "default_mqtt_port")]
    pub port: u16,

    /// MQTT client ID
    #[serde(default = "default_mqtt_client_id")]
    pub client_id: String,

    /// MQTT topic prefix
    #[serde(default = "default_mqtt_prefix")]
    pub topic_prefix: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HttpConfig {
    /// Unix socket path for health endpoint
    #[serde(default = "default_socket_path")]
    pub socket_path: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeviceConfig {
    /// Device name in Freebox (primary_name)
    pub freebox_name: String,

    /// Device type (phone, tablet, laptop, etc.)
    #[serde(default = "default_device_type")]
    pub device_type: String,

    /// Friendly name for MQTT topics
    #[serde(default)]
    pub friendly_name: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PollingConfig {
    /// Presence check interval in seconds
    #[serde(default = "default_presence_interval")]
    pub presence_seconds: u64,

    /// Connection status check interval in seconds
    #[serde(default = "default_connection_interval")]
    pub connection_seconds: u64,

    /// Downloads check interval in seconds
    #[serde(default = "default_downloads_interval")]
    pub downloads_seconds: u64,

    /// Full device list refresh interval in seconds
    #[serde(default = "default_devices_interval")]
    pub devices_seconds: u64,
}

// Default values
fn default_freebox_url() -> String {
    "http://mafreebox.freebox.fr".to_string()
}

fn default_app_id() -> String {
    "symbion.freebox".to_string()
}

fn default_mqtt_host() -> String {
    "127.0.0.1".to_string()
}

fn default_mqtt_port() -> u16 {
    1883
}

fn default_mqtt_client_id() -> String {
    "symbion-plugin-freebox".to_string()
}

fn default_mqtt_prefix() -> String {
    "symbion/freebox".to_string()
}

fn default_socket_path() -> String {
    "/run/symbion-plugins/freebox.sock".to_string()
}

fn default_device_type() -> String {
    "unknown".to_string()
}

fn default_presence_interval() -> u64 {
    30
}

fn default_connection_interval() -> u64 {
    60
}

fn default_downloads_interval() -> u64 {
    30
}

fn default_devices_interval() -> u64 {
    300
}

impl Default for PollingConfig {
    fn default() -> Self {
        Self {
            presence_seconds: default_presence_interval(),
            connection_seconds: default_connection_interval(),
            downloads_seconds: default_downloads_interval(),
            devices_seconds: default_devices_interval(),
        }
    }
}

impl Config {
    /// Load configuration from file
    pub fn load(path: &str) -> anyhow::Result<Self> {
        let content = std::fs::read_to_string(path)?;
        let config: Config = toml::from_str(&content)?;
        Ok(config)
    }
}
