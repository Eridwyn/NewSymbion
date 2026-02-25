use super::AppState;
use axum::extract::{Path, Query, State};
use axum::http::StatusCode;
use axum::Json;
use crate::notes_bridge;
use crate::context_intelligence::DecisionSignal;
use std::collections::HashMap;
use utoipa::ToSchema;

// Types referenced by utoipa::path body annotations
use crate::context::{ContextState, ManualOverride, ModeHistoryEntry, ModeStats, ProductivityMetrics};
use crate::modes::types::{DynamicMode, CreateModeRequest, UpdateModeRequest};
use crate::schedule::types::{Schedule, ScheduleRule, CreateRuleRequest, UpdateRuleRequest, UpdateDefaultModeRequest, CurrentScheduleInfo};

/// GET /context/current — Return the current contextual mode state.
#[utoipa::path(
    get,
    path = "/v1/context/current",
    tag = "Context",
    security(("bearer_auth" = [])),
    responses(
        (status = 200, description = "État contextuel courant", body = ContextState),
        (status = 401, description = "Non authentifié"),
        (status = 500, description = "Erreur interne"),
    )
)]
pub(super) async fn get_context_current(State(app): State<AppState>) -> Result<Json<crate::context::ContextState>, StatusCode> {
    match app.context_engine.get_state() {
        Some(state) => Ok(Json(state)),
        None => Err(StatusCode::INTERNAL_SERVER_ERROR),
    }
}

/// Request body for POST /context/override to manually force a contextual mode.
#[derive(serde::Deserialize, ToSchema)]
pub(crate) struct ContextOverrideRequest {
    mode: String,  // Slug du mode dynamique: "pro", "focus", "maison", "veille", ou custom
    duration_minutes: i64,
    reason: Option<String>,
}

/// POST /context/override — Force a manual contextual mode override and record intelligence feedback.
#[utoipa::path(
    post,
    path = "/v1/context/override",
    tag = "Context",
    security(("bearer_auth" = [])),
    params(("X-CSRF-Token" = String, Header, description = "CSRF nonce")),
    request_body = ContextOverrideRequest,
    responses(
        (status = 200, description = "Override appliqué", body = ContextState),
        (status = 400, description = "Mode inconnu"),
        (status = 401, description = "Non authentifié"),
        (status = 500, description = "Erreur interne"),
    )
)]
pub(super) async fn set_context_override(
    State(app): State<AppState>,
    Json(req): Json<ContextOverrideRequest>,
) -> Result<Json<crate::context::ContextState>, StatusCode> {
    let mode_slug = req.mode.to_lowercase();

    // Look up mode from mode_registry to validate and get theme
    let dynamic_mode = app.mode_registry.get_by_slug(&mode_slug)
        .ok_or_else(|| {
            eprintln!("[context] Unknown mode slug: {}", mode_slug);
            StatusCode::BAD_REQUEST
        })?;

    // Convert DynamicMode theme to context::Theme
    let theme = crate::context::Theme {
        primary: dynamic_mode.theme.primary,
        bg: dynamic_mode.theme.background,
        accent: dynamic_mode.theme.accent,
    };

    let reason = req.reason.unwrap_or_else(|| "Override manuel".to_string());

    // Get current mode before override (use mode_slug for proper comparison)
    let old_mode = app.context_engine.get_state()
        .and_then(|s| s.mode_slug)
        .unwrap_or_else(|| "unknown".to_string());

    match app.context_engine.set_override_dynamic(
        dynamic_mode.slug.clone(),
        theme,
        req.duration_minutes,
        reason.clone(),
    ) {
        Some(state) => {
            // Dispatch mode change event for automations
            let new_mode = state.mode_slug.clone().unwrap_or_else(|| dynamic_mode.slug.clone());
            if old_mode != new_mode {
                app.automation_dispatcher.dispatch_mode_change(&old_mode, &new_mode, &reason);

                // Record feedback for intelligence learning (manual override = user preference)
                let signals = app.context_intelligence.collect_signals().await;
                app.context_intelligence.record_feedback(&new_mode, signals);
                eprintln!("[context] 📚 v1 learning: {} → {}", old_mode, new_mode);

                // Record correction for v2 intelligence learning (highest priority: UserCorrection)
                let vector = crate::intelligence::VectorBuilder::new(&app.feature_registry).build();
                app.inference_engine.record_correction(&vector, &new_mode);
                eprintln!("[intelligence] 📚 v2 correction recorded: {} (sample count will grow)", new_mode);
            }
            Ok(Json(state))
        }
        None => Err(StatusCode::INTERNAL_SERVER_ERROR),
    }
}

