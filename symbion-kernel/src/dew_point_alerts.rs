/**
 * SYMBION KERNEL - Dew Point-Based Humidity Alert System (F1 Enhanced)
 *
 * RÔLE : Évaluation physique rigoureuse du risque de condensation
 *        Remplace les seuils arbitraires par un modèle basé sur le point de rosée
 *
 * PHYSIQUE :
 * - Point de rosée (T_dew) calculé via formule de Magnus
 * - Delta de température surface (deltaT = T_surface - T_dew)
 * - Risque condensation déterminé par proximité au point de rosée
 *
 * NIVEAUX D'ALERTE (5 niveaux progressifs avec conditions temporelles) :
 * 1. Weak      : RH > 55% pendant 6h   → "Humidité en tendance haute"
 * 2. Moderate  : RH > 60% pendant 3h   → "Humidité excessive prolongée"
 * 3. Strong    : RH > 65% pendant 1h OU deltaT < 3°C pendant 1h → "Risque de condensation"
 * 4. Critical  : RH > 70% pendant 20min OU deltaT < 2°C pendant 20min → "Condensation très probable"
 * 5. Danger    : RH > 75% pendant 5min OU deltaT ≤ 0°C pendant 5min → "Condensation certaine"
 *
 * MIGRATION DEPUIS ANCIEN SYSTÈME :
 * - Remplace EnvironmentStatus::Humid/RiskMold par DewPointAlertLevel
 * - Conserve RoomEnvironmentState.history pour calculs temporels
 * - Remplace decision/environment.rs rules par evaluate_dew_point_alert()
 */

use crate::environment::{EnvReading, RoomEnvironmentState};
use chrono::{Duration, Utc};
use serde::{Deserialize, Serialize};

/// Configuration centralisée des paramètres d'alertes humidité
///
/// Tous les seuils et durées sont modifiables ici pour fine-tuning
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DewPointAlertConfig {
    /// Constantes Magnus pour point de rosée (formule standard)
    pub magnus_a: f32, // Default: 17.62
    pub magnus_b: f32, // Default: 243.12

    /// Estimation T_surface si capteur indisponible (T_surface ≈ T_air - offset)
    /// Valeur typique: 2-4°C pour murs extérieurs non isolés
    pub surface_temp_offset: f32, // Default: 3.0

    /// Seuil 1: Alerte faible
    pub weak_rh_threshold: f32,      // Default: 55.0%
    pub weak_duration_hours: f32,    // Default: 6.0h

    /// Seuil 2: Alerte modérée
    pub moderate_rh_threshold: f32,  // Default: 60.0%
    pub moderate_duration_hours: f32, // Default: 3.0h

    /// Seuil 3: Alerte forte (pré-condensation)
    pub strong_rh_threshold: f32,    // Default: 65.0%
    pub strong_delta_t: f32,         // Default: 3.0°C
    pub strong_duration_hours: f32,  // Default: 1.0h

    /// Seuil 4: Alerte critique
    pub critical_rh_threshold: f32,  // Default: 70.0%
    pub critical_delta_t: f32,       // Default: 2.0°C
    pub critical_duration_minutes: u32, // Default: 20 min

    /// Seuil 5: Danger immédiat
    pub danger_rh_threshold: f32,    // Default: 75.0%
    pub danger_delta_t: f32,         // Default: 0.0°C (condensation)
    pub danger_duration_minutes: u32, // Default: 5 min
}

impl Default for DewPointAlertConfig {
    fn default() -> Self {
        Self {
            magnus_a: 17.62,
            magnus_b: 243.12,
            surface_temp_offset: 3.0,

            weak_rh_threshold: 55.0,
            weak_duration_hours: 6.0,

            moderate_rh_threshold: 60.0,
            moderate_duration_hours: 3.0,

            strong_rh_threshold: 65.0,
            strong_delta_t: 3.0,
            strong_duration_hours: 1.0,

            critical_rh_threshold: 70.0,
            critical_delta_t: 2.0,
            critical_duration_minutes: 20,

            danger_rh_threshold: 75.0,
            danger_delta_t: 0.0,
            danger_duration_minutes: 5,
        }
    }
}

