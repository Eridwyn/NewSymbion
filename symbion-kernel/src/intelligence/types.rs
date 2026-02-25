//! Intelligence Engine Types
//!
//! Common data structures used across the intelligence system.

use serde::{Deserialize, Serialize};
use time::OffsetDateTime;
use utoipa::ToSchema;

// ============================================================================
// Signal Collection
// ============================================================================

/// Snapshot of all contextual signals at a point in time
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct ContextSignals {
    // Temporal
    pub hour: u8,                    // 0-23
    pub day_of_week: u8,             // 0-6 (Mon-Sun) - Monday-based indexing
    pub is_weekend: bool,
    pub is_holiday: bool,            // Future: API for holidays

    // Agent activity
    pub agent_online: bool,          // True if user agent is connected
    pub agent_idle_seconds: u64,     // Seconds since last activity
    pub cpu_usage: f32,              // 0-100
    pub active_processes: Vec<String>, // Top running processes
    pub is_screen_locked: bool,      // Future: Agent capability

    // Environment
    pub temperature: Option<f32>,
    pub humidity: Option<f32>,

    // Current context
    pub current_mode: String,
    pub time_in_current_mode_minutes: i64,
    #[serde(with = "time::serde::iso8601::option")]
    #[schema(value_type = Option<String>)]
    pub last_manual_change: Option<OffsetDateTime>,
}

// ============================================================================
// Prediction
// ============================================================================

/// Result of a mode prediction
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct ModePrediction {
    pub mode: String,               // Mode slug prédit (ou "unknown" si incertain)
    pub confidence: f32,            // 0.0 - 1.0 (normalisé)
    pub reasons: Vec<String>,       // Human-readable explanations
    pub contributing_factors: Vec<(String, f32)>, // (signal_name, weight)
    /// Top 3 modes avec scores normalisés (évite explosion combinatoire UI)
    #[serde(default)]
    pub top_modes: Vec<(String, f32)>,
    /// True si confiance globale trop faible pour prédire
    #[serde(default)]
    pub is_uncertain: bool,
}

/// Single prediction from one signal source
#[derive(Debug, Clone)]
pub struct SinglePrediction {
    pub mode: String,
    pub confidence: f32,
    pub reason: String,
}

// ============================================================================
// Learning
// ============================================================================

/// A pattern learned from user behavior
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct LearnedPattern {
    pub mode: String,
    pub day_of_week: u8,
    pub hour: u8,
    pub confidence: f32,
    pub occurrences: u32,
    #[serde(with = "time::serde::iso8601")]
    #[schema(value_type = String)]
    pub last_seen: OffsetDateTime,
    pub source: PatternSource,
}

/// Where a pattern came from
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, ToSchema)]
pub enum PatternSource {
    /// Detected from history analysis
    Historical,
    /// Learned from user correction
    UserCorrection,
    /// Imported from existing automation
    Automation,
}

/// Record of a prediction for learning purposes
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct PredictionRecord {
    #[serde(with = "time::serde::iso8601")]
    #[schema(value_type = String)]
    pub timestamp: OffsetDateTime,
    pub predicted_mode: String,
    pub actual_mode: Option<String>,  // Set when user corrects
    pub confidence: f32,
    pub was_correct: Option<bool>,
    /// Source of outcome (v1.1.9): auto_applied, suggestion, ignored
    #[serde(default)]
    pub outcome_source: Option<PredictionOutcome>,
}

/// How a prediction was handled (v1.1.9)
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, ToSchema)]
pub enum PredictionOutcome {
    /// Auto-applied (high confidence + established)
    AutoApplied,
    /// Suggestion sent (push or silent)
    Suggestion,
    /// Ignored (confidence too low)
    Ignored,
}

/// User feedback on a prediction
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct UserFeedback {
    #[serde(with = "time::serde::iso8601")]
    #[schema(value_type = String)]
    pub timestamp: OffsetDateTime,
    pub predicted_mode: String,
    pub actual_mode: String,       // What the user chose
    pub signals_snapshot: ContextSignals,
    pub was_correction: bool,      // true if different from prediction
}

/// Pattern with computed decay for export/debug (v1.1.9).
/// Note: `decayed_confidence` is a time-dependent snapshot computed at export time.
/// The same pattern exported at different times will have different decayed values.
/// Use `days_since_seen` for reproducible comparisons.
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct PatternExport {
    pub mode: String,
    pub day_of_week: u8,
    pub hour: u8,
    pub confidence: f32,
    pub decayed_confidence: f32,
    pub occurrences: u32,
    #[serde(with = "time::serde::iso8601")]
    #[schema(value_type = String)]
    pub last_seen: OffsetDateTime,
    pub source: PatternSource,
    pub days_since_seen: u32,
}

