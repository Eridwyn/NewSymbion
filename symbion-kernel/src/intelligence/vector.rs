//! Context Vector
//!
//! Normalized feature vector for mode prediction.
//! Each dimension represents a probability (0.0-1.0) with explainability.
//!
//! ## Architecture
//!
//! ```text
//! FeatureRegistry → VectorBuilder → ContextVector → Inference
//! ```
//!
//! ## Dimensions
//!
//! - `home_prob`: Probability of being in home/relaxation mode
//! - `work_prob`: Probability of being in work/professional mode
//! - `focus_prob`: Probability of being in deep focus mode
//! - `sleep_prob`: Probability of being in sleep/away mode
//! - `pc_active`: PC activity level (0.0=off, 1.0=fully active)

use std::collections::HashMap;
use serde::Serialize;
use time::OffsetDateTime;

use super::{FeatureRegistry, FeatureValue, feature_ids};

// ============================================================================
// Context Vector
// ============================================================================

/// Normalized context vector for mode prediction
#[derive(Debug, Clone, Serialize)]
pub struct ContextVector {
    /// Normalized dimensions (0.0-1.0)
    pub dimensions: HashMap<String, f32>,

    /// Explanations per dimension (why chain)
    pub why: HashMap<String, Vec<WhyItem>>,

    /// When this vector was built
    #[serde(with = "time::serde::iso8601")]
    pub built_at: OffsetDateTime,

    /// Number of features used to build this vector
    pub feature_count: usize,
}

/// Explanation item for a dimension contribution
#[derive(Debug, Clone, Serialize)]
pub struct WhyItem {
    /// Feature that contributed to this dimension
    pub feature_id: String,

    /// Contribution weight (-1.0 to +1.0)
    pub contribution: f32,

    /// Raw value for debugging
    pub raw_value: String,
}

impl ContextVector {
    /// Create an empty vector (all dimensions at 0.5 = unknown)
    pub fn empty() -> Self {
        let mut dimensions = HashMap::new();
        dimensions.insert("home_prob".to_string(), 0.5);
        dimensions.insert("work_prob".to_string(), 0.5);
        dimensions.insert("focus_prob".to_string(), 0.5);
        dimensions.insert("sleep_prob".to_string(), 0.5);
        dimensions.insert("pc_active".to_string(), 0.0);

        Self {
            dimensions,
            why: HashMap::new(),
            built_at: OffsetDateTime::now_utc(),
            feature_count: 0,
        }
    }

    /// Get a dimension value (default 0.5 if not found)
    pub fn get(&self, dimension: &str) -> f32 {
        *self.dimensions.get(dimension).unwrap_or(&0.5)
    }

    /// Get the most likely mode based on probabilities
    pub fn best_mode(&self) -> (&str, f32) {
        let modes = [
            ("focus", self.get("focus_prob")),
            ("pro", self.get("work_prob")),
            ("maison", self.get("home_prob")),
            ("veille", self.get("sleep_prob")),
        ];

        modes.iter()
            .max_by(|a, b| a.1.partial_cmp(&b.1).unwrap_or(std::cmp::Ordering::Equal))
            .map(|(mode, prob)| (*mode, *prob))
            .unwrap_or(("unknown", 0.0))
    }

    /// Check if we have enough confidence to make a prediction
    pub fn has_sufficient_data(&self) -> bool {
        self.feature_count >= 2 && self.get("pc_active") > 0.1
    }
}

// ============================================================================
// Dimension Constants
// ============================================================================

/// Well-known dimension names
pub mod dimensions {
    pub const HOME_PROB: &str = "home_prob";
    pub const WORK_PROB: &str = "work_prob";
    pub const FOCUS_PROB: &str = "focus_prob";
    pub const SLEEP_PROB: &str = "sleep_prob";
    pub const PC_ACTIVE: &str = "pc_active";
}

// ============================================================================
// Vector Builder
// ============================================================================

