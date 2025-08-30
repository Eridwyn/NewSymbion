/**
 * SYMBION KERNEL - Point d'entrée principal du serveur Symbion
 * 
 * RÔLE : Orchestration de tous les modules : config, MQTT, HTTP, health, ports.
 * Bootstrap du système complet avec gestion d'erreurs et logging.
 * 
 * ARCHITECTURE : Event-driven via MQTT + API REST + Data Ports + monitoring temps réel.
 * UTILITÉ : Cerveau central de l'écosystème Symbion, point d'administration unique.
 */

mod models;
mod state;
mod mqtt;
mod http;
mod config;
mod wol;
mod contracts;
mod health;
mod ports;
mod plugins;

use crate::models::HostsMap;
use crate::state::{new_state, Shared};
use crate::config::{load_config, HostsConfig};
use crate::http::AppState;
use crate::contracts::ContractRegistry;
use crate::health::HealthTracker;
use crate::ports::create_default_ports;
use crate::plugins::PluginManager;

use std::collections::HashMap;
use std::net::SocketAddr;
use tokio::net::TcpListener;

#[tokio::main]
async fn main() {
    // maps et conf partagées
    let states = new_state::<HostsMap>(HashMap::new());
    let cfg_loaded: HostsConfig = load_config().await;
    let cfg: Shared<HostsConfig> = new_state(cfg_loaded);
    
    // chargement des contrats MQTT
    let contracts = match ContractRegistry::load_contracts_from_dir("../contracts/mqtt").await {
        Ok(registry) => {
            println!("[kernel] loaded {} contracts", registry.list_contracts().len());
            registry
        }
        Err(e) => {
            eprintln!("[kernel] failed to load contracts: {}", e);
            ContractRegistry::new()
        }
    };

    // health tracker
    let health_tracker = HealthTracker::new();

    // data ports
    std::fs::create_dir_all("./data").unwrap_or_else(|e| {
        eprintln!("[kernel] warning: failed to create data dir: {}", e);
    });
    
    let ports = match create_default_ports("./data") {
        Ok(registry) => {
            println!("[kernel] initialized {} data ports", registry.list_ports().len());
            new_state(registry)
        }
        Err(e) => {
            eprintln!("[kernel] failed to initialize ports: {}", e);
            new_state(crate::ports::PortRegistry::new())
        }
    };

    // plugin manager
    std::fs::create_dir_all("../plugins").unwrap_or_else(|e| {
        eprintln!("[kernel] warning: failed to create plugins dir: {}", e);
    });
    
    let mut plugin_manager = PluginManager::new("../plugins");
    match plugin_manager.discover_plugins().await {
        Ok(discovered) => {
            println!("[kernel] discovered {} plugins", discovered.len());
            plugin_manager.auto_start_plugins();
        }
        Err(e) => {
            eprintln!("[kernel] failed to discover plugins: {}", e);
        }
    }
    let plugins = new_state(plugin_manager);

    // MQTT remplit les states
    mqtt::spawn_mqtt_listener(states.clone(), cfg.clone());

    // démarre le healthcheck périodique des plugins
    plugins::spawn_plugin_health_monitor(plugins.clone());

    // démarre la publication auto du health
    health_tracker.spawn_health_publisher(cfg.clone(), contracts.clone(), states.clone(), plugins.clone());

    // fabrique l'état unique pour Axum
    let app_state = AppState { states, cfg, contracts, health_tracker, ports, plugins };

    // HTTP
    let app = http::build_router(app_state);

    let addr = SocketAddr::from(([0,0,0,0], 8080));
    println!("[kernel] listening on http://{addr}");
    let listener = TcpListener::bind(addr).await.unwrap();
    axum::serve(listener, app).await.unwrap();
}