// ============================================================================
// Decision Engine Feedback
// ============================================================================

/// Signal type from Decision Engine outcomes
/// Used to provide feedback to Intelligence from automated decisions
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum DecisionSignal {
    /// Action approved automatically (high trust) → Strong positive reinforcement
    ApprovedAuto,
    /// Action approved after MFA validation → Weak positive (needed human confirmation)
    ApprovedMFA,
    /// Action denied by user (MFA refused) → Strong negative signal
    Denied,
    /// MFA validation expired (user didn't respond) → Ambiguous, no learning
    Expired,
    /// Action blocked by guards (context changed, expired, etc.) → Context was invalid
    Blocked,
}

// ============================================================================
// Drift Detection
// ============================================================================

/// Detected change in user habits
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct HabitDrift {
    pub mode: String,
    pub day_of_week: u8,
    pub old_hour: u8,
    pub new_hour: u8,
    pub shift_hours: i8,
    pub suggestion: String,
}

// ============================================================================
// Status
// ============================================================================

/// Current status of the intelligence engine
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct IntelligenceStatus {
    pub enabled: bool,
    pub config: super::config::IntelligenceConfig,
    pub learned_patterns_count: usize,
    pub auto_created_automations: usize,
    pub last_prediction: Option<ModePrediction>,
    pub accuracy_last_7_days: f32,
}

/// Health counters for observability (v1.1.9)
#[derive(Debug, Clone, Default, Serialize, Deserialize, ToSchema)]
pub struct HealthCounters {
    #[schema(value_type = Option<String>)]
    pub date: Option<time::Date>,
    pub push_sent: u32,
    pub suggestions_generated: u32,
    pub auto_applied: u32,
    pub denied: u32,
    // Purge tracking (P0.5)
    #[serde(with = "time::serde::iso8601::option", default)]
    #[schema(value_type = Option<String>)]
    pub purge_last_run_at: Option<OffsetDateTime>,
    pub purge_removed_count_last_run: u32,
}

/// Detailed accuracy stats with denominators (v1.1.9 P0 fix)
#[derive(Debug, Clone, Default, Serialize, Deserialize, ToSchema)]
pub struct AccuracyStats {
    /// Total predictions made in period
    pub predictions_total: u32,
    /// Predictions that received feedback (was_correct is Some)
    pub predictions_scored: u32,
    /// Predictions that were auto-applied
    pub predictions_auto_applied: u32,
    /// Predictions with user feedback (manual correction or approval)
    pub predictions_user_feedback: u32,
    /// Predictions ignored (confidence < suggestion_threshold)
    pub predictions_ignored: u32,
    /// Correct predictions out of scored
    pub correct_count: u32,
    /// Accuracy strict: correct / total (None if < 20 predictions = unreliable)
    pub accuracy_strict: Option<f32>,
    /// Accuracy feedback-only: correct / scored (None if < 20 predictions)
    pub accuracy_feedback: Option<f32>,
    /// Warning if sample size is too small
    pub warning: Option<String>,
    /// Minimum sample size for reliable accuracy (20)
    pub min_sample_size: u32,
}

// ============================================================================
// Shadow Mode Statistics (v2 Stabilization)
// ============================================================================

/// Statistics for v1 vs v2 shadow mode comparison
#[derive(Debug, Clone, Default, Serialize, Deserialize, ToSchema)]
pub struct ShadowStats {
    /// When tracking started
    #[serde(with = "time::serde::iso8601::option", default)]
    #[schema(value_type = Option<String>)]
    pub tracking_since: Option<OffsetDateTime>,

    /// Total predictions compared
    pub total_comparisons: u32,

    /// Times v1 and v2 agreed
    pub agreements: u32,

    /// Times v1 and v2 disagreed
    pub disagreements: u32,

    /// Agreement rate (0.0-1.0)
    pub agreement_rate: f32,

    /// Last 24h specific stats
    pub last_24h: ShadowPeriodStats,

    /// v2 would-apply count (when v2 met auto-apply criteria)
    pub v2_would_apply_count: u32,

    /// v2 blocked count (when v2 prediction was blocked by guards)
    pub v2_blocked_count: u32,

    /// Blocked reasons histogram
    pub blocked_reasons: std::collections::HashMap<String, u32>,