/// Builds a ContextVector from FeatureRegistry
pub struct VectorBuilder<'a> {
    registry: &'a FeatureRegistry,
    dimensions: HashMap<String, f32>,
    why: HashMap<String, Vec<WhyItem>>,
    feature_count: usize,
}

impl<'a> VectorBuilder<'a> {
    /// Create a new builder from a feature registry
    pub fn new(registry: &'a FeatureRegistry) -> Self {
        Self {
            registry,
            dimensions: HashMap::new(),
            why: HashMap::new(),
            feature_count: 0,
        }
    }

    /// Build the context vector
    pub fn build(mut self) -> ContextVector {
        // Initialize with neutral values
        self.dimensions.insert(dimensions::HOME_PROB.to_string(), 0.5);
        self.dimensions.insert(dimensions::WORK_PROB.to_string(), 0.5);
        self.dimensions.insert(dimensions::FOCUS_PROB.to_string(), 0.5);
        self.dimensions.insert(dimensions::SLEEP_PROB.to_string(), 0.5);
        self.dimensions.insert(dimensions::PC_ACTIVE.to_string(), 0.0);

        // Process features
        self.process_agent_features();
        self.process_time_features();
        self.process_environment_features();
        self.process_presence_features();
        self.process_process_features();

        // Normalize probabilities to sum to ~1.0
        self.normalize_mode_probabilities();

        ContextVector {
            dimensions: self.dimensions,
            why: self.why,
            built_at: OffsetDateTime::now_utc(),
            feature_count: self.feature_count,
        }
    }

    /// Process agent-related features (online, cpu, memory)
    fn process_agent_features(&mut self) {
        // Agent online → PC is active
        if let Some(sample) = self.registry.get(feature_ids::AGENT_ONLINE) {
            self.feature_count += 1;
            if sample.value.as_bool().unwrap_or(false) {
                self.add_contribution(dimensions::PC_ACTIVE, 0.8, sample.confidence,
                    feature_ids::AGENT_ONLINE, "true");
            }
        }

        // CPU usage → activity level
        if let Some(sample) = self.registry.get(feature_ids::AGENT_CPU_USAGE) {
            self.feature_count += 1;
            if let Some(cpu) = sample.value.as_float() {
                let cpu_normalized = (cpu / 100.0).clamp(0.0, 1.0) as f32;

                // High CPU → likely focus/work
                if cpu_normalized > 0.3 {
                    self.add_contribution(dimensions::PC_ACTIVE, cpu_normalized * 0.5, sample.confidence,
                        feature_ids::AGENT_CPU_USAGE, &format!("{:.1}%", cpu));
                    self.add_contribution(dimensions::FOCUS_PROB, cpu_normalized * 0.2, sample.confidence,
                        feature_ids::AGENT_CPU_USAGE, &format!("{:.1}%", cpu));
                }
            }
        }

        // Memory usage → sustained activity
        if let Some(sample) = self.registry.get(feature_ids::AGENT_MEMORY_USAGE) {
            self.feature_count += 1;
            if let Some(mem) = sample.value.as_float() {
                let mem_normalized = (mem / 100.0).clamp(0.0, 1.0) as f32;

                if mem_normalized > 0.4 {
                    self.add_contribution(dimensions::PC_ACTIVE, mem_normalized * 0.3, sample.confidence,
                        feature_ids::AGENT_MEMORY_USAGE, &format!("{:.1}%", mem));
                }
            }
        }

        // Idle time → sleep probability (if we had this feature)
        if let Some(sample) = self.registry.get(feature_ids::AGENT_IDLE_SECONDS) {
            self.feature_count += 1;
            if let Some(idle) = sample.value.as_float() {
                let idle_minutes = idle / 60.0;

                if idle_minutes > 30.0 {
                    // Long idle → likely away/sleep
                    let sleep_contrib = ((idle_minutes - 30.0) / 60.0).clamp(0.0, 0.5) as f32;
                    self.add_contribution(dimensions::SLEEP_PROB, sleep_contrib, sample.confidence,
                        feature_ids::AGENT_IDLE_SECONDS, &format!("{:.0}min idle", idle_minutes));
                    self.add_contribution(dimensions::PC_ACTIVE, -sleep_contrib, sample.confidence,
                        feature_ids::AGENT_IDLE_SECONDS, &format!("{:.0}min idle", idle_minutes));
                }
            }
        }
    }

