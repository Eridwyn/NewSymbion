//! Intelligence Engine Configuration
//!
//! Contains all configurable parameters for the context intelligence system.

use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

/// Configuration for the intelligence engine
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct IntelligenceConfig {
    /// Threshold for auto-applying mode changes without validation (0.0-1.0)
    /// Default: 0.60 (60% confidence required)
    pub auto_apply_threshold: f32,

    /// Threshold for suggesting mode changes via notification (0.0-1.0)
    /// Default: 0.30 (30% confidence required)
    pub suggestion_threshold: f32,

    /// Minimum occurrences of a pattern before learning it
    /// Default: 3
    pub min_pattern_occurrences: u32,

    /// Weights for different signal sources
    pub weights: SignalWeights,

    /// Enable auto-creation of automations from learned patterns
    pub auto_create_automations: bool,

    /// Enable automatic adaptation when habits change
    pub auto_adapt: bool,

    /// Check interval in seconds for the intelligence monitor
    pub check_interval_seconds: u64,

    // ========== v1.1.9 Stabilization Parameters ==========

    /// Decay coefficients for pattern aging by recency bracket:
    /// - [0] <7 days (fresh): 1.0 = full weight, pattern is recent and reliable
    /// - [1] <30 days (recent): 0.9 = slight decay, still highly relevant
    /// - [2] <90 days (aging): 0.7 = moderate decay, may be seasonal
    /// - [3] >90 days (old): 0.4 = significant decay, kept for long-term trends
    pub decay_coefficients: [f32; 4],

    /// Days before a dead pattern is eligible for purge
    /// Default: 90 (acceptable), 120 for margin
    pub purge_threshold_days: u32,

    /// Maximum push notifications per day
    /// Default: 5
    pub max_push_per_day: u32,

    /// Cooldown in minutes between suggestions for same mode
    /// Default: 60
    pub suggestion_cooldown_minutes: u32,

    /// Quiet hours start (23 = 23:00, no push except 0.9+ established)
    pub quiet_hours_start: u8,

    /// Quiet hours end (7 = 07:00)
    pub quiet_hours_end: u8,

    // ========== Session Hysteresis Parameters ==========

    /// Confidence threshold to EXIT current mode (lower = more sticky)
    /// Default: 0.35
    pub session_exit_threshold: f32,

    // ========== v2 Stabilization Parameters ==========

    /// v2 configuration for strict auto-apply guards
    pub v2: V2StabilizationConfig,
}

/// v2 Stabilization configuration - strict guards before activation
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct V2StabilizationConfig {
    /// Auto-apply threshold for v2 (stricter than v1)
    /// Default: 0.70 (70% confidence required)
    pub auto_apply_threshold: f32,

    /// Minimum total samples before auto-apply is allowed
    /// Default: 20
    pub min_samples_total: usize,

    /// Minimum non-bootstrap samples required
    /// Default: 5
    pub min_samples_non_bootstrap: usize,

    /// Minimum recent samples (within recent_days_window)
    /// Default: 5
    pub min_recent_samples: usize,

    /// Window for "recent" samples in days
    /// Default: 14
    pub recent_days_window: i64,

    /// Minimum session stability in minutes before auto-apply
    /// Default: 3
    pub min_session_stable_minutes: i64,

    /// Bootstrap sample decay half-life in days (faster decay)
    /// Default: 7 (half weight after 7 days)
    pub bootstrap_decay_half_life_days: f32,

    /// Enable v2 auto-apply (false = shadow mode only)
    /// Default: false
    pub auto_apply_enabled: bool,

    /// Enable v2 suggestions (notifications)
    /// Default: false
    pub suggestions_enabled: bool,
}

