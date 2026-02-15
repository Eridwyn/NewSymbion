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
mod plugin_proxy;  // Dynamic plugin routing via Unix sockets
mod plugin_health;  // Plugin health monitoring and auto-recovery
mod environment_alerts;  // Environment alert monitor with notifications
mod automations;  // Automation rules engine
mod automations_http;  // Automation API endpoints
mod modes;  // Dynamic modes with custom themes
mod schedule;  // Time-based scheduling for modes
mod notifications;  // Notification manager with FCM, SMTP, ntfy.sh
mod notification_config;  // Notification configuration (enable/disable, templates)
mod intelligence;  // Intelligence types and config (extracted for modularity)
mod context_intelligence;  // Intelligent context adaptation system
mod intelligence_http;  // Intelligence API endpoints
mod plugins;  // Plugin Contract v1 - Plugin system structures and types
mod mqtt_watchdog;  // MQTT connection watchdog - detects half-dead connections

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
use std::io::Write;

/// [SECURITY] P0-1: Validation des secrets obligatoires au démarrage
/// Le kernel REFUSE de démarrer si ces variables ne sont pas définies.
/// Aucun fallback, aucune valeur par défaut - sécurité non négociable.
fn validate_required_secrets() {
    let mut missing = Vec::new();

    // JWT_SECRET: Obligatoire pour l'authentification
    if std::env::var("SYMBION_JWT_SECRET").is_err() {
        missing.push("SYMBION_JWT_SECRET");
    }

    // API_KEY: Obligatoire pour les WebSockets et fallback auth
    if std::env::var("SYMBION_API_KEY").is_err() {
        missing.push("SYMBION_API_KEY");
    }

    if !missing.is_empty() {
        eprintln!("\n╔════════════════════════════════════════════════════════════════╗");
        eprintln!("║ 🔴 SECURITY: Missing required environment variables            ║");
        eprintln!("╠════════════════════════════════════════════════════════════════╣");
        for var in &missing {
            eprintln!("║   ❌ {}                                       ║", var);
        }
        eprintln!("╠════════════════════════════════════════════════════════════════╣");
        eprintln!("║ The kernel CANNOT start without these secrets configured.      ║");
        eprintln!("║ See .env.example for required configuration.                   ║");
        eprintln!("╚════════════════════════════════════════════════════════════════╝\n");
        std::process::exit(1);
    }

    println!("[SECURITY] All required secrets validated ✓");
}