    /// Process time-related features
    fn process_time_features(&mut self) {
        // Use current time in Paris timezone
        let now = super::local_now();
        let hour = now.hour() as f32;
        let weekday = now.weekday().number_days_from_monday(); // 0=Monday

        self.feature_count += 1;

        // Weekend → home probability (time is always confidence 1.0)
        let is_weekend = weekday >= 5;
        if is_weekend {
            self.add_contribution(dimensions::HOME_PROB, 0.3, 1.0, "time.weekend", "true");
            self.add_contribution(dimensions::WORK_PROB, -0.2, 1.0, "time.weekend", "true");
        }

        // Hour-based probabilities
        match hour as u8 {
            0..=6 => {
                // Night: sleep mode
                self.add_contribution(dimensions::SLEEP_PROB, 0.4, 1.0, "time.hour", &format!("{}h", hour as u8));
                self.add_contribution(dimensions::HOME_PROB, 0.2, 1.0, "time.hour", &format!("{}h", hour as u8));
            }
            7..=8 => {
                // Early morning: transition
                self.add_contribution(dimensions::HOME_PROB, 0.2, 1.0, "time.hour", &format!("{}h", hour as u8));
            }
            9..=12 => {
                // Morning work hours
                if !is_weekend {
                    self.add_contribution(dimensions::WORK_PROB, 0.2, 1.0, "time.hour", &format!("{}h", hour as u8));
                    self.add_contribution(dimensions::FOCUS_PROB, 0.15, 1.0, "time.hour", &format!("{}h", hour as u8));
                }
            }
            13..=14 => {
                // Lunch break
                self.add_contribution(dimensions::HOME_PROB, 0.1, 1.0, "time.hour", &format!("{}h", hour as u8));
            }
            15..=18 => {
                // Afternoon work
                if !is_weekend {
                    self.add_contribution(dimensions::WORK_PROB, 0.15, 1.0, "time.hour", &format!("{}h", hour as u8));
                    self.add_contribution(dimensions::FOCUS_PROB, 0.1, 1.0, "time.hour", &format!("{}h", hour as u8));
                }
            }
            19..=22 => {
                // Evening: home/leisure
                self.add_contribution(dimensions::HOME_PROB, 0.3, 1.0, "time.hour", &format!("{}h", hour as u8));
                self.add_contribution(dimensions::WORK_PROB, -0.1, 1.0, "time.hour", &format!("{}h", hour as u8));
            }
            23 => {
                // Late night
                self.add_contribution(dimensions::SLEEP_PROB, 0.2, 1.0, "time.hour", &format!("{}h", hour as u8));
                self.add_contribution(dimensions::HOME_PROB, 0.2, 1.0, "time.hour", &format!("{}h", hour as u8));
            }
            _ => {}
        }
    }

    /// Process environment features (temperature, humidity)
    fn process_environment_features(&mut self) {
        if let Some(sample) = self.registry.get(feature_ids::ENV_TEMPERATURE) {
            self.feature_count += 1;
            if let Some(temp) = sample.value.as_float() {
                // Comfortable temp (18-24°C) → slight home preference
                if (18.0..=24.0).contains(&temp) {
                    self.add_contribution(dimensions::HOME_PROB, 0.05, sample.confidence,
                        feature_ids::ENV_TEMPERATURE, &format!("{:.1}°C", temp));
                }
            }
        }

        // Humidity is informational, doesn't directly affect mode
        if self.registry.get(feature_ids::ENV_HUMIDITY).is_some() {
            self.feature_count += 1;
        }
    }

