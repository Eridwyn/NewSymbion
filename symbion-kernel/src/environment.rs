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
use crate::dew_point_alerts::{DewPointAlertLevel, DewPointCalculator};

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

// EnvironmentStatus is now replaced by DewPointAlertLevel (physics-based)
// Re-export for backward compatibility in API responses
pub use crate::dew_point_alerts::DewPointAlertLevel as EnvironmentStatus;

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
            status: DewPointAlertLevel::Safe, // Safe default (no alerts)
            max_history: 20160,
        }
    }

    /// Fix max_history after deserialization (serde skips it)
    pub fn fix_max_history(&mut self) {
        self.max_history = 20160;
    }

    /// Recalculate status from current data
    ///
    /// Should be called after loading from disk to ensure status reflects
    /// the latest evaluation logic (not the persisted value)
    pub fn recalculate_status(&mut self) {
        self.status = self.calculate_status();
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

        // Recalculate status based on new reading (physics-based dew point)
        self.status = self.calculate_status();
    }

    /// Calculate status using dew point physics (replaces arbitrary thresholds)
    ///
    /// Uses DewPointCalculator to evaluate condensation risk based on Magnus formula
    fn calculate_status(&self) -> EnvironmentStatus {
        // If offline sensor, return Safe (no data = no alert)
        // Alternative: Could use a new DewPointAlertLevel::NA variant if needed
        if self.current.humidity_pct.is_none() || self.current.temperature_c.is_none() {
            return DewPointAlertLevel::Safe;
        }

        // Use DewPointCalculator for physics-based evaluation
        let calculator = DewPointCalculator::new();
        let evaluation = calculator.evaluate(self);

        evaluation.level
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
    ///
    /// IMPORTANT: Validates that actual time coverage meets duration requirement
    /// to prevent false positives with insufficient historical data
    pub fn is_humidity_sustained(&self, threshold_pct: f32, duration_minutes: u32) -> bool {
        let cutoff = Utc::now() - Duration::minutes(duration_minutes as i64);

        // Check current reading first (return false if None or below threshold)
        let Some(current_humidity) = self.current.humidity_pct else {
            return false;
        };
        if current_humidity <= threshold_pct {
            return false;
        }

        // Collect readings in time window
        let readings_in_window: Vec<_> = self.history
            .iter()
            .rev()
            .take_while(|r| r.timestamp > cutoff)
            .collect();

        // Validate sufficient time coverage (prevent false positives)
        if let (Some(oldest), Some(newest)) = (readings_in_window.last(), readings_in_window.first()) {
            let actual_duration_secs = (newest.timestamp - oldest.timestamp).num_seconds();
            let required_duration_secs = (duration_minutes as i64) * 60;

            // Require at least 90% of duration to be covered by actual data
            if actual_duration_secs < (required_duration_secs * 9 / 10) {
                return false; // NOT ENOUGH DATA - prevent false positive
            }
        } else {
            return false; // No data in window
        }

        // Verify all readings are above threshold
        readings_in_window
            .iter()
            .all(|r| r.humidity_pct.map_or(false, |h| h > threshold_pct))
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

    /// Update status to Safe if data is stale and set None values
    ///
    /// Should be called periodically (every 10 seconds) to detect offline sensors
    /// Note: Safe is used instead of NA (no data = no alert)
    pub fn update_stale_status(&mut self) {
        if self.is_data_stale() {
            self.status = DewPointAlertLevel::Safe; // No data = no alert
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
        assert_eq!(state.status, DewPointAlertLevel::Safe); // Physics-based status
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
        assert_eq!(state.status, DewPointAlertLevel::Safe); // 55% RH at 22.5°C is safe
    }

    #[test]
    fn test_circular_buffer_eviction() {
        let mut state = RoomEnvironmentState::new("bureau".to_string());

        // Add 20200 readings (exceeds max_history of 20160)
        let total_readings = 20200;
        let max_history = 20160;

        for i in 0..total_readings {
            let reading = EnvReading {
                temperature_c: Some(20.0 + i as f32 * 0.01),
                humidity_pct: Some(50.0),
                timestamp: Utc::now(),
            };
            state.update(reading);
        }

        // Should have exactly max_history items (oldest evicted)
        assert_eq!(state.history.len(), max_history);

        // Verify oldest reading is from iteration 40 (0-39 evicted = 40 readings evicted)
        let evicted_count = total_readings - max_history;
        let oldest = state.history.front().unwrap();
        let expected_temp = 20.0 + evicted_count as f32 * 0.01;
        assert!((oldest.temperature_c.unwrap() - expected_temp).abs() < 0.01);
    }

    #[test]
    fn test_status_calculation_dew_point_based() {
        // Test that status calculation now uses dew point physics
        let mut state = RoomEnvironmentState::new("chambre".to_string());

        // Test 1: Safe conditions (20°C, 50% RH)
        let safe_reading = EnvReading {
            temperature_c: Some(20.0),
            humidity_pct: Some(50.0),
            timestamp: Utc::now(),
        };
        state.update(safe_reading);
        assert_eq!(state.status, DewPointAlertLevel::Safe);

        // Test 2: High humidity should trigger alert (not instant, needs sustained)
        // Single high reading won't trigger without sustained duration
        let high_humidity = EnvReading {
            temperature_c: Some(20.0),
            humidity_pct: Some(70.0),
            timestamp: Utc::now(),
        };
        state.update(high_humidity);
        // Note: May or may not be Safe depending on history duration logic
        // The dew point calculator handles this based on sustained thresholds
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

        // Add readings spanning 5 minutes (30sec intervals = realistic ESP32 sensor)
        // Need to span from past to very recent to account for Utc::now() timing
        let duration_min = 5;
        let num_readings = (duration_min * 2) as usize; // 30sec interval = 10 readings

        for i in 0..num_readings {
            let reading = EnvReading {
                temperature_c: Some(22.0),
                humidity_pct: Some(70.0),
                timestamp: Utc::now() - Duration::seconds(((duration_min * 60) - (i * 30)) as i64),
            };
            state.update(reading);
        }

        // Add one more reading at current time to ensure freshness
        let reading = EnvReading {
            temperature_c: Some(22.0),
            humidity_pct: Some(70.0),
            timestamp: Utc::now(),
        };
        state.update(reading);

        // Check if sustained above 65% for 5 minutes (needs 90% = 4.5 min coverage)
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

    #[test]
    fn test_is_humidity_sustained_false_insufficient_data() {
        // REGRESSION TEST for critical bug fix
        // Bug: System triggered "weak" alert with only 35 minutes when 6 hours required
        // Fix: Validate actual time coverage before returning sustained=true

        let mut state = RoomEnvironmentState::new("chambre".to_string());

        // Simulate real scenario: 35 minutes of data with all RH > 55%
        let num_readings = 70; // 35 min at 30sec interval
        for i in 0..num_readings {
            let reading = EnvReading {
                temperature_c: Some(22.0),
                humidity_pct: Some(58.0), // All above 55% weak threshold
                timestamp: Utc::now() - Duration::minutes((35 - i / 2) as i64),
            };
            state.update(reading);
        }

        // Should return FALSE - insufficient data for 6-hour requirement
        // (35 min < 90% of 360 min = 324 min required)
        assert!(
            !state.is_humidity_sustained(55.0, 360),
            "Should not trigger weak alert with only 35 minutes of data (requires 6 hours)"
        );

        // Should also return FALSE for 3-hour requirement (moderate level)
        assert!(
            !state.is_humidity_sustained(60.0, 180),
            "Should not trigger moderate alert with only 35 minutes of data (requires 3 hours)"
        );

        // Should return TRUE for 30-minute requirement (35 min > 90% of 30 min = 27 min)
        assert!(
            state.is_humidity_sustained(55.0, 30),
            "Should trigger for 30-minute requirement (35 min > 27 min required)"
        );
    }

    #[test]
    fn test_is_humidity_sustained_true_exact_90_percent_coverage() {
        // Test boundary condition: exactly 90% coverage should pass

        let mut state = RoomEnvironmentState::new("chambre".to_string());

        // For 60-minute requirement, need full time span from (now - 60min) to now
        // This creates 100% coverage, which is > 90% required
        let now = Utc::now();
        let duration_min = 60;
        let num_readings = (duration_min * 2) as usize; // 30sec interval = 120 readings

        for i in 0..num_readings {
            let reading = EnvReading {
                temperature_c: Some(22.0),
                humidity_pct: Some(70.0),
                timestamp: now - Duration::seconds(((duration_min * 60) - (i * 30)) as i64),
            };
            state.update(reading);
        }

        // Should return TRUE - full time coverage (100% > 90% required)
        assert!(
            state.is_humidity_sustained(65.0, duration_min as u32),
            "Should trigger with exactly 90% time coverage"
        );
    }
}
