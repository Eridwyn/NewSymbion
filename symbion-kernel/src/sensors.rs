/**
 * SENSORS MANAGER - Gestion des capteurs IoT distribués (F1 - Environment Monitoring)
 *
 * RÔLE : Registration, persistence, télémétrie capteurs environnementaux multi-rooms
 *
 * ARCHITECTURE : Registry sensors avec auto-registration MQTT + persistence JSON
 * UTILITÉ : Infrastructure scalable N capteurs (BME280, DHT22, SCD30, etc.)
 *
 * PATTERN : Similaire à AgentRegistry mais simplifié pour IoT sensors
 *
 * PERSISTENCE :
 * - Sensors metadata : Auto-save on registration (sensors.json)
 * - Environment histories : Debounced save every 5 min (sensors_environments.json)
 * - Call save_environments_to_disk() periodically in background task
 *
 * USAGE EXAMPLE (periodic save):
 * ```rust
 * let registry = Arc::new(SensorRegistry::new("sensors.json"));
 * let registry_clone = registry.clone();
 * tokio::spawn(async move {
 *     let mut interval = tokio::time::interval(Duration::from_secs(300)); // 5 min
 *     loop {
 *         interval.tick().await;
 *         if let Err(e) = registry_clone.save_environments_to_disk() {
 *             eprintln!("[sensors] failed to save environments: {}", e);
 *         }
 *     }
 * });
 * ```
 */

use crate::environment::{EnvReading, RoomEnvironmentState};
use anyhow::Result;
use chrono::{DateTime, Utc};
use parking_lot::RwLock;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::Path;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use utoipa::ToSchema;

/// MQTT sensor registration message
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct SensorRegistrationMessage {
    pub sensor_id: String,
    pub sensor_type: String,
    pub room_id: String,
    pub firmware_version: Option<String>,
}

/// MQTT environment reading message
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct SensorEnvMessage {
    pub sensor_id: String,
    pub temperature_c: f32,
    pub humidity_pct: f32,
    pub battery_pct: Option<u8>,
    pub signal_rssi: Option<i16>,
}

/// Sensor information (auto-registered via MQTT)
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct Sensor {
    /// Unique sensor ID (e.g., "esp32-chambre-01", "esp32-salon-02")
    pub sensor_id: String,

    /// Sensor type (e.g., "bme280", "dht22", "scd30")
    pub sensor_type: String,

    /// Room/location ID (e.g., "chambre", "salon", "bureau")
    pub room_id: String,

    /// Firmware version (optional)
    pub firmware_version: Option<String>,

    /// Registration timestamp
    pub registered_at: DateTime<Utc>,

    /// Last seen timestamp (updated on each reading)
    pub last_seen: DateTime<Utc>,

    /// Sensor status
    pub status: SensorStatus,

    /// Battery percentage (optional, for battery-powered sensors)
    pub battery_pct: Option<u8>,

    /// WiFi RSSI signal strength (optional)
    pub signal_rssi: Option<i16>,

    /// Timestamp Unix de soft-delete (None = actif, Some = supprimé)
    /// Purge automatique après 7 jours
    #[serde(skip_serializing_if = "Option::is_none")]
    pub deleted_at: Option<i64>,
}

/// Sensor status enum
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, ToSchema)]
#[serde(rename_all = "lowercase")]
pub enum SensorStatus {
    /// Sensor online and sending data
    Online,

    /// Sensor offline (no data for >10 min)
    Offline,

    /// Sensor in maintenance mode
    Maintenance,

    /// Sensor error (bad readings, calibration needed)
    Error,
}

/// Sensor registry with environment state tracking
pub struct SensorRegistry {
    /// Map: sensor_id -> Sensor info
    sensors: RwLock<HashMap<String, Sensor>>,

    /// Map: sensor_id -> RoomEnvironmentState (readings history)
    environments: RwLock<HashMap<String, RoomEnvironmentState>>,

    /// Persistence file path (sensors metadata)
    persistence_path: String,

    /// Persistence file path (environments history)
    persistence_env_path: String,

    /// Dirty flag for debounced environment persistence
    dirty_env: Arc<AtomicBool>,

    /// SQLite database (None = JSON-only fallback mode)
    db: Option<crate::database::SharedDatabase>,
}

/// Shared reference to SensorRegistry (thread-safe)
pub type SharedSensorRegistry = Arc<SensorRegistry>;

impl SensorRegistry {
    /// Create new sensor registry
    pub fn new(persistence_path: impl AsRef<Path>) -> Self {
        let path_str = persistence_path.as_ref().to_string_lossy().to_string();
        let env_path = path_str.replace(".json", "_environments.json");

        Self {
            sensors: RwLock::new(HashMap::new()),
            environments: RwLock::new(HashMap::new()),
            persistence_path: path_str,
            persistence_env_path: env_path,
            dirty_env: Arc::new(AtomicBool::new(false)),
            db: None,
        }
    }