/// Convert a `Mode` enum variant to its lowercase string slug.
pub(super) fn mode_to_str(mode: &crate::context::Mode) -> String {
    use crate::context::Mode;
    match mode {
        Mode::Pro => "pro".to_string(),
        Mode::Maison => "maison".to_string(),
        Mode::Veille => "veille".to_string(),
    }
}

/// POST /context/clear — Cancel the active manual mode override and revert to automatic context.
#[utoipa::path(
    post,
    path = "/v1/context/clear",
    tag = "Context",
    security(("bearer_auth" = [])),
    params(("X-CSRF-Token" = String, Header, description = "CSRF nonce")),
    responses(
        (status = 200, description = "Override annulé", body = ContextState),
        (status = 204, description = "Aucun override actif"),
        (status = 401, description = "Non authentifié"),
    )
)]
pub(super) async fn clear_context_override(State(app): State<AppState>) -> Result<Json<crate::context::ContextState>, StatusCode> {
    let agents_map = app.agents.list_agents().await;
    let agents_list: Vec<crate::agents::Agent> = agents_map.values().cloned().collect();

    match app.context_engine.clear_override(&agents_list) {
        Some(state) => Ok(Json(state)),
        None => Err(StatusCode::NO_CONTENT),  // Pas d'override actif
    }
}

/// GET /context/history — Return the chronological history of mode changes.
#[utoipa::path(
    get,
    path = "/v1/context/history",
    tag = "Context",
    security(("bearer_auth" = [])),
    responses(
        (status = 200, description = "Historique des changements de mode", body = Vec<ModeHistoryEntry>),
        (status = 401, description = "Non authentifié"),
    )
)]
pub(super) async fn get_context_history(State(app): State<AppState>) -> Json<Vec<crate::context::ModeHistoryEntry>> {
    Json(app.context_engine.get_history())
}

/// GET /context/stats — Return aggregated usage statistics per contextual mode.
#[utoipa::path(
    get,
    path = "/v1/context/stats",
    tag = "Context",
    security(("bearer_auth" = [])),
    responses(
        (status = 200, description = "Statistiques d'usage par mode", body = Vec<ModeStats>),
        (status = 401, description = "Non authentifié"),
    )
)]
pub(super) async fn get_context_stats(State(app): State<AppState>) -> Json<Vec<crate::context::ModeStats>> {
    Json(app.context_engine.calculate_stats())
}

// Note: GET /context/patterns removed - use /intelligence/patterns instead

/// GET /context/productivity — Return productivity metrics broken down by contextual mode.
#[utoipa::path(
    get,
    path = "/v1/context/productivity",
    tag = "Context",
    security(("bearer_auth" = [])),
    responses(
        (status = 200, description = "Métriques de productivité par mode", body = Vec<ProductivityMetrics>),
        (status = 401, description = "Non authentifié"),
    )
)]
pub(super) async fn get_context_productivity(State(app): State<AppState>) -> Json<Vec<crate::context::ProductivityMetrics>> {
    Json(app.context_engine.calculate_productivity())
}

// ============ MEMO HANDLERS (Plugin Bridge Only) ============

/// GET /memo — List notes via the notes plugin bridge, with optional query filters.
pub(super) async fn handle_memo_list(
    State(app): State<AppState>,
    Query(params): Query<HashMap<String, String>>,
) -> Result<Json<serde_json::Value>, StatusCode> {
    // Notes uniquement via plugin - pas de fallback
    if let Some(ref bridge) = app.notes_bridge {
        return notes_bridge::list_notes_endpoint(
            axum::extract::State(bridge.clone()),
            axum::extract::Query(params)
        ).await;
    }

    // Plugin notes non disponible
    Err(StatusCode::SERVICE_UNAVAILABLE)
}

