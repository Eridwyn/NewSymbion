// Decision Engine HTTP Handlers - PR3 API Endpoints
// Spec: PR3 P0 v3.1 REFINED - HTTP API

use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    Json,
};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use utoipa::{IntoParams, ToSchema};

/// État Decision Engine partagé
#[derive(Clone)]
pub struct DecisionEngineState {
    pub engine: Arc<crate::decision::DecisionEngine>,
    pub validation_manager: Arc<crate::decision::ValidationManager>,
    pub override_manager: Arc<crate::decision::OverrideManager>,
    pub audit_manager: Arc<crate::decision::AuditManager>,
    pub agent_health_manager: Arc<crate::decision::AgentHealthManager>,
    pub metrics: Arc<crate::decision::DecisionMetrics>,
}

/// POST /v1/decision/evaluate - Évaluer une action
#[derive(Debug, Deserialize, ToSchema)]
pub struct EvaluateRequest {
    pub action: crate::decision::Action,
    pub context: crate::decision::DecisionContext,
}

pub async fn evaluate_action(
    State(state): State<DecisionEngineState>,
    Json(req): Json<EvaluateRequest>,
) -> Json<crate::decision::DecisionResult> {
    let result = state.engine.decide(&req.action, &req.context);

    // Record dans audit (créer DecisionRecord manuellement)
    let record = crate::decision::DecisionRecord {
        decision_id: result.decision_id.clone(),
        trace_id: result.trace_id.clone(),
        action_type: req.action.action_type.clone(),
        agent_id: req.action.agent_id.clone(),
        impact_level: req.action.impact_level,
        outcome: result.outcome.clone(),
        trust_score: None, // TODO: récupérer trust_score si disponible
        timestamp: time::OffsetDateTime::now_utc(),
        config_version: state.engine.config().version,
    };
    state.audit_manager.add_record(record);

    // Record métrique
    state.metrics.record_decision(&result.outcome);

    // Si RequireValidation, créer la validation
    if matches!(result.outcome, crate::decision::DecisionOutcome::RequireValidation { .. }) {
        if let Ok(validation_request) = state.validation_manager.create_validation(&result, &req.action, &req.context) {
            println!("[decision] Validation created: {}", validation_request.validation_id);
        } else {
            eprintln!("[decision] Failed to create validation for decision {}", result.decision_id);
        }
    }

    Json(result)
}

/// GET /v1/decision/audit - Audit trail
#[derive(Debug, Deserialize, ToSchema, IntoParams)]
pub struct AuditQueryParams {
    pub limit: Option<usize>,
    pub agent_id: Option<String>,
    pub trace_id: Option<String>,
}

pub async fn get_audit_trail(
    State(state): State<DecisionEngineState>,
    Query(params): Query<AuditQueryParams>,
) -> Json<serde_json::Value> {
    let records = if let Some(agent_id) = params.agent_id {
        state.audit_manager.get_by_agent(&agent_id)
    } else if let Some(trace_id) = params.trace_id {
        state.audit_manager.get_by_trace(&trace_id)
    } else if let Some(limit) = params.limit {
        state.audit_manager.get_last(limit)
    } else {
        state.audit_manager.get_all()
    };

    Json(serde_json::json!({
        "records": records,
        "stats": state.audit_manager.stats(),
    }))
}

/// GET /v1/decision/metrics - Prometheus export
pub async fn get_metrics(
    State(state): State<DecisionEngineState>,
) -> Result<String, StatusCode> {
    let audit_stats = state.audit_manager.stats();
    let validation_stats = state.validation_manager.stats();
    let override_stats = state.override_manager.stats();
    let agent_health_stats = state.agent_health_manager.stats();

    let output = state.metrics.export_prometheus(
        &audit_stats,
        &validation_stats,
        &override_stats,
        &agent_health_stats,
    );

    Ok(output)
}

/// GET /v1/decision/validations/pending - Validations en attente
pub async fn list_pending_validations(
    State(state): State<DecisionEngineState>,
) -> Json<Vec<crate::decision::ValidationRequest>> {
    let pending = state.validation_manager.list_pending();
    Json(pending)
}

/// GET /v1/decision/validations/expired - Validations expirées
pub async fn list_expired_validations(
    State(state): State<DecisionEngineState>,
) -> Json<Vec<crate::decision::ValidationRequest>> {
    let expired = state.validation_manager.list_expired();
    Json(expired)
}

/// DELETE /v1/decision/validation/:id - Supprimer une validation expirée
pub async fn delete_validation(
    State(state): State<DecisionEngineState>,
    Path(validation_id): Path<String>,
) -> Result<StatusCode, StatusCode> {
    let deleted = state.validation_manager.delete_validation(&validation_id);

    if deleted {
        Ok(StatusCode::NO_CONTENT)
    } else {
        Err(StatusCode::NOT_FOUND)
    }
}

/// DELETE /v1/decision/validations/expired - Supprimer toutes les validations expirées
pub async fn delete_all_expired_validations(
    State(state): State<DecisionEngineState>,
) -> Json<serde_json::Value> {
    let count = state.validation_manager.delete_all_expired();

    Json(serde_json::json!({
        "deleted": count,
        "message": format!("{} validation(s) expirée(s) supprimée(s)", count)
    }))
}