/// Niveau d'alerte basé sur point de rosée (5 niveaux progressifs)
///
/// Ordre de priorité : Danger > Critical > Strong > Moderate > Weak > Safe
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DewPointAlertLevel {
    /// Conditions normales - aucun risque
    Safe,

    /// Humidité en tendance haute (RH > 55% pendant 6h)
    /// Action: Surveiller, aérer préventivement
    Weak,

    /// Humidité excessive prolongée (RH > 60% pendant 3h)
    /// Action: Ventilation recommandée
    Moderate,

    /// Risque de condensation (RH > 65% 1h OU deltaT < 3°C 1h)
    /// Action: Ventilation urgente, surveillance surfaces
    Strong,

    /// Condensation très probable (RH > 70% 20min OU deltaT < 2°C 20min)
    /// Action: Ventilation immédiate, déshumidificateur
    Critical,

    /// Condensation certaine / surfaces humides (RH > 75% 5min OU deltaT ≤ 0°C 5min)
    /// Action: Intervention urgente, risque moisissure
    Danger,
}

impl DewPointAlertLevel {
    /// Message descriptif pour chaque niveau
    pub fn message(&self) -> &'static str {
        match self {
            Self::Safe => "Conditions normales",
            Self::Weak => "Humidité en tendance haute",
            Self::Moderate => "Humidité excessive prolongée",
            Self::Strong => "Risque de condensation",
            Self::Critical => "Condensation très probable",
            Self::Danger => "Condensation certaine / surfaces humides",
        }
    }

    /// Suggestion d'action pour chaque niveau
    pub fn suggestion(&self) -> &'static str {
        match self {
            Self::Safe => "Aucune action requise",
            Self::Weak => "Surveiller, aérer préventivement",
            Self::Moderate => "Ventilation recommandée",
            Self::Strong => "Ventilation urgente, surveiller surfaces froides",
            Self::Critical => "Ventilation immédiate, envisager déshumidificateur",
            Self::Danger => "Intervention urgente - Risque moisissure imminent",
        }
    }
}

/// Résultat d'évaluation avec diagnostics détaillés
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DewPointEvaluation {
    /// Niveau d'alerte déterminé
    pub level: DewPointAlertLevel,

    /// Point de rosée calculé (°C)
    pub dew_point_c: Option<f32>,

    /// Delta température surface (T_surface - T_dew) en °C
    /// None si T_surface ou T_dew indisponible
    pub delta_t: Option<f32>,

    /// Température air actuelle (°C)
    pub air_temp_c: Option<f32>,

    /// Humidité relative actuelle (%)
    pub humidity_pct: Option<f32>,

    /// Température surface estimée/mesurée (°C)
    pub surface_temp_c: Option<f32>,

    /// Message descriptif du niveau
    pub message: String,

    /// Suggestion d'action
    pub suggestion: String,

    /// Diagnostics pour debugging
    pub diagnostics: serde_json::Value,
}

/// Calculateur de point de rosée et évaluateur d'alertes
pub struct DewPointCalculator {
    config: DewPointAlertConfig,
}

impl DewPointCalculator {
    /// Créer calculateur avec config par défaut
    pub fn new() -> Self {
        Self {
            config: DewPointAlertConfig::default(),
        }
    }

    /// Créer calculateur avec config personnalisée
    pub fn with_config(config: DewPointAlertConfig) -> Self {
        Self { config }
    }

    /// Accès à la configuration (pour diagnostic et tests)
    pub fn config(&self) -> &DewPointAlertConfig {
        &self.config
    }

