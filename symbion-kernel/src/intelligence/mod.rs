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

// Re-export commonly used types at module level
pub use config::{IntelligenceConfig, SignalWeights};
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
