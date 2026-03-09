/**
 * Bootstrap: Background Tasks & Monitoring
 *
 * Spawns all async background tasks: agent monitoring, sensor monitoring,
 * health publishing, context monitoring, automation listeners, plugin health, etc.
 */

use crate::agents::AgentRegistry;
use crate::automations::{AutomationStore, EventDispatcher};
use crate::bootstrap::database::DatabaseSubsystem;
use crate::bootstrap::decision::DecisionSubsystem;
use crate::bootstrap::intelligence::IntelligenceSubsystem;
use crate::config::HostsConfig;
use crate::contracts::ContractRegistry;
use crate::context::ContextEngine;
use crate::dashboard_events::DashboardEventPublisher;
use crate::health::HealthTracker;
use crate::plugin_proxy::PluginRegistry;
use crate::state::Shared;
use rumqttc::AsyncClient;
use std::sync::Arc;

pub struct TasksContext {
    pub automations_store: Arc<AutomationStore>,
    pub automation_dispatcher: EventDispatcher,
    pub plugin_registry: PluginRegistry,
}

pub async fn spawn_background_tasks(
    db_sub: &DatabaseSubsystem,
    dec_sub: &DecisionSubsystem,
    int_sub: &IntelligenceSubsystem,
    context_engine: &Arc<ContextEngine>,
    mqtt_client: &AsyncClient,
    mqtt_watchdog: &crate::mqtt_watchdog::SharedMqttWatchdog,
    cfg: &Shared<HostsConfig>,
    contracts: &ContractRegistry,
    health_tracker: &HealthTracker,
    dashboard_events: &DashboardEventPublisher,
) -> TasksContext {
    // === Registries & Dispatchers (needed by tasks below) ===

    // Automations Store
    let mut automations_store = crate::automations::AutomationStore::new(std::path::PathBuf::from("./data"))
        .expect("Failed to initialize automations store");
    if let Some(ref db) = db_sub.db {
        automations_store = automations_store.with_database(db.clone());
    }
    let automations_store = Arc::new(automations_store);
    match automations_store.ensure_system_defaults() {
        Ok(count) if count > 0 => eprintln!("[kernel] created {} default system automations", count),
        Ok(_) => {}
        Err(e) => eprintln!("[kernel] warning: failed to create system automations: {}", e),
    }
    eprintln!("[kernel] initialized Automations Store");

    // Automations Event Dispatcher (broadcast channel for triggers)
    let (automation_dispatcher, automation_receiver) = EventDispatcher::new();
    eprintln!("[kernel] initialized Automations Event Dispatcher");

    // Connect dispatcher to agents for status events
    db_sub.agents.set_automation_dispatcher(automation_dispatcher.clone()).await;

    // Dynamic Plugin Registry
    let plugin_registry = PluginRegistry::new();

    // === Spawn monitoring tasks ===

    // MQTT watchdog task
    {
        let watchdog_state = mqtt_watchdog.clone();
        let watchdog_agents = db_sub.agents.clone();
        tokio::spawn(async move {
            let config = crate::mqtt_watchdog::MqttWatchdogConfig::default();
            crate::mqtt_watchdog::run_watchdog(
                watchdog_state,
                config,
                move || {
                    let agents_clone = watchdog_agents.clone();
                    tokio::task::block_in_place(|| {
                        tokio::runtime::Handle::current().block_on(async {
                            agents_clone.list_agents().await.len() > 0
                        })
                    })
                },
            ).await;
        });
        println!("[kernel] spawned MQTT watchdog task");
    }

    // Agent monitoring tasks
    AgentRegistry::start_agent_monitoring(db_sub.agents.clone(), 2);
    AgentRegistry::start_periodic_save(db_sub.agents.clone());
    AgentRegistry::start_command_timeout_checker(db_sub.agents.clone());

    // Sensor monitoring tasks
    crate::sensors::SensorRegistry::start_periodic_env_save(db_sub.sensor_registry.clone());
    crate::sensors::SensorRegistry::start_periodic_monitoring(db_sub.sensor_registry.clone());

    // Health publisher
    health_tracker.spawn_health_publisher(
        cfg.clone(),
        contracts.clone(),
        db_sub.agents.clone(),
        plugin_registry.clone(),
        dashboard_events.clone(),
    );

    // Context monitor (mode detection every 30s)
    crate::context::ContextEngine::spawn_context_monitor(
        context_engine.clone(),
        db_sub.agents.clone(),
        mqtt_client.clone(),
        dashboard_events.clone(),
        Some(automation_dispatcher.clone()),
    );

    // Note: MQTT listener (spawn_mqtt_listener) is spawned from main.rs
    // because it depends on states/cfg/notes_bridge which span all subsystems.

    // Plugin discovery
    if let Err(e) = plugin_registry.discover_plugins().await {
        eprintln!("[kernel] failed to discover plugins: {}", e);
    }

    // Spawn automation listener with all integrations
    crate::automations::spawn_automation_listener(
        automations_store.clone(),
        context_engine.clone(),
        db_sub.agents.clone(),
        db_sub.sensor_registry.clone(),
        db_sub.notifications_manager.clone(),
        automation_receiver,
        Some(dec_sub.decision_engine.clone()),
        Some(dec_sub.trust_tracker.clone()),
        Some(dec_sub.decision_validation_manager.clone()),
        Some(dec_sub.pending_action_registry.clone()),
        Some(db_sub.context_intelligence.clone()),
        Some(db_sub.mode_registry.clone()),
        Some(int_sub.feature_registry.clone()),
    );
    eprintln!("[kernel] started Automations Event Listener");

    // Automation Scheduler
    let automation_scheduler = crate::automations::AutomationScheduler::new(
        automations_store.clone(),
        automation_dispatcher.clone(),
    );
    automation_scheduler.spawn();

    // Plugin Health Monitor
    let plugin_health_monitor = crate::plugin_health::PluginHealthMonitor::new();
    plugin_health_monitor.spawn_health_monitor(plugin_registry.clone(), automation_dispatcher.clone());

    // Environment Alert Monitor
    let env_alert_monitor = crate::environment_alerts::EnvironmentAlertMonitor::new(
        db_sub.sensor_registry.clone(),
        db_sub.notifications_manager.clone(),
        db_sub.notification_config.clone(),
        Some(automation_dispatcher.clone()),
    );
    env_alert_monitor.spawn_monitor();
    println!("[kernel] environment alert monitor started");

    // Context Intelligence Monitor (shadow mode v1/v2)
    crate::context_intelligence::ContextIntelligence::spawn_intelligence_monitor(
        db_sub.context_intelligence.clone(),
        db_sub.mode_registry.clone(),
        db_sub.notifications_manager.clone(),
        int_sub.feature_registry.clone(),
        int_sub.inference_engine.clone(),
        db_sub.agents.clone(),
    );
    println!("[kernel] context intelligence monitor started (with v2 shadow mode)");

    TasksContext {
        automations_store,
        automation_dispatcher,
        plugin_registry,
    }
}