    /// Calculer point de rosée via formule de Magnus
    ///
    /// Formule :
    ///   alpha = (a * T / (b + T)) + ln(RH/100)
    ///   T_dew = (b * alpha) / (a - alpha)
    ///
    /// Avec constantes Magnus : a = 17.62, b = 243.12
    ///
    /// Précision : ±0.4°C pour T entre -40°C et 50°C, RH entre 1% et 100%
    ///
    /// Returns None si RH ou T invalides
    pub fn calculate_dew_point(&self, air_temp_c: f32, humidity_pct: f32) -> Option<f32> {
        // Validation inputs
        if humidity_pct <= 0.0 || humidity_pct > 100.0 {
            return None; // RH invalide
        }
        if air_temp_c < -40.0 || air_temp_c > 50.0 {
            return None; // T hors plage fiable
        }

        let a = self.config.magnus_a;
        let b = self.config.magnus_b;

        // Formule Magnus
        let alpha = (a * air_temp_c / (b + air_temp_c)) + (humidity_pct / 100.0).ln();
        let dew_point = (b * alpha) / (a - alpha);

        Some(dew_point)
    }

    /// Estimer température de surface si non mesurée
    ///
    /// Estimation simplifiée : T_surface ≈ T_air - offset
    /// Offset typique 2-4°C pour murs extérieurs mal isolés
    pub fn estimate_surface_temp(&self, air_temp_c: f32) -> f32 {
        air_temp_c - self.config.surface_temp_offset
    }

    /// Évaluer niveau d'alerte complet basé sur historique
    ///
    /// Logique d'évaluation (du plus critique au moins critique) :
    /// 1. Vérifier Danger (RH > 75% 5min OU deltaT ≤ 0°C 5min)
    /// 2. Vérifier Critical (RH > 70% 20min OU deltaT < 2°C 20min)
    /// 3. Vérifier Strong (RH > 65% 1h OU deltaT < 3°C 1h)
    /// 4. Vérifier Moderate (RH > 60% 3h)
    /// 5. Vérifier Weak (RH > 55% 6h)
    /// 6. Sinon Safe
    pub fn evaluate(&self, state: &RoomEnvironmentState) -> DewPointEvaluation {
        let current = &state.current;

        // Extraire mesures actuelles
        let air_temp = current.temperature_c;
        let humidity = current.humidity_pct;

        // Calculer point de rosée si données disponibles
        let dew_point = match (air_temp, humidity) {
            (Some(t), Some(rh)) => self.calculate_dew_point(t, rh),
            _ => None,
        };

        // Estimer température de surface
        let surface_temp = air_temp.map(|t| self.estimate_surface_temp(t));

        // Calculer deltaT si possible
        let delta_t = match (surface_temp, dew_point) {
            (Some(ts), Some(td)) => Some(ts - td),
            _ => None,
        };

        // Évaluer niveau (du plus critique au moins critique)
        let level = if self.is_danger_level(state, delta_t) {
            DewPointAlertLevel::Danger
        } else if self.is_critical_level(state, delta_t) {
            DewPointAlertLevel::Critical
        } else if self.is_strong_level(state, delta_t) {
            DewPointAlertLevel::Strong
        } else if self.is_moderate_level(state) {
            DewPointAlertLevel::Moderate
        } else if self.is_weak_level(state) {
            DewPointAlertLevel::Weak
        } else {
            DewPointAlertLevel::Safe
        };

        DewPointEvaluation {
            level,
            dew_point_c: dew_point,
            delta_t,
            air_temp_c: air_temp,
            humidity_pct: humidity,
            surface_temp_c: surface_temp,
            message: level.message().to_string(),
            suggestion: level.suggestion().to_string(),
            diagnostics: serde_json::json!({
                "config": {
                    "surface_offset_c": self.config.surface_temp_offset,
                    "magnus_a": self.config.magnus_a,
                    "magnus_b": self.config.magnus_b,
                },
                "calculations": {
                    "dew_point_c": dew_point,
                    "surface_temp_c": surface_temp,
                    "delta_t_c": delta_t,
                },
            }),
        }
    }