    /// Last comparison timestamp
    #[serde(with = "time::serde::iso8601::option", default)]
    #[schema(value_type = Option<String>)]
    pub last_comparison_at: Option<OffsetDateTime>,
}

/// Period-specific shadow stats
#[derive(Debug, Clone, Default, Serialize, Deserialize, ToSchema)]
pub struct ShadowPeriodStats {
    pub comparisons: u32,
    pub agreements: u32,
    pub disagreements: u32,
    pub agreement_rate: f32,
}

/// Detailed prediction log entry for auditability
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct PredictionLogEntry {
    /// Unique trace ID
    pub trace_id: String,

    /// Timestamp
    #[serde(with = "time::serde::iso8601")]
    #[schema(value_type = String)]
    pub timestamp: OffsetDateTime,

    /// Summary of context vector (top dimensions)
    pub vector_summary: VectorSummary,

    /// v2 Prediction result
    pub prediction: PredictionSummary,

    /// Current session state
    pub session_state: SessionSummary,

    /// Auto-apply guard result
    pub auto_apply_result: AutoApplyResult,
}

/// Compact vector summary for logging
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct VectorSummary {
    pub top_dimensions: Vec<(String, f32)>,
    pub top_features: Vec<String>,
    pub feature_count: usize,
}

/// Compact prediction summary for logging
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct PredictionSummary {
    pub mode: String,
    pub confidence: f32,
    pub samples_used: usize,
    pub is_confident: bool,
    pub alternatives: Vec<(String, f32)>,
}

/// Session state summary for logging
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct SessionSummary {
    pub current_mode: String,
    pub time_in_mode_minutes: i64,
    pub is_override_active: bool,
}

/// Auto-apply result for logging
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct AutoApplyResult {
    pub allowed: bool,
    pub blocked_reason: Option<String>,
    pub would_change_mode: bool,
}

// ============================================================================
// Helper Functions
// ============================================================================

/// Adaptive modifier with diminishing returns at extremes (v1.1.9)
///
/// Prevents oscillations by damping changes when confidence is already extreme.
/// Examples:
/// - current=0.50 → damping=1.0  → +0.30 * 1.0 = +0.30 (full effect)
/// - current=0.70 → damping=0.6  → +0.30 * 0.6 = +0.18
/// - current=0.90 → damping=0.3  → +0.30 * 0.3 = +0.09 (minimum)
pub fn adaptive_modifier(base: f32, current: f32) -> f32 {
    // Distance from center (0.5)
    let distance = (current - 0.5).abs();  // 0.0 to 0.45
    // Damping: near center = full effect, extremes = attenuated (min 0.3)
    let damping = (1.0 - distance * 2.0).max(0.3);
    base * damping
}

/// Convert day number to French name
/// Uses Monday-based indexing: 0=Lundi, 6=Dimanche (matches number_from_monday() - 1)
pub fn day_name(day: u8) -> &'static str {
    match day {
        0 => "lundi",
        1 => "mardi",
        2 => "mercredi",
        3 => "jeudi",
        4 => "vendredi",
        5 => "samedi",
        6 => "dimanche",
        _ => "inconnu",
    }
}

/// Get display name for a mode
pub fn mode_display_name(mode: &str) -> String {
    match mode {
        "pro" | "cravate" => "Professionnel".to_string(),
        "focus" => "Focus".to_string(),
        "maison" | "intime" => "Maison".to_string(),
        "veille" | "neutre" => "Veille".to_string(),
        // Modes custom: capitaliser la première lettre
        other => {
            let mut chars = other.chars();
            match chars.next() {
                Some(c) => c.to_uppercase().chain(chars).collect(),
                None => "Inconnu".to_string(),
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_pattern_source_serialization() {
        let source = PatternSource::UserCorrection;
        let json = serde_json::to_string(&source).unwrap();
        assert!(json.contains("UserCorrection"));

        let parsed: PatternSource = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed, PatternSource::UserCorrection);
    }

    #[test]
    fn test_adaptive_modifier() {
        // Center: full effect
        assert!((adaptive_modifier(0.3, 0.5) - 0.3).abs() < 0.01);

        // Near extreme: attenuated
        let at_90 = adaptive_modifier(0.3, 0.9);
        assert!(at_90 < 0.15); // Should be damped

        // Near zero: also attenuated
        let at_10 = adaptive_modifier(0.3, 0.1);
        assert!(at_10 < 0.15);
    }

    #[test]
    fn test_day_name() {
        assert_eq!(day_name(0), "lundi");
        assert_eq!(day_name(6), "dimanche");
        assert_eq!(day_name(7), "inconnu");
    }
}
