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
mod notes_bridge;
mod agents;
mod auth;
mod context;

use crate::models::HostsMap;
use crate::state::{new_state, Shared};
use crate::config::{load_config, HostsConfig};
use crate::http::AppState;
use crate::contracts::ContractRegistry;
use crate::health::HealthTracker;
use crate::ports::create_default_ports;
use crate::plugins::PluginManager;
use crate::notes_bridge::{NotesBridge, SharedNotesBridge};
use crate::agents::{AgentRegistry, SharedAgentRegistry};
use crate::auth::AuthManager;
use crate::context::ContextEngine;

use std::collections::HashMap;
use std::net::SocketAddr;
use std::sync::Arc;

#[tokio::main]
async fn main() {
    // Charger les variables d'environnement depuis .env (si présent)
    dotenvy::dotenv().ok(); // Ok si .env n'existe pas
    
    // maps et conf partagées
    let states = new_state::<HostsMap>(HashMap::new());
    let cfg_loaded: HostsConfig = load_config().await;
    let cfg: Shared<HostsConfig> = new_state(cfg_loaded.clone());
    
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

    // context engine
    let context_engine = Arc::new(ContextEngine::new());
    println!("[kernel] initialized context engine");

    // auth manager
    let auth_manager = match AuthManager::new() {
        Ok(manager) => manager,
        Err(e) => {
            eprintln!("[kernel] failed to initialize auth manager: {}", e);
            std::process::exit(1);
        }
    };

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
    std::fs::create_dir_all("./plugins").unwrap_or_else(|e| {
        eprintln!("[kernel] warning: failed to create plugins dir: {}", e);
    });
    
    let mut plugin_manager = PluginManager::new("./plugins");
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

    // Client MQTT partagé pour le kernel et bridge notes
    let mqtt_client = match mqtt::create_mqtt_client(&cfg_loaded) {
        Ok(client) => client,
        Err(e) => {
            eprintln!("[kernel] failed to create MQTT client: {}", e);
            std::process::exit(1);
        }
    };

    // Bridge notes pour API /ports/memo → plugin via MQTT  
    let notes_bridge: Option<SharedNotesBridge> = Some(Arc::new(NotesBridge::new(mqtt_client.clone())));

    // Agent registry avec persistance et MQTT
    let mut agent_registry = AgentRegistry::new("./data/agents.json").with_mqtt_client(mqtt_client.clone());
    if let Err(e) = agent_registry.load_agents().await {
        eprintln!("[kernel] failed to load agents: {}", e);
    }
    let agents: SharedAgentRegistry = Arc::new(agent_registry);

    // MQTT remplit les states + agents
    mqtt::spawn_mqtt_listener(states.clone(), cfg.clone(), notes_bridge.clone(), Some(agents.clone()), Some(health_tracker.clone()));

    // démarre le healthcheck périodique des plugins
    plugins::spawn_plugin_health_monitor(plugins.clone());
    
    // démarre le monitoring des agents (timeout 2min)
    AgentRegistry::start_agent_monitoring(agents.clone(), 2);

    // démarre la publication auto du health
    health_tracker.spawn_health_publisher(cfg.clone(), contracts.clone(), agents.clone(), plugins.clone());

    // démarre le monitoring contextuel (détection mode toutes les 30s)
    context::ContextEngine::spawn_context_monitor(context_engine.clone(), agents.clone(), mqtt_client.clone());

    // fabrique l'état unique pour Axum
    let app_state = AppState {
        states,
        cfg,
        contracts,
        health_tracker,
        auth_manager,
        ports,
        plugins,
        notes_bridge,
        agents: agents.clone(),
        context_engine: context_engine.clone(),
    };

    // HTTPS avec TLS
    let app = http::build_router(app_state);

    // Charger certificats TLS depuis variables d'environnement
    let cert_path = std::env::var("SYMBION_TLS_CERT_PATH")
        .unwrap_or_else(|_| "symbion-kernel/certs/cert.pem".to_string());
    let key_path = std::env::var("SYMBION_TLS_KEY_PATH")
        .unwrap_or_else(|_| "symbion-kernel/certs/key.pem".to_string());

    let port: u16 = std::env::var("SYMBION_HTTPS_PORT")
        .ok()
        .and_then(|p| p.parse().ok())
        .unwrap_or(8443);

    let addr = SocketAddr::from(([0,0,0,0], port));

    // Configuration TLS avec rustls
    let config = axum_server::tls_rustls::RustlsConfig::from_pem_file(&cert_path, &key_path)
        .await
        .expect(&format!("Failed to load TLS certificates from {} and {}", cert_path, key_path));

    println!("[kernel] 🔒 HTTPS enabled - listening on https://{}", addr);
    println!("[kernel] TLS cert: {}", cert_path);
    println!("[kernel] TLS key: {}", key_path);

    axum_server::bind_rustls(addr, config)
        .serve(app.into_make_service())
        .await
        .unwrap();
}
