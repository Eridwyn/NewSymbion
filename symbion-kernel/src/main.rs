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
mod notes_bridge;
mod notes_ws;
mod agents;
mod auth;
mod context;
mod dashboard_events;
mod mfa;
mod csrf;
mod device_trust;
mod decision;
mod decision_http;
mod webauthn;
mod environment;
mod dew_point_alerts;  // F1: Physics-based humidity alerts (Magnus dew point formula)
mod sensors;  // F1: Sensor registry for scalable IoT sensors
mod environment_http;  // F1: API endpoints for environment monitoring
// F4: Mobile API removed - now part of symbion-plugin-notifications
mod plugin_proxy;  // Dynamic plugin routing via Unix sockets
mod plugin_health;  // Plugin health monitoring and auto-recovery
mod notification_client;  // Safe notification client (checks plugin availability)
mod environment_alerts;  // Environment alert monitor with notifications

use crate::models::HostsMap;
use crate::state::{new_state, Shared};
use crate::config::{load_config, HostsConfig};
use crate::http::AppState;
use crate::contracts::ContractRegistry;
use crate::health::HealthTracker;
use crate::notes_bridge::{NotesBridge, SharedNotesBridge};
use crate::agents::{AgentRegistry, SharedAgentRegistry};
use crate::auth::AuthManager;
use crate::context::ContextEngine;
use crate::mfa::MfaManager;
use crate::csrf::CsrfManager;
use crate::webauthn::WebAuthnManager;

use std::collections::HashMap;
use std::net::SocketAddr;
use std::sync::Arc;

