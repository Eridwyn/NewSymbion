use anyhow::{Context, Result};
use serde::Deserialize;

#[derive(Debug, Clone, Deserialize)]
pub struct Config {
    pub nut: NutConfig,
    pub mqtt: MqttConfig,
    pub http: HttpConfig,
    #[serde(default = "default_poll_interval")]
    pub poll_interval_seconds: u64,
}

#[derive(Debug, Clone, Deserialize)]
pub struct NutConfig {
    pub host: String,
    #[serde(default = "default_nut_port")]
    pub port: u16,
    #[serde(default = "default_ups_name")]
    pub ups_name: String,
    pub username: Option<String>,
    pub password: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct MqttConfig {
    pub host: String,
    #[serde(default = "default_mqtt_port")]
    pub port: u16,
    #[serde(default = "default_client_id")]
    pub client_id: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct HttpConfig {
    #[serde(default = "default_socket_path")]
    pub socket_path: String,
}

fn default_nut_port() -> u16 { 3493 }
fn default_ups_name() -> String { "ups".to_string() }
fn default_mqtt_port() -> u16 { 1883 }
fn default_client_id() -> String { "symbion-plugin-synology".to_string() }
fn default_socket_path() -> String { "/run/symbion-plugins/synology.sock".to_string() }
fn default_poll_interval() -> u64 { 30 }

impl Config {
    pub fn load(path: &str) -> Result<Self> {
        let content = std::fs::read_to_string(path)
            .with_context(|| format!("Cannot read config: {}", path))?;
        toml::from_str(&content)
            .with_context(|| "Failed to parse synology.toml")
    }
}