    /// Attach a database for SQLite persistence.
    /// If set, environments are persisted to DB (primary) + JSON (fallback).
    pub fn with_database(mut self, db: crate::database::SharedDatabase) -> Self {
        self.db = Some(db);
        self
    }

    /// Register or update a sensor (auto-registration via MQTT)
    pub fn register_sensor(&self, sensor: Sensor) -> Result<()> {
        let sensor_id = sensor.sensor_id.clone();
        let room_id = sensor.room_id.clone();

        // Insert or update sensor info
        self.sensors.write().insert(sensor_id.clone(), sensor);

        // Create environment state if first time
        if !self.environments.read().contains_key(&sensor_id) {
            let env_state = RoomEnvironmentState::new(room_id);
            self.environments.write().insert(sensor_id.clone(), env_state);
        }

        // Persist to disk
        self.save_to_disk()?;

        Ok(())
    }

    /// Update sensor reading (called on MQTT env data)
    pub fn update_reading(&self, sensor_id: &str, reading: EnvReading) -> Result<()> {
        // Update environment state
        if let Some(env_state) = self.environments.write().get_mut(sensor_id) {
            env_state.update(reading);
            // Mark environments as dirty for debounced persistence
            self.dirty_env.store(true, Ordering::Relaxed);
        } else {
            anyhow::bail!("Sensor {} not registered", sensor_id);
        }

        // Update last_seen timestamp
        if let Some(sensor) = self.sensors.write().get_mut(sensor_id) {
            sensor.last_seen = Utc::now();
            sensor.status = SensorStatus::Online;
        }

        Ok(())
    }

    /// Get sensor info
    pub fn get_sensor(&self, sensor_id: &str) -> Option<Sensor> {
        self.sensors.read().get(sensor_id).cloned()
    }

    /// Get environment state for sensor
    pub fn get_environment(&self, sensor_id: &str) -> Option<RoomEnvironmentState> {
        self.environments.read().get(sensor_id).cloned()
    }

    /// Get environment state by room_id (merges data from all sensors in that room)
    /// Returns the most recent reading across all sensors for that room
    pub fn get_environment_by_room(&self, room_id: &str) -> Option<RoomEnvironmentState> {
        // Find all sensors for this room (exclut les soft-deleted)
        let sensors_in_room: Vec<String> = self
            .sensors
            .read()
            .values()
            .filter(|s| s.room_id == room_id && s.deleted_at.is_none())
            .map(|s| s.sensor_id.clone())
            .collect();

        if sensors_in_room.is_empty() {
            return None;
        }

        // Get environment states for all sensors in room
        let environments = self.environments.read();
        let room_envs: Vec<RoomEnvironmentState> = sensors_in_room
            .iter()
            .filter_map(|sensor_id| environments.get(sensor_id).cloned())
            .collect();

        if room_envs.is_empty() {
            return None;
        }

        // Return the most recent one (by current.timestamp)
        room_envs
            .into_iter()
            .max_by_key(|env| env.current.timestamp)
    }

    /// List all registered sensors (exclut les soft-deleted)
    pub fn list_sensors(&self) -> Vec<Sensor> {
        self.sensors.read()
            .values()
            .filter(|s| s.deleted_at.is_none())
            .cloned()
            .collect()
    }

    /// List all unique room IDs (for automations schema)
    pub fn list_rooms(&self) -> Vec<String> {
        let mut rooms: Vec<String> = self
            .sensors
            .read()
            .values()
            .filter(|s| s.deleted_at.is_none())
            .map(|s| s.room_id.clone())
            .collect();
        rooms.sort();
        rooms.dedup();
        rooms
    }

    /// List all environment states (exclut les soft-deleted)
    pub fn list_environments(&self) -> HashMap<String, RoomEnvironmentState> {
        let deleted_sensors: Vec<String> = self.sensors.read()
            .iter()
            .filter(|(_, s)| s.deleted_at.is_some())
            .map(|(id, _)| id.clone())
            .collect();

        self.environments.read()
            .iter()
            .filter(|(id, _)| !deleted_sensors.contains(id))
            .map(|(k, v)| (k.clone(), v.clone()))
            .collect()
    }

