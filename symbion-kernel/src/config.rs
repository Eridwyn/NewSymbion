use serde::{Deserialize, Serialize};
use std::{collections::HashMap, path::Path};
use tokio::fs;

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct HostsConfig {
    pub hosts: HashMap<String, HostConf>,
    pub wol: Option<WolConf>,
    pub mqtt: Option<MqttConf>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct HostConf {
    pub mac: String,
    pub hint: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct WolConf {
    pub command: String, // ex: "/home/mark/Bureau/Symbion/wake.sh {host_id} {mac} {hint}"
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct MqttConf {
    pub host: String,
    pub port: u16,
}

impl Default for HostsConfig {
    fn default() -> Self {
        Self {
            hosts: HashMap::new(),
            wol: None,
            mqtt: Some(MqttConf { host: "localhost".into(), port: 1883 }),
        }
    }
}

pub async fn load_config() -> HostsConfig {
    let path = std::env::var("SYMBION_KERNEL_CONFIG").unwrap_or_else(|_| "kernel.yaml".into());
    if Path::new(&path).exists() {
        let txt = fs::read_to_string(&path).await.unwrap_or_default();
        if txt.trim().is_empty() { return HostsConfig::default(); }
        serde_yaml::from_str(&txt).unwrap_or_else(|e| {
            eprintln!("[kernel] config invalide: {e}");
            HostsConfig::default()
        })
    } else {
        eprintln!("[kernel] pas de kernel.yaml, usage config par défaut");
        HostsConfig::default()
    }
}
