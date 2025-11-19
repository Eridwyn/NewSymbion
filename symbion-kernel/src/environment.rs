/**
 * SYMBION KERNEL - Environment Monitoring Module (F1 - Organ Environment)
 *
 * RÔLE : Gestion état environnemental (température, humidité) pour espaces surveillés
 *
 * ARCHITECTURE :
 * - RoomEnvironmentState : État temps réel + historique circulaire (VecDeque, max 20160 items)
 * - EnvironmentStatus : Enum états (Ok, Humid, RiskMold, Cold)
 * - EnvReading : Snapshot température/humidité/timestamp
 * - Persistence : Sauvegarde périodique (5 min) + chargement au démarrage
 *
 * UTILITÉ : Fondation alertes intelligentes humidité/température (Decision Engine intégration)
 *
 * SOURCES DE DONNÉES :
 * - MQTT topic `symbion/sensors/{room_id}/env@v1` (ESP32 + BME280 sensors)
 * - Fréquence : 1 message / 30 sec (2 readings/min, 20160 = 7 days)
 * - Offline detection : N/A status si pas de réponse depuis 5 minutes
 */

use chrono::{DateTime, Duration, Utc};
use serde::{Deserialize, Serialize};
use std::collections::VecDeque;

/// Single environment reading (temperature + humidity + timestamp)
///
/// temperature_c and humidity_pct are Option<f32> to handle offline sensors:
/// - Some(value) when sensor is online
/// - None when sensor is offline (serializes to JSON null instead of NaN)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EnvReading {
    pub temperature_c: Option<f32>,
    pub humidity_pct: Option<f32>,
    pub timestamp: DateTime<Utc>,
}

/// Environment status classification based on thresholds
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum EnvironmentStatus {
    /// Normal conditions (humidity 40-60%, temp 18-26°C)
    Ok,
    /// Humid but not critical (humidity 60-75%)
    Humid,
    /// Risk of mold growth (humidity >75%)
    RiskMold,
    /// Too cold for comfort (temp <16°C at night)
    Cold,
    /// No recent data (>30 sec since last reading)
    NA,
}

/// Room environment state with circular buffer history
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RoomEnvironmentState {
    /// Unique room identifier (e.g., "chambre", "salon")
    pub room_id: String,

    /// Current environment reading
    pub current: EnvReading,

    /// Historical readings (circular buffer, FIFO eviction)
    pub history: VecDeque<EnvReading>,

    /// Current status classification
    pub status: EnvironmentStatus,

    /// Maximum history items (default 20160 = 7 days at 30sec interval)
    #[serde(skip)]
    max_history: usize,
}

impl RoomEnvironmentState {
    /// Create new room state with default values
    pub fn new(room_id: String) -> Self {
        Self {
            room_id,
            current: EnvReading {
                temperature_c: Some(20.0),
                humidity_pct: Some(50.0),
                timestamp: Utc::now(),
            },
            history: VecDeque::with_capacity(20160),
            status: EnvironmentStatus::Ok,
            max_history: 20160,
        }
    }

    /// Fix max_history after deserialization (serde skips it)
    pub fn fix_max_history(&mut self) {
        self.max_history = 20160;
    }

    /// Update state with new reading
    ///
    /// - Updates current reading
    /// - Adds to history (FIFO eviction if full)
    /// - Recalculates status
    pub fn update(&mut self, reading: EnvReading) {
        self.current = reading.clone();

        // Add to history with circular buffer eviction
        self.history.push_back(reading);
        if self.history.len() > self.max_history {
            self.history.pop_front();
        }

        // Recalculate status based on new reading
        self.status = Self::calculate_status(&self.current);
    }

    /// Calculate status from reading thresholds
    fn calculate_status(reading: &EnvReading) -> EnvironmentStatus {
        // If values are None (offline sensor), return NA
        let Some(humidity) = reading.humidity_pct else {
            return EnvironmentStatus::NA;
        };
        let Some(temperature) = reading.temperature_c else {
            return EnvironmentStatus::NA;
        };

        // Priority: RiskMold > Humid > Cold > Ok
        if humidity > 75.0 {
            EnvironmentStatus::RiskMold
        } else if humidity > 60.0 {
            EnvironmentStatus::Humid
        } else if temperature < 16.0 {
            EnvironmentStatus::Cold
        } else {
            EnvironmentStatus::Ok
        }
    }

    /// Get historical readings for last N hours
    ///
    /// Returns readings within time window (newest to oldest)
    pub fn get_history(&self, hours: u32) -> Vec<EnvReading> {
        let cutoff = Utc::now() - Duration::hours(hours as i64);

        self.history
            .iter()
            .rev() // Newest first
            .filter(|r| r.timestamp > cutoff)
            .cloned()
            .collect()
    }

