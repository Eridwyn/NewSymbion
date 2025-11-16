/**
 * SENSORS MANAGER - Gestion des capteurs IoT distribués (F1 - Environment Monitoring)
 *
 * RÔLE : Registration, persistence, télémétrie capteurs environnementaux multi-rooms
 *
 * ARCHITECTURE : Registry sensors avec auto-registration MQTT + persistence JSON
 * UTILITÉ : Infrastructure scalable N capteurs (BME280, DHT22, SCD30, etc.)
 *
 * PATTERN : Similaire à AgentRegistry mais simplifié pour IoT sensors
 */

use crate::environment::{EnvReading, RoomEnvironmentState};
use anyhow::Result;
use chrono::{DateTime, Utc};
use parking_lot::RwLock;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::Path;
use std::sync::Arc;

/// MQTT sensor registration message
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SensorRegistrationMessage {
    pub sensor_id: String,
    pub sensor_type: String,
    pub room_id: String,
    pub firmware_version: Option<String>,
}

/// MQTT environment reading message
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SensorEnvMessage {
    pub sensor_id: String,
    pub temperature_c: f32,
    pub humidity_pct: f32,
    pub battery_pct: Option<u8>,
    pub signal_rssi: Option<i16>,
}

/// Sensor information (auto-registered via MQTT)
#[derive(Debug, Clone, Serialize, Deserialize)]
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
}

/// Sensor status enum
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
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

    /// Persistence file path
    persistence_path: String,
}

/// Shared reference to SensorRegistry (thread-safe)
pub type SharedSensorRegistry = Arc<SensorRegistry>;

impl SensorRegistry {
    /// Create new sensor registry
    pub fn new(persistence_path: impl AsRef<Path>) -> Self {
        Self {
            sensors: RwLock::new(HashMap::new()),
            environments: RwLock::new(HashMap::new()),
            persistence_path: persistence_path.as_ref().to_string_lossy().to_string(),
        }
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

    /// List all registered sensors
    pub fn list_sensors(&self) -> Vec<Sensor> {
        self.sensors.read().values().cloned().collect()
    }

    /// List all environment states
    pub fn list_environments(&self) -> HashMap<String, RoomEnvironmentState> {
        self.environments.read().clone()
    }

    /// Unregister sensor (manual removal)
    pub fn unregister_sensor(&self, sensor_id: &str) -> Result<()> {
        self.sensors.write().remove(sensor_id);
        self.environments.write().remove(sensor_id);
        self.save_to_disk()?;
        Ok(())
    }

    /// Update sensor status
    pub fn update_status(&self, sensor_id: &str, status: SensorStatus) -> Result<()> {
        if let Some(sensor) = self.sensors.write().get_mut(sensor_id) {
            sensor.status = status;
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

    /// Check offline sensors (no data for >10 min)
    pub fn check_offline_sensors(&self) {
        let now = Utc::now();
        let offline_threshold = chrono::Duration::minutes(10);

        for sensor in self.sensors.write().values_mut() {
            if now.signed_duration_since(sensor.last_seen) > offline_threshold {
                sensor.status = SensorStatus::Offline;
            }
        }
    }

    /// Save registry to disk (JSON persistence)
    fn save_to_disk(&self) -> Result<()> {
        let sensors = self.sensors.read();
        let json = serde_json::to_string_pretty(&*sensors)?;
        std::fs::write(&self.persistence_path, json)?;
        Ok(())
    }

    /// Load registry from disk
    pub fn load_from_disk(&self) -> Result<()> {
        if !Path::new(&self.persistence_path).exists() {
            return Ok(()); // No file yet, skip
        }

        let json = std::fs::read_to_string(&self.persistence_path)?;
        let sensors: HashMap<String, Sensor> = serde_json::from_str(&json)?;

        // Restore sensors
        *self.sensors.write() = sensors.clone();

        // Recreate environment states (history not persisted for now)
        for sensor in sensors.values() {
            let env_state = RoomEnvironmentState::new(sensor.room_id.clone());
            self.environments
                .write()
                .insert(sensor.sensor_id.clone(), env_state);
        }

        Ok(())
    }

    /// Get count of registered sensors
    pub fn sensor_count(&self) -> usize {
        self.sensors.read().len()
    }

    /// Get count of online sensors
    pub fn online_sensor_count(&self) -> usize {
        self.sensors
            .read()
            .values()
            .filter(|s| s.status == SensorStatus::Online)
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
        };

        self.register_sensor(sensor)?;
        println!("[sensors] registered sensor: {} (auto-registration)", msg.sensor_id);
        Ok(())
    }

    /// Handle environment reading MQTT message
    pub fn handle_env_reading(&self, msg: SensorEnvMessage) -> Result<()> {
        let reading = EnvReading {
            temperature_c: msg.temperature_c,
            humidity_pct: msg.humidity_pct,
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
        };
        registry.register_sensor(sensor).unwrap();

        // Update reading
        let reading = EnvReading {
            temperature_c: 22.5,
            humidity_pct: 55.0,
            timestamp: Utc::now(),
        };
        registry
            .update_reading("esp32-salon-01", reading.clone())
            .unwrap();

        // Verify environment state updated
        let env = registry.get_environment("esp32-salon-01").unwrap();
        assert_eq!(env.current.temperature_c, 22.5);
        assert_eq!(env.current.humidity_pct, 55.0);
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
        };
        registry.register_sensor(sensor).unwrap();
        assert_eq!(registry.sensor_count(), 1);

        registry.unregister_sensor("esp32-remove-me").unwrap();
        assert_eq!(registry.sensor_count(), 0);
        assert!(registry.get_sensor("esp32-remove-me").is_none());
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
        };
        registry.register_sensor(sensor_offline).unwrap();

        assert_eq!(registry.sensor_count(), 3);
        assert_eq!(registry.online_sensor_count(), 2);
    }
}