    /// Process presence features (phone on network = someone home)
    fn process_presence_features(&mut self) {
        // Phone presence is a strong indicator of being home
        if let Some(sample) = self.registry.get(feature_ids::PRESENCE_PHONE) {
            self.feature_count += 1;
            let conf = sample.confidence;

            if sample.value.as_bool().unwrap_or(false) {
                // Phone on network → strong home signal
                self.add_contribution(dimensions::HOME_PROB, 0.35, conf,
                    feature_ids::PRESENCE_PHONE, "phone on network");
                // Less likely to be away if phone is home
                self.add_contribution(dimensions::WORK_PROB, -0.15, conf,
                    feature_ids::PRESENCE_PHONE, "phone on network");
            } else {
                // Phone not on network → might be away
                self.add_contribution(dimensions::HOME_PROB, -0.25, conf,
                    feature_ids::PRESENCE_PHONE, "phone away");
                // Could be at work
                self.add_contribution(dimensions::WORK_PROB, 0.1, conf,
                    feature_ids::PRESENCE_PHONE, "phone away");
            }
        }

        // Anyone home summary (aggregates all tracked devices)
        if let Some(sample) = self.registry.get(feature_ids::PRESENCE_ANYONE_HOME) {
            self.feature_count += 1;
            let conf = sample.confidence;

            if sample.value.as_bool().unwrap_or(false) {
                self.add_contribution(dimensions::HOME_PROB, 0.2, conf,
                    feature_ids::PRESENCE_ANYONE_HOME, "someone home");
            } else {
                self.add_contribution(dimensions::HOME_PROB, -0.3, conf,
                    feature_ids::PRESENCE_ANYONE_HOME, "nobody home");
            }
        }
    }

    /// Process running processes to detect work/leisure patterns
    fn process_process_features(&mut self) {
        if let Some(sample) = self.registry.get("process.list") {
            let conf = sample.confidence;
            if let FeatureValue::StringList(processes) = &sample.value {
                self.feature_count += 1;

                // Detect IDE/dev tools → focus mode
                let ide_processes = ["rustrover", "intellij", "vscode", "code", "pycharm",
                    "webstorm", "rider", "goland", "clion", "datagrip", "phpstorm"];
                let ide_count = processes.iter()
                    .filter(|p| ide_processes.iter().any(|ide| p.to_lowercase().contains(ide)))
                    .count();

                if ide_count > 0 {
                    let contrib = (ide_count as f32 * 0.15).min(0.4);
                    self.add_contribution(dimensions::FOCUS_PROB, contrib, conf,
                        "process.category.ide", &format!("{} IDE(s)", ide_count));
                    self.add_contribution(dimensions::WORK_PROB, contrib * 0.5, conf,
                        "process.category.ide", &format!("{} IDE(s)", ide_count));
                }

                // Detect media apps → home/leisure mode
                let media_processes = ["netflix", "kodi", "plex", "vlc", "mpv", "stremio"];
                let media_count = processes.iter()
                    .filter(|p| media_processes.iter().any(|m| p.to_lowercase().contains(m)))
                    .count();

                if media_count > 0 {
                    let contrib = (media_count as f32 * 0.2).min(0.4);
                    self.add_contribution(dimensions::HOME_PROB, contrib, conf,
                        "process.category.media", &format!("{} media app(s)", media_count));
                    self.add_contribution(dimensions::FOCUS_PROB, -contrib * 0.3, conf,
                        "process.category.media", &format!("{} media app(s)", media_count));
                }

                // Detect communication apps → could be work or personal
                let comm_processes = ["slack", "teams", "discord", "zoom", "meet"];
                let comm_count = processes.iter()
                    .filter(|p| comm_processes.iter().any(|c| p.to_lowercase().contains(c)))
                    .count();

                if comm_count > 0 {
                    // Communication is ambiguous, slight work bias during work hours
                    let now = super::local_now();
                    let hour = now.hour();

                    if (9..18).contains(&hour) {
                        self.add_contribution(dimensions::WORK_PROB, 0.1, conf,
                            "process.category.communication", &format!("{} comm app(s)", comm_count));
                    }
                }
            }
        }
    }

