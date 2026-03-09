use serde::Deserialize;

#[derive(Debug, Clone, Deserialize)]
pub struct Config {
    pub mqtt: MqttConfig,
    pub http: HttpConfig,
    pub database: DatabaseConfig,
    pub worker: WorkerConfig,
}

#[derive(Debug, Clone, Deserialize)]
pub struct MqttConfig {
    #[serde(default = "default_mqtt_host")]
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

#[derive(Debug, Clone, Deserialize)]
pub struct DatabaseConfig {
    #[serde(default = "default_db_path")]
    pub path: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct WorkerConfig {
    #[serde(default = "default_debounce")]
    pub debounce_seconds: u64,
}

fn default_mqtt_host() -> String { "127.0.0.1".to_string() }
fn default_mqtt_port() -> u16 { 1883 }
fn default_client_id() -> String { "symbion-plugin-library".to_string() }
fn default_socket_path() -> String { "/run/symbion-plugins/library.sock".to_string() }
fn default_db_path() -> String { "data/library.db".to_string() }
fn default_debounce() -> u64 { 5 }

impl Config {
    pub fn load(path: &str) -> anyhow::Result<Self> {
        let content = std::fs::read_to_string(path)?;
        let config: Config = toml::from_str(&content)?;
        Ok(config)
    }
}
