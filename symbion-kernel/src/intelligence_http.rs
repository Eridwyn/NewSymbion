/**
 * SYMBION KERNEL - Intelligence API Endpoints
 *
 * ROLE: REST API for Context Intelligence Engine
 *
 * ENDPOINTS:
 * - GET  /v1/intelligence/status   - Current engine status
 * - GET  /v1/intelligence/health   - Health counters (24h) (v1.1.9)
 * - GET  /v1/intelligence/patterns - Learned patterns
 * - GET  /v1/intelligence/predictions - Prediction history
 * - GET  /v1/intelligence/signals  - Current context signals
 * - GET  /v1/intelligence/features    - FeatureRegistry state (v2)
 * - GET  /v1/intelligence/vector      - ContextVector with why-chain (v2)
 * - GET  /v1/intelligence/prediction2 - Case-based inference prediction (v2)
 * - POST /v1/intelligence/feedback    - Record user feedback
 * - PUT  /v1/intelligence/config   - Update configuration
 */

use axum::{
    extract::State,
    routing::{get, post, put},
    Json, Router,
};
use serde::{Deserialize, Serialize};

use axum::extract::Query;
use crate::http::AppState;
use crate::context_intelligence::{
    AccuracyStats, ContextSignals, HealthCounters, IntelligenceConfig, IntelligenceStatus,
    LearnedPattern, ModePrediction, PredictionRecord, PatternExport,
};
use crate::intelligence::{
    FeatureSample, FeatureRegistrySummary, ContextVector, VectorBuilder,
    PredictionV2, InferenceStats, ShadowStats, SampleStats, V2StabilizationConfig,
};

// ============================================================================
// Response Types
// ============================================================================

#[derive(Serialize)]
pub struct IntelligenceStatusResponse {
    pub status: IntelligenceStatus,
    pub current_prediction: Option<ModePrediction>,
}

#[derive(Serialize)]
pub struct PatternsResponse {
    pub patterns: Vec<LearnedPattern>,
    pub count: usize,
}

#[derive(Serialize)]
pub struct PredictionsResponse {
    pub predictions: Vec<PredictionRecord>,
    pub count: usize,
    pub accuracy_7_days: f32,
}

#[derive(Serialize)]
pub struct SignalsResponse {
    pub signals: ContextSignals,
    pub prediction: ModePrediction,
}

#[derive(Deserialize)]
pub struct FeedbackRequest {
    pub chosen_mode: String,
}

#[derive(Serialize)]
pub struct FeedbackResponse {
    pub success: bool,
    pub message: String,
}

#[derive(Serialize)]
pub struct ConfigResponse {
    pub config: IntelligenceConfig,
}

/// Query params for pattern export (v1.1.9)
#[derive(Deserialize, Default)]
pub struct PatternExportQuery {
    /// Filter by mode (e.g., "pro", "maison")
    pub mode: Option<String>,
    /// Filter by day of week (0=Mon, 6=Sun)
    pub day: Option<u8>,
    /// Minimum confidence filter
    pub min_confidence: Option<f32>,
    /// Sort by: "confidence" (default), "occurrences", "last_seen"
    pub sort_by: Option<String>,
    /// Limit results (default: all)
    pub limit: Option<usize>,
}

#[derive(Serialize)]
pub struct PatternExportResponse {
    pub patterns: Vec<PatternExport>,
    pub count: usize,
    pub filters_applied: Vec<String>,
}

/// Health counters response (v1.1.9)
#[derive(Serialize)]
pub struct HealthResponse {
    pub counters: HealthCounters,
    /// Detailed accuracy with denominators (v1.1.9 P0 fix)
    pub accuracy: AccuracyStats,
    pub patterns_active: usize,
    pub patterns_established: usize,
}

/// Features response (v2 Intelligence)
#[derive(Serialize)]
pub struct FeaturesResponse {
    pub features: Vec<FeatureSample>,
    pub summary: FeatureRegistrySummary,
}

/// Vector response (v2 Intelligence)
#[derive(Serialize)]
pub struct VectorResponse {
    pub vector: ContextVector,
    pub best_mode: String,
    pub best_mode_confidence: f32,
    pub has_sufficient_data: bool,
}

/// Prediction v2 response (case-based inference)
#[derive(Serialize)]
pub struct Prediction2Response {
    pub prediction: PredictionV2,
    pub vector: ContextVector,
    pub stats: InferenceStats,
}