    /// Soft-delete sensor (sera purgé après 7 jours)
    pub fn unregister_sensor(&self, sensor_id: &str) -> Result<()> {
        let should_save = {
            let mut sensors = self.sensors.write();
            if let Some(sensor) = sensors.get_mut(sensor_id) {
                if sensor.deleted_at.is_some() {
                    return Ok(()); // Déjà supprimé
                }
                sensor.deleted_at = Some(Utc::now().timestamp());
                println!("[sensors] soft-deleted sensor {} (will be purged in 7 days)", sensor_id);
                true
            } else {
                false
            }
        }; // Write lock libéré ici AVANT save_to_disk()

        if should_save {
            self.save_to_disk()?;
        }
        Ok(())
    }

    /// Purge les capteurs soft-deleted depuis plus de 7 jours
    pub fn purge_deleted_sensors(&self) -> usize {
        let now = Utc::now().timestamp();
        let seven_days_secs = 7 * 24 * 60 * 60; // 7 jours en secondes

        let sensors_to_purge: Vec<String> = self.sensors.read()
            .iter()
            .filter_map(|(id, sensor)| {
                if let Some(deleted_at) = sensor.deleted_at {
                    if now - deleted_at > seven_days_secs {
                        return Some(id.clone());
                    }
                }
                None
            })
            .collect();

        let count = sensors_to_purge.len();
        if count > 0 {
            let mut sensors = self.sensors.write();
            let mut environments = self.environments.write();
            for sensor_id in &sensors_to_purge {
                println!("[sensors] purging sensor {} (deleted > 7 days ago)", sensor_id);
                sensors.remove(sensor_id);
                environments.remove(sensor_id);
            }
            drop(sensors);
            drop(environments);
            let _ = self.save_to_disk();
        }
        count
    }

    /// Update sensor status
    pub fn update_status(&self, sensor_id: &str, status: SensorStatus) -> Result<()> {
        let found = {
            let mut sensors = self.sensors.write();
            if let Some(sensor) = sensors.get_mut(sensor_id) {
                sensor.status = status;
                true
            } else {
                false
            }
        }; // Write lock libéré ici

        if found {
            self.save_to_disk()?;
            Ok(())
        } else {
            anyhow::bail!("Sensor {} not found", sensor_id);
        }
    }

    /// Update sensor battery level
    pub fn update_battery(&self, sensor_id: &str, battery_pct: u8) -> Result<()> {
        if let Some(sensor) = self.sensors.write().get_mut(sensor_id) {
            sensor.battery_pct = Some(battery_pct);
            Ok(())
        } else {
            anyhow::bail!("Sensor {} not found", sensor_id);
        }
    }

    /// Check offline sensors (no data for >90 sec = 3 missed readings at 30sec interval)
    pub fn check_offline_sensors(&self) {
        let now = Utc::now();
        let offline_threshold = chrono::Duration::seconds(90);

        for sensor in self.sensors.write().values_mut() {
            if now.signed_duration_since(sensor.last_seen) > offline_threshold {
                sensor.status = SensorStatus::Offline;
            }
        }
    }

    /// Update environment statuses to N/A if data is stale (>30 sec)
    ///
    /// Should be called periodically (every 10 seconds) to detect stale data
    pub fn update_stale_environment_statuses(&self) {
        for env_state in self.environments.write().values_mut() {
            env_state.update_stale_status();
        }
    }

    /// Save sensors registry (SQLite primary + JSON fallback)
    fn save_to_disk(&self) -> Result<()> {
        // Try SQLite first
        if let Some(ref db) = self.db {
            let sensors = self.sensors.read();
            let rows: Vec<crate::database::sensor_meta_queries::SensorMetaRow> = sensors.values().map(|s| {
                crate::database::sensor_meta_queries::SensorMetaRow {
                    sensor_id: s.sensor_id.clone(),
                    sensor_type: s.sensor_type.clone(),
                    room_id: s.room_id.clone(),
                    firmware_version: s.firmware_version.clone(),
                    registered_at: s.registered_at.to_rfc3339(),
                    last_seen: s.last_seen.to_rfc3339(),
                    status: format!("{:?}", s.status).to_lowercase(),
                    battery_pct: s.battery_pct.map(|b| b as f64),
                    signal_rssi: s.signal_rssi.map(|r| r as f64),
                    deleted_at: s.deleted_at.map(|ts| {
                        chrono::DateTime::<Utc>::from_timestamp(ts, 0)
                            .map(|dt| dt.to_rfc3339())
                            .unwrap_or_default()
                    }),
                }
            }).collect();
            match crate::database::sensor_meta_queries::upsert_all_sensors(db, &rows) {
                Ok(_) => {} // SQLite save succeeded, continue to JSON fallback
                Err(e) => eprintln!("[sensors] SQLite metadata save failed: {}", e),
            }
        }

        // JSON fallback (always write for backward compatibility during transition)
        let sensors = self.sensors.read();
        let json = serde_json::to_string_pretty(&*sensors)?;
        std::fs::write(&self.persistence_path, json)?;
        Ok(())
    }

