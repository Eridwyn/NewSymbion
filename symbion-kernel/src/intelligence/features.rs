//! Feature Registry
//!
//! Data-driven feature storage with TTL expiration.
//! Features are derived from raw signals (agents, sensors, plugins).
//!
//! The core doesn't interpret raw values like "JetBrains" - instead,
//! external classifiers produce typed features like `process.category.ide = true`.

use std::collections::HashMap;
use std::sync::Arc;
use parking_lot::RwLock;
use serde::{Deserialize, Serialize};
use time::OffsetDateTime;

// ============================================================================
// Feature Value Types
// ============================================================================

/// Typed feature value
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(tag = "type", content = "value")]
pub enum FeatureValue {
    Bool(bool),
    Int(i64),
    Float(f64),
    String(String),
    StringList(Vec<String>),
}

impl FeatureValue {
    /// Get as bool (false if not bool type)
    pub fn as_bool(&self) -> Option<bool> {
        match self {
            FeatureValue::Bool(v) => Some(*v),
            _ => None,
        }
    }

    /// Get as f64 (converts int to float)
    pub fn as_float(&self) -> Option<f64> {
        match self {
            FeatureValue::Float(v) => Some(*v),
            FeatureValue::Int(v) => Some(*v as f64),
            _ => None,
        }
    }

    /// Get as string
    pub fn as_string(&self) -> Option<&str> {
        match self {
            FeatureValue::String(v) => Some(v),
            _ => None,
        }
    }
}

// ============================================================================
// Feature Sample
// ============================================================================

/// A single feature sample with metadata and TTL
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FeatureSample {
    /// Unique feature identifier (e.g., "agent.cpu.usage", "process.category.ide")
    pub feature_id: String,

    /// The feature value
    pub value: FeatureValue,

    /// Confidence in this value (0.0-1.0)
    pub confidence: f32,

    /// Source that produced this feature (e.g., "agent.pc-bureau", "classifier.process")
    pub source: String,

    /// When this sample was recorded
    #[serde(with = "time::serde::iso8601")]
    pub timestamp: OffsetDateTime,

    /// Time-to-live in seconds (0 = never expires)
    pub ttl_seconds: u32,
}

impl FeatureSample {
    /// Check if this sample has expired
    pub fn is_expired(&self) -> bool {
        if self.ttl_seconds == 0 {
            return false; // Never expires
        }
        let age = OffsetDateTime::now_utc() - self.timestamp;
        age.whole_seconds() > self.ttl_seconds as i64
    }

    /// Get age in seconds
    pub fn age_seconds(&self) -> i64 {
        (OffsetDateTime::now_utc() - self.timestamp).whole_seconds()
    }
}

// ============================================================================
// Feature Registry
// ============================================================================

/// Shared type alias for FeatureRegistry
pub type SharedFeatureRegistry = Arc<FeatureRegistry>;

/// In-memory feature store with TTL expiration
pub struct FeatureRegistry {
    /// Feature storage: feature_id -> sample
    features: RwLock<HashMap<String, FeatureSample>>,

    /// Last cleanup timestamp
    last_cleanup: RwLock<OffsetDateTime>,

    /// Cleanup interval in seconds
    cleanup_interval_seconds: u64,
}

impl Default for FeatureRegistry {
    fn default() -> Self {
        Self::new()
    }
}

impl FeatureRegistry {
    /// Create a new empty registry
    pub fn new() -> Self {
        Self {
            features: RwLock::new(HashMap::new()),
            last_cleanup: RwLock::new(OffsetDateTime::now_utc()),
            cleanup_interval_seconds: 60, // Cleanup every minute
        }
    }

    /// Set a feature value
    pub fn set(&self, sample: FeatureSample) {
        self.maybe_cleanup();
        self.features.write().insert(sample.feature_id.clone(), sample);
    }

    /// Set a feature with builder pattern
    pub fn set_feature(
        &self,
        feature_id: &str,
        value: FeatureValue,
        source: &str,
        confidence: f32,
        ttl_seconds: u32,
    ) {
        let sample = FeatureSample {
            feature_id: feature_id.to_string(),
            value,
            confidence,
            source: source.to_string(),
            timestamp: OffsetDateTime::now_utc(),
            ttl_seconds,
        };
        self.set(sample);
    }

    /// Get a feature by ID (returns None if expired or not found)
    pub fn get(&self, feature_id: &str) -> Option<FeatureSample> {
        let features = self.features.read();
        features.get(feature_id).and_then(|sample| {
            if sample.is_expired() {
                None
            } else {
                Some(sample.clone())
            }
        })
    }