/// POST /memo — Create a new note via the notes plugin bridge, injecting contextual mode automatically.
pub(super) async fn handle_memo_create(
    State(app): State<AppState>,
    Json(note_data): Json<serde_json::Value>,
) -> Result<Json<serde_json::Value>, StatusCode> {
    // Notes uniquement via plugin - pas de fallback
    if let Some(ref bridge) = app.notes_bridge {
        // Récupérer le contexte fourni ou utiliser le mode actuel
        let context = note_data.get("context")
            .and_then(|v| v.as_str())
            .map(|s| s.to_string())
            .or_else(|| {
                // Injecter automatiquement le mode contextuel actuel (supporte modes dynamiques)
                app.context_engine.get_state()
                    .and_then(|state| state.mode_slug.or_else(|| Some(format!("{:?}", state.mode).to_lowercase())))
            });

        // Convertir les données en format CreateNoteRequest
        let create_request = notes_bridge::CreateNoteRequest {
            content: note_data.get("content")
                .and_then(|v| v.as_str())
                .unwrap_or("").to_string(),
            urgent: note_data.get("urgent")
                .and_then(|v| v.as_bool()),
            context,
            tags: note_data.get("tags")
                .and_then(|v| v.as_array())
                .map(|arr| arr.iter().filter_map(|v| v.as_str().map(|s| s.to_string())).collect()),
            status: note_data.get("status")
                .and_then(|v| v.as_str())
                .map(|s| s.to_string()),
        };

        return notes_bridge::create_note_endpoint(
            axum::extract::State(bridge.clone()),
            axum::extract::Json(create_request)
        ).await;
    }

    // Plugin notes non disponible
    Err(StatusCode::SERVICE_UNAVAILABLE)
}

/// DELETE /memo/:id — Delete a note by ID via the notes plugin bridge.
pub(super) async fn handle_memo_delete(
    State(app): State<AppState>,
    Path(id): Path<String>,
) -> Result<Json<serde_json::Value>, StatusCode> {
    // Notes uniquement via plugin - pas de fallback
    if let Some(ref bridge) = app.notes_bridge {
        return notes_bridge::delete_note_endpoint(
            axum::extract::State(bridge.clone()),
            axum::extract::Path(id)
        ).await;
    }

    // Plugin notes non disponible
    Err(StatusCode::SERVICE_UNAVAILABLE)
}

/// PUT /memo/:id — Update an existing note by ID via the notes plugin bridge.
pub(super) async fn handle_memo_update(
    State(app): State<AppState>,
    Path(id): Path<String>,
    Json(note_data): Json<serde_json::Value>,
) -> Result<Json<serde_json::Value>, StatusCode> {
    // Notes uniquement via plugin - pas de fallback
    if let Some(ref bridge) = app.notes_bridge {
        let create_request = notes_bridge::CreateNoteRequest {
            content: note_data.get("content")
                .and_then(|v| v.as_str())
                .unwrap_or("").to_string(),
            urgent: note_data.get("urgent")
                .and_then(|v| v.as_bool()),
            context: note_data.get("context")
                .and_then(|v| v.as_str())
                .map(|s| s.to_string()),
            tags: note_data.get("tags")
                .and_then(|v| v.as_array())
                .map(|arr| arr.iter().filter_map(|v| v.as_str().map(|s| s.to_string())).collect()),
            status: note_data.get("status")
                .and_then(|v| v.as_str())
                .map(|s| s.to_string()),
        };

        return notes_bridge::update_note_endpoint(
            axum::extract::State(bridge.clone()),
            axum::extract::Path(id),
            axum::extract::Json(create_request)
        ).await;
    }

    // Plugin notes non disponible
    Err(StatusCode::SERVICE_UNAVAILABLE)
}

// ============================================================================
// Dynamic Modes API
// ============================================================================

/// GET /modes - Liste tous les modes
#[utoipa::path(
    get,
    path = "/v1/modes",
    tag = "Modes",
    security(("bearer_auth" = [])),
    responses(
        (status = 200, description = "Liste de tous les modes", body = Vec<DynamicMode>),
        (status = 401, description = "Non authentifié"),
    )
)]
pub(super) async fn list_modes(
    State(app): State<AppState>,
) -> Json<Vec<crate::modes::DynamicMode>> {
    Json(app.mode_registry.list_all())
}