    /// Save environments (SQLite primary, JSON fallback, debounced)
    /// Call this periodically (e.g., every 5 minutes) to persist history
    pub fn save_environments_to_disk(&self) -> Result<()> {
        // Only save if dirty flag is set
        if !self.dirty_env.load(Ordering::Relaxed) {
            return Ok(());
        }

        // Try SQLite first (if configured)
        if let Some(ref db) = self.db {
            match self.save_environments_to_db(db) {
                Ok(count) => {
                    self.dirty_env.store(false, Ordering::Relaxed);
                    eprintln!("[sensors] saved {} environment readings to SQLite", count);
                    return Ok(());
                }
                Err(e) => {
                    eprintln!("[sensors] SQLite save failed, falling back to JSON: {}", e);
                    // Fall through to JSON
                }
            }
        }

        // JSON fallback (original code)
        let environments = self.environments.read();
        let json = serde_json::to_string_pretty(&*environments)?;
        let json_size_kb = json.len() / 1024;
        std::fs::write(&self.persistence_env_path, json)?;

        // Clear dirty flag
        self.dirty_env.store(false, Ordering::Relaxed);

        eprintln!(
            "[sensors] saved {} environment histories to JSON (~{} KB)",
            environments.len(),
            json_size_kb
        );

        Ok(())
    }

    /// Save current environment readings to SQLite.
    fn save_environments_to_db(&self, db: &crate::database::Database) -> Result<usize> {
        use crate::database::sensor_queries::{insert_readings, EnvRow};

        let environments = self.environments.read();
        let sensors = self.sensors.read();

        let mut rows = Vec::new();
        for (sensor_id, env_state) in environments.iter() {
            let room_id = sensors.get(sensor_id)
                .map(|s| s.room_id.clone())
                .unwrap_or_else(|| env_state.room_id.clone());

            // Save current reading (new since last save)
            let status_str = format!("{:?}", env_state.status).to_lowercase();
            rows.push(EnvRow {
                sensor_id: sensor_id.clone(),
                room_id,
                temperature_c: env_state.current.temperature_c.map(|t| t as f64),
                humidity_pct: env_state.current.humidity_pct.map(|h| h as f64),
                status: status_str,
                recorded_at: env_state.current.timestamp.to_rfc3339(),
            });
        }

        insert_readings(db, &rows)
    }