    /// Get feature value as bool
    pub fn get_bool(&self, feature_id: &str) -> Option<bool> {
        self.get(feature_id).and_then(|s| s.value.as_bool())
    }

    /// Get feature value as float
    pub fn get_float(&self, feature_id: &str) -> Option<f64> {
        self.get(feature_id).and_then(|s| s.value.as_float())
    }

    /// Get feature value as string
    pub fn get_string(&self, feature_id: &str) -> Option<String> {
        self.get(feature_id).and_then(|s| s.value.as_string().map(|s| s.to_string()))
    }

    /// Get all non-expired features
    pub fn get_all(&self) -> Vec<FeatureSample> {
        self.maybe_cleanup();
        self.features
            .read()
            .values()
            .filter(|s| !s.is_expired())
            .cloned()
            .collect()
    }

    /// Get all features matching a prefix (e.g., "agent." or "process.category.")
    pub fn get_by_prefix(&self, prefix: &str) -> Vec<FeatureSample> {
        self.features
            .read()
            .values()
            .filter(|s| s.feature_id.starts_with(prefix) && !s.is_expired())
            .cloned()
            .collect()
    }

    /// Get feature count (non-expired only)
    pub fn count(&self) -> usize {
        self.features
            .read()
            .values()
            .filter(|s| !s.is_expired())
            .count()
    }

    /// Remove a feature by ID
    pub fn remove(&self, feature_id: &str) -> Option<FeatureSample> {
        self.features.write().remove(feature_id)
    }

    /// Clear all features
    pub fn clear(&self) {
        self.features.write().clear();
    }

    /// Force cleanup of expired features
    pub fn cleanup(&self) -> usize {
        let mut features = self.features.write();
        let initial_count = features.len();
        features.retain(|_, sample| !sample.is_expired());
        let removed = initial_count - features.len();
        *self.last_cleanup.write() = OffsetDateTime::now_utc();
        removed
    }

    /// Maybe run cleanup if interval has passed
    fn maybe_cleanup(&self) {
        let last = *self.last_cleanup.read();
        let elapsed = (OffsetDateTime::now_utc() - last).whole_seconds();
        if elapsed >= self.cleanup_interval_seconds as i64 {
            self.cleanup();
        }
    }

    /// Get summary for debugging
    pub fn summary(&self) -> FeatureRegistrySummary {
        let features = self.features.read();
        let total = features.len();
        let expired = features.values().filter(|s| s.is_expired()).count();
        let by_source: HashMap<String, usize> = features
            .values()
            .filter(|s| !s.is_expired())
            .fold(HashMap::new(), |mut acc, s| {
                *acc.entry(s.source.clone()).or_insert(0) += 1;
                acc
            });

        FeatureRegistrySummary {
            total_count: total,
            active_count: total - expired,
            expired_count: expired,
            by_source,
            last_cleanup: *self.last_cleanup.read(),
        }
    }
}

/// Summary of registry state for debugging
#[derive(Debug, Clone, Serialize)]
pub struct FeatureRegistrySummary {
    pub total_count: usize,
    pub active_count: usize,
    pub expired_count: usize,
    pub by_source: HashMap<String, usize>,
    #[serde(with = "time::serde::iso8601")]
    pub last_cleanup: OffsetDateTime,
}

// ============================================================================
// Feature ID Constants
// ============================================================================

/// Well-known feature IDs
pub mod feature_ids {
    // Agent features
    pub const AGENT_ONLINE: &str = "agent.online";
    pub const AGENT_IDLE_SECONDS: &str = "agent.idle.seconds";
    pub const AGENT_CPU_USAGE: &str = "agent.cpu.usage";
    pub const AGENT_MEMORY_USAGE: &str = "agent.memory.usage";

    // Process category features (from classifier)
    pub const PROCESS_CATEGORY_IDE: &str = "process.category.ide";
    pub const PROCESS_CATEGORY_BROWSER: &str = "process.category.browser";
    pub const PROCESS_CATEGORY_MEDIA: &str = "process.category.media";
    pub const PROCESS_CATEGORY_COMMUNICATION: &str = "process.category.communication";
    pub const PROCESS_CATEGORY_GAMING: &str = "process.category.gaming";

    // Network features
    pub const NET_SSID: &str = "net.ssid";
    pub const NET_CONNECTED: &str = "net.connected";

    // Environment features
    pub const ENV_TEMPERATURE: &str = "env.temperature";
    pub const ENV_HUMIDITY: &str = "env.humidity";

