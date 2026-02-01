/**
 * SYMBION KERNEL - Intelligence API Endpoints
 *
 * ROLE: REST API for Context Intelligence Engine
 *
 * ENDPOINTS:
 * - GET  /v1/intelligence/status   - Current engine status
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

use crate::http::AppState;
use crate::context_intelligence::{
    ContextSignals, IntelligenceConfig, IntelligenceStatus,
    LearnedPattern, ModePrediction, PredictionRecord,
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

// ============================================================================
// Router
// ============================================================================

pub fn intelligence_routes() -> Router<AppState> {
    Router::new()
        .route("/status", get(get_status))
        .route("/patterns", get(get_patterns))
        .route("/predictions", get(get_predictions))
        .route("/signals", get(get_signals))
        .route("/feedback", post(post_feedback))
        .route("/config", get(get_config).put(put_config))
}