    /// Load registry from disk (SQLite primary for metadata + environments, JSON fallback)
    pub fn load_from_disk(&self) -> Result<()> {
        // Try SQLite first for sensor metadata
        let loaded_from_db = if let Some(ref db) = self.db {
            match crate::database::sensor_meta_queries::count_sensors(db) {
                Ok(count) if count > 0 => {
                    match crate::database::sensor_meta_queries::list_sensors(db) {
                        Ok(rows) => {
                            let mut sensors = HashMap::new();
                            for row in rows {
                                let registered_at = chrono::DateTime::parse_from_rfc3339(&row.registered_at)
                                    .map(|dt| dt.with_timezone(&Utc))
                                    .unwrap_or_else(|_| Utc::now());
                                let last_seen = chrono::DateTime::parse_from_rfc3339(&row.last_seen)
                                    .map(|dt| dt.with_timezone(&Utc))
                                    .unwrap_or_else(|_| Utc::now());
                                let status = match row.status.as_str() {
                                    "online" => SensorStatus::Online,
                                    "offline" => SensorStatus::Offline,
                                    "maintenance" => SensorStatus::Maintenance,
                                    "error" => SensorStatus::Error,
                                    _ => SensorStatus::Offline,
                                };
                                let deleted_at = row.deleted_at.and_then(|ts_str| {
                                    chrono::DateTime::parse_from_rfc3339(&ts_str)
                                        .ok()
                                        .map(|dt| dt.timestamp())
                                });
                                let sensor = Sensor {
                                    sensor_id: row.sensor_id.clone(),
                                    sensor_type: row.sensor_type,
                                    room_id: row.room_id,
                                    firmware_version: row.firmware_version,
                                    registered_at,
                                    last_seen,
                                    status,
                                    battery_pct: row.battery_pct.map(|b| b as u8),
                                    signal_rssi: row.signal_rssi.map(|r| r as i16),
                                    deleted_at,
                                };
                                sensors.insert(row.sensor_id, sensor);
                            }
                            let loaded = sensors.len();
                            *self.sensors.write() = sensors;
                            eprintln!("[sensors] loaded {} sensor metadata from SQLite", loaded);
                            true
                        }
                        Err(e) => {
                            eprintln!("[sensors] SQLite metadata load failed, falling back to JSON: {}", e);
                            false
                        }
                    }
                }
                _ => false, // Empty or error, fall through to JSON
            }
        } else {
            false
        };

        // JSON fallback for sensor metadata
        if !loaded_from_db {
            if !Path::new(&self.persistence_path).exists() {
                return Ok(()); // No file yet, skip
            }

            let json = std::fs::read_to_string(&self.persistence_path)?;
            let sensors: HashMap<String, Sensor> = serde_json::from_str(&json)?;

            // Seed to SQLite if DB is available
            if let Some(ref db) = self.db {
                let rows: Vec<crate::database::sensor_meta_queries::SensorMetaRow> = sensors.values().map(|s| {
                    crate::database::sensor_meta_queries::SensorMetaRow {
                        sensor_id: s.sensor_id.clone(),
                        sensor_type: s.sensor_type.clone(),
                        room_id: s.room_id.clone(),
                        firmware_version: s.firmware_version.clone(),
                        registered_at: s.registered_at.to_rfc3339(),
                        last_seen: s.last_seen.to_rfc3339(),
                        status: format!("{:?}", s.status).to_lowercase(),
                        battery_pct: s.battery_pct.map(|b| b as f64),
                        signal_rssi: s.signal_rssi.map(|r| r as f64),
                        deleted_at: s.deleted_at.map(|ts| {
                            chrono::DateTime::<Utc>::from_timestamp(ts, 0)
                                .map(|dt| dt.to_rfc3339())
                                .unwrap_or_default()
                        }),
                    }
                }).collect();
                match crate::database::sensor_meta_queries::upsert_all_sensors(db, &rows) {
                    Ok(n) => eprintln!("[sensors] seeded {} sensor metadata to SQLite from JSON", n),
                    Err(e) => eprintln!("[sensors] failed to seed sensor metadata to SQLite: {}", e),
                }
            }

            *self.sensors.write() = sensors.clone();
        }

        let sensors = self.sensors.read().clone();

        // Load environment histories if available
        self.load_environments_from_disk()?;

        // If no environment file, create empty states for registered sensors
        let environments = self.environments.read();
        if environments.is_empty() {
            drop(environments); // Release read lock
            for sensor in sensors.values() {
                let env_state = RoomEnvironmentState::new(sensor.room_id.clone());
                self.environments
                    .write()
                    .insert(sensor.sensor_id.clone(), env_state);
            }
        }

        Ok(())
    }

    /// Load environments (SQLite primary, JSON fallback)
    fn load_environments_from_disk(&self) -> Result<()> {
        // Try SQLite first (if configured)
        if let Some(ref db) = self.db {
            match self.load_environments_from_db(db) {
                Ok(count) if count > 0 => {
                    eprintln!("[sensors] loaded {} environment histories from SQLite", count);
                    return Ok(());
                }
                Ok(_) => {
                    eprintln!("[sensors] SQLite empty, trying JSON fallback");
                }
                Err(e) => {
                    eprintln!("[sensors] SQLite load failed, falling back to JSON: {}", e);
                }
            }
        }

        // JSON fallback (original code)
        if !Path::new(&self.persistence_env_path).exists() {
            return Ok(()); // No environment file yet, skip
        }

        let json = std::fs::read_to_string(&self.persistence_env_path)?;
        let mut environments: HashMap<String, RoomEnvironmentState> = serde_json::from_str(&json)?;

        // Fix max_history and recalculate status for all loaded states
        // (serde skips max_history, and status needs re-eval with latest logic)
        for env_state in environments.values_mut() {
            env_state.fix_max_history();
            env_state.recalculate_status();
        }

        eprintln!(
            "[sensors] loaded {} environment histories from JSON (~{} KB)",
            environments.len(),
            json.len() / 1024
        );

        *self.environments.write() = environments;

        Ok(())
    }

    /// Load environments from SQLite into in-memory state.
    fn load_environments_from_db(&self, db: &crate::database::Database) -> Result<usize> {
        use crate::database::sensor_queries::load_latest_per_sensor;

        // Load latest 20160 readings per sensor (7 days at 30s intervals)
        let rows = load_latest_per_sensor(db, 20160)?;
        if rows.is_empty() {
            return Ok(0);
        }

        let mut environments: HashMap<String, RoomEnvironmentState> = HashMap::new();

        for row in &rows {
            let env_state = environments
                .entry(row.sensor_id.clone())
                .or_insert_with(|| RoomEnvironmentState::new(row.room_id.clone()));

            let timestamp = chrono::DateTime::parse_from_rfc3339(&row.recorded_at)
                .map(|dt| dt.with_timezone(&chrono::Utc))
                .unwrap_or_else(|_| chrono::Utc::now());

            let reading = EnvReading {
                temperature_c: row.temperature_c.map(|t| t as f32),
                humidity_pct: row.humidity_pct.map(|h| h as f32),
                timestamp,
            };
            env_state.update(reading);
        }

        let count = environments.len();
        *self.environments.write() = environments;

        Ok(count)
    }

