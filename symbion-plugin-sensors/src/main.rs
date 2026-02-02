/**
 * SYMBION PLUGIN - Environment Sensors
 *
 * Contract v1.0 compliant plugin for IoT environmental sensors
 *
 * ARCHITECTURE:
 * - Plugin = exécutant pur (pas de décision)
 * - Actions reçues via POST /actions (Kernel → Plugin)
 * - Events émis via MQTT (Plugin → Kernel)
 *
 * MQTT TOPICS (Contract v1.0):
 * - symbion/plugins/sensors/manifest  (publish at startup)
 * - symbion/plugins/sensors/events    (emit events)
 * - symbion/plugins/sensors/health    (heartbeat every 30s)
 *
 * LEGACY TOPICS (backward compatibility):
 * - symbion/sensors/registration@v1   (ESP32 auto-registration)
 * - symbion/sensors/+/env@v1          (environment readings)
 */

use parking_lot::RwLock;
use rumqttc::{AsyncClient, Event, MqttOptions, Packet, Publish, QoS};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;
use time::OffsetDateTime;

// HTTP server imports
use axum::{
    extract::{Path as AxumPath, State},
    http::StatusCode,
    routing::get,
    Json, Router,
};
use symbion_plugin_common::{PluginHttpServer, PluginRegistrationBuilder};
use tokio::signal::unix::{signal, SignalKind};
use tokio::sync::broadcast;
use uuid::Uuid;

// ============================================================================
// CONTRACT v1.0 CONSTANTS
// ============================================================================

const SPEC_VERSION: &str = "1.0";
const PLUGIN_ID: &str = "sensors";
const PLUGIN_VERSION: &str = "1.1.0";

// ============================================================================
// CONTRACT v1.0 MQTT TOPICS
// ============================================================================

mod topics {
    pub const MANIFEST: &str = "symbion/plugins/sensors/manifest";
    pub const EVENTS: &str = "symbion/plugins/sensors/events";
    pub const HEALTH: &str = "symbion/plugins/sensors/health";
    // Legacy topics (backward compatibility)
    pub const LEGACY_REGISTRATION: &str = "symbion/sensors/registration@v1";
    pub const LEGACY_ENV_PATTERN: &str = "symbion/sensors/+/env@v1";
}

// ============================================================================
// CONTRACT v1.0 STRUCTURES
// ============================================================================

/// Action request from Kernel (Contract v1.0)
#[derive(Debug, Clone, Deserialize)]
pub struct ActionRequest {
    pub spec_version: String,
    pub action_id: Uuid,
    pub action_type: String,
    pub payload: serde_json::Value,
    #[serde(default)]
    pub metadata: serde_json::Value,
}

/// Action response to Kernel (Contract v1.0)
#[derive(Debug, Clone, Serialize)]
pub struct ActionResponse {
    pub spec_version: String,
    pub action_id: Uuid,
    pub status: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub result: Option<serde_json::Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<serde_json::Value>,
    pub execution_time_ms: u64,
}

impl ActionResponse {
    fn success(action_id: Uuid, result: serde_json::Value, execution_time_ms: u64) -> Self {
        Self {
            spec_version: SPEC_VERSION.to_string(),
            action_id,
            status: "success".to_string(),
            result: Some(result),
            error: None,
            execution_time_ms,
        }
    }

    fn error(action_id: Uuid, error_msg: &str, execution_time_ms: u64) -> Self {
        Self {
            spec_version: SPEC_VERSION.to_string(),
            action_id,
            status: "error".to_string(),
            result: None,
            error: Some(serde_json::json!({ "message": error_msg })),
            execution_time_ms,
        }
    }
}

/// Event message to Kernel (Contract v1.0)
#[derive(Debug, Clone, Serialize)]
pub struct EventMessage {
    pub spec_version: String,
    pub event_type: String,
    pub plugin_id: String,
    pub payload: serde_json::Value,
    pub timestamp: String,
}