    /// Check if humidity sustained above threshold for duration
    ///
    /// Used by Decision Engine rules for alert triggering
    pub fn is_humidity_sustained(&self, threshold_pct: f32, duration_minutes: u32) -> bool {
        let cutoff = Utc::now() - Duration::minutes(duration_minutes as i64);

        // Check current reading first (return false if None or below threshold)
        let Some(current_humidity) = self.current.humidity_pct else {
            return false;
        };
        if current_humidity <= threshold_pct {
            return false;
        }

        // Verify all readings in window are above threshold
        let sustained = self.history
            .iter()
            .rev()
            .take_while(|r| r.timestamp > cutoff)
            .all(|r| r.humidity_pct.map_or(false, |h| h > threshold_pct));

        sustained
    }

    /// Get average temperature over last N hours
    pub fn avg_temperature(&self, hours: u32) -> Option<f32> {
        let readings = self.get_history(hours);

        // Filter out None values (offline sensors)
        let valid_temps: Vec<f32> = readings
            .iter()
            .filter_map(|r| r.temperature_c)
            .collect();

        if valid_temps.is_empty() {
            return None;
        }

        let sum: f32 = valid_temps.iter().sum();
        Some(sum / valid_temps.len() as f32)
    }

    /// Get average humidity over last N hours
    pub fn avg_humidity(&self, hours: u32) -> Option<f32> {
        let readings = self.get_history(hours);

        // Filter out None values (offline sensors)
        let valid_humidity: Vec<f32> = readings
            .iter()
            .filter_map(|r| r.humidity_pct)
            .collect();

        if valid_humidity.is_empty() {
            return None;
        }

        let sum: f32 = valid_humidity.iter().sum();
        Some(sum / valid_humidity.len() as f32)
    }

    /// Check if data is stale (>30 sec since last reading)
    ///
    /// Returns true if current reading timestamp is older than 30 seconds
    pub fn is_data_stale(&self) -> bool {
        let elapsed = Utc::now() - self.current.timestamp;
        elapsed.num_seconds() > 30
    }