    /// Get count of registered sensors (exclut les soft-deleted)
    pub fn sensor_count(&self) -> usize {
        self.sensors.read()
            .values()
            .filter(|s| s.deleted_at.is_none())
            .count()
    }

    /// Get count of online sensors (exclut les soft-deleted)
    pub fn online_sensor_count(&self) -> usize {
        self.sensors
            .read()
            .values()
            .filter(|s| s.status == SensorStatus::Online && s.deleted_at.is_none())
            .count()
    }

    /// Handle sensor registration MQTT message
    pub fn handle_registration(&self, msg: SensorRegistrationMessage) -> Result<()> {
        let sensor = Sensor {
            sensor_id: msg.sensor_id.clone(),
            sensor_type: msg.sensor_type,
            room_id: msg.room_id,
            firmware_version: msg.firmware_version,
            registered_at: Utc::now(),
            last_seen: Utc::now(),
            status: SensorStatus::Online,
            battery_pct: None,
            signal_rssi: None,
            deleted_at: None,
        };

        self.register_sensor(sensor)?;
        println!("[sensors] registered sensor: {} (auto-registration)", msg.sensor_id);
        Ok(())
    }

    /// Handle environment reading MQTT message
    pub fn handle_env_reading(&self, msg: SensorEnvMessage) -> Result<()> {
        let reading = EnvReading {
            temperature_c: Some(msg.temperature_c),
            humidity_pct: Some(msg.humidity_pct),
            timestamp: Utc::now(),
        };

        self.update_reading(&msg.sensor_id, reading)?;

        // Update battery if provided
        if let Some(battery) = msg.battery_pct {
            let _ = self.update_battery(&msg.sensor_id, battery);
        }

        // Update signal if provided
        if let Some(rssi) = msg.signal_rssi {
            if let Some(sensor) = self.sensors.write().get_mut(&msg.sensor_id) {
                sensor.signal_rssi = Some(rssi);
            }
        }

        Ok(())
    }

    /// Start periodic environment save task (debounced, every 5 minutes)
    ///
    /// Spawns a background tokio task that saves environment histories to disk
    /// only when dirty flag is set (i.e., new readings received).
    ///
    /// Call this once during kernel initialization.
    pub fn start_periodic_env_save(registry: SharedSensorRegistry) {
        tokio::spawn(async move {
            let mut interval = tokio::time::interval(tokio::time::Duration::from_secs(300)); // 5 min
            loop {
                interval.tick().await;

                // Save environments if dirty flag is set
                if let Err(e) = registry.save_environments_to_disk() {
                    eprintln!("[sensors] periodic environment save failed: {}", e);
                }
            }
        });
        println!("[sensors] started periodic environment save task (5 min interval)");
    }

    /// Start periodic sensor monitoring (every 10 sec)
    ///
    /// Checks for:
    /// - Stale environment data (>30 sec) → status N/A
    /// - Offline sensors (>90 sec = 3 missed readings) → status Offline
    /// - Purge soft-deleted sensors after 7 days (hourly check)
    ///
    /// Call this once during kernel initialization.
    pub fn start_periodic_monitoring(registry: SharedSensorRegistry) {
        tokio::spawn(async move {
            let mut interval = tokio::time::interval(tokio::time::Duration::from_secs(10)); // 10 sec
            let mut purge_counter = 0u32; // Compteur pour purge horaire (360 ticks = 1h)
            loop {
                interval.tick().await;
                purge_counter += 1;

                // Update environment statuses (stale data → N/A)
                registry.update_stale_environment_statuses();

                // Check for offline sensors (>90 sec)
                registry.check_offline_sensors();

                // Purge des capteurs soft-deleted après 7 jours (toutes les heures = 360 ticks de 10s)
                if purge_counter >= 360 {
                    purge_counter = 0;
                    let purged = registry.purge_deleted_sensors();
                    if purged > 0 {
                        println!("[sensors] purged {} sensors older than 7 days", purged);
                    }
                }
            }
        });
        println!("[sensors] started periodic monitoring task (10 sec interval)");
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn test_register_sensor() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("sensors_test.json");
        let registry = SensorRegistry::new(&path);

        let sensor = Sensor {
            sensor_id: "esp32-chambre-01".to_string(),
            sensor_type: "bme280".to_string(),
            room_id: "chambre".to_string(),
            firmware_version: Some("0.1.0".to_string()),
            registered_at: Utc::now(),
            last_seen: Utc::now(),
            status: SensorStatus::Online,
            battery_pct: None,
            signal_rssi: Some(-45),
            deleted_at: None,
        };

        registry.register_sensor(sensor.clone()).unwrap();

        let retrieved = registry.get_sensor("esp32-chambre-01").unwrap();
        assert_eq!(retrieved.sensor_id, "esp32-chambre-01");
        assert_eq!(retrieved.room_id, "chambre");
        assert_eq!(retrieved.sensor_type, "bme280");
    }