    /// Niveau 5: Danger - Condensation certaine
    fn is_danger_level(&self, state: &RoomEnvironmentState, delta_t: Option<f32>) -> bool {
        let duration_min = self.config.danger_duration_minutes;

        // Condition 1: RH > 75% pendant 5 min
        let rh_danger = state.is_humidity_sustained(
            self.config.danger_rh_threshold,
            duration_min,
        );

        // Condition 2: deltaT ≤ 0°C pendant 5 min (condensation active)
        let delta_danger = self.is_delta_t_sustained_below(
            state,
            self.config.danger_delta_t,
            duration_min,
        );

        rh_danger || delta_danger
    }

    /// Niveau 4: Critical - Condensation très probable
    fn is_critical_level(&self, state: &RoomEnvironmentState, delta_t: Option<f32>) -> bool {
        let duration_min = self.config.critical_duration_minutes;

        // Condition 1: RH > 70% pendant 20 min
        let rh_critical = state.is_humidity_sustained(
            self.config.critical_rh_threshold,
            duration_min,
        );

        // Condition 2: deltaT < 2°C pendant 20 min
        let delta_critical = self.is_delta_t_sustained_below(
            state,
            self.config.critical_delta_t,
            duration_min,
        );

        rh_critical || delta_critical
    }

    /// Niveau 3: Strong - Risque condensation
    fn is_strong_level(&self, state: &RoomEnvironmentState, delta_t: Option<f32>) -> bool {
        let duration_min = (self.config.strong_duration_hours * 60.0) as u32;

        // Condition 1: RH > 65% pendant 1h
        let rh_strong = state.is_humidity_sustained(
            self.config.strong_rh_threshold,
            duration_min,
        );

        // Condition 2: deltaT < 3°C pendant 1h
        let delta_strong = self.is_delta_t_sustained_below(
            state,
            self.config.strong_delta_t,
            duration_min,
        );

        rh_strong || delta_strong
    }

    /// Niveau 2: Moderate - Humidité excessive
    fn is_moderate_level(&self, state: &RoomEnvironmentState) -> bool {
        let duration_min = (self.config.moderate_duration_hours * 60.0) as u32;

        state.is_humidity_sustained(self.config.moderate_rh_threshold, duration_min)
    }

    /// Niveau 1: Weak - Tendance haute
    fn is_weak_level(&self, state: &RoomEnvironmentState) -> bool {
        let duration_min = (self.config.weak_duration_hours * 60.0) as u32;

        state.is_humidity_sustained(self.config.weak_rh_threshold, duration_min)
    }

    /// Vérifier si deltaT est resté sous seuil pendant durée donnée
    ///
    /// Similaire à is_humidity_sustained mais pour deltaT
    ///
    /// IMPORTANT: Validates that actual time coverage meets duration requirement
    /// to prevent false positives with insufficient historical data
    fn is_delta_t_sustained_below(
        &self,
        state: &RoomEnvironmentState,
        threshold: f32,
        duration_minutes: u32,
    ) -> bool {
        let cutoff = Utc::now() - Duration::minutes(duration_minutes as i64);

        // Calculer deltaT actuel
        let current_delta = self.calculate_current_delta_t(state);

        // Vérifier deltaT actuel
        let Some(delta) = current_delta else {
            return false; // Pas de données
        };
        if delta > threshold {
            return false; // Actuel au-dessus du seuil
        }

        // Collect readings in time window
        let readings_in_window: Vec<_> = state
            .history
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

        // Vérifier que tous les deltaT sont sous le seuil
        readings_in_window
            .iter()
            .all(|reading| {
                let delta = self.calculate_reading_delta_t(reading);
                delta.map_or(false, |d| d < threshold)
            })
    }