#[tokio::main]
async fn main() {
    // Charger les variables d'environnement depuis .env (si présent)
    dotenvy::dotenv().ok(); // Ok si .env n'existe pas

    // PR5: Panic hook pour logging avant crash (aide au debugging)
    std::panic::set_hook(Box::new(|panic_info| {
        use time::OffsetDateTime;
        let timestamp = OffsetDateTime::now_utc();
        let payload = if let Some(s) = panic_info.payload().downcast_ref::<&str>() {
            s.to_string()
        } else if let Some(s) = panic_info.payload().downcast_ref::<String>() {
            s.clone()
        } else {
            "Unknown panic payload".to_string()
        };

        let location = if let Some(loc) = panic_info.location() {
            format!("{}:{}:{}", loc.file(), loc.line(), loc.column())
        } else {
            "Unknown location".to_string()
        };

        eprintln!("\n╔════════════════════════════════════════════════════════════════╗");
        eprintln!("║ 🔴 KERNEL PANIC DETECTED - PR5                                  ║");
        eprintln!("╠════════════════════════════════════════════════════════════════╣");
        eprintln!("║ Timestamp: {}-{:02}-{:02} {:02}:{:02}:{:02} UTC                ║",
            timestamp.year(), timestamp.month() as u8, timestamp.day(),
            timestamp.hour(), timestamp.minute(), timestamp.second());
        eprintln!("║ Location:  {:<54} ║", &location[..location.len().min(54)]);
        eprintln!("║ Message:   {:<54} ║", &payload[..payload.len().min(54)]);
        eprintln!("╠════════════════════════════════════════════════════════════════╣");
        eprintln!("║ Systemd will auto-restart kernel in 5 seconds...               ║");
        eprintln!("║ Check logs: journalctl -u symbion-kernel -n 100                ║");
        eprintln!("╚════════════════════════════════════════════════════════════════╝\n");
    }));

    // Initialiser le CryptoProvider pour Rustls (fix crash rustls 0.23)
    let _ = rustls::crypto::ring::default_provider().install_default();

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

    // mfa manager
    let mfa_manager = Arc::new(MfaManager::new(
        "Symbion".to_string(),
        "Symbion".to_string(),
    ));
    println!("[kernel] initialized MFA manager");

    // csrf manager
    let csrf_manager = Arc::new(CsrfManager::new());
    println!("[kernel] initialized CSRF manager");

    // webauthn manager
    let rp_id = std::env::var("SYMBION_WEBAUTHN_RP_ID")
        .unwrap_or_else(|_| "symbion.local".to_string());
    let rp_origin = std::env::var("SYMBION_WEBAUTHN_RP_ORIGIN")
        .unwrap_or_else(|_| "https://symbion.local:3000".to_string());
    let webauthn_storage_path = std::path::PathBuf::from("./data/webauthn_credentials.json");
    let webauthn_manager = match WebAuthnManager::new(&rp_id, &rp_origin, webauthn_storage_path) {
        Ok(manager) => Arc::new(manager),
        Err(e) => {
            eprintln!("[kernel] failed to initialize webauthn manager: {}", e);
            std::process::exit(1);
        }
    };
    println!("[kernel] initialized WebAuthn manager");

    // device trust manager
    let device_trust_manager = match crate::device_trust::DeviceTrustManager::new() {
        Ok(manager) => Arc::new(manager),
        Err(e) => {
            eprintln!("[kernel] failed to initialize device trust manager: {}", e);
            std::process::exit(1);
        }
    };
    println!("[kernel] initialized Device Trust manager");

    // Client MQTT partagé pour le kernel et bridge notes
    let mqtt_client = match mqtt::create_mqtt_client(&cfg_loaded) {
        Ok(client) => client,
        Err(e) => {
            eprintln!("[kernel] failed to create MQTT client: {}", e);
            std::process::exit(1);
        }
    };

    // Dashboard event publisher pour événements temps réel
    let dashboard_events = dashboard_events::DashboardEventPublisher::new(mqtt_client.clone());
    println!("[kernel] initialized dashboard event publisher");

    // Bridge notes pour API /ports/memo → plugin via MQTT
    let notes_bridge: Option<SharedNotesBridge> = Some(Arc::new(NotesBridge::new(mqtt_client.clone())));

    // Agent registry avec persistance et MQTT
    let mut agent_registry = AgentRegistry::new("./data/agents.json").with_mqtt_client(mqtt_client.clone());
    if let Err(e) = agent_registry.load_agents().await {
        eprintln!("[kernel] failed to load agents: {}", e);
    }
    let agents: SharedAgentRegistry = Arc::new(agent_registry);

    // F1: Sensor Registry pour capteurs environnementaux distribués
    let sensor_registry_instance = crate::sensors::SensorRegistry::new("./data/sensors.json");
    if let Err(e) = sensor_registry_instance.load_from_disk() {
        eprintln!("[kernel] warning: failed to load sensors from disk: {}", e);
    }
    let sensor_registry = Arc::new(sensor_registry_instance);
    println!("[kernel] initialized Sensor Registry (F1 Environment)");

    // MQTT remplit les states + agents + sensors (F1)
    mqtt::spawn_mqtt_listener(states.clone(), cfg.clone(), notes_bridge.clone(), Some(agents.clone()), Some(sensor_registry.clone()), Some(health_tracker.clone()), Some(dashboard_events.clone()));

    // démarre le monitoring des agents (timeout 2min)
    AgentRegistry::start_agent_monitoring(agents.clone(), 2);

    // démarre la sauvegarde périodique débounced des agents (toutes les 5min si modifiés)
    AgentRegistry::start_periodic_save(agents.clone());

    // démarre la sauvegarde périodique débounced des environnements sensors (toutes les 5min si modifiés)
    crate::sensors::SensorRegistry::start_periodic_env_save(sensor_registry.clone());

    // démarre le monitoring périodique des sensors (toutes les 10s : stale data + offline sensors)
    crate::sensors::SensorRegistry::start_periodic_monitoring(sensor_registry.clone());

    // Dynamic Plugin Registry - création anticipée pour le health tracking
    let plugin_registry = crate::plugin_proxy::PluginRegistry::new();

    // démarre la publication auto du health
    health_tracker.spawn_health_publisher(cfg.clone(), contracts.clone(), agents.clone(), plugin_registry.clone(), dashboard_events.clone());

    // démarre le monitoring contextuel (détection mode toutes les 30s)
    context::ContextEngine::spawn_context_monitor(context_engine.clone(), agents.clone(), mqtt_client.clone(), dashboard_events.clone());

    // Decision Engine PR3 - Initialisation
    let decision_clock = Arc::new(crate::decision::SystemClock);
    let decision_config = crate::decision::DecisionConfig::default();

    let decision_validation_manager = Arc::new(crate::decision::ValidationManager::new(
        decision_clock.clone(),
        1800, // TTL 30 minutes (plus raisonnable pour validation humaine)
    ));

    let decision_override_manager = Arc::new(crate::decision::OverrideManager::new(
        decision_clock.clone(),
        86400, // Default TTL 24h
    ));

    let decision_audit_manager = Arc::new(crate::decision::AuditManager::new(
        decision_clock.clone(),
        10000, // Max 10k records
    ));

    // Agent Health Mapping configuration
    let agent_health_mapping = crate::decision::AgentHealthMapping {
        online_min_score: 0.8,
        active_min_score: 0.7,
        idle_min_score: 0.5,
        degraded_min_score: 0.3,
        degraded_consecutive_threshold: 3,
        stale_max_age_secs: 120, // 2 minutes
    };

    let decision_agent_health_manager = Arc::new(crate::decision::AgentHealthManager::new(
        decision_clock.clone(),
        agent_health_mapping,
    ));

    let decision_metrics = Arc::new(crate::decision::DecisionMetrics::new());

    // Guards Evaluator avec stratégie court-circuit OnBlock
    let guards_evaluator = crate::decision::GuardsEvaluator::new(
        crate::decision::ShortCircuitStrategy::OnBlock
    );

    // Trust Calculator
    let trust_calculator = crate::decision::TrustCalculator::new(
        decision_config.clone(),
        decision_clock.clone(),
    );

    let decision_engine = Arc::new(crate::decision::DecisionEngine::new(
        guards_evaluator,
        trust_calculator,
        decision_config,
    ));

    println!("[kernel] initialized Decision Engine PR3");

    // Dynamic Plugin Registry - découverte automatique des plugins Unix sockets
    if let Err(e) = plugin_registry.discover_plugins().await {
        eprintln!("[kernel] failed to discover plugins: {}", e);
    }

    // Notification Client - interface sécurisée (vérifie si plugin dispo)
    let notification_client = notification_client::NotificationClient::new(
        mqtt_client.clone(),
        plugin_registry.clone(),
    );
    println!("[kernel] notification client initialized (plugin-aware)");

    // Plugin Health Monitor - surveillance automatique et auto-recovery
    let plugin_health_monitor = crate::plugin_health::PluginHealthMonitor::new();
    plugin_health_monitor.spawn_health_monitor(plugin_registry.clone(), notification_client.clone());

    // Environment Alert Monitor - surveillance alertes moisissure avec notifications
    let env_alert_monitor = environment_alerts::EnvironmentAlertMonitor::new(
        sensor_registry.clone(),
        notification_client.clone(),
    );
    env_alert_monitor.spawn_monitor();
    println!("[kernel] environment alert monitor started");

    // fabrique l'état unique pour Axum
    let app_state = AppState {
        states,
        cfg,
        contracts,
        health_tracker,
        auth_manager,
        mfa_manager,
        csrf_manager,
        device_trust_manager,
        webauthn_manager,
        notes_bridge,
        agents: agents.clone(),
        context_engine: context_engine.clone(),
        dashboard_events,
        decision_engine,
        decision_validation_manager,
        decision_override_manager,
        decision_audit_manager,
        decision_agent_health_manager,
        decision_metrics,
        sensors: sensor_registry,
        plugin_registry,
        notification_client,
    };

    // HTTPS avec TLS (PWA + mTLS)
    let app_https = http::build_router(app_state.clone());

    // Charger certificats TLS depuis variables d'environnement
    let cert_path = std::env::var("SYMBION_TLS_CERT_PATH")
        .unwrap_or_else(|_| "symbion-kernel/certs/cert-mkcert.pem".to_string());
    let key_path = std::env::var("SYMBION_TLS_KEY_PATH")
        .unwrap_or_else(|_| "symbion-kernel/certs/key-mkcert.pem".to_string());

    let https_port: u16 = std::env::var("SYMBION_HTTPS_PORT")
        .ok()
        .and_then(|p| p.parse().ok())
        .unwrap_or(8443);

    let http_port: u16 = std::env::var("SYMBION_HTTP_PORT")
        .ok()
        .and_then(|p| p.parse().ok())
        .unwrap_or(8080);

    let https_addr = SocketAddr::from(([0,0,0,0], https_port));
    let http_addr = SocketAddr::from(([0,0,0,0], http_port));

    // Configuration TLS avec rustls
    let tls_config = axum_server::tls_rustls::RustlsConfig::from_pem_file(&cert_path, &key_path)
        .await
        .expect(&format!("Failed to load TLS certificates from {} and {}", cert_path, key_path));

    println!("[kernel] 🔒 HTTPS enabled - listening on https://{}", https_addr);
    println!("[kernel] TLS cert: {}", cert_path);
    println!("[kernel] TLS key: {}", key_path);

    // Serveur HTTP simple pour redirection vers HTTPS
    let redirect_app = http::build_redirect_router(https_port);
    println!("[kernel] 🔄 HTTP redirect enabled - listening on http://{} → https://localhost:{}", http_addr, https_port);

    // F4: Mobile API removed - now part of symbion-plugin-notifications (port 8445)

    // Lancer les deux serveurs en parallèle
    let https_server = axum_server::bind_rustls(https_addr, tls_config)
        .serve(app_https.into_make_service());

    let http_server = axum::serve(
        tokio::net::TcpListener::bind(http_addr).await.unwrap(),
        redirect_app.into_make_service()
    );

    tokio::try_join!(https_server, http_server).unwrap();
}
