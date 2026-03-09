/**
 * Bootstrap: Database & Persistence Subsystem
 *
 * Initializes SQLite, wires database to auth/agents/sensors/modes/context,
 * and initializes registries that depend on persistence.
 */

use crate::agents::{AgentRegistry, SharedAgentRegistry};
use crate::auth::AuthManager;
use crate::bootstrap::auth::AuthSubsystem;
use crate::context::ContextEngine;
use crate::context_intelligence::SharedContextIntelligence;
use crate::database::SharedDatabase;
use crate::device_trust::DeviceTrustManager;
use crate::modes::SharedModeRegistry;
use rumqttc::AsyncClient;
use crate::notifications::SharedNotificationManager;
use crate::notification_config::SharedNotificationConfigManager;
use crate::sensors::SharedSensorRegistry;
use crate::webauthn::WebAuthnManager;
use std::sync::Arc;

pub struct DatabaseSubsystem {
    pub db: Option<SharedDatabase>,
    pub auth_manager: AuthManager,
    pub mfa_manager: Arc<crate::mfa::MfaManager>,
    pub csrf_manager: Arc<crate::csrf::CsrfManager>,
    pub device_trust_manager: Arc<DeviceTrustManager>,
    pub webauthn_manager: Arc<WebAuthnManager>,
    pub agents: SharedAgentRegistry,
    pub sensor_registry: SharedSensorRegistry,
    pub mode_registry: SharedModeRegistry,
    pub context_intelligence: SharedContextIntelligence,
    pub notifications_manager: SharedNotificationManager,
    pub notification_config: SharedNotificationConfigManager,
}

pub async fn init_database(
    auth: AuthSubsystem,
    mut agent_registry: AgentRegistry,
    context_engine: &Arc<ContextEngine>,
    mqtt_client: &AsyncClient,
) -> DatabaseSubsystem {
    // === SQLite Database ===
    let db: Option<SharedDatabase> = match crate::database::Database::open("./data/symbion.db") {
        Ok(db) => {
            let db = Arc::new(db);
            eprintln!("[kernel] SQLite database initialized (WAL mode, r2d2 pool)");
            if let Err(e) = crate::database::Database::import_json_if_needed(
                &db,
                "./data/sensors_environments.json",
                "./data/automations_history.json",
            ) {
                eprintln!("[kernel] JSON import warning (non-fatal): {}", e);
            }
            Some(db)
        }
        Err(e) => {
            eprintln!("[kernel] WARNING: SQLite init failed, JSON-only mode: {}", e);
            None
        }
    };

    // Wire auth managers to database
    let auth_manager = if let Some(ref db) = db {
        auth.auth_manager.with_database(db.clone())
    } else {
        auth.auth_manager
    };
    let device_trust_manager = if let Some(ref db) = db {
        auth.device_trust_manager.with_database(db.clone())
    } else {
        auth.device_trust_manager
    };
    let device_trust_manager = Arc::new(device_trust_manager);
    let webauthn_manager = if let Some(ref db) = db {
        auth.webauthn_manager.with_database(db.clone())
    } else {
        auth.webauthn_manager
    };
    let webauthn_manager = Arc::new(webauthn_manager);
    if let Some(ref db) = db {
        agent_registry.set_database(db.clone()).await;
    }
    let agents: SharedAgentRegistry = Arc::new(agent_registry);

    // Sensor Registry
    let mut sensor_registry_instance = crate::sensors::SensorRegistry::new("./data/sensors.json");
    if let Some(ref db) = db {
        sensor_registry_instance = sensor_registry_instance.with_database(db.clone());
    }
    if let Err(e) = sensor_registry_instance.load_from_disk() {
        eprintln!("[kernel] warning: failed to load sensors from disk: {}", e);
    }
    let sensor_registry = Arc::new(sensor_registry_instance);
    eprintln!("[kernel] initialized Sensor Registry (F1 Environment)");

    // Mode Registry
    let mut mode_registry = crate::modes::ModeRegistry::new(std::path::PathBuf::from("./data"));
    if let Some(ref db) = db {
        mode_registry.with_database(db.clone());
    }
    let mode_registry = Arc::new(mode_registry);
    eprintln!("[kernel] initialized Mode Registry ({} modes)", mode_registry.count());

    // Wire context engine to SQLite
    if let Some(ref db) = db {
        context_engine.set_database(db.clone());
    }

    // Context Intelligence Engine
    let context_intelligence = Arc::new(crate::context_intelligence::ContextIntelligence::new(
        context_engine.clone(),
        agents.clone(),
        sensor_registry.clone(),
    ));
    if let Some(ref db) = db {
        context_intelligence.set_database(db.clone());
    }
    context_intelligence.init_patterns_from_history();
    eprintln!("[kernel] initialized Context Intelligence Engine");

    // Notifications Manager (with MQTT for PWA push)
    let notifications_manager = crate::notifications::NotificationManager::new(Some(mqtt_client.clone()));
    let notifications_manager = if let Some(ref db) = db {
        notifications_manager.with_database(db.clone())
    } else {
        notifications_manager
    };
    let notifications_manager = Arc::new(notifications_manager);
    eprintln!("[kernel] initialized Notifications Manager");

    // Notification Configuration Manager
    let notification_config = crate::notification_config::NotificationConfigManager::new();
    let notification_config = if let Some(ref db) = db {
        notification_config.with_database(db.clone())
    } else {
        notification_config
    };
    let notification_config = Arc::new(notification_config);
    eprintln!("[kernel] initialized Notification Config Manager ({} types)", notification_config.list_all().len());

    DatabaseSubsystem {
        db,
        auth_manager,
        mfa_manager: auth.mfa_manager,
        csrf_manager: auth.csrf_manager,
        device_trust_manager,
        webauthn_manager,
        agents,
        sensor_registry,
        mode_registry,
        context_intelligence,
        notifications_manager,
        notification_config,
    }
}
