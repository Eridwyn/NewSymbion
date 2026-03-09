// SYMBION KERNEL - Point d'entrée principal du serveur Symbion
//
// RÔLE : Orchestration du bootstrap séquentiel de tous les sous-systèmes.
// La logique d'initialisation détaillée est dans bootstrap/*.
//
// ARCHITECTURE : Event-driven via MQTT + API REST + Data Ports + monitoring temps réel.

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
mod database;
mod dew_point_alerts;
mod sensors;
mod environment_http;
mod plugin_proxy;
mod plugin_health;
mod environment_alerts;
mod automations;
mod automations_http;
mod modes;
mod schedule;
mod notifications;
mod notification_config;
mod intelligence;
mod context_intelligence;
mod intelligence_http;
mod plugins;
mod mqtt_watchdog;
mod rate_limiter;
mod openapi;
mod file_hub;
mod bootstrap;

use crate::models::HostsMap;
use crate::state::{new_state, Shared};
use crate::config::{load_config, HostsConfig};
use crate::http::AppState;
use crate::contracts::ContractRegistry;
use crate::health::HealthTracker;
use crate::notes_bridge::{NotesBridge, SharedNotesBridge};
use crate::agents::AgentRegistry;
use crate::context::ContextEngine;

use std::collections::HashMap;
use std::sync::Arc;
use std::io::Write;

#[tokio::main]
async fn main() {
    // Force line-buffered stdout for systemd journal capture
    let _ = std::io::stdout().flush();

    // Load .env, validate secrets, install panic hook & crypto provider
    dotenvy::dotenv().ok();
    bootstrap::auth::validate_required_secrets();
    install_panic_hook();
    let _ = rustls::crypto::ring::default_provider().install_default();

    let boot_start = std::time::Instant::now();

    // === Phase 1: Core config & shared state ===
    let states = new_state::<HostsMap>(HashMap::new());
    let cfg_loaded: HostsConfig = load_config().await;
    let cfg: Shared<HostsConfig> = new_state(cfg_loaded.clone());

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

    let health_tracker = HealthTracker::new();
    let context_engine = Arc::new(ContextEngine::new());
    println!("[kernel] initialized context engine");

    // === Phase 2: Auth subsystem ===
    let auth = bootstrap::auth::init_auth();
    eprintln!("[kernel] ⏱ auth subsystem ready in {:?}", boot_start.elapsed());

    // === Phase 3: MQTT client & bridges ===
    let mqtt_client = match mqtt::create_mqtt_client(&cfg_loaded) {
        Ok(client) => client,
        Err(e) => {
            eprintln!("[kernel] failed to create MQTT client: {}", e);
            std::process::exit(1);
        }
    };
    let dashboard_events = dashboard_events::DashboardEventPublisher::new(mqtt_client.clone());
    let mqtt_watchdog = mqtt_watchdog::create_watchdog();
    let notes_bridge: Option<SharedNotesBridge> = Some(Arc::new(NotesBridge::new(mqtt_client.clone())));

    let mut agent_registry = AgentRegistry::new("./data/agents.json").with_mqtt_client(mqtt_client.clone());
    if let Err(e) = agent_registry.load_agents().await {
        eprintln!("[kernel] failed to load agents: {}", e);
    }

    // === Phase 4: Database + persistence wiring ===
    let db_sub = bootstrap::database::init_database(auth, agent_registry, &context_engine, &mqtt_client).await;

    // === Phase 5: Intelligence engines ===
    let int_sub = bootstrap::intelligence::init_intelligence(&db_sub.db);
    eprintln!("[kernel] ⏱ intelligence subsystem ready in {:?}", boot_start.elapsed());

    // === Phase 6: Decision engine ===
    let dec_sub = bootstrap::decision::init_decision(&db_sub.db);
    eprintln!("[kernel] ⏱ decision subsystem ready in {:?}", boot_start.elapsed());

    // === Phase 7: Background tasks ===
    // MQTT listener (spans all subsystems — kept in main)
    mqtt::spawn_mqtt_listener(
        states.clone(), cfg.clone(), notes_bridge.clone(),
        Some(db_sub.agents.clone()), Some(db_sub.sensor_registry.clone()),
        Some(health_tracker.clone()), Some(dashboard_events.clone()),
        Some(mqtt_watchdog.clone()), Some(db_sub.notifications_manager.clone()),
        Some(int_sub.feature_registry.clone()),
    );

    let tasks = bootstrap::tasks::spawn_background_tasks(
        &db_sub, &dec_sub, &int_sub,
        &context_engine, &mqtt_client, &mqtt_watchdog,
        &cfg, &contracts, &health_tracker, &dashboard_events,
    ).await;

    // === Phase 8: Assemble AppState & run servers ===
    let app_state = AppState {
        states,
        cfg,
        contracts,
        health_tracker,
        auth_manager: db_sub.auth_manager,
        mfa_manager: db_sub.mfa_manager,
        csrf_manager: db_sub.csrf_manager,
        device_trust_manager: db_sub.device_trust_manager,
        webauthn_manager: db_sub.webauthn_manager,
        notes_bridge,
        agents: db_sub.agents,
        context_engine,
        dashboard_events,
        decision_engine: dec_sub.decision_engine,
        decision_validation_manager: dec_sub.decision_validation_manager,
        decision_override_manager: dec_sub.decision_override_manager,
        decision_audit_manager: dec_sub.decision_audit_manager,
        decision_agent_health_manager: dec_sub.decision_agent_health_manager,
        decision_metrics: dec_sub.decision_metrics,
        sensors: db_sub.sensor_registry,
        plugin_registry: tasks.plugin_registry,
        automations: tasks.automations_store,
        automation_dispatcher: tasks.automation_dispatcher,
        pending_action_registry: dec_sub.pending_action_registry,
        mode_registry: db_sub.mode_registry,
        schedule_registry: int_sub.schedule_registry,
        notifications_manager: db_sub.notifications_manager,
        notification_config: db_sub.notification_config,
        context_intelligence: db_sub.context_intelligence,
        feature_registry: int_sub.feature_registry,
        inference_engine: int_sub.inference_engine,
        session_manager: int_sub.session_manager,
        trust_tracker: dec_sub.trust_tracker,
        rate_limiter: rate_limiter::RateLimitStore::new(),
        file_hub: {
            let hub = Arc::new(file_hub::FileHub::new(std::path::Path::new("./data")));
            let hub_cleanup = hub.clone();
            tokio::spawn(async move {
                let mut interval = tokio::time::interval(std::time::Duration::from_secs(300));
                loop {
                    interval.tick().await;
                    hub_cleanup.cleanup_expired().await;
                }
            });
            Some(hub)
        },
    };

    bootstrap::server::run_servers(app_state, boot_start).await;
}

/// PR5: Panic hook for logging before crash
fn install_panic_hook() {
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
}