    #[test]
    fn test_update_reading() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("sensors_test.json");
        let registry = SensorRegistry::new(&path);

        // Register sensor first
        let sensor = Sensor {
            sensor_id: "esp32-salon-01".to_string(),
            sensor_type: "dht22".to_string(),
            room_id: "salon".to_string(),
            firmware_version: None,
            registered_at: Utc::now(),
            last_seen: Utc::now(),
            status: SensorStatus::Online,
            battery_pct: Some(85),
            signal_rssi: None,
            deleted_at: None,
        };
        registry.register_sensor(sensor).unwrap();

        // Update reading
        let reading = EnvReading {
            temperature_c: Some(22.5),
            humidity_pct: Some(55.0),
            timestamp: Utc::now(),
        };
        registry
            .update_reading("esp32-salon-01", reading.clone())
            .unwrap();

        // Verify environment state updated
        let env = registry.get_environment("esp32-salon-01").unwrap();
        assert_eq!(env.current.temperature_c, Some(22.5));
        assert_eq!(env.current.humidity_pct, Some(55.0));
        assert_eq!(env.history.len(), 1);
    }

    #[test]
    fn test_list_sensors() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("sensors_test.json");
        let registry = SensorRegistry::new(&path);

        // Register 3 sensors
        for i in 1..=3 {
            let sensor = Sensor {
                sensor_id: format!("esp32-test-{:02}", i),
                sensor_type: "bme280".to_string(),
                room_id: "test".to_string(),
                firmware_version: None,
                registered_at: Utc::now(),
                last_seen: Utc::now(),
                status: SensorStatus::Online,
                battery_pct: None,
                signal_rssi: None,
                deleted_at: None,
            };
            registry.register_sensor(sensor).unwrap();
        }

        let sensors = registry.list_sensors();
        assert_eq!(sensors.len(), 3);
        assert_eq!(registry.sensor_count(), 3);
    }

    #[test]
    fn test_unregister_sensor() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("sensors_test.json");
        let registry = SensorRegistry::new(&path);

        let sensor = Sensor {
            sensor_id: "esp32-remove-me".to_string(),
            sensor_type: "bme280".to_string(),
            room_id: "test".to_string(),
            firmware_version: None,
            registered_at: Utc::now(),
            last_seen: Utc::now(),
            status: SensorStatus::Online,
            battery_pct: None,
            signal_rssi: None,
            deleted_at: None,
        };
        registry.register_sensor(sensor).unwrap();
        assert_eq!(registry.sensor_count(), 1);

        // Soft delete - le capteur existe toujours mais est filtré
        registry.unregister_sensor("esp32-remove-me").unwrap();
        assert_eq!(registry.sensor_count(), 0); // Filtré de la liste
        // Le capteur existe encore (soft delete) mais avec deleted_at set
        let deleted_sensor = registry.get_sensor("esp32-remove-me").unwrap();
        assert!(deleted_sensor.deleted_at.is_some());
    }

    #[test]
    fn test_persistence() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("sensors_persist.json");

        // Create registry and register sensor
        {
            let registry = SensorRegistry::new(&path);
            let sensor = Sensor {
                sensor_id: "esp32-persist-01".to_string(),
                sensor_type: "bme280".to_string(),
                room_id: "persist".to_string(),
                firmware_version: Some("0.2.0".to_string()),
                registered_at: Utc::now(),
                last_seen: Utc::now(),
                status: SensorStatus::Online,
                battery_pct: Some(90),
                signal_rssi: Some(-50),
                deleted_at: None,
            };
            registry.register_sensor(sensor).unwrap();
        }

        // Load in new registry instance
        {
            let registry = SensorRegistry::new(&path);
            registry.load_from_disk().unwrap();

            let sensor = registry.get_sensor("esp32-persist-01").unwrap();
            assert_eq!(sensor.room_id, "persist");
            assert_eq!(sensor.firmware_version, Some("0.2.0".to_string()));
            assert_eq!(sensor.battery_pct, Some(90));
        }
    }

    #[test]
    fn test_check_offline_sensors() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("sensors_test.json");
        let registry = SensorRegistry::new(&path);

        // Register sensor with old last_seen (11 minutes ago)
        let old_timestamp = Utc::now() - chrono::Duration::minutes(11);
        let sensor = Sensor {
            sensor_id: "esp32-offline-test".to_string(),
            sensor_type: "bme280".to_string(),
            room_id: "test".to_string(),
            firmware_version: None,
            registered_at: old_timestamp,
            last_seen: old_timestamp,
            status: SensorStatus::Online,
            battery_pct: None,
            signal_rssi: None,
            deleted_at: None,
        };
        registry.register_sensor(sensor).unwrap();

        // Check offline
        registry.check_offline_sensors();

        let sensor = registry.get_sensor("esp32-offline-test").unwrap();
        assert_eq!(sensor.status, SensorStatus::Offline);
    }

    #[test]
    fn test_online_sensor_count() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("sensors_test.json");
        let registry = SensorRegistry::new(&path);

        // Register 2 online sensors
        for i in 1..=2 {
            let sensor = Sensor {
                sensor_id: format!("esp32-online-{}", i),
                sensor_type: "bme280".to_string(),
                room_id: "test".to_string(),
                firmware_version: None,
                registered_at: Utc::now(),
                last_seen: Utc::now(),
                status: SensorStatus::Online,
                battery_pct: None,
                signal_rssi: None,
                deleted_at: None,
            };
            registry.register_sensor(sensor).unwrap();
        }

        // Register 1 offline sensor
        let sensor_offline = Sensor {
            sensor_id: "esp32-offline-01".to_string(),
            sensor_type: "bme280".to_string(),
            room_id: "test".to_string(),
            firmware_version: None,
            registered_at: Utc::now(),
            last_seen: Utc::now(),
            status: SensorStatus::Offline,
            battery_pct: None,
            signal_rssi: None,
            deleted_at: None,
        };
        registry.register_sensor(sensor_offline).unwrap();

        assert_eq!(registry.sensor_count(), 3);
        assert_eq!(registry.online_sensor_count(), 2);
    }

    #[test]
    fn test_environment_persistence() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("sensors_env_persist.json");

        // Create registry, register sensor, and add readings
        {
            let registry = SensorRegistry::new(&path);
            let sensor = Sensor {
                sensor_id: "esp32-env-test".to_string(),
                sensor_type: "bme280".to_string(),
                room_id: "env_room".to_string(),
                firmware_version: Some("1.0.0".to_string()),
                registered_at: Utc::now(),
                last_seen: Utc::now(),
                status: SensorStatus::Online,
                battery_pct: Some(95),
                signal_rssi: Some(-42),
                deleted_at: None,
            };
            registry.register_sensor(sensor).unwrap();

            // Add multiple readings to build history
            for i in 0..5 {
                let reading = EnvReading {
                    temperature_c: Some(20.0 + i as f32),
                    humidity_pct: Some(50.0 + i as f32),
                    timestamp: Utc::now(),
                };
                registry.update_reading("esp32-env-test", reading).unwrap();
            }

            // Manually save environments
            registry.save_environments_to_disk().unwrap();
        }

        // Load in new registry instance
        {
            let registry = SensorRegistry::new(&path);
            registry.load_from_disk().unwrap();

            // Verify environment was restored
            let env = registry.get_environment("esp32-env-test").unwrap();
            assert_eq!(env.room_id, "env_room");
            assert_eq!(env.history.len(), 5);

            // Verify last reading
            assert_eq!(env.current.temperature_c, Some(24.0)); // 20.0 + 4
            assert_eq!(env.current.humidity_pct, Some(54.0)); // 50.0 + 4
        }
    }

    #[test]
    fn test_environment_debounced_save() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("sensors_debounce.json");
        let registry = SensorRegistry::new(&path);

        let sensor = Sensor {
            sensor_id: "esp32-debounce-test".to_string(),
            sensor_type: "bme280".to_string(),
            room_id: "debounce_room".to_string(),
            firmware_version: None,
            registered_at: Utc::now(),
            last_seen: Utc::now(),
            status: SensorStatus::Online,
            battery_pct: None,
            signal_rssi: None,
            deleted_at: None,
        };
        registry.register_sensor(sensor).unwrap();

        // First save should succeed (dirty flag set)
        let reading = EnvReading {
            temperature_c: Some(21.0),
            humidity_pct: Some(52.0),
            timestamp: Utc::now(),
        };
        registry.update_reading("esp32-debounce-test", reading).unwrap();
        assert!(registry.save_environments_to_disk().is_ok());

        // Second save without update should skip (dirty flag cleared)
        assert!(registry.save_environments_to_disk().is_ok());
    }
}