    // Time features (computed by kernel)
    pub const TIME_HOUR: &str = "time.hour";
    pub const TIME_DAY_OF_WEEK: &str = "time.day_of_week";
    pub const TIME_IS_WEEKEND: &str = "time.is_weekend";
    pub const TIME_IS_BUSINESS_HOURS: &str = "time.is_business_hours";

    // Context features
    pub const CONTEXT_MODE: &str = "context.mode";
    pub const CONTEXT_TIME_IN_MODE: &str = "context.time_in_mode";
}

// ============================================================================
// Default TTL Values
// ============================================================================

/// Default TTL values for different feature types
pub mod ttl {
    /// Agent heartbeat features (expire quickly if no update)
    pub const AGENT: u32 = 120; // 2 minutes

    /// Process classification (refresh with each heartbeat)
    pub const PROCESS: u32 = 120; // 2 minutes

    /// Network features (might change less frequently)
    pub const NETWORK: u32 = 300; // 5 minutes

    /// Environment features (sensors send every 30s typically)
    pub const ENVIRONMENT: u32 = 180; // 3 minutes

    /// Time features (recomputed frequently, short TTL)
    pub const TIME: u32 = 60; // 1 minute

    /// Context features (mode changes are event-driven)
    pub const CONTEXT: u32 = 0; // Never expires (updated on change)
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_feature_value_types() {
        let bool_val = FeatureValue::Bool(true);
        assert_eq!(bool_val.as_bool(), Some(true));
        assert_eq!(bool_val.as_float(), None);

        let float_val = FeatureValue::Float(3.14);
        assert_eq!(float_val.as_float(), Some(3.14));
        assert_eq!(float_val.as_bool(), None);

        let int_val = FeatureValue::Int(42);
        assert_eq!(int_val.as_float(), Some(42.0)); // Int converts to float
    }

    #[test]
    fn test_feature_registry_basic() {
        let registry = FeatureRegistry::new();

        // Set a feature
        registry.set_feature(
            "test.feature",
            FeatureValue::Bool(true),
            "test",
            1.0,
            60,
        );

        // Get it back
        let sample = registry.get("test.feature").unwrap();
        assert_eq!(sample.value.as_bool(), Some(true));
        assert_eq!(sample.source, "test");

        // Count
        assert_eq!(registry.count(), 1);

        // Remove
        registry.remove("test.feature");
        assert!(registry.get("test.feature").is_none());
    }

    #[test]
    fn test_feature_expiration() {
        let sample = FeatureSample {
            feature_id: "test".to_string(),
            value: FeatureValue::Bool(true),
            confidence: 1.0,
            source: "test".to_string(),
            timestamp: OffsetDateTime::now_utc() - time::Duration::seconds(120),
            ttl_seconds: 60, // 60 second TTL, but 120 seconds old
        };

        assert!(sample.is_expired());

        // Sample with 0 TTL never expires
        let never_expires = FeatureSample {
            ttl_seconds: 0,
            ..sample.clone()
        };
        assert!(!never_expires.is_expired());
    }

    #[test]
    fn test_get_by_prefix() {
        let registry = FeatureRegistry::new();

        registry.set_feature("agent.cpu", FeatureValue::Float(50.0), "agent", 1.0, 0);
        registry.set_feature("agent.memory", FeatureValue::Float(60.0), "agent", 1.0, 0);
        registry.set_feature("env.temp", FeatureValue::Float(22.0), "sensor", 1.0, 0);

        let agent_features = registry.get_by_prefix("agent.");
        assert_eq!(agent_features.len(), 2);

        let env_features = registry.get_by_prefix("env.");
        assert_eq!(env_features.len(), 1);
    }

    #[test]
    fn test_cleanup() {
        let registry = FeatureRegistry::new();

        // Add expired feature
        let expired = FeatureSample {
            feature_id: "expired".to_string(),
            value: FeatureValue::Bool(true),
            confidence: 1.0,
            source: "test".to_string(),
            timestamp: OffsetDateTime::now_utc() - time::Duration::seconds(120),
            ttl_seconds: 60,
        };
        registry.set(expired);

        // Add valid feature
        registry.set_feature("valid", FeatureValue::Bool(true), "test", 1.0, 300);

        // Before cleanup, both exist in storage
        assert_eq!(registry.features.read().len(), 2);

        // Cleanup removes expired
        let removed = registry.cleanup();
        assert_eq!(removed, 1);
        assert_eq!(registry.features.read().len(), 1);
    }
}
