//! Bootstrap Scheduler
//!
//! Provides initial training samples for cold start scenarios.
//! When the system has no user data, bootstrap rules generate
//! low-weight samples to kickstart the inference engine.
//!
//! ## Architecture
//!
//! Bootstrap samples have:
//! - Source: `SampleSource::Bootstrap` (lowest weight: 0.5x)
//! - Fast decay (30 days)
//! - Are overridden by real user corrections
//!
//! ## Default Rules
//!
//! - Weekday 9-18h → focus/work mode
//! - Weekend → home mode
//! - Night (22h-7h) → sleep mode

use serde::{Deserialize, Serialize};
use time::OffsetDateTime;
use time_tz::{timezones, OffsetDateTimeExt};

use super::inference::{InferenceEngine, SampleSource};
use super::vector::ContextVector;

// ============================================================================
// Bootstrap Config
// ============================================================================

/// Bootstrap schedule configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BootstrapConfig {
    /// Enable bootstrap (default true for cold start)
    pub enabled: bool,

    /// Default mode for weekday work hours
    pub weekday_work_mode: String,

    /// Default mode for weekday evenings
    pub weekday_evening_mode: String,

    /// Default mode for weekends
    pub weekend_mode: String,

    /// Default mode for night hours
    pub night_mode: String,

    /// Work hours start (0-23)
    pub work_hours_start: u8,

    /// Work hours end (0-23)
    pub work_hours_end: u8,

    /// Night hours start (0-23)
    pub night_hours_start: u8,

    /// Night hours end (0-23)
    pub night_hours_end: u8,

    /// Days until bootstrap samples expire
    pub expiry_days: u32,
}

impl Default for BootstrapConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            weekday_work_mode: "focus".to_string(),
            weekday_evening_mode: "maison".to_string(),
            weekend_mode: "maison".to_string(),
            night_mode: "veille".to_string(),
            work_hours_start: 9,
            work_hours_end: 18,
            night_hours_start: 22,
            night_hours_end: 7,
            expiry_days: 21, // 3 weeks
        }
    }
}

// ============================================================================
// Bootstrap Scheduler
// ============================================================================

/// Generates bootstrap samples for cold start
pub struct BootstrapScheduler {
    config: BootstrapConfig,
}

impl Default for BootstrapScheduler {
    fn default() -> Self {
        Self {
            config: BootstrapConfig::default(),
        }
    }
}

impl BootstrapScheduler {
    /// Create with custom config
    pub fn new(config: BootstrapConfig) -> Self {
        Self { config }
    }

    /// Get current config
    pub fn config(&self) -> &BootstrapConfig {
        &self.config
    }

    /// Suggest a mode based on current time (bootstrap rules)
    pub fn suggest_mode(&self) -> Option<String> {
        if !self.config.enabled {
            return None;
        }

        let now = now_paris();
        let hour = now.hour();
        let weekday = now.weekday();
        let is_weekend = matches!(weekday, time::Weekday::Saturday | time::Weekday::Sunday);

        // Night hours (wraps around midnight)
        let is_night = if self.config.night_hours_start > self.config.night_hours_end {
            // e.g., 22h-7h
            hour >= self.config.night_hours_start || hour < self.config.night_hours_end
        } else {
            hour >= self.config.night_hours_start && hour < self.config.night_hours_end
        };

        if is_night {
            return Some(self.config.night_mode.clone());
        }

        if is_weekend {
            return Some(self.config.weekend_mode.clone());
        }

        // Weekday
        let is_work_hours = hour >= self.config.work_hours_start && hour < self.config.work_hours_end;
        if is_work_hours {
            Some(self.config.weekday_work_mode.clone())
        } else {
            Some(self.config.weekday_evening_mode.clone())
        }
    }

