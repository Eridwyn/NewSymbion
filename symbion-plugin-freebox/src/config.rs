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

    /// App token obtained during authorization.
    ///
    /// SECURITY NOTE: This token is stored in plaintext in the TOML config file.
    /// For production deployments, prefer setting the FREEBOX_APP_TOKEN environment
    /// variable instead, which takes precedence over this field. If neither is set,
    /// the plugin will fail to start.
    #[serde(default)]
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
    15  // Check presence every 15s for responsive detection
}

fn default_connection_interval() -> u64 {
    30  // Connection status every 30s
}

fn default_downloads_interval() -> u64 {
    30
}

fn default_devices_interval() -> u64 {
    120  // Full device list every 2 minutes
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
    /// Load configuration from file.
    ///
    /// Environment variable overrides:
    /// - `FREEBOX_APP_TOKEN`: overrides `freebox.app_token` from the TOML file.
    ///   This is the recommended way to provide the token in production to avoid
    ///   storing secrets in plaintext config files.
    pub fn load(path: &str) -> anyhow::Result<Self> {
        let content = std::fs::read_to_string(path)?;
        let mut config: Config = toml::from_str(&content)?;

        // Environment variable override for sensitive app_token
        if let Ok(token) = std::env::var("FREEBOX_APP_TOKEN") {
            if !token.is_empty() {
                config.freebox.app_token = token;
            }
        }

        // Validate that app_token is set (from either source)
        if config.freebox.app_token.is_empty() {
            anyhow::bail!(
                "Freebox app_token is required. Set it in the TOML config file \
                 or via the FREEBOX_APP_TOKEN environment variable."
            );
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
    fn test_polling_config_default() {
        let config = PollingConfig::default();
        assert_eq!(config.presence_seconds, 15);
        assert_eq!(config.connection_seconds, 30);
        assert_eq!(config.downloads_seconds, 30);
        assert_eq!(config.devices_seconds, 120);
    }

    #[test]
    fn test_default_functions() {
        // Test all 11 default functions
        assert_eq!(default_freebox_url(), "http://mafreebox.freebox.fr");
        assert_eq!(default_app_id(), "symbion.freebox");
        assert_eq!(default_mqtt_host(), "127.0.0.1");
        assert_eq!(default_mqtt_port(), 1883);
        assert_eq!(default_mqtt_client_id(), "symbion-plugin-freebox");
        assert_eq!(default_mqtt_prefix(), "symbion/freebox");
        assert_eq!(default_socket_path(), "/run/symbion-plugins/freebox.sock");
        assert_eq!(default_device_type(), "unknown");
        assert_eq!(default_presence_interval(), 15);
        assert_eq!(default_connection_interval(), 30);
        assert_eq!(default_downloads_interval(), 30);
        assert_eq!(default_devices_interval(), 120);
    }

    #[test]
    fn test_config_load_valid() {
        let toml_content = r#"
[freebox]
api_url = "http://192.168.1.254"
app_id = "test.app"
app_token = "test-token-12345"

[mqtt]
host = "192.168.1.1"
port = 1883
client_id = "test-client"
topic_prefix = "test/freebox"

[http]
socket_path = "/tmp/freebox.sock"

[polling]
presence_seconds = 10
connection_seconds = 20
downloads_seconds = 25
devices_seconds = 100

[devices.phone]
freebox_name = "iPhone"
device_type = "phone"
friendly_name = "Mon iPhone"
"#;

        let dir = std::env::temp_dir().join("symbion-freebox-test-valid");
        let _ = std::fs::create_dir_all(&dir);
        let path = dir.join("config.toml");
        std::fs::write(&path, toml_content).unwrap();

        let config = Config::load(path.to_str().unwrap()).unwrap();
        let _ = std::fs::remove_dir_all(&dir);

        // Verify Freebox config
        assert_eq!(config.freebox.api_url, "http://192.168.1.254");
        assert_eq!(config.freebox.app_id, "test.app");
        assert_eq!(config.freebox.app_token, "test-token-12345");

        // Verify MQTT config
        assert_eq!(config.mqtt.host, "192.168.1.1");
        assert_eq!(config.mqtt.port, 1883);
        assert_eq!(config.mqtt.client_id, "test-client");
        assert_eq!(config.mqtt.topic_prefix, "test/freebox");

        // Verify HTTP config
        assert_eq!(config.http.socket_path, "/tmp/freebox.sock");

        // Verify polling config
        assert_eq!(config.polling.presence_seconds, 10);
        assert_eq!(config.polling.connection_seconds, 20);
        assert_eq!(config.polling.downloads_seconds, 25);
        assert_eq!(config.polling.devices_seconds, 100);

        // Verify device config
        assert_eq!(config.devices.len(), 1);
        let device = config.devices.get("phone").unwrap();
        assert_eq!(device.freebox_name, "iPhone");
        assert_eq!(device.device_type, "phone");
        assert_eq!(device.friendly_name, Some("Mon iPhone".to_string()));
    }

    #[test]
    fn test_config_load_env_var_override() {
        let toml_content = r#"
[freebox]
api_url = "http://192.168.1.254"
app_id = "test.app"
app_token = "toml-token"

[mqtt]
host = "127.0.0.1"

[http]
socket_path = "/tmp/freebox-env-test.sock"
"#;

        let dir = std::env::temp_dir().join("symbion-freebox-test-env");
        let _ = std::fs::create_dir_all(&dir);
        let path = dir.join("config.toml");
        std::fs::write(&path, toml_content).unwrap();

        // Set env var - should override TOML value
        std::env::set_var("FREEBOX_APP_TOKEN", "env-secret-token");
        let config = Config::load(path.to_str().unwrap()).unwrap();
        assert_eq!(config.freebox.app_token, "env-secret-token");

        // Clean up
        std::env::remove_var("FREEBOX_APP_TOKEN");
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn test_config_load_missing_token_fails() {
        let toml_content = r#"
[freebox]
api_url = "http://192.168.1.254"
app_id = "test.app"

[mqtt]
host = "127.0.0.1"

[http]
socket_path = "/tmp/freebox-notoken-test.sock"
"#;

        let dir = std::env::temp_dir().join("symbion-freebox-test-notoken");
        let _ = std::fs::create_dir_all(&dir);
        let path = dir.join("config.toml");
        std::fs::write(&path, toml_content).unwrap();

        // Ensure env var is not set
        std::env::remove_var("FREEBOX_APP_TOKEN");
        let result = Config::load(path.to_str().unwrap());
        assert!(result.is_err());
        let err_msg = result.unwrap_err().to_string();
        assert!(err_msg.contains("app_token"), "Error should mention app_token: {}", err_msg);

        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn test_config_serialization_roundtrip() {
        // Create a config with all fields
        let mut devices = HashMap::new();
        devices.insert(
            "laptop".to_string(),
            DeviceConfig {
                freebox_name: "MacBook".to_string(),
                device_type: "laptop".to_string(),
                friendly_name: Some("Mon MacBook".to_string()),
            },
        );

        let original = Config {
            freebox: FreeboxConfig {
                api_url: "http://test.local".to_string(),
                app_id: "test.app".to_string(),
                app_token: "secret-token".to_string(),
            },
            mqtt: MqttConfig {
                host: "mqtt.local".to_string(),
                port: 8883,
                client_id: "test-id".to_string(),
                topic_prefix: "test/prefix".to_string(),
            },
            http: HttpConfig {
                socket_path: "/tmp/test.sock".to_string(),
            },
            devices,
            polling: PollingConfig {
                presence_seconds: 5,
                connection_seconds: 10,
                downloads_seconds: 15,
                devices_seconds: 60,
            },
        };

        // Serialize to TOML
        let toml_str = toml::to_string(&original).unwrap();

        // Deserialize back
        let deserialized: Config = toml::from_str(&toml_str).unwrap();

        // Verify all fields match
        assert_eq!(deserialized.freebox.api_url, original.freebox.api_url);
        assert_eq!(deserialized.freebox.app_id, original.freebox.app_id);
        assert_eq!(deserialized.freebox.app_token, original.freebox.app_token);
        assert_eq!(deserialized.mqtt.host, original.mqtt.host);
        assert_eq!(deserialized.mqtt.port, original.mqtt.port);
        assert_eq!(deserialized.mqtt.client_id, original.mqtt.client_id);
        assert_eq!(deserialized.mqtt.topic_prefix, original.mqtt.topic_prefix);
        assert_eq!(deserialized.http.socket_path, original.http.socket_path);
        assert_eq!(deserialized.polling.presence_seconds, original.polling.presence_seconds);
        assert_eq!(deserialized.polling.connection_seconds, original.polling.connection_seconds);
        assert_eq!(deserialized.polling.downloads_seconds, original.polling.downloads_seconds);
        assert_eq!(deserialized.polling.devices_seconds, original.polling.devices_seconds);
        assert_eq!(deserialized.devices.len(), 1);

        let device = deserialized.devices.get("laptop").unwrap();
        assert_eq!(device.freebox_name, "MacBook");
        assert_eq!(device.device_type, "laptop");
        assert_eq!(device.friendly_name, Some("Mon MacBook".to_string()));
    }
}
