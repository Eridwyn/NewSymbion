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
 * - POST /v1/intelligence/feedback - Record user feedback
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
    ContextSignals, HealthCounters, IntelligenceConfig, IntelligenceStatus,
    LearnedPattern, ModePrediction, PredictionRecord, PatternExport,
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
    pub accuracy_7_days: f32,
    pub patterns_active: usize,
    pub patterns_established: usize,
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
/// Returns health counters (24h) and key metrics (v1.1.9)
async fn get_health(State(app): State<AppState>) -> Json<HealthResponse> {
    let counters = app.context_intelligence.get_health_counters();
    let accuracy = app.context_intelligence.calculate_accuracy(7);
    let patterns = app.context_intelligence.get_patterns();
    let config = app.context_intelligence.get_config();

    // Count established patterns (occurrences >= min_pattern_occurrences)
    let patterns_established = patterns.iter()
        .filter(|p| p.occurrences >= config.min_pattern_occurrences)
        .count();

    Json(HealthResponse {
        counters,
        accuracy_7_days: accuracy,
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
/// Update intelligence configuration
async fn put_config(
    State(app): State<AppState>,
    Json(config): Json<IntelligenceConfig>,
) -> Json<ConfigResponse> {
    app.context_intelligence.update_config(config.clone());
    Json(ConfigResponse { config })
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
        .route("/feedback", post(post_feedback))
        .route("/config", get(get_config).put(put_config))
}