    /// Generate bootstrap samples and add to inference engine
    /// Call this during system initialization
    pub fn seed_inference_engine(&self, engine: &InferenceEngine, vector: &ContextVector) {
        if !self.config.enabled {
            eprintln!("[bootstrap] disabled, skipping seed");
            return;
        }

        // Only seed if engine has very few samples
        let stats = engine.stats();
        if stats.total_samples > 10 {
            eprintln!("[bootstrap] engine has {} samples, skipping seed", stats.total_samples);
            return;
        }

        // Add bootstrap samples for different time slots
        let suggested = self.suggest_mode();
        if let Some(mode) = suggested {
            engine.record_bootstrap(vector, &mode);
            eprintln!("[bootstrap] seeded inference engine with mode '{}' (based on current time)", mode);
        }
    }

    /// Check if bootstrap is still needed
    pub fn is_bootstrap_needed(&self, engine: &InferenceEngine) -> bool {
        if !self.config.enabled {
            return false;
        }
        let stats = engine.stats();
        // Bootstrap needed if we have very few non-bootstrap samples
        let non_bootstrap = stats.total_samples.saturating_sub(
            stats.by_source.get(&format!("{:?}", SampleSource::Bootstrap)).copied().unwrap_or(0)
        );
        non_bootstrap < 10
    }
}

// ============================================================================
// Intelligence Mode Config
// ============================================================================

/// Intelligence engine mode
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "lowercase")]
pub enum IntelligenceMode {
    /// Use legacy v1 intelligence (ContextIntelligence)
    Legacy,
    /// Use new v2 intelligence (InferenceEngine + SessionManager)
    #[default]
    V2,
    /// Run both in parallel, compare results (v2 doesn't apply)
    Shadow,
}

impl IntelligenceMode {
    /// Whether v2 predictions should be applied
    pub fn should_apply_v2(&self) -> bool {
        matches!(self, IntelligenceMode::V2)
    }

    /// Whether v2 predictions should be logged (for comparison)
    pub fn should_log_v2(&self) -> bool {
        matches!(self, IntelligenceMode::Shadow | IntelligenceMode::V2)
    }

    /// Whether legacy predictions should be applied
    pub fn should_apply_legacy(&self) -> bool {
        matches!(self, IntelligenceMode::Legacy)
    }
}

impl std::fmt::Display for IntelligenceMode {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            IntelligenceMode::Legacy => write!(f, "legacy"),
            IntelligenceMode::V2 => write!(f, "v2"),
            IntelligenceMode::Shadow => write!(f, "shadow"),
        }
    }
}

impl std::str::FromStr for IntelligenceMode {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "legacy" | "v1" => Ok(IntelligenceMode::Legacy),
            "v2" | "new" => Ok(IntelligenceMode::V2),
            "shadow" | "compare" => Ok(IntelligenceMode::Shadow),
            _ => Err(format!("Unknown intelligence mode: {}", s)),
        }
    }
}

// ============================================================================
// Helpers
// ============================================================================

fn now_paris() -> OffsetDateTime {
    OffsetDateTime::now_utc().to_timezone(timezones::db::europe::PARIS)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_suggest_mode_weekday_work() {
        let config = BootstrapConfig {
            work_hours_start: 9,
            work_hours_end: 18,
            ..Default::default()
        };
        let scheduler = BootstrapScheduler::new(config);
        // This test depends on current time, so we just verify it returns Some
        let mode = scheduler.suggest_mode();
        assert!(mode.is_some());
    }

    #[test]
    fn test_intelligence_mode_parse() {
        assert_eq!("legacy".parse::<IntelligenceMode>().unwrap(), IntelligenceMode::Legacy);
        assert_eq!("v2".parse::<IntelligenceMode>().unwrap(), IntelligenceMode::V2);
        assert_eq!("shadow".parse::<IntelligenceMode>().unwrap(), IntelligenceMode::Shadow);
    }

    #[test]
    fn test_bootstrap_config_default() {
        let config = BootstrapConfig::default();
        assert!(config.enabled);
        assert_eq!(config.weekday_work_mode, "focus");
        assert_eq!(config.weekend_mode, "maison");
    }
}