impl V2StabilizationConfig {
    /// Validate cross-field constraints. Logs warnings and clamps invalid values.
    pub fn validate(&mut self) {
        self.auto_apply_threshold = self.auto_apply_threshold.clamp(0.0, 1.0);
        if self.min_samples_non_bootstrap > self.min_samples_total {
            eprintln!("[config] min_samples_non_bootstrap ({}) > min_samples_total ({}), clamping",
                self.min_samples_non_bootstrap, self.min_samples_total);
            self.min_samples_non_bootstrap = self.min_samples_total;
        }
        if self.bootstrap_decay_half_life_days <= 0.0 {
            eprintln!("[config] bootstrap_decay_half_life_days must be > 0, using 7.0");
            self.bootstrap_decay_half_life_days = 7.0;
        }
        if self.recent_days_window <= 0 {
            self.recent_days_window = 14;
        }
    }
}

impl Default for V2StabilizationConfig {
    fn default() -> Self {
        Self {
            auto_apply_threshold: 0.70,
            min_samples_total: 20,
            min_samples_non_bootstrap: 5,
            min_recent_samples: 5,
            recent_days_window: 14,
            min_session_stable_minutes: 3,
            bootstrap_decay_half_life_days: 7.0,
            auto_apply_enabled: true,   // v1.5.0: enabled after live validation (guards verified passing)
            suggestions_enabled: true,  // v1.5.0: enabled to push mid-confidence (0.30-0.50) suggestions
        }
    }
}

impl Default for IntelligenceConfig {
    fn default() -> Self {
        Self {
            auto_apply_threshold: 0.50,  // v1.5.0: lowered to match real-world v2 confidence ceiling (~0.5-0.7)
            suggestion_threshold: 0.30,  // v1.1.10: lowered to show more suggestions
            min_pattern_occurrences: 3,
            weights: SignalWeights::default(),
            auto_create_automations: true,
            auto_adapt: true,
            check_interval_seconds: 30,
            // v1.1.9 stabilization
            decay_coefficients: [1.0, 0.9, 0.7, 0.4],  // Softer than 1.0/0.8/0.5/0.2
            purge_threshold_days: 90,
            max_push_per_day: 5,
            suggestion_cooldown_minutes: 60,
            quiet_hours_start: 23,
            quiet_hours_end: 7,
            // session hysteresis
            session_exit_threshold: 0.35,
            // v2 stabilization
            v2: V2StabilizationConfig::default(),
        }
    }
}

/// Weights for different signal sources in prediction
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct SignalWeights {
    /// Weight for temporal signals (hour + day of week)
    pub temporal: f32,
    /// Weight for behavioral patterns (manual changes history)
    pub behavioral: f32,
    /// Weight for agent activity (CPU, processes, idle time)
    pub agent_activity: f32,
    /// Weight for environmental factors (temperature, humidity)
    pub environmental: f32,
    /// Weight for momentum (time in current mode)
    pub momentum: f32,
}

impl Default for SignalWeights {
    fn default() -> Self {
        let weights = Self {
            temporal: 0.35,       // Time patterns are reliable
            behavioral: 0.35,     // Learned patterns matter
            agent_activity: 0.15, // Increased: active apps are strong signal
            environmental: 0.05,
            momentum: 0.10,
        };
        debug_assert!({
            let sum = weights.temporal + weights.behavioral + weights.agent_activity +
                      weights.environmental + weights.momentum;
            (sum - 1.0).abs() < 0.01
        }, "SignalWeights must sum to 1.0");
        weights
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_config() {
        let config = IntelligenceConfig::default();
        assert_eq!(config.auto_apply_threshold, 0.50);
        assert_eq!(config.suggestion_threshold, 0.30);
        assert_eq!(config.min_pattern_occurrences, 3);
        assert!(config.auto_create_automations);
    }

    #[test]
    fn test_signal_weights_sum_to_one() {
        let weights = SignalWeights::default();
        let sum = weights.temporal + weights.behavioral + weights.agent_activity +
                  weights.environmental + weights.momentum;
        assert!((sum - 1.0).abs() < 0.001, "Weights should sum to 1.0, got {}", sum);
    }
}