impl EventMessage {
    fn new(event_type: &str, payload: serde_json::Value) -> Self {
        Self {
            spec_version: SPEC_VERSION.to_string(),
            event_type: event_type.to_string(),
            plugin_id: PLUGIN_ID.to_string(),
            payload,
            timestamp: time::OffsetDateTime::now_utc()
                .format(&time::format_description::well_known::Rfc3339)
                .unwrap_or_else(|_| "unknown".to_string()),
        }
    }
}

/// Health status for heartbeat (Contract v1.0)
#[derive(Debug, Clone, Serialize)]
pub struct HealthStatus {
    pub spec_version: String,
    pub plugin_id: String,
    pub status: String,
    pub uptime_seconds: u64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub last_action_at: Option<String>,
}

// ============================================================================
// PLUGIN DATA STRUCTURES
// ============================================================================

/// Sensor metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Sensor {
    pub sensor_id: String,
    pub sensor_type: String,
    pub room_id: String,
    pub firmware_version: String,
    #[serde(with = "time::serde::rfc3339")]
    pub registered_at: OffsetDateTime,
    #[serde(with = "time::serde::rfc3339")]
    pub last_seen: OffsetDateTime,
    pub status: SensorStatus,
    pub battery_pct: Option<f32>,
    pub signal_rssi: Option<i32>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum SensorStatus {
    Online,
    Offline,
    LowBattery,
}

/// Environment reading from sensor
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EnvironmentReading {
    pub temperature_c: f32,
    pub humidity_pct: f32,
    #[serde(with = "time::serde::rfc3339")]
    pub timestamp: OffsetDateTime,
}

