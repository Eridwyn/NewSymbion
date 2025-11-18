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
use rumqttc::{AsyncClient, Event, EventLoop, MqttOptions, Packet, Publish, QoS};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;
use time::OffsetDateTime;

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
    WarningVentilate,  // humidity >65%
    RiskMold,          // humidity >70%
    TempLow,           // temp <16°C
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

            let status = Self::evaluate_status(&env_reading);

            let mut environments = self.environments.write();
            let room_env = environments
                .entry(sensor.room_id.clone())
                .or_insert_with(|| RoomEnvironmentState {
                    room_id: sensor.room_id.clone(),
                    current: env_reading.clone(),
                    history: Vec::new(),
                    status,
                });

            room_env.current = env_reading.clone();
            room_env.status = status;
            room_env.history.push(env_reading);

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

    fn evaluate_status(reading: &EnvironmentReading) -> EnvironmentStatus {
        if reading.humidity_pct > 70.0 {
            EnvironmentStatus::RiskMold
        } else if reading.humidity_pct > 65.0 {
            EnvironmentStatus::WarningVentilate
        } else if reading.temperature_c < 16.0 {
            EnvironmentStatus::TempLow
        } else {
            EnvironmentStatus::Normal
        }
    }

    fn list_sensors(&self) -> Vec<Sensor> {
        self.sensors.read().values().cloned().collect()
    }

    fn get_environment(&self, room_id: &str) -> Option<RoomEnvironmentState> {
        self.environments.read().get(room_id).cloned()
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("[sensors-plugin] Symbion Environment Sensors Plugin v0.1.0");
    println!("[sensors-plugin] Starting...");

    let registry = Arc::new(SensorRegistry::new());

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

    // Main event loop
    loop {
        match eventloop.poll().await {
            Ok(Event::Incoming(Packet::Publish(publish))) => {
                handle_mqtt_message(&registry, &client, publish).await;
            }
            Ok(Event::Incoming(_)) => {}
            Ok(Event::Outgoing(_)) => {}
            Err(e) => {
                eprintln!("[sensors-plugin] MQTT error: {:?}", e);
                tokio::time::sleep(Duration::from_secs(1)).await;
            }
        }
    }
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