    /// Calculer deltaT pour lecture actuelle
    fn calculate_current_delta_t(&self, state: &RoomEnvironmentState) -> Option<f32> {
        self.calculate_reading_delta_t(&state.current)
    }

    /// Calculer deltaT pour une lecture donnée
    fn calculate_reading_delta_t(&self, reading: &EnvReading) -> Option<f32> {
        let air_temp = reading.temperature_c?;
        let humidity = reading.humidity_pct?;

        let dew_point = self.calculate_dew_point(air_temp, humidity)?;
        let surface_temp = self.estimate_surface_temp(air_temp);

        Some(surface_temp - dew_point)
    }
}

impl Default for DewPointCalculator {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::environment::RoomEnvironmentState;

    #[test]
    fn test_calculate_dew_point_typical_values() {
        let calc = DewPointCalculator::new();

        // Cas 1: 20°C, 60% RH → ~12°C point de rosée
        let dew = calc.calculate_dew_point(20.0, 60.0).unwrap();
        assert!((dew - 12.0).abs() < 0.5, "Dew point should be ~12°C, got {}", dew);

        // Cas 2: 25°C, 50% RH → ~13.9°C
        let dew = calc.calculate_dew_point(25.0, 50.0).unwrap();
        assert!((dew - 13.9).abs() < 0.5, "Dew point should be ~13.9°C, got {}", dew);

        // Cas 3: 15°C, 80% RH → ~11.8°C
        let dew = calc.calculate_dew_point(15.0, 80.0).unwrap();
        assert!((dew - 11.8).abs() < 0.5, "Dew point should be ~11.8°C, got {}", dew);
    }

    #[test]
    fn test_calculate_dew_point_invalid_inputs() {
        let calc = DewPointCalculator::new();

        // RH invalides
        assert!(calc.calculate_dew_point(20.0, 0.0).is_none());
        assert!(calc.calculate_dew_point(20.0, -10.0).is_none());
        assert!(calc.calculate_dew_point(20.0, 110.0).is_none());

        // Température hors plage
        assert!(calc.calculate_dew_point(-50.0, 50.0).is_none());
        assert!(calc.calculate_dew_point(60.0, 50.0).is_none());
    }

    #[test]
    fn test_estimate_surface_temp() {
        let calc = DewPointCalculator::new();

        // Avec offset par défaut de 3°C
        assert_eq!(calc.estimate_surface_temp(20.0), 17.0);
        assert_eq!(calc.estimate_surface_temp(15.0), 12.0);
    }

    #[test]
    fn test_evaluate_safe_conditions() {
        let calc = DewPointCalculator::new();
        let state = RoomEnvironmentState::new("chambre".to_string());

        // État par défaut: 20°C, 50% RH → Safe
        let eval = calc.evaluate(&state);
        assert_eq!(eval.level, DewPointAlertLevel::Safe);
        assert!(eval.dew_point_c.is_some());
        assert!(eval.delta_t.is_some());
    }

    #[test]
    fn test_evaluate_weak_level() {
        let calc = DewPointCalculator::new();
        let mut state = RoomEnvironmentState::new("chambre".to_string());

        // Ajouter 6h de lectures à 57% RH (au-dessus seuil 55%, sous 60%)
        // Need proper time span with 30sec intervals
        let duration_min = 6 * 60; // 360 minutes = 6 hours
        let num_readings = (duration_min * 2) as usize; // 30sec interval = 720 readings

        for i in 0..num_readings {
            let reading = EnvReading {
                temperature_c: Some(20.0),
                humidity_pct: Some(57.0), // Above weak threshold (55%)
                timestamp: Utc::now() - Duration::seconds(((duration_min * 60) - (i * 30)) as i64),
            };
            state.update(reading);
        }

        // Add current reading to ensure freshness
        let reading = EnvReading {
            temperature_c: Some(20.0),
            humidity_pct: Some(57.0),
            timestamp: Utc::now(),
        };
        state.update(reading);

        let eval = calc.evaluate(&state);
        assert_eq!(eval.level, DewPointAlertLevel::Weak);
        assert_eq!(eval.message, "Humidité en tendance haute");
    }