    /// Update status to N/A if data is stale and set None values
    ///
    /// Should be called periodically (every 10 seconds) to detect offline sensors
    pub fn update_stale_status(&mut self) {
        if self.is_data_stale() {
            self.status = EnvironmentStatus::NA;
            // Set None values to indicate data is not available (JSON null)
            self.current.temperature_c = None;
            self.current.humidity_pct = None;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_new_room_state_defaults() {
        let state = RoomEnvironmentState::new("chambre".to_string());

        assert_eq!(state.room_id, "chambre");
        assert_eq!(state.current.temperature_c, Some(20.0));
        assert_eq!(state.current.humidity_pct, Some(50.0));
        assert_eq!(state.status, EnvironmentStatus::Ok);
        assert_eq!(state.history.len(), 0);
    }

    #[test]
    fn test_update_reading() {
        let mut state = RoomEnvironmentState::new("salon".to_string());

        let reading = EnvReading {
            temperature_c: Some(22.5),
            humidity_pct: Some(55.0),
            timestamp: Utc::now(),
        };

        state.update(reading.clone());

        assert_eq!(state.current.temperature_c, Some(22.5));
        assert_eq!(state.current.humidity_pct, Some(55.0));
        assert_eq!(state.history.len(), 1);
        assert_eq!(state.status, EnvironmentStatus::Ok);
    }

    #[test]
    fn test_circular_buffer_eviction() {
        let mut state = RoomEnvironmentState::new("bureau".to_string());

        // Add 150 readings (exceeds max_history of 100)
        for i in 0..150 {
            let reading = EnvReading {
                temperature_c: Some(20.0 + i as f32 * 0.1),
                humidity_pct: Some(50.0),
                timestamp: Utc::now(),
            };
            state.update(reading);
        }

        // Should have exactly 100 items (oldest evicted)
        assert_eq!(state.history.len(), 100);

        // Verify oldest reading is from iteration 50 (0-49 evicted)
        let oldest = state.history.front().unwrap();
        assert!((oldest.temperature_c.unwrap() - 25.0).abs() < 0.01); // 20.0 + 50 * 0.1 = 25.0
    }

    #[test]
    fn test_status_calculation_humid() {
        let state = RoomEnvironmentState::new("chambre".to_string());

        let reading = EnvReading {
            temperature_c: Some(22.0),
            humidity_pct: Some(65.0), // Humid threshold (60-75%)
            timestamp: Utc::now(),
        };

        let status = RoomEnvironmentState::calculate_status(&reading);
        assert_eq!(status, EnvironmentStatus::Humid);
    }

    #[test]
    fn test_status_calculation_risk_mold() {
        let state = RoomEnvironmentState::new("chambre".to_string());

        let reading = EnvReading {
            temperature_c: Some(22.0),
            humidity_pct: Some(78.0), // Risk mold threshold (>75%)
            timestamp: Utc::now(),
        };

        let status = RoomEnvironmentState::calculate_status(&reading);
        assert_eq!(status, EnvironmentStatus::RiskMold);
    }

    #[test]
    fn test_status_calculation_cold() {
        let state = RoomEnvironmentState::new("chambre".to_string());

        let reading = EnvReading {
            temperature_c: Some(14.0), // Cold threshold (<16°C)
            humidity_pct: Some(50.0),
            timestamp: Utc::now(),
        };

        let status = RoomEnvironmentState::calculate_status(&reading);
        assert_eq!(status, EnvironmentStatus::Cold);
    }

    #[test]
    fn test_get_history_filter_by_hours() {
        let mut state = RoomEnvironmentState::new("salon".to_string());

        // Add reading from 3 hours ago
        let old_reading = EnvReading {
            temperature_c: Some(18.0),
            humidity_pct: Some(45.0),
            timestamp: Utc::now() - Duration::hours(3),
        };
        state.update(old_reading);

        // Add recent reading (now)
        let recent_reading = EnvReading {
            temperature_c: Some(22.0),
            humidity_pct: Some(55.0),
            timestamp: Utc::now(),
        };
        state.update(recent_reading);

        // Get last 2 hours (should exclude 3h old reading)
        let history_2h = state.get_history(2);
        assert_eq!(history_2h.len(), 1);
        assert_eq!(history_2h[0].temperature_c, Some(22.0));

        // Get last 5 hours (should include both)
        let history_5h = state.get_history(5);
        assert_eq!(history_5h.len(), 2);
    }

    #[test]
    fn test_is_humidity_sustained_true() {
        let mut state = RoomEnvironmentState::new("chambre".to_string());

        // Add 5 readings with high humidity (1 per min for 5 min)
        for i in 0..5 {
            let reading = EnvReading {
                temperature_c: Some(22.0),
                humidity_pct: Some(70.0),
                timestamp: Utc::now() - Duration::minutes(5 - i),
            };
            state.update(reading);
        }

        // Check if sustained above 65% for 5 minutes
        assert!(state.is_humidity_sustained(65.0, 5));
    }

    #[test]
    fn test_is_humidity_sustained_false_current_below() {
        let mut state = RoomEnvironmentState::new("chambre".to_string());

        // Add 3 readings with high humidity
        for i in 0..3 {
            let reading = EnvReading {
                temperature_c: Some(22.0),
                humidity_pct: Some(70.0),
                timestamp: Utc::now() - Duration::minutes(3 - i),
            };
            state.update(reading);
        }

        // Current reading drops below threshold
        let reading = EnvReading {
            temperature_c: Some(22.0),
            humidity_pct: Some(60.0), // Below threshold
            timestamp: Utc::now(),
        };
        state.update(reading);

        // Should return false (current reading not sustained)
        assert!(!state.is_humidity_sustained(65.0, 5));
    }

    #[test]
    fn test_is_humidity_sustained_false_intermittent() {
        let mut state = RoomEnvironmentState::new("chambre".to_string());

        // Add mixed readings (not all above threshold)
        let readings = vec![
            (70.0, 5), // High
            (60.0, 4), // Low (breaks sustained)
            (70.0, 3), // High
            (70.0, 2), // High
            (70.0, 0), // High (current)
        ];

        for (humidity, minutes_ago) in readings {
            let reading = EnvReading {
                temperature_c: Some(22.0),
                humidity_pct: Some(humidity),
                timestamp: Utc::now() - Duration::minutes(minutes_ago),
            };
            state.update(reading);
        }

        // Should return false (not ALL readings sustained)
        assert!(!state.is_humidity_sustained(65.0, 5));
    }

    #[test]
    fn test_avg_temperature() {
        let mut state = RoomEnvironmentState::new("bureau".to_string());

        // Add 3 readings with different temperatures
        let temps = vec![18.0, 20.0, 22.0];
        for temp in temps {
            let reading = EnvReading {
                temperature_c: Some(temp),
                humidity_pct: Some(50.0),
                timestamp: Utc::now(),
            };
            state.update(reading);
        }

        let avg = state.avg_temperature(1).unwrap();
        assert!((avg - 20.0).abs() < 0.01); // (18+20+22)/3 = 20
    }

    #[test]
    fn test_avg_humidity() {
        let mut state = RoomEnvironmentState::new("salon".to_string());

        // Add 4 readings with different humidity
        let humidities = vec![50.0, 55.0, 60.0, 65.0];
        for humidity in humidities {
            let reading = EnvReading {
                temperature_c: Some(22.0),
                humidity_pct: Some(humidity),
                timestamp: Utc::now(),
            };
            state.update(reading);
        }

        let avg = state.avg_humidity(1).unwrap();
        assert!((avg - 57.5).abs() < 0.01); // (50+55+60+65)/4 = 57.5
    }

    #[test]
    fn test_avg_empty_history() {
        let state = RoomEnvironmentState::new("vide".to_string());

        // No readings in history yet
        assert_eq!(state.avg_temperature(1), None);
        assert_eq!(state.avg_humidity(1), None);
    }
}