/// Shadow stats response (v2 stabilization)
#[derive(Serialize)]
pub struct ShadowStatsResponse {
    pub shadow_stats: ShadowStats,
    pub sample_stats: SampleStats,
    pub v2_config: V2StabilizationConfig,
    /// Days since shadow mode started
    pub observation_days: i64,
}

// ============================================================================
// Handlers
// ============================================================================

/// GET /v1/intelligence/status
/// Returns current status of the intelligence engine
async fn get_status(State(app): State<AppState>) -> Json<IntelligenceStatusResponse> {
    let status = app.context_intelligence.get_status();
    let current_prediction = status.last_prediction.clone();

    Json(IntelligenceStatusResponse {
        status,
        current_prediction,
    })
}

/// GET /v1/intelligence/health
/// Returns health counters (24h) and detailed accuracy metrics (v1.1.9)
async fn get_health(State(app): State<AppState>) -> Json<HealthResponse> {
    let counters = app.context_intelligence.get_health_counters();
    let accuracy = app.context_intelligence.calculate_accuracy_detailed(7);
    let patterns = app.context_intelligence.get_patterns();
    let config = app.context_intelligence.get_config();

    // Count established patterns (occurrences >= min_pattern_occurrences)
    let patterns_established = patterns.iter()
        .filter(|p| p.occurrences >= config.min_pattern_occurrences)
        .count();

    Json(HealthResponse {
        counters,
        accuracy,
        patterns_active: patterns.len(),
        patterns_established,
    })
}

/// GET /v1/intelligence/patterns
/// Returns all learned patterns
async fn get_patterns(State(app): State<AppState>) -> Json<PatternsResponse> {
    let patterns = app.context_intelligence.get_patterns();
    let count = patterns.len();

    Json(PatternsResponse { patterns, count })
}

/// GET /v1/intelligence/predictions
/// Returns prediction history with accuracy stats
async fn get_predictions(State(app): State<AppState>) -> Json<PredictionsResponse> {
    let predictions = app.context_intelligence.get_prediction_history();
    let count = predictions.len();
    let accuracy = app.context_intelligence.calculate_accuracy(7);

    Json(PredictionsResponse {
        predictions,
        count,
        accuracy_7_days: accuracy,
    })
}

/// GET /v1/intelligence/signals
/// Returns current context signals and prediction
async fn get_signals(State(app): State<AppState>) -> Json<SignalsResponse> {
    let signals = app.context_intelligence.collect_signals().await;
    let prediction = app.context_intelligence.predict_mode(&signals);

    Json(SignalsResponse {
        signals,
        prediction,
    })
}

/// POST /v1/intelligence/feedback
/// Record user feedback when manually changing mode
async fn post_feedback(
    State(app): State<AppState>,
    Json(req): Json<FeedbackRequest>,
) -> Json<FeedbackResponse> {
    // Collect current signals for the feedback record
    let signals = app.context_intelligence.collect_signals().await;

    // Record the feedback
    app.context_intelligence.record_feedback(&req.chosen_mode, signals);

    Json(FeedbackResponse {
        success: true,
        message: format!("Feedback recorded for mode '{}'", req.chosen_mode),
    })
}

/// GET /v1/intelligence/config
/// Returns current configuration
async fn get_config(State(app): State<AppState>) -> Json<ConfigResponse> {
    let config = app.context_intelligence.get_config();
    Json(ConfigResponse { config })
}

