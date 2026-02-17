//! Context Intelligence Engine
//!
//! Intelligent autonomous context adaptation system.
//!
//! ## Architecture
//!
//! The intelligence engine follows a pipeline architecture:
//! ```text
//! Signal → Features → Vector → Inference → Sessions → Decision
//! ```
//!
//! ## Modules
//!
//! - `config`: Configuration parameters and signal weights
//! - `types`: Common data structures (signals, predictions, patterns)
//! - `features`: FeatureRegistry with TTL-based expiration
//! - `vector`: ContextVector with normalized dimensions and explainability
//! - `inference`: Case-based inference with weighted voting
//!
//! ## Features
//!
//! - Multi-signal collection (time, agent activity, environment)
//! - Pattern learning from user behavior
//! - Mode prediction with confidence scores
//! - Feedback loop for continuous improvement
//! - Auto-creation of automations from learned patterns
//! - Drift detection and adaptation

pub mod config;
pub mod types;
pub mod features;
pub mod vector;
pub mod inference;
pub mod sessions;
pub mod classifier;
pub mod bootstrap;

// Re-export commonly used types at module level
pub use config::{IntelligenceConfig, SignalWeights, V2StabilizationConfig};
pub use types::{
    AccuracyStats,
    ContextSignals,
    DecisionSignal,
    HabitDrift,
    HealthCounters,
    IntelligenceStatus,
    LearnedPattern,
    ModePrediction,
    PatternExport,
    PatternSource,
    PredictionOutcome,
    PredictionRecord,
    SinglePrediction,
    UserFeedback,
    adaptive_modifier,
    day_name,
    mode_display_name,
    // v2 stabilization types
    ShadowStats,
    ShadowPeriodStats,
    PredictionLogEntry,
    VectorSummary,
    PredictionSummary,
    SessionSummary,
    AutoApplyResult,
};

// Re-export feature types
pub use features::{
    FeatureRegistry,
    FeatureRegistrySummary,
    FeatureSample,
    FeatureValue,
    SharedFeatureRegistry,
    feature_ids,
    ttl,
};

// Re-export vector types
pub use vector::{
    ContextVector,
    VectorBuilder,
    WhyItem,
    dimensions,
};

// Re-export inference types
pub use inference::{
    InferenceEngine,
    InferenceConfig,
    InferenceStats,
    SharedInferenceEngine,
    TrainingSample,
    SampleSource,
    SampleVector,
    PredictionV2,
    ModeScore,
    PredictionReason,
    // v2 stabilization types
    SampleStats,
    AutoApplyGuard,
    GuardChecks,
};

// Re-export session types
pub use sessions::{
    SessionManager,
    SharedSessionManager,
    SessionConfig,
    SessionSource,
    SessionStats,
    ActiveSession,
    TransitionDecision,
};

// Re-export classifier types
pub use classifier::{
    ProcessClassifier,
    SharedProcessClassifier,
    ClassifierConfig,
    ClassificationResult,
};

// Re-export bootstrap types
pub use bootstrap::{
    BootstrapScheduler,
    BootstrapConfig,
    IntelligenceMode,
};

// ============================================================================
// Shared Timezone Helper
// ============================================================================

use std::sync::OnceLock;
use time::OffsetDateTime;
use time_tz::{timezones, OffsetDateTimeExt, Tz};

/// Cached timezone from SYMBION_TIMEZONE env var (default: Europe/Paris)
static SYMBION_TZ: OnceLock<&'static Tz> = OnceLock::new();

/// Get the configured timezone
fn get_timezone() -> &'static Tz {
    SYMBION_TZ.get_or_init(|| {
        let tz_name = std::env::var("SYMBION_TIMEZONE").unwrap_or_else(|_| "Europe/Paris".to_string());
        match timezones::get_by_name(&tz_name) {
            Some(tz) => {
                eprintln!("[timezone] Using timezone: {}", tz_name);
                tz
            }
            None => {
                eprintln!("[timezone] Unknown timezone '{}', falling back to Europe/Paris", tz_name);
                timezones::db::europe::PARIS
            }
        }
    })
}

/// Get current time in the configured local timezone
pub fn local_now() -> OffsetDateTime {
    OffsetDateTime::now_utc().to_timezone(get_timezone())
}