/// GET /modes/:id - Récupère un mode par ID
#[utoipa::path(
    get,
    path = "/v1/modes/{id}",
    tag = "Modes",
    security(("bearer_auth" = [])),
    params(("id" = String, Path, description = "ID ou slug du mode")),
    responses(
        (status = 200, description = "Mode trouvé", body = DynamicMode),
        (status = 401, description = "Non authentifié"),
        (status = 404, description = "Mode introuvable"),
    )
)]
pub(super) async fn get_mode(
    State(app): State<AppState>,
    Path(id): Path<String>,
) -> Result<Json<crate::modes::DynamicMode>, StatusCode> {
    // Essayer par ID d'abord, puis par slug
    if let Some(mode) = app.mode_registry.get(&id) {
        return Ok(Json(mode));
    }
    if let Some(mode) = app.mode_registry.get_by_slug(&id) {
        return Ok(Json(mode));
    }
    Err(StatusCode::NOT_FOUND)
}

/// POST /modes - Crée un nouveau mode
#[utoipa::path(
    post,
    path = "/v1/modes",
    tag = "Modes",
    security(("bearer_auth" = [])),
    params(("X-CSRF-Token" = String, Header, description = "CSRF nonce")),
    request_body = CreateModeRequest,
    responses(
        (status = 200, description = "Mode créé", body = DynamicMode),
        (status = 400, description = "Données invalides"),
        (status = 401, description = "Non authentifié"),
    )
)]
pub(super) async fn create_mode(
    State(app): State<AppState>,
    Json(request): Json<crate::modes::CreateModeRequest>,
) -> Result<Json<crate::modes::DynamicMode>, (StatusCode, String)> {
    app.mode_registry.create(request)
        .map(Json)
        .map_err(|e| (StatusCode::BAD_REQUEST, e))
}

/// PUT /modes/:id - Met à jour un mode
#[utoipa::path(
    put,
    path = "/v1/modes/{id}",
    tag = "Modes",
    security(("bearer_auth" = [])),
    params(
        ("id" = String, Path, description = "ID du mode"),
        ("X-CSRF-Token" = String, Header, description = "CSRF nonce"),
    ),
    request_body = UpdateModeRequest,
    responses(
        (status = 200, description = "Mode mis à jour", body = DynamicMode),
        (status = 400, description = "Données invalides"),
        (status = 401, description = "Non authentifié"),
    )
)]
pub(super) async fn update_mode(
    State(app): State<AppState>,
    Path(id): Path<String>,
    Json(request): Json<crate::modes::UpdateModeRequest>,
) -> Result<Json<crate::modes::DynamicMode>, (StatusCode, String)> {
    app.mode_registry.update(&id, request)
        .map(Json)
        .map_err(|e| (StatusCode::BAD_REQUEST, e))
}

/// DELETE /modes/:id - Supprime un mode
#[utoipa::path(
    delete,
    path = "/v1/modes/{id}",
    tag = "Modes",
    security(("bearer_auth" = [])),
    params(
        ("id" = String, Path, description = "ID du mode"),
        ("X-CSRF-Token" = String, Header, description = "CSRF nonce"),
    ),
    responses(
        (status = 204, description = "Mode supprimé"),
        (status = 400, description = "Suppression impossible (mode système)"),
        (status = 401, description = "Non authentifié"),
    )
)]
pub(super) async fn delete_mode(
    State(app): State<AppState>,
    Path(id): Path<String>,
) -> Result<StatusCode, (StatusCode, String)> {
    app.mode_registry.delete(&id)
        .map(|_| StatusCode::NO_CONTENT)
        .map_err(|e| (StatusCode::BAD_REQUEST, e))
}

// ============================================================================
// Schedule API
// ============================================================================

/// GET /schedule - Récupère le planning complet
#[utoipa::path(
    get,
    path = "/v1/schedule",
    tag = "Schedule",
    security(("bearer_auth" = [])),
    responses(
        (status = 200, description = "Planning complet", body = Schedule),
        (status = 401, description = "Non authentifié"),
    )
)]
pub(super) async fn get_schedule(
    State(app): State<AppState>,
) -> Json<crate::schedule::Schedule> {
    Json(app.schedule_registry.get_schedule())
}

/// GET /schedule/rules - Liste toutes les règles
#[utoipa::path(
    get,
    path = "/v1/schedule/rules",
    tag = "Schedule",
    security(("bearer_auth" = [])),
    responses(
        (status = 200, description = "Liste des règles de planning", body = Vec<ScheduleRule>),
        (status = 401, description = "Non authentifié"),
    )
)]
pub(super) async fn list_schedule_rules(
    State(app): State<AppState>,
) -> Json<Vec<crate::schedule::ScheduleRule>> {
    Json(app.schedule_registry.list_rules())
}