#[tokio::main]
async fn main() {
    // Force line-buffered stdout for systemd journal capture
    // Sans ça, certains println! sont perdus dans le buffering
    let _ = std::io::stdout().flush();

    // Charger les variables d'environnement depuis .env (si présent)
    dotenvy::dotenv().ok(); // Ok si .env n'existe pas

    // [SECURITY] P0-1: Validation obligatoire des secrets au démarrage
    // Le kernel REFUSE de démarrer si les secrets critiques sont absents
    validate_required_secrets();

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

    // MQTT Watchdog - détecte les connexions half-dead (pub OK, sub KO)
    let mqtt_watchdog = mqtt_watchdog::create_watchdog();
    println!("[kernel] initialized MQTT watchdog");

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

    // Mode Registry pour modes dynamiques
    let mode_registry = crate::modes::create_shared_registry(std::path::PathBuf::from("./data"));
    eprintln!("[kernel] initialized Mode Registry ({} modes)", mode_registry.count());

    // Context Intelligence Engine - Intelligent autonomous context adaptation
    let context_intelligence = Arc::new(crate::context_intelligence::ContextIntelligence::new(
        context_engine.clone(),
        agents.clone(),
        sensor_registry.clone(),
    ));
    // Initialize patterns from historical data
    context_intelligence.init_patterns_from_history();
    eprintln!("[kernel] initialized Context Intelligence Engine");

    // Feature Registry for data-driven intelligence (v2)
    let feature_registry = Arc::new(crate::intelligence::FeatureRegistry::new());
    eprintln!("[kernel] initialized Feature Registry");

    // Inference Engine for case-based mode prediction (v2) with persistence
    let samples_path = std::path::PathBuf::from("./data/intelligence_samples.json");
    let inference_engine = Arc::new(crate::intelligence::InferenceEngine::with_persistence(
        crate::intelligence::InferenceConfig::default(),
        samples_path,
    ));
    eprintln!("[kernel] initialized Inference Engine v2 (with persistence)");

    // Session Manager for hysteresis-based mode transitions (v2)
    let session_manager = Arc::new(crate::intelligence::SessionManager::default());
    eprintln!("[kernel] initialized Session Manager v2");

    // Bootstrap scheduler for cold start (seeds inference engine if needed)
    let bootstrap_scheduler = crate::intelligence::BootstrapScheduler::default();
    let initial_vector = crate::intelligence::VectorBuilder::new(&feature_registry).build();
    bootstrap_scheduler.seed_inference_engine(&inference_engine, &initial_vector);

    // Schedule Registry pour planning horaire
    let schedule_registry = crate::schedule::create_shared_registry(std::path::PathBuf::from("./data"));
    eprintln!("[kernel] initialized Schedule Registry ({} rules)", schedule_registry.count_rules());

    // Notifications Manager (FCM, SMTP, ntfy.sh, MQTT for PWA)
    let notifications_manager = crate::notifications::create_shared_manager(Some(mqtt_client.clone()));
    eprintln!("[kernel] initialized Notifications Manager");

    // Notification Configuration Manager
    let notification_config = crate::notification_config::create_shared_config_manager();
    eprintln!("[kernel] initialized Notification Config Manager ({} types)", notification_config.list_all().len());

    // Automations Store
    let automations_store = Arc::new(
        crate::automations::AutomationStore::new(std::path::PathBuf::from("./data"))
            .expect("Failed to initialize automations store")
    );
    // Create default system automations if needed (environment alerts, plugin health)
    match automations_store.ensure_system_defaults() {
        Ok(count) if count > 0 => eprintln!("[kernel] created {} default system automations", count),
        Ok(_) => {}
        Err(e) => eprintln!("[kernel] warning: failed to create system automations: {}", e),
    }
    eprintln!("[kernel] initialized Automations Store");

    // Automations Event Dispatcher (broadcast channel for triggers)
    let (automation_dispatcher, automation_receiver) = crate::automations::EventDispatcher::new();
    eprintln!("[kernel] initialized Automations Event Dispatcher");

    // Connecter le dispatcher aux agents pour événements status
    agents.set_automation_dispatcher(automation_dispatcher.clone()).await;

    // MQTT remplit les states + agents + sensors (F1) + notifications ack + features
    mqtt::spawn_mqtt_listener(states.clone(), cfg.clone(), notes_bridge.clone(), Some(agents.clone()), Some(sensor_registry.clone()), Some(health_tracker.clone()), Some(dashboard_events.clone()), Some(mqtt_watchdog.clone()), Some(notifications_manager.clone()), Some(feature_registry.clone()));

    // Spawn MQTT watchdog task - détecte les connexions half-dead
    {
        let watchdog_state = mqtt_watchdog.clone();
        let watchdog_agents = agents.clone();
        tokio::spawn(async move {
            let config = mqtt_watchdog::MqttWatchdogConfig::default();
            mqtt_watchdog::run_watchdog(
                watchdog_state,
                config,
                move || {
                    // Vérifie si des agents sont enregistrés (sync check via blocking)
                    // Note: Cette closure est appelée périodiquement par le watchdog
                    let agents_clone = watchdog_agents.clone();
                    tokio::task::block_in_place(|| {
                        tokio::runtime::Handle::current().block_on(async {
                            agents_clone.list_agents().await.len() > 0
                        })
                    })
                }
            ).await;
        });
        println!("[kernel] spawned MQTT watchdog task");
    }

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
    context::ContextEngine::spawn_context_monitor(context_engine.clone(), agents.clone(), mqtt_client.clone(), dashboard_events.clone(), Some(automation_dispatcher.clone()));

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

    // Trust Tracker for evolving trust statistics (Phase 7)
    // Must be created BEFORE TrustCalculator so it can use the tracker
    let trust_tracker = Arc::new(crate::decision::TrustTracker::new("./data"));
    println!("[kernel] initialized Trust Tracker (evolving statistics)");

    // Trust Calculator with Trust Tracker integration
    let trust_calculator = crate::decision::TrustCalculator::with_trust_tracker(
        decision_config.clone(),
        decision_clock.clone(),
        trust_tracker.clone(),
    );

    let decision_engine = Arc::new(crate::decision::DecisionEngine::new(
        guards_evaluator,
        trust_calculator,
        decision_config,
    ));

    println!("[kernel] initialized Decision Engine PR3 (with Trust Tracker)");

    // Pending Action Registry for post-approval execution
    let pending_action_registry = Arc::new(crate::automations::PendingActionRegistry::new());
    println!("[kernel] initialized Pending Action Registry");

    // Dynamic Plugin Registry - découverte automatique des plugins Unix sockets
    if let Err(e) = plugin_registry.discover_plugins().await {
        eprintln!("[kernel] failed to discover plugins: {}", e);
    }

    // Spawn automation listener with Decision Engine + Trust Tracker + Intelligence + ModeRegistry + FeatureRegistry (Phase 7 + Invariant 2)
    crate::automations::spawn_automation_listener(
        automations_store.clone(),
        context_engine.clone(),
        agents.clone(),
        sensor_registry.clone(),
        notifications_manager.clone(), // Direct NotificationManager (no plugin wrapper)
        automation_receiver,
        Some(decision_engine.clone()), // Decision Engine for trust evaluation
        Some(trust_tracker.clone()),   // Trust Tracker for evolving statistics
        Some(decision_validation_manager.clone()), // Validation Manager for pending approvals
        Some(pending_action_registry.clone()), // Pending Action Registry for post-approval execution
        Some(context_intelligence.clone()), // Intelligence for feedback loop (Decision → Intelligence)
        Some(mode_registry.clone()), // Mode Registry for validating dynamic modes (Invariant 2)
        Some(feature_registry.clone()), // Feature Registry for condition evaluation (presence, env, etc.)
    );
    eprintln!("[kernel] started Automations Event Listener (with DecisionEngine + TrustTracker + ValidationManager + PendingActionRegistry + NotificationsManager + Intelligence + ModeRegistry + FeatureRegistry)");

    // Automation Scheduler - polling for scheduled triggers
    let automation_scheduler = crate::automations::AutomationScheduler::new(
        automations_store.clone(),
        automation_dispatcher.clone(),
    );
    automation_scheduler.spawn();

    // Plugin Health Monitor - surveillance automatique et auto-recovery
    // Dispatches events to automation system (which handles notifications via configured automations)
    let plugin_health_monitor = crate::plugin_health::PluginHealthMonitor::new();
    plugin_health_monitor.spawn_health_monitor(plugin_registry.clone(), automation_dispatcher.clone());

    // Environment Alert Monitor - surveillance alertes moisissure avec notifications
    let env_alert_monitor = environment_alerts::EnvironmentAlertMonitor::new(
        sensor_registry.clone(),
        notifications_manager.clone(),
        notification_config.clone(),
        Some(automation_dispatcher.clone()),
    );
    env_alert_monitor.spawn_monitor();
    println!("[kernel] environment alert monitor started");

    // Context Intelligence Monitor - autonomous mode prediction and adaptation
    // With shadow mode: compares v1 and v2 predictions in parallel
    crate::context_intelligence::ContextIntelligence::spawn_intelligence_monitor(
        context_intelligence.clone(),
        mode_registry.clone(),
        notifications_manager.clone(),
        feature_registry.clone(),
        inference_engine.clone(),
        agents.clone(),
    );
    println!("[kernel] context intelligence monitor started (with v2 shadow mode)");

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
        automations: automations_store,
        automation_dispatcher: automation_dispatcher.clone(),
        pending_action_registry,
        mode_registry,
        schedule_registry,
        notifications_manager,
        notification_config,
        context_intelligence: context_intelligence.clone(),
        feature_registry: feature_registry.clone(),
        inference_engine: inference_engine.clone(),
        session_manager: session_manager.clone(),
        trust_tracker: trust_tracker.clone(),
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

    // Lancer les deux serveurs en parallèle
    let https_server = axum_server::bind_rustls(https_addr, tls_config)
        .serve(app_https.into_make_service());

    let http_server = axum::serve(
        tokio::net::TcpListener::bind(http_addr).await.unwrap(),
        redirect_app.into_make_service()
    );

    tokio::try_join!(https_server, http_server).unwrap();
}