    #[test]
    fn test_evaluate_moderate_level() {
        let calc = DewPointCalculator::new();
        let mut state = RoomEnvironmentState::new("chambre".to_string());

        // 3h de lectures à 62% RH (au-dessus seuil 60%)
        // Need proper time span with 30sec intervals
        let duration_min = 3 * 60; // 180 minutes = 3 hours
        let num_readings = (duration_min * 2) as usize; // 30sec interval = 360 readings

        for i in 0..num_readings {
            let reading = EnvReading {
                temperature_c: Some(20.0),
                humidity_pct: Some(62.0), // Above moderate threshold (60%)
                timestamp: Utc::now() - Duration::seconds(((duration_min * 60) - (i * 30)) as i64),
            };
            state.update(reading);
        }

        // Add current reading to ensure freshness
        let reading = EnvReading {
            temperature_c: Some(20.0),
            humidity_pct: Some(62.0),
            timestamp: Utc::now(),
        };
        state.update(reading);

        let eval = calc.evaluate(&state);
        assert_eq!(eval.level, DewPointAlertLevel::Moderate);
    }

    #[test]
    fn test_evaluate_danger_level_high_rh() {
        let calc = DewPointCalculator::new();
        let mut state = RoomEnvironmentState::new("chambre".to_string());

        // 5 minutes de lectures à 78% RH (au-dessus seuil 75%)
        // Need proper time span with 30sec intervals
        let duration_min = 5;
        let num_readings = (duration_min * 2) as usize; // 30sec interval = 10 readings

        for i in 0..num_readings {
            let reading = EnvReading {
                temperature_c: Some(20.0),
                humidity_pct: Some(78.0), // Above danger threshold (75%)
                timestamp: Utc::now() - Duration::seconds(((duration_min * 60) - (i * 30)) as i64),
            };
            state.update(reading);
        }

        // Add current reading to ensure freshness
        let reading = EnvReading {
            temperature_c: Some(20.0),
            humidity_pct: Some(78.0),
            timestamp: Utc::now(),
        };
        state.update(reading);

        let eval = calc.evaluate(&state);
        assert_eq!(eval.level, DewPointAlertLevel::Danger);
        assert_eq!(eval.message, "Condensation certaine / surfaces humides");
    }

    #[test]
    fn test_alert_level_ordering() {
        // Vérifier ordre priorité
        assert!(DewPointAlertLevel::Danger > DewPointAlertLevel::Critical);
        assert!(DewPointAlertLevel::Critical > DewPointAlertLevel::Strong);
        assert!(DewPointAlertLevel::Strong > DewPointAlertLevel::Moderate);
        assert!(DewPointAlertLevel::Moderate > DewPointAlertLevel::Weak);
        assert!(DewPointAlertLevel::Weak > DewPointAlertLevel::Safe);
    }

    #[test]
    fn test_config_customization() {
        let mut config = DewPointAlertConfig::default();
        config.danger_rh_threshold = 80.0; // Plus strict
        config.surface_temp_offset = 5.0;  // Murs très mal isolés

        let calc = DewPointCalculator::with_config(config);

        // Vérifier offset appliqué
        assert_eq!(calc.estimate_surface_temp(20.0), 15.0);
    }

    #[test]
    fn test_dew_point_evaluation_with_diagnostics() {
        let calc = DewPointCalculator::new();
        let state = RoomEnvironmentState::new("chambre".to_string());

        let eval = calc.evaluate(&state);

        // Vérifier diagnostics présents
        assert!(eval.diagnostics["config"]["surface_offset_c"].is_number());
        assert!(eval.diagnostics["calculations"]["dew_point_c"].is_number());
    }
}