/// GET /schedule/current - Récupère le mode actif selon le planning
#[utoipa::path(
    get,
    path = "/v1/schedule/current",
    tag = "Schedule",
    security(("bearer_auth" = [])),
    responses(
        (status = 200, description = "Mode actif selon le planning", body = CurrentScheduleInfo),
        (status = 401, description = "Non authentifié"),
    )
)]
pub(super) async fn get_current_schedule_mode(
    State(app): State<AppState>,
) -> Json<crate::schedule::CurrentScheduleInfo> {
    Json(app.schedule_registry.get_current_mode())
}

/// POST /schedule/rules - Crée une nouvelle règle
#[utoipa::path(
    post,
    path = "/v1/schedule/rules",
    tag = "Schedule",
    security(("bearer_auth" = [])),
    params(("X-CSRF-Token" = String, Header, description = "CSRF nonce")),
    request_body = CreateRuleRequest,
    responses(
        (status = 200, description = "Règle créée", body = ScheduleRule),
        (status = 400, description = "Données invalides"),
        (status = 401, description = "Non authentifié"),
    )
)]
pub(super) async fn create_schedule_rule(
    State(app): State<AppState>,
    Json(request): Json<crate::schedule::CreateRuleRequest>,
) -> Result<Json<crate::schedule::ScheduleRule>, (StatusCode, String)> {
    app.schedule_registry.create_rule(request)
        .map(Json)
        .map_err(|e| (StatusCode::BAD_REQUEST, e))
}

/// PUT /schedule/rules/:id - Met à jour une règle
#[utoipa::path(
    put,
    path = "/v1/schedule/rules/{id}",
    tag = "Schedule",
    security(("bearer_auth" = [])),
    params(
        ("id" = String, Path, description = "ID de la règle"),
        ("X-CSRF-Token" = String, Header, description = "CSRF nonce"),
    ),
    request_body = UpdateRuleRequest,
    responses(
        (status = 200, description = "Règle mise à jour", body = ScheduleRule),
        (status = 400, description = "Données invalides"),
        (status = 401, description = "Non authentifié"),
    )
)]
pub(super) async fn update_schedule_rule(
    State(app): State<AppState>,
    Path(id): Path<String>,
    Json(request): Json<crate::schedule::UpdateRuleRequest>,
) -> Result<Json<crate::schedule::ScheduleRule>, (StatusCode, String)> {
    app.schedule_registry.update_rule(&id, request)
        .map(Json)
        .map_err(|e| (StatusCode::BAD_REQUEST, e))
}

/// DELETE /schedule/rules/:id - Supprime une règle
#[utoipa::path(
    delete,
    path = "/v1/schedule/rules/{id}",
    tag = "Schedule",
    security(("bearer_auth" = [])),
    params(
        ("id" = String, Path, description = "ID de la règle"),
        ("X-CSRF-Token" = String, Header, description = "CSRF nonce"),
    ),
    responses(
        (status = 204, description = "Règle supprimée"),
        (status = 400, description = "Suppression impossible"),
        (status = 401, description = "Non authentifié"),
    )
)]
pub(super) async fn delete_schedule_rule(
    State(app): State<AppState>,
    Path(id): Path<String>,
) -> Result<StatusCode, (StatusCode, String)> {
    app.schedule_registry.delete_rule(&id)
        .map(|_| StatusCode::NO_CONTENT)
        .map_err(|e| (StatusCode::BAD_REQUEST, e))
}

/// PUT /schedule/default - Définit le mode par défaut
#[utoipa::path(
    put,
    path = "/v1/schedule/default",
    tag = "Schedule",
    security(("bearer_auth" = [])),
    params(("X-CSRF-Token" = String, Header, description = "CSRF nonce")),
    request_body = UpdateDefaultModeRequest,
    responses(
        (status = 200, description = "Mode par défaut mis à jour"),
        (status = 400, description = "Mode invalide"),
        (status = 401, description = "Non authentifié"),
    )
)]
pub(super) async fn set_schedule_default_mode(
    State(app): State<AppState>,
    Json(request): Json<crate::schedule::UpdateDefaultModeRequest>,
) -> Result<StatusCode, (StatusCode, String)> {
    app.schedule_registry.set_default_mode(request.default_mode_id)
        .map(|_| StatusCode::OK)
        .map_err(|e| (StatusCode::BAD_REQUEST, e))
}