/// Room environment state with circular buffer history
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RoomEnvironmentState {
    pub room_id: String,
    pub current: EnvironmentReading,
    pub history: Vec<EnvironmentReading>,
    pub status: EnvironmentStatus,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum EnvironmentStatus {
    Normal,
    MoldRisk,   // Humidité excessive / risque de moisissure
    TempLow,    // temp <16°C
}

/// Sensor registration message from MQTT
#[derive(Debug, Deserialize)]
struct SensorRegistration {
    sensor_id: String,
    sensor_type: String,
    room_id: String,
    firmware_version: String,
}

/// Environment reading from MQTT
#[derive(Debug, Clone, Deserialize)]
struct EnvReadingMqtt {
    sensor_id: String,
    temperature_c: f32,
    humidity_pct: f32,
    signal_rssi: Option<i32>,
}

/// Sensor registry with thread-safe access
struct SensorRegistry {
    sensors: RwLock<HashMap<String, Sensor>>,
    environments: RwLock<HashMap<String, RoomEnvironmentState>>,
}

impl SensorRegistry {
    fn new() -> Self {
        Self {
            sensors: RwLock::new(HashMap::new()),
            environments: RwLock::new(HashMap::new()),
        }
    }

    fn register_sensor(&self, reg: SensorRegistration) -> Sensor {
        let now = OffsetDateTime::now_utc();
        let sensor = Sensor {
            sensor_id: reg.sensor_id.clone(),
            sensor_type: reg.sensor_type,
            room_id: reg.room_id,
            firmware_version: reg.firmware_version,
            registered_at: now,
            last_seen: now,
            status: SensorStatus::Online,
            battery_pct: None,
            signal_rssi: None,
        };

        self.sensors.write().insert(reg.sensor_id, sensor.clone());
        println!("[sensors-plugin] registered sensor: {}", sensor.sensor_id);
        sensor
    }

    fn add_reading(&self, reading: EnvReadingMqtt) -> Result<(), String> {
        // Update sensor last_seen
        {
            let mut sensors = self.sensors.write();
            let sensor = sensors
                .get_mut(&reading.sensor_id)
                .ok_or_else(|| format!("Sensor {} not registered", reading.sensor_id))?;

            sensor.last_seen = OffsetDateTime::now_utc();
            sensor.status = SensorStatus::Online;
            if let Some(rssi) = reading.signal_rssi {
                sensor.signal_rssi = Some(rssi);
            }
        }

        // Add reading to environment history
        let sensor = self.sensors.read().get(&reading.sensor_id).cloned();
        if let Some(sensor) = sensor {
            let env_reading = EnvironmentReading {
                temperature_c: reading.temperature_c,
                humidity_pct: reading.humidity_pct,
                timestamp: OffsetDateTime::now_utc(),
            };

            let mut environments = self.environments.write();
            let room_env = environments
                .entry(sensor.room_id.clone())
                .or_insert_with(|| RoomEnvironmentState {
                    room_id: sensor.room_id.clone(),
                    current: env_reading.clone(),
                    history: Vec::new(),
                    status: EnvironmentStatus::Normal,
                });

            room_env.current = env_reading.clone();
            room_env.history.push(env_reading);

            // Evaluate status based on humidity history duration
            room_env.status = Self::evaluate_status_with_history(&room_env.history);

            // Retention policy: Keep 7 days of data
            // ESP32 sends ~5 sec → 12 readings/min → 17,280 readings/day → ~2,100 per week
            // Keep max 2,100 readings (~7 days with 5sec interval)
            const MAX_READINGS: usize = 2100;
            if room_env.history.len() > MAX_READINGS {
                room_env.history.remove(0);
            }

            // Time-based cleanup: Remove readings older than 7 days
            let seven_days_ago = OffsetDateTime::now_utc() - Duration::from_secs(7 * 24 * 3600);
            room_env.history.retain(|r| r.timestamp > seven_days_ago);

            println!(
                "[sensors-plugin] reading: {} = {:.1}°C, {:.1}% (status: {:?})",
                sensor.sensor_id, reading.temperature_c, reading.humidity_pct, room_env.status
            );
        }

        Ok(())
    }

    /// Evaluate environment status based on humidity duration
    ///
    /// Alert "Humidité excessive / risque de moisissure" if ANY condition met:
    /// - >50% pendant 12h
    /// - >60% pendant 6h
    /// - >70% pendant 2h
    /// - >75% pendant 10 minutes
    ///
    /// Also checks: temp <16°C → TempLow
    fn evaluate_status_with_history(history: &[EnvironmentReading]) -> EnvironmentStatus {
        if history.is_empty() {
            return EnvironmentStatus::Normal;
        }

        let current = &history[history.len() - 1];

        // Priority: Check low temperature first
        if current.temperature_c < 16.0 {
            return EnvironmentStatus::TempLow;
        }

        let now = OffsetDateTime::now_utc();

        // Condition 4: >75% pendant 10 minutes
        // ~12 readings/min * 10 min = 120 readings
        // Require 80% = 96 readings
        let ten_min_ago = now - Duration::from_secs(10 * 60);
        let count_75 = history.iter()
            .filter(|r| r.timestamp > ten_min_ago && r.humidity_pct > 75.0)
            .count();
        if count_75 >= 96 {
            return EnvironmentStatus::MoldRisk;
        }

        // Condition 3: >70% pendant 2h
        // ~12 readings/min * 60 min * 2h = 1440 readings
        // Require 80% = 1152 readings
        let two_hours_ago = now - Duration::from_secs(2 * 3600);
        let count_70 = history.iter()
            .filter(|r| r.timestamp > two_hours_ago && r.humidity_pct > 70.0)
            .count();
        if count_70 >= 1152 {
            return EnvironmentStatus::MoldRisk;
        }

        // Condition 2: >60% pendant 6h
        // ~12 readings/min * 60 min * 6h = 4320 readings
        // Require 80% = 3456 readings
        let six_hours_ago = now - Duration::from_secs(6 * 3600);
        let count_60 = history.iter()
            .filter(|r| r.timestamp > six_hours_ago && r.humidity_pct > 60.0)
            .count();
        if count_60 >= 3456 {
            return EnvironmentStatus::MoldRisk;
        }

        // Condition 1: >50% pendant 12h
        // ~12 readings/min * 60 min * 12h = 8640 readings
        // Require 80% = 6912 readings
        let twelve_hours_ago = now - Duration::from_secs(12 * 3600);
        let count_50 = history.iter()
            .filter(|r| r.timestamp > twelve_hours_ago && r.humidity_pct > 50.0)
            .count();
        if count_50 >= 6912 {
            return EnvironmentStatus::MoldRisk;
        }

        EnvironmentStatus::Normal
    }

    fn list_sensors(&self) -> Vec<Sensor> {
        self.sensors.read().values().cloned().collect()
    }

    fn get_environment(&self, room_id: &str) -> Option<RoomEnvironmentState> {
        self.environments.read().get(room_id).cloned()
    }
}

// ========== HTTP REST API Handlers (Legacy + Contract v1.0 compatible) ==========

/// GET /sensors - Liste tous les sensors enregistrés
async fn list_sensors_http(
    State(state): State<AppState>,
) -> Json<serde_json::Value> {
    let sensors = state.registry.list_sensors();
    Json(serde_json::json!({
        "sensors": sensors,
        "count": sensors.len()
    }))
}

/// GET /environment/:room_id - Récupère l'état environnemental d'une pièce
async fn get_environment_http(
    State(state): State<AppState>,
    AxumPath(room_id): AxumPath<String>,
) -> Result<Json<RoomEnvironmentState>, (StatusCode, String)> {
    match state.registry.get_environment(&room_id) {
        Some(env) => Ok(Json(env)),
        None => Err((
            StatusCode::NOT_FOUND,
            format!("No environment data for room '{}'", room_id),
        )),
    }
}

/// Health check endpoint (Contract v1.0)
async fn health_check() -> Json<serde_json::Value> {
    use std::sync::OnceLock;
    static START_TIME: OnceLock<std::time::Instant> = OnceLock::new();
    let start = START_TIME.get_or_init(std::time::Instant::now);
    let uptime_secs = start.elapsed().as_secs();

    Json(serde_json::json!({
        "status": "healthy",
        "plugin_id": PLUGIN_ID,
        "spec_version": SPEC_VERSION,
        "uptime_seconds": uptime_secs
    }))
}

// ============================================================================
// CONTRACT v1.0 ACTION HANDLERS
// ============================================================================

/// AppState for Contract v1.0 handlers
#[derive(Clone)]
pub struct AppState {
    registry: Arc<SensorRegistry>,
    mqtt_client: AsyncClient,
}

/// POST /actions - Contract v1.0 action endpoint
async fn handle_action(
    State(state): State<AppState>,
    Json(request): Json<ActionRequest>,
) -> Json<serde_json::Value> {
    let start = std::time::Instant::now();

    println!(
        "[sensors] Contract v1.0 action received: {} (id: {})",
        request.action_type, request.action_id
    );

    let response = match request.action_type.as_str() {
        "list_sensors" => handle_list_sensors(&state, &request).await,
        "get_environment" => handle_get_environment(&state, &request).await,
        "get_sensor" => handle_get_sensor(&state, &request).await,
        _ => ActionResponse::error(
            request.action_id,
            &format!("Unknown action type: {}", request.action_type),
            start.elapsed().as_millis() as u64,
        ),
    };

    Json(serde_json::to_value(response).unwrap_or_else(|_| serde_json::json!({"error": "serialization failed"})))
}

async fn handle_list_sensors(state: &AppState, request: &ActionRequest) -> ActionResponse {
    let start = std::time::Instant::now();
    let sensors = state.registry.list_sensors();

    ActionResponse::success(
        request.action_id,
        serde_json::json!({
            "sensors": sensors,
            "count": sensors.len()
        }),
        start.elapsed().as_millis() as u64,
    )
}

async fn handle_get_environment(state: &AppState, request: &ActionRequest) -> ActionResponse {
    let start = std::time::Instant::now();

    let room_id = request.payload.get("room_id")
        .and_then(|v| v.as_str())
        .unwrap_or("");

    if room_id.is_empty() {
        return ActionResponse::error(
            request.action_id,
            "Missing required parameter: room_id",
            start.elapsed().as_millis() as u64,
        );
    }

    match state.registry.get_environment(room_id) {
        Some(env) => ActionResponse::success(
            request.action_id,
            serde_json::to_value(env).unwrap_or_default(),
            start.elapsed().as_millis() as u64,
        ),
        None => ActionResponse::error(
            request.action_id,
            &format!("No environment data for room '{}'", room_id),
            start.elapsed().as_millis() as u64,
        ),
    }
}

async fn handle_get_sensor(state: &AppState, request: &ActionRequest) -> ActionResponse {
    let start = std::time::Instant::now();

    let sensor_id = request.payload.get("sensor_id")
        .and_then(|v| v.as_str())
        .unwrap_or("");

    if sensor_id.is_empty() {
        return ActionResponse::error(
            request.action_id,
            "Missing required parameter: sensor_id",
            start.elapsed().as_millis() as u64,
        );
    }

    let sensors = state.registry.sensors.read();
    match sensors.get(sensor_id) {
        Some(sensor) => ActionResponse::success(
            request.action_id,
            serde_json::to_value(sensor).unwrap_or_default(),
            start.elapsed().as_millis() as u64,
        ),
        None => ActionResponse::error(
            request.action_id,
            &format!("Sensor '{}' not found", sensor_id),
            start.elapsed().as_millis() as u64,
        ),
    }
}

// ============================================================================
// CONTRACT v1.0 EVENT EMISSION
// ============================================================================

/// Emit an event to MQTT (Contract v1.0)
async fn emit_event(client: &AsyncClient, event: EventMessage) {
    match serde_json::to_string(&event) {
        Ok(payload) => {
            if let Err(e) = client.publish(topics::EVENTS, QoS::AtLeastOnce, false, payload).await {
                eprintln!("[sensors] failed to emit event {}: {:?}", event.event_type, e);
            } else {
                println!("[sensors] emitted event: {}", event.event_type);
            }
        }
        Err(e) => {
            eprintln!("[sensors] failed to serialize event: {:?}", e);
        }
    }
}

/// Construit le router HTTP pour le plugin sensors (Contract v1.0)
fn build_router(state: AppState) -> Router {
    use axum::routing::post;

    Router::new()
        .route("/health", get(health_check))
        .route("/actions", post(handle_action))
        .route("/sensors", get(list_sensors_http))
        .route("/environment/:room_id", get(get_environment_http))
        .with_state(state)
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("[sensors] Symbion Environment Sensors Plugin {} (Contract v{})", PLUGIN_VERSION, SPEC_VERSION);
    println!("[sensors] Starting...");

    // Unix socket path
    let socket_path = "/run/symbion-plugins/sensors.sock";

    // Cleanup old socket at startup
    if std::path::Path::new(socket_path).exists() {
        eprintln!("[sensors] cleaning up old socket at startup");
        let _ = std::fs::remove_file(socket_path);
    }

    // Create shutdown channel for graceful termination
    let (shutdown_tx, mut shutdown_rx) = broadcast::channel::<()>(1);

    // MQTT setup (before HTTP so we can pass client to AppState)
    let mut mqttoptions = MqttOptions::new("symbion-plugin-sensors", "127.0.0.1", 1883);
    mqttoptions.set_keep_alive(Duration::from_secs(30));
    mqttoptions.set_clean_session(true);

    let (client, mut eventloop) = AsyncClient::new(mqttoptions, 10);

    // Create registry and AppState
    let registry = Arc::new(SensorRegistry::new());
    let app_state = AppState {
        registry: registry.clone(),
        mqtt_client: client.clone(),
    };

    // Build router with AppState (Contract v1.0)
    let app = build_router(app_state);

    // Start HTTP server
    let socket_path_clone = socket_path.to_string();
    tokio::spawn(async move {
        println!("[sensors] Starting HTTP server on Unix socket: {}", socket_path_clone);
        if let Err(e) = PluginHttpServer::new(&socket_path_clone, app).serve().await {
            eprintln!("[sensors] HTTP server error: {:?}", e);
        }
    });

    // Wait for socket to be created
    tokio::time::sleep(Duration::from_millis(100)).await;

    // Publish manifest (Contract v1.0)
    let manifest = include_str!("../manifest.json");
    if let Err(e) = client.publish(topics::MANIFEST, QoS::AtLeastOnce, true, manifest).await {
        eprintln!("[sensors] failed to publish manifest: {:?}", e);
    } else {
        println!("[sensors] ✅ manifest published on {}", topics::MANIFEST);
    }

    // Subscribe to legacy sensor topics (backward compatibility)
    client
        .subscribe(topics::LEGACY_REGISTRATION, QoS::AtLeastOnce)
        .await?;
    client
        .subscribe(topics::LEGACY_ENV_PATTERN, QoS::AtLeastOnce)
        .await?;

    println!("[sensors] MQTT subscriptions active, Contract v1.0 ready");

    // Heartbeat loop (Contract v1.0 - every 30 seconds)
    let heartbeat_client = client.clone();
    tokio::spawn(async move {
        use std::sync::OnceLock;
        static START_TIME: OnceLock<std::time::Instant> = OnceLock::new();
        let start = START_TIME.get_or_init(std::time::Instant::now);

        let mut interval = tokio::time::interval(Duration::from_secs(30));
        loop {
            interval.tick().await;
            let health = HealthStatus {
                spec_version: SPEC_VERSION.to_string(),
                plugin_id: PLUGIN_ID.to_string(),
                status: "healthy".to_string(),
                uptime_seconds: start.elapsed().as_secs(),
                last_action_at: None,
            };
            if let Ok(payload) = serde_json::to_string(&health) {
                let _ = heartbeat_client.publish(topics::HEALTH, QoS::AtLeastOnce, false, payload).await;
            }
        }
    });

    // Service Discovery (legacy, but still useful)
    let socket_path_clone = socket_path.to_string();
    tokio::spawn(async move {
        tokio::time::sleep(Duration::from_millis(500)).await;

        match PluginRegistrationBuilder::new("sensors", &socket_path_clone)
            .route("/sensors")
            .route("/environment/:room_id")
            .route("/health")
            .route("/actions")
            .version(PLUGIN_VERSION)
            .description("Environment sensors plugin (Contract v1.0)")
            .register()
            .await
        {
            Ok(_) => println!("[sensors] ✅ Registered with kernel via Service Discovery"),
            Err(e) => eprintln!("[sensors] ❌ Failed to register with kernel: {}", e),
        }
    });

    // Signal handlers for graceful shutdown (SIGTERM from systemd, SIGINT from Ctrl+C)
    let socket_path_for_cleanup = socket_path.to_string();
    let shutdown_tx_clone = shutdown_tx.clone();
    tokio::spawn(async move {
        let mut sigterm = signal(SignalKind::terminate()).expect("failed to install SIGTERM handler");
        let mut sigint = signal(SignalKind::interrupt()).expect("failed to install SIGINT handler");

        tokio::select! {
            _ = sigterm.recv() => {
                eprintln!("[sensors] received SIGTERM, shutting down gracefully...");
            }
            _ = sigint.recv() => {
                eprintln!("[sensors] received SIGINT (Ctrl+C), shutting down gracefully...");
            }
        }

        // Cleanup socket
        if std::path::Path::new(&socket_path_for_cleanup).exists() {
            eprintln!("[sensors] cleaning up socket: {}", socket_path_for_cleanup);
            let _ = std::fs::remove_file(&socket_path_for_cleanup);
        }

        // Signal main loop to exit
        let _ = shutdown_tx_clone.send(());
    });

    // Main event loop
    loop {
        tokio::select! {
            // Check for shutdown signal
            _ = shutdown_rx.recv() => {
                eprintln!("[sensors] shutdown signal received, exiting main loop");
                break;
            }
            // Process MQTT events
            event = eventloop.poll() => {
                match event {
                    Ok(Event::Incoming(Packet::Publish(publish))) => {
                        handle_mqtt_message(&registry, &client, publish).await;
                    }
                    Ok(Event::Incoming(_)) => {}
                    Ok(Event::Outgoing(_)) => {}
                    Err(e) => {
                        eprintln!("[sensors-plugin] MQTT error: {:?}", e);
                        eprintln!("[sensors-plugin] Fatal error - exiting to allow restart");
                        break;
                    }
                }
            }
        }
    }

    eprintln!("[sensors] exited main loop, performing final cleanup");
    Ok(())
}

async fn handle_mqtt_message(
    registry: &Arc<SensorRegistry>,
    client: &AsyncClient,
    publish: Publish,
) {
    let topic = publish.topic.as_str();
    let payload = String::from_utf8_lossy(&publish.payload);

    if topic == topics::LEGACY_REGISTRATION {
        // Sensor registration
        match serde_json::from_str::<SensorRegistration>(&payload) {
            Ok(reg) => {
                let sensor = registry.register_sensor(reg);
                println!(
                    "[sensors] sensor registered: {} ({})",
                    sensor.sensor_id, sensor.room_id
                );

                // Emit Contract v1.0 event
                let event = EventMessage::new(
                    "sensor_registered",
                    serde_json::json!({
                        "sensor_id": sensor.sensor_id,
                        "room_id": sensor.room_id,
                        "sensor_type": sensor.sensor_type
                    }),
                );
                emit_event(client, event).await;
            }
            Err(e) => {
                eprintln!("[sensors] failed to parse registration: {}", e);
            }
        }
    } else if topic.starts_with("symbion/sensors/") && topic.ends_with("/env@v1") {
        // Environment reading
        match serde_json::from_str::<EnvReadingMqtt>(&payload) {
            Ok(reading) => {
                let sensor_id = reading.sensor_id.clone();

                // Get previous status before adding reading
                let prev_status = {
                    let sensors = registry.sensors.read();
                    sensors.get(&sensor_id).and_then(|s| {
                        registry.environments.read().get(&s.room_id).map(|e| e.status)
                    })
                };

                if let Err(e) = registry.add_reading(reading.clone()) {
                    eprintln!("[sensors] failed to add reading: {}", e);
                } else {
                    // Check if status changed (for alert events)
                    let new_status = {
                        let sensors = registry.sensors.read();
                        sensors.get(&sensor_id).and_then(|s| {
                            registry.environments.read().get(&s.room_id).map(|e| e.status)
                        })
                    };

                    // Emit environment_alert if status changed to an alert state
                    if let (Some(prev), Some(new)) = (prev_status, new_status) {
                        if prev != new && (new == EnvironmentStatus::MoldRisk || new == EnvironmentStatus::TempLow) {
                            let event = EventMessage::new(
                                "environment_alert",
                                serde_json::json!({
                                    "sensor_id": sensor_id,
                                    "status": format!("{:?}", new),
                                    "temperature_c": reading.temperature_c,
                                    "humidity_pct": reading.humidity_pct
                                }),
                            );
                            emit_event(client, event).await;
                        }
                    }
                }
            }
            Err(e) => {
                eprintln!("[sensors] failed to parse env reading: {}", e);
            }
        }
    }
}
