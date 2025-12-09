/**
 * SYMBION PLUGIN - Environment Sensors (F1)
 *
 * RÔLE : Plugin standalone pour gestion sensors IoT environnementaux
 *
 * COMMUNICATION MQTT :
 * - Subscribe : symbion/sensors/registration@v1 (auto-registration ESP32)
 * - Subscribe : symbion/sensors/+/env@v1 (lectures environnement)
 * - Publish   : symbion/plugin/sensors/response@v1 (réponses API)
 * - Publish   : symbion/dashboard/environment@v1 (push dashboard)
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
#[derive(Debug, Deserialize)]
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

// ========== HTTP REST API Handlers ==========

/// GET /sensors - Liste tous les sensors enregistrés
async fn list_sensors_http(
    State(registry): State<Arc<SensorRegistry>>,
) -> Json<serde_json::Value> {
    let sensors = registry.list_sensors();
    Json(serde_json::json!({
        "sensors": sensors,
        "count": sensors.len()
    }))
}

/// GET /environment/:room_id - Récupère l'état environnemental d'une pièce
async fn get_environment_http(
    State(registry): State<Arc<SensorRegistry>>,
    AxumPath(room_id): AxumPath<String>,
) -> Result<Json<RoomEnvironmentState>, (StatusCode, String)> {
    match registry.get_environment(&room_id) {
        Some(env) => Ok(Json(env)),
        None => Err((
            StatusCode::NOT_FOUND,
            format!("No environment data for room '{}'", room_id),
        )),
    }
}

/// Health check endpoint
async fn health_check() -> Json<serde_json::Value> {
    use std::sync::OnceLock;
    static START_TIME: OnceLock<std::time::Instant> = OnceLock::new();
    let start = START_TIME.get_or_init(std::time::Instant::now);
    let uptime_secs = start.elapsed().as_secs();

    Json(serde_json::json!({
        "status": "healthy",
        "plugin": "sensors",
        "version": "0.1.0",
        "uptime_seconds": uptime_secs
    }))
}

/// Construit le router HTTP pour le plugin sensors
fn build_router(registry: Arc<SensorRegistry>) -> Router {
    Router::new()
        .route("/health", get(health_check))
        .route("/sensors", get(list_sensors_http))
        .route("/environment/:room_id", get(get_environment_http))
        .with_state(registry)
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("[sensors-plugin] Symbion Environment Sensors Plugin v0.1.0");
    println!("[sensors-plugin] Starting...");

    let registry = Arc::new(SensorRegistry::new());

    // Unix socket path
    let socket_path = "/run/symbion-plugins/sensors.sock";

    // Cleanup old socket at startup (triple safety net)
    if std::path::Path::new(socket_path).exists() {
        eprintln!("[sensors] cleaning up old socket at startup");
        let _ = std::fs::remove_file(socket_path);
    }

    // Create shutdown channel for graceful termination
    let (shutdown_tx, mut shutdown_rx) = broadcast::channel::<()>(1);

    // Construire le router HTTP
    let app = build_router(registry.clone());

    // Démarrer le serveur HTTP en arrière-plan
    let socket_path_clone = socket_path.to_string();
    tokio::spawn(async move {
        println!("[sensors-plugin] Starting HTTP server on Unix socket: {}", socket_path_clone);
        if let Err(e) = PluginHttpServer::new(&socket_path_clone, app).serve().await {
            eprintln!("[sensors-plugin] HTTP server error: {:?}", e);
        }
    });

    // Attendre que le socket soit créé
    tokio::time::sleep(Duration::from_millis(100)).await;

    // Service Discovery: Auto-registration avec le kernel
    let socket_path_clone = socket_path.to_string();
    tokio::spawn(async move {
        tokio::time::sleep(Duration::from_millis(500)).await;

        match PluginRegistrationBuilder::new("sensors", &socket_path_clone)
            .route("/sensors")
            .route("/environment/:room_id")
            .route("/health")
            .version("1.0.0")
            .description("Environment sensors plugin with MQTT and HTTP API")
            .register()
            .await
        {
            Ok(_) => println!("[sensors-plugin] ✅ Registered with kernel via Service Discovery"),
            Err(e) => eprintln!("[sensors-plugin] ❌ Failed to register with kernel: {}", e),
        }
    });

    // MQTT setup
    let mut mqttoptions = MqttOptions::new("symbion-plugin-sensors", "127.0.0.1", 1883);
    mqttoptions.set_keep_alive(Duration::from_secs(30));
    mqttoptions.set_clean_session(true);

    let (client, mut eventloop) = AsyncClient::new(mqttoptions, 10);

    // Subscribe to sensor topics
    client
        .subscribe("symbion/sensors/registration@v1", QoS::AtLeastOnce)
        .await?;
    client
        .subscribe("symbion/sensors/+/env@v1", QoS::AtLeastOnce)
        .await?;

    println!("[sensors-plugin] MQTT subscriptions active");
    println!("[sensors-plugin] Waiting for sensor data...");

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
    _client: &AsyncClient,
    publish: Publish,
) {
    let topic = publish.topic.as_str();
    let payload = String::from_utf8_lossy(&publish.payload);

    if topic == "symbion/sensors/registration@v1" {
        // Sensor registration
        match serde_json::from_str::<SensorRegistration>(&payload) {
            Ok(reg) => {
                let sensor = registry.register_sensor(reg);
                println!(
                    "[sensors-plugin] sensor registered: {} ({})",
                    sensor.sensor_id, sensor.room_id
                );
            }
            Err(e) => {
                eprintln!("[sensors-plugin] failed to parse registration: {}", e);
            }
        }
    } else if topic.starts_with("symbion/sensors/") && topic.ends_with("/env@v1") {
        // Environment reading
        match serde_json::from_str::<EnvReadingMqtt>(&payload) {
            Ok(reading) => {
                if let Err(e) = registry.add_reading(reading) {
                    eprintln!("[sensors-plugin] failed to add reading: {}", e);
                }
            }
            Err(e) => {
                eprintln!("[sensors-plugin] failed to parse env reading: {}", e);
            }
        }
    }
}