/// PUT /v1/intelligence/config
/// Update intelligence configuration with validation (v1.1.9 security)
async fn put_config(
    State(app): State<AppState>,
    Json(requested): Json<IntelligenceConfig>,
) -> Json<ConfigUpdateResponse> {
    // Get previous config for diff and rollback
    let previous = app.context_intelligence.get_config();

    // Clone requested to track what was clamped
    let mut config = requested.clone();
    let mut clamped_fields = Vec::new();

    // Validate and clamp values to safe ranges, tracking clamps
    macro_rules! clamp_field {
        ($field:ident, $min:expr, $max:expr) => {
            let clamped = config.$field.clamp($min, $max);
            if clamped != requested.$field {
                clamped_fields.push(format!(
                    "{}: {} → {} (range {}-{})",
                    stringify!($field), requested.$field, clamped, $min, $max
                ));
            }
            config.$field = clamped;
        };
    }

    clamp_field!(auto_apply_threshold, 0.50, 0.95);
    clamp_field!(suggestion_threshold, 0.20, 0.80);
    clamp_field!(min_pattern_occurrences, 1, 10);
    clamp_field!(check_interval_seconds, 10, 300);
    clamp_field!(max_push_per_day, 0, 50);
    clamp_field!(suggestion_cooldown_minutes, 5, 180);
    clamp_field!(purge_threshold_days, 30, 365);
    clamp_field!(quiet_hours_start, 0, 23);
    clamp_field!(quiet_hours_end, 0, 23);

    // Clamp decay coefficients
    for (i, coeff) in config.decay_coefficients.iter_mut().enumerate() {
        let clamped = coeff.clamp(0.1, 1.0);
        if clamped != requested.decay_coefficients[i] {
            clamped_fields.push(format!(
                "decay_coefficients[{}]: {} → {} (range 0.1-1.0)",
                i, requested.decay_coefficients[i], clamped
            ));
        }
        *coeff = clamped;
    }

    // Log clamped fields as warning
    if !clamped_fields.is_empty() {
        eprintln!(
            "[intelligence] ⚠️ Config clamped: {}",
            clamped_fields.join(", ")
        );
    }

    // Log the diff
    let changes = diff_config(&previous, &config);
    if !changes.is_empty() {
        eprintln!(
            "[intelligence] 🔧 Config updated: {}",
            changes.join(", ")
        );
    }

    // Update config
    app.context_intelligence.update_config(config.clone());

    Json(ConfigUpdateResponse {
        config,
        previous_config: previous,
        changes_applied: changes,
        clamped_fields,
        timestamp: time::OffsetDateTime::now_utc().to_string(),
    })
}

/// Response for config update with audit trail
#[derive(Serialize)]
pub struct ConfigUpdateResponse {
    pub config: IntelligenceConfig,
    pub previous_config: IntelligenceConfig,
    pub changes_applied: Vec<String>,
    /// Fields that were clamped to safe ranges (empty if all values were valid)
    pub clamped_fields: Vec<String>,
    pub timestamp: String,
}

/// Generate diff between two configs
fn diff_config(old: &IntelligenceConfig, new: &IntelligenceConfig) -> Vec<String> {
    let mut changes = Vec::new();

    if old.auto_apply_threshold != new.auto_apply_threshold {
        changes.push(format!("auto_apply_threshold: {:.2}→{:.2}", old.auto_apply_threshold, new.auto_apply_threshold));
    }
    if old.suggestion_threshold != new.suggestion_threshold {
        changes.push(format!("suggestion_threshold: {:.2}→{:.2}", old.suggestion_threshold, new.suggestion_threshold));
    }
    if old.max_push_per_day != new.max_push_per_day {
        changes.push(format!("max_push_per_day: {}→{}", old.max_push_per_day, new.max_push_per_day));
    }
    if old.suggestion_cooldown_minutes != new.suggestion_cooldown_minutes {
        changes.push(format!("cooldown: {}→{} min", old.suggestion_cooldown_minutes, new.suggestion_cooldown_minutes));
    }
    if old.purge_threshold_days != new.purge_threshold_days {
        changes.push(format!("purge_days: {}→{}", old.purge_threshold_days, new.purge_threshold_days));
    }
    if old.quiet_hours_start != new.quiet_hours_start || old.quiet_hours_end != new.quiet_hours_end {
        changes.push(format!("quiet_hours: {}h-{}h→{}h-{}h",
            old.quiet_hours_start, old.quiet_hours_end,
            new.quiet_hours_start, new.quiet_hours_end));
    }
    if old.decay_coefficients != new.decay_coefficients {
        changes.push(format!("decay_coefficients: {:?}→{:?}", old.decay_coefficients, new.decay_coefficients));
    }

    changes
}

/// GET /v1/intelligence/patterns/export
/// Export patterns with decay calculation and filtering (v1.1.9)
async fn get_patterns_export(
    State(app): State<AppState>,
    Query(query): Query<PatternExportQuery>,
) -> Json<PatternExportResponse> {
    let mut patterns = app.context_intelligence.get_patterns_with_decay();
    let mut filters = Vec::new();

    // Apply filters
    if let Some(ref mode) = query.mode {
        patterns.retain(|p| p.mode.eq_ignore_ascii_case(mode));
        filters.push(format!("mode={}", mode));
    }

    if let Some(day) = query.day {
        patterns.retain(|p| p.day_of_week == day);
        filters.push(format!("day={}", day));
    }

    if let Some(min_conf) = query.min_confidence {
        patterns.retain(|p| p.decayed_confidence >= min_conf);
        filters.push(format!("min_confidence={:.2}", min_conf));
    }

    // Sort
    let sort_by = query.sort_by.as_deref().unwrap_or("confidence");
    match sort_by {
        "occurrences" => patterns.sort_by(|a, b| b.occurrences.cmp(&a.occurrences)),
        "last_seen" => patterns.sort_by(|a, b| b.last_seen.cmp(&a.last_seen)),
        _ => patterns.sort_by(|a, b| b.decayed_confidence.partial_cmp(&a.decayed_confidence).unwrap_or(std::cmp::Ordering::Equal)),
    }
    if sort_by != "confidence" {
        filters.push(format!("sort_by={}", sort_by));
    }

    // Limit
    if let Some(limit) = query.limit {
        patterns.truncate(limit);
        filters.push(format!("limit={}", limit));
    }

    let count = patterns.len();
    Json(PatternExportResponse {
        patterns,
        count,
        filters_applied: filters,
    })
}