    /// Add a contribution to a dimension with explanation
    /// The contribution is multiplied by confidence (0.0-1.0) to weight by source reliability.
    fn add_contribution(&mut self, dimension: &str, contribution: f32, confidence: f32, feature_id: &str, raw_value: &str) {
        let confidence = confidence.clamp(0.0, 1.0);
        let effective = contribution * confidence;
        let current = self.dimensions.get(dimension).copied().unwrap_or(0.5);
        let new_value = (current + effective).clamp(0.0, 1.0);
        self.dimensions.insert(dimension.to_string(), new_value);

        let why_list = self.why.entry(dimension.to_string()).or_insert_with(Vec::new);
        why_list.push(WhyItem {
            feature_id: feature_id.to_string(),
            contribution: effective,
            raw_value: raw_value.to_string(),
        });
    }

    /// Normalize mode probabilities so they roughly sum to 1.0
    fn normalize_mode_probabilities(&mut self) {
        let mode_dims = [
            dimensions::HOME_PROB,
            dimensions::WORK_PROB,
            dimensions::FOCUS_PROB,
            dimensions::SLEEP_PROB,
        ];

        let sum: f32 = mode_dims.iter()
            .map(|d| self.dimensions.get(*d).copied().unwrap_or(0.0))
            .sum();

        if sum > 1e-6 {
            for dim in mode_dims {
                if let Some(val) = self.dimensions.get_mut(dim) {
                    *val /= sum;
                }
            }
        } else {
            eprintln!("[vector] Zero-sum mode probabilities — normalization skipped");
        }
    }
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Arc;

    #[test]
    fn test_empty_vector() {
        let vector = ContextVector::empty();
        assert_eq!(vector.get("home_prob"), 0.5);
        assert_eq!(vector.get("unknown_dim"), 0.5); // Default
        assert_eq!(vector.feature_count, 0);
        assert!(!vector.has_sufficient_data());
    }

    #[test]
    fn test_vector_builder_basic() {
        let registry = FeatureRegistry::new();

        // Add some features
        registry.set_feature(
            feature_ids::AGENT_ONLINE,
            FeatureValue::Bool(true),
            "test",
            1.0,
            60,
        );

        let vector = VectorBuilder::new(&registry).build();

        // PC should be active since agent is online
        assert!(vector.get(dimensions::PC_ACTIVE) > 0.5);
        assert!(vector.feature_count >= 1);
    }

    #[test]
    fn test_best_mode() {
        let mut vector = ContextVector::empty();
        vector.dimensions.insert("focus_prob".to_string(), 0.6);
        vector.dimensions.insert("home_prob".to_string(), 0.2);
        vector.dimensions.insert("work_prob".to_string(), 0.15);
        vector.dimensions.insert("sleep_prob".to_string(), 0.05);

        let (mode, prob) = vector.best_mode();
        assert_eq!(mode, "focus");
        assert!((prob - 0.6).abs() < 0.01);
    }

    #[test]
    fn test_why_chain() {
        let registry = FeatureRegistry::new();

        registry.set_feature(
            feature_ids::AGENT_CPU_USAGE,
            FeatureValue::Float(75.0),
            "test",
            1.0,
            60,
        );

        let vector = VectorBuilder::new(&registry).build();

        // Should have explanation for PC_ACTIVE
        let pc_why = vector.why.get(dimensions::PC_ACTIVE);
        assert!(pc_why.is_some());
    }
}