/// POST /v1/decision/validation/:id/resolve - Résoudre validation
#[derive(Debug, Clone, Deserialize, ToSchema)]
pub struct ResolveValidationRequest {
    pub approved: bool,
    pub username: String,
}

pub async fn resolve_validation(
    State(state): State<DecisionEngineState>,
    Path(validation_id): Path<String>,
    Json(req): Json<ResolveValidationRequest>,
) -> Result<Json<crate::decision::ValidationRequest>, StatusCode> {
    let resolved = state
        .validation_manager
        .resolve_validation(&validation_id, req.approved, &req.username)
        .map_err(|e| {
            eprintln!("[decision_http] Resolve validation error: {}", e);
            StatusCode::BAD_REQUEST
        })?;

    // Record métrique
    if req.approved {
        state.metrics.record_validation_approved();
    } else {
        state.metrics.record_validation_denied();
    }

    Ok(Json(resolved))
}

/// POST /v1/decision/override - Créer override (MFA requis)
#[derive(Debug, Deserialize, ToSchema)]
pub struct CreateOverrideRequest {
    pub decision_id: String,
    pub override_type: crate::decision::OverrideType,
    pub reason: String,
    pub username: String,
    pub mfa_verified: bool,
}

pub async fn create_override(
    State(state): State<DecisionEngineState>,
    Json(req): Json<CreateOverrideRequest>,
) -> Result<Json<crate::decision::MasterOverride>, StatusCode> {
    let override_entry = state
        .override_manager
        .create_override(
            &req.decision_id,
            req.override_type,
            &req.reason,
            &req.username,
            req.mfa_verified,
        )
        .map_err(|e| {
            eprintln!("[decision_http] Create override error: {}", e);
            StatusCode::FORBIDDEN
        })?;

    // Record métrique
    state.metrics.record_override_created();

    Ok(Json(override_entry))
}

/// GET /v1/decision/overrides/active - Overrides actifs
pub async fn list_active_overrides(
    State(state): State<DecisionEngineState>,
) -> Json<Vec<crate::decision::MasterOverride>> {
    let active = state.override_manager.list_active();
    Json(active)
}

/// DELETE /v1/decision/override/:id - Révoquer override (MFA requis)
#[derive(Debug, Deserialize, ToSchema)]
pub struct RevokeOverrideRequest {
    pub username: String,
    pub mfa_verified: bool,
}

pub async fn revoke_override(
    State(state): State<DecisionEngineState>,
    Path(override_id): Path<String>,
    Json(req): Json<RevokeOverrideRequest>,
) -> Result<StatusCode, StatusCode> {
    state
        .override_manager
        .revoke_override(&override_id, &req.username, req.mfa_verified)
        .map_err(|e| {
            eprintln!("[decision_http] Revoke override error: {}", e);
            StatusCode::FORBIDDEN
        })?;

    // Record métrique
    state.metrics.record_override_revoked();

    Ok(StatusCode::NO_CONTENT)
}

/// GET /v1/decision/config - Configuration actuelle
pub async fn get_config(
    State(state): State<DecisionEngineState>,
) -> Json<crate::decision::DecisionConfig> {
    let config = state.engine.config();
    Json(config)
}

/// PUT /v1/decision/config - Mettre à jour configuration
pub async fn update_config(
    State(state): State<DecisionEngineState>,
    Json(config): Json<crate::decision::DecisionConfig>,
) -> StatusCode {
    state.engine.update_config(config);
    StatusCode::OK
}

/// GET /v1/decision/agent-health - États santé agents
pub async fn get_agent_health(
    State(state): State<DecisionEngineState>,
) -> Json<serde_json::Value> {
    let stats = state.agent_health_manager.stats();

    Json(serde_json::json!({
        "stats": stats,
    }))
}

/// GET /v1/decision/stats - Statistiques globales
#[derive(Debug, Serialize, ToSchema)]
pub struct DecisionStats {
    pub audit: crate::decision::AuditStats,
    pub validation: crate::decision::ValidationStats,
    pub override_stats: crate::decision::OverrideStats,
    pub agent_health: crate::decision::AgentHealthStats,
}

pub async fn get_stats(
    State(state): State<DecisionEngineState>,
) -> Json<DecisionStats> {
    Json(DecisionStats {
        audit: state.audit_manager.stats(),
        validation: state.validation_manager.stats(),
        override_stats: state.override_manager.stats(),
        agent_health: state.agent_health_manager.stats(),
    })
}

/// Construire les routes Decision Engine
pub fn build_decision_routes(state: DecisionEngineState) -> axum::Router {
    use axum::routing::{delete, get, post};

    axum::Router::new()
        .route("/evaluate", post(evaluate_action))
        .route("/audit", get(get_audit_trail))
        .route("/metrics", get(get_metrics))
        .route("/validations/pending", get(list_pending_validations))
        .route("/validations/expired", get(list_expired_validations).delete(delete_all_expired_validations))
        .route("/validation/:id/resolve", post(resolve_validation))
        .route("/validation/:id", delete(delete_validation))
        .route("/override", post(create_override))
        .route("/overrides/active", get(list_active_overrides))
        .route("/override/:id", delete(revoke_override))
        .route("/config", get(get_config).put(update_config))
        .route("/agent-health", get(get_agent_health))
        .route("/stats", get(get_stats))
        .with_state(state)
}