/// GET /v1/intelligence/features
/// Returns current features from FeatureRegistry (v2 Intelligence)
async fn get_features(State(app): State<AppState>) -> Json<FeaturesResponse> {
    let features = app.feature_registry.get_all();
    let summary = app.feature_registry.summary();

    Json(FeaturesResponse { features, summary })
}

/// GET /v1/intelligence/vector
/// Returns current ContextVector built from features (v2 Intelligence)
async fn get_vector(State(app): State<AppState>) -> Json<VectorResponse> {
    let vector = VectorBuilder::new(&app.feature_registry).build();
    let (best_mode, confidence) = vector.best_mode();
    let best_mode_str = best_mode.to_string();
    let has_sufficient_data = vector.has_sufficient_data();

    Json(VectorResponse {
        vector,
        best_mode: best_mode_str,
        best_mode_confidence: confidence,
        has_sufficient_data,
    })
}

/// GET /v1/intelligence/prediction2
/// Returns v2 prediction using case-based inference
async fn get_prediction2(State(app): State<AppState>) -> Json<Prediction2Response> {
    // Build current context vector
    let vector = VectorBuilder::new(&app.feature_registry).build();

    // Get prediction from inference engine
    let prediction = app.inference_engine.predict(&vector);

    // Get engine stats
    let stats = app.inference_engine.stats();

    Json(Prediction2Response {
        prediction,
        vector,
        stats,
    })
}

/// Session response (v2 Intelligence)
#[derive(Serialize)]
pub struct SessionResponse {
    pub session: crate::intelligence::ActiveSession,
    pub stats: crate::intelligence::SessionStats,
    pub config: crate::intelligence::SessionConfig,
}

/// GET /v1/intelligence/session
/// Returns current session with hysteresis info (v2 Intelligence)
async fn get_session(State(app): State<AppState>) -> Json<SessionResponse> {
    let session = app.session_manager.current_session();
    let stats = app.session_manager.stats();
    let config = app.session_manager.config();

    Json(SessionResponse {
        session,
        stats,
        config,
    })
}

/// GET /v1/intelligence/shadow-stats
/// Returns v1 vs v2 shadow mode comparison statistics (v2 stabilization)
async fn get_shadow_stats(State(app): State<AppState>) -> Json<ShadowStatsResponse> {
    let shadow_stats = app.context_intelligence.get_shadow_stats();
    let config = app.context_intelligence.get_config();
    let sample_stats = app.inference_engine.sample_stats(config.v2.recent_days_window);

    let observation_days = shadow_stats.tracking_since
        .map(|ts| (time::OffsetDateTime::now_utc() - ts).whole_days())
        .unwrap_or(0);

    Json(ShadowStatsResponse {
        shadow_stats,
        sample_stats,
        v2_config: config.v2,
        observation_days,
    })
}

// ============================================================================
// Router
// ============================================================================

pub fn intelligence_routes() -> Router<AppState> {
    Router::new()
        .route("/status", get(get_status))
        .route("/health", get(get_health))
        .route("/patterns", get(get_patterns))
        .route("/patterns/export", get(get_patterns_export))
        .route("/predictions", get(get_predictions))
        .route("/signals", get(get_signals))
        .route("/features", get(get_features))      // v2 Intelligence
        .route("/vector", get(get_vector))          // v2 Intelligence
        .route("/prediction2", get(get_prediction2)) // v2 Intelligence
        .route("/session", get(get_session))         // v2 Intelligence
        .route("/shadow-stats", get(get_shadow_stats)) // v2 Stabilization
        .route("/feedback", post(post_feedback))
        .route("/config", get(get_config).put(put_config))
}
