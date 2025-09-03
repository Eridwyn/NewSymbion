// Tauri setup wizard for Symbion Agent
// Integrates with config.rs and updater.rs modules

use tauri::command;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

// Import our existing modules
mod config;
mod updater;

use config::{AgentConfig, MqttConfig, ElevationConfig, UpdateConfig, UpdateChannel, AgentInfo};
use updater::{AgentUpdater, UpdateInfo};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SetupStep {
    pub id: String,
    pub title: String,
    pub description: String,
    pub completed: bool,
    pub required: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SetupWizardState {
    pub steps: Vec<SetupStep>,
    pub current_step: usize,
    pub config: AgentConfig,
    pub is_first_time: bool,
}

// Tauri commands for frontend integration
#[command]
async fn get_setup_state() -> Result<SetupWizardState, String> {
    let is_first_time = AgentConfig::is_first_time_setup();
    let config = if is_first_time {
        AgentConfig::default()
    } else {
        AgentConfig::load().await.map_err(|e| e.to_string())?
    };
    
    let steps = vec![
        SetupStep {
            id: "welcome".to_string(),
            title: "Bienvenue".to_string(),
            description: "Configuration initiale de l'agent Symbion".to_string(),
            completed: false,
            required: true,
        },
        SetupStep {
            id: "mqtt".to_string(),
            title: "Configuration MQTT".to_string(),
            description: "Connexion au broker MQTT Symbion".to_string(),
            completed: !is_first_time,
            required: true,
        },
        SetupStep {
            id: "elevation".to_string(),
            title: "Privilèges système".to_string(),
            description: "Configuration élévation pour contrôle système".to_string(),
            completed: false,
            required: false,
        },
        SetupStep {
            id: "updates".to_string(),
            title: "Mises à jour automatiques".to_string(),
            description: "Configuration auto-update depuis GitHub".to_string(),
            completed: false,
            required: false,
        },
        SetupStep {
            id: "summary".to_string(),
            title: "Résumé".to_string(),
            description: "Validation et finalisation de la configuration".to_string(),
            completed: false,
            required: true,
        },
    ];
    
    Ok(SetupWizardState {
        steps,
        current_step: 0,
        config,
        is_first_time,
    })
}

#[command]
async fn save_mqtt_config(broker_host: String, broker_port: u16, client_id: Option<String>) -> Result<(), String> {
    let mut config = AgentConfig::load().await.map_err(|e| e.to_string())?;
    
    config.mqtt.broker_host = broker_host;
    config.mqtt.broker_port = broker_port;
    config.mqtt.client_id = client_id;
    
    config.save().await.map_err(|e| e.to_string())
}

#[command]
async fn save_elevation_config(store_credentials: bool, auto_elevate: bool, password: Option<String>) -> Result<(), String> {
    let mut config = AgentConfig::load().await.map_err(|e| e.to_string())?;
    
    config.elevation.store_credentials = store_credentials;
    config.elevation.auto_elevate = auto_elevate;
    config.elevation.cached_password = password;
    
    config.save().await.map_err(|e| e.to_string())
}

#[command]
async fn save_update_config(auto_update: bool, channel: String, check_interval_hours: u32, github_repo: String) -> Result<(), String> {
    let mut config = AgentConfig::load().await.map_err(|e| e.to_string())?;
    
    let update_channel = match channel.as_str() {
        "stable" => UpdateChannel::Stable,
        "beta" => UpdateChannel::Beta,
        "dev" => UpdateChannel::Dev,
        _ => UpdateChannel::Stable,
    };
    
    config.update.auto_update = auto_update;
    config.update.channel = update_channel;
    config.update.check_interval_hours = check_interval_hours;
    config.update.github_repo = github_repo;
    
    config.save().await.map_err(|e| e.to_string())
}

#[command]
async fn test_mqtt_connection(broker_host: String, broker_port: u16) -> Result<bool, String> {
    // Basic connectivity test
    use std::net::TcpStream;
    use std::time::Duration;
    
    let address = format!("{}:{}", broker_host, broker_port);
    match TcpStream::connect_timeout(
        &address.parse().map_err(|e| format!("Invalid address: {}", e))?,
        Duration::from_secs(5)
    ) {
        Ok(_) => Ok(true),
        Err(e) => Err(format!("Connection failed: {}", e)),
    }
}

#[command]
async fn check_for_updates() -> Result<UpdateInfo, String> {
    let config = AgentConfig::load().await.map_err(|e| e.to_string())?;
    let updater = AgentUpdater::new(config);
    
    updater.check_update().await.map_err(|e| e.to_string())
}

#[command]
async fn perform_update(update_info: UpdateInfo) -> Result<(), String> {
    let config = AgentConfig::load().await.map_err(|e| e.to_string())?;
    let updater = AgentUpdater::new(config);
    
    updater.perform_update(&update_info).await.map_err(|e| e.to_string())
}

#[command]
fn get_system_info() -> HashMap<String, String> {
    let mut info = HashMap::new();
    
    info.insert("os".to_string(), std::env::consts::OS.to_string());
    info.insert("arch".to_string(), std::env::consts::ARCH.to_string());
    info.insert("hostname".to_string(), 
        hostname::get().unwrap_or_default().to_string_lossy().to_string());
    info.insert("version".to_string(), env!("CARGO_PKG_VERSION").to_string());
    
    info
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_shell::init())
        .invoke_handler(tauri::generate_handler![
            get_setup_state,
            save_mqtt_config,
            save_elevation_config, 
            save_update_config,
            test_mqtt_connection,
            check_for_updates,
            perform_update,
            get_system_info
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}