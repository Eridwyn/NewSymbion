/**
 * SYMBION KERNEL - Automations HTTP Endpoints
 *
 * ROLE: REST API for automation rules management
 *
 * SECURITY:
 * - GET endpoints: JWT auth only (read-only)
 * - POST/PUT/DELETE/PATCH: JWT + CSRF protection
 *
 * ENDPOINTS:
 * - GET    /v1/automations              → List all automations
 * - GET    /v1/automations/{id}         → Get automation detail
 * - GET    /v1/automations/schema       → Get schema for rule builder
 * - GET    /v1/automations/history      → Get execution history
 * - POST   /v1/automations              → Create automation
 * - PUT    /v1/automations/{id}         → Update automation
 * - DELETE /v1/automations/{id}         → Soft-delete automation
 * - PATCH  /v1/automations/{id}/enable  → Toggle enabled
 * - POST   /v1/automations/{id}/test    → Dry-run test
 */

use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    Json,
};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};

use crate::automations::{
    Automation, AutomationRequest, AutomationSchema, AutomationsListResponse, ExecutionRecord,
    SchemaRegistry, SensorInfo, ToggleRequest,
};
use crate::http::AppState;

/// Query params for history endpoint
#[derive(Debug, Deserialize)]
pub struct HistoryQuery {
    #[serde(default = "default_limit")]
    pub limit: usize,
}

fn default_limit() -> usize {
    50
}

/// Error response structure
fn error_response(message: &str) -> Json<Value> {
    Json(json!({ "error": message }))
}

// ===== GET Endpoints (JWT auth only) =====

/// GET /v1/automations - List all automations
pub async fn list_automations(
    State(app): State<AppState>,
) -> Result<Json<AutomationsListResponse>, StatusCode> {
    let automations = app.automations.list();
    let count = automations.len();
    let enabled_count = automations.iter().filter(|a| a.enabled).count();

    Ok(Json(AutomationsListResponse {
        automations,
        count,
        enabled_count,
    }))
}

/// GET /v1/automations/{id} - Get automation detail
pub async fn get_automation(
    State(app): State<AppState>,
    Path(automation_id): Path<String>,
) -> Result<Json<Automation>, (StatusCode, Json<Value>)> {
    app.automations
        .get(&automation_id)
        .map(Json)
        .ok_or((StatusCode::NOT_FOUND, error_response("Automation not found")))
}

/// GET /v1/automations/schema - Get schema for rule builder
///
/// Returns typed schema from SchemaRegistry with dynamic values from kernel registries.
/// This provides a structured schema for the PWA rule builder with:
/// - Triggers: events that start an automation
/// - Conditions: filters to evaluate before executing
/// - Actions: what to execute when conditions are met
/// - Dynamic values: agents, rooms, sensors, modes from live registries
pub async fn get_automations_schema(
    State(app): State<AppState>,
) -> Json<AutomationSchema> {
    // Collect dynamic values from kernel registries
    let agents: Vec<(String, String)> = app
        .agents
        .list_agents()
        .await
        .values()
        .map(|a| (a.agent_id.clone(), a.hostname.clone()))
        .collect();

    let rooms: Vec<String> = app.sensors.list_rooms();

    // Collect sensors with their info
    let sensors: Vec<SensorInfo> = app
        .sensors
        .list_sensors()
        .into_iter()
        .map(|s| SensorInfo {
            sensor_id: s.sensor_id,
            sensor_type: s.sensor_type,
            room_id: s.room_id,
            status: format!("{:?}", s.status).to_lowercase(),
        })
        .collect();

    // Use SchemaRegistry to build typed schema
    Json(SchemaRegistry::get_schema(&agents, &rooms, &sensors))
}

/// GET /v1/automations/history - Get execution history
pub async fn get_automations_history(
    State(app): State<AppState>,
    Query(params): Query<HistoryQuery>,
) -> Json<Vec<ExecutionRecord>> {
    Json(app.automations.get_history(params.limit))
}

// ===== POST/PUT/DELETE/PATCH Endpoints (CSRF protected) =====

/// POST /v1/automations - Create automation
pub async fn create_automation(
    State(app): State<AppState>,
    Json(request): Json<AutomationRequest>,
) -> Result<(StatusCode, Json<Automation>), (StatusCode, Json<Value>)> {
    // Validate request
    if request.name.trim().is_empty() {
        return Err((StatusCode::UNPROCESSABLE_ENTITY, error_response("Name is required")));
    }
    if request.actions.is_empty() {
        return Err((StatusCode::UNPROCESSABLE_ENTITY, error_response("At least one action is required")));
    }

    match app.automations.create(request) {
        Ok(automation) => Ok((StatusCode::CREATED, Json(automation))),
        Err(e) => {
            eprintln!("[automations] Create error: {}", e);
            Err((StatusCode::INTERNAL_SERVER_ERROR, error_response("Failed to create automation")))
        }
    }
}

/// PUT /v1/automations/{id} - Update automation
pub async fn update_automation(
    State(app): State<AppState>,
    Path(automation_id): Path<String>,
    Json(request): Json<AutomationRequest>,
) -> Result<Json<Automation>, (StatusCode, Json<Value>)> {
    // Validate request
    if request.name.trim().is_empty() {
        return Err((StatusCode::UNPROCESSABLE_ENTITY, error_response("Name is required")));
    }
    if request.actions.is_empty() {
        return Err((StatusCode::UNPROCESSABLE_ENTITY, error_response("At least one action is required")));
    }

    match app.automations.update(&automation_id, request) {
        Ok(Some(automation)) => Ok(Json(automation)),
        Ok(None) => Err((StatusCode::NOT_FOUND, error_response("Automation not found"))),
        Err(e) => {
            eprintln!("[automations] Update error: {}", e);
            Err((StatusCode::INTERNAL_SERVER_ERROR, error_response("Failed to update automation")))
        }
    }
}

/// DELETE /v1/automations/{id} - Soft-delete automation
pub async fn delete_automation(
    State(app): State<AppState>,
    Path(automation_id): Path<String>,
) -> Result<StatusCode, (StatusCode, Json<Value>)> {
    match app.automations.delete(&automation_id) {
        Ok(true) => {
            println!("[automations] Soft-deleted automation: {}", automation_id);
            Ok(StatusCode::NO_CONTENT)
        }
        Ok(false) => Err((StatusCode::NOT_FOUND, error_response("Automation not found or already deleted"))),
        Err(e) => {
            eprintln!("[automations] Delete error: {}", e);
            Err((StatusCode::INTERNAL_SERVER_ERROR, error_response("Failed to delete automation")))
        }
    }
}

/// PATCH /v1/automations/{id}/enable - Toggle enabled
pub async fn toggle_automation(
    State(app): State<AppState>,
    Path(automation_id): Path<String>,
    Json(request): Json<ToggleRequest>,
) -> Result<Json<Automation>, (StatusCode, Json<Value>)> {
    match app.automations.toggle(&automation_id, request.enabled) {
        Ok(Some(automation)) => {
            println!("[automations] Toggled automation {} to enabled={}", automation_id, request.enabled);
            Ok(Json(automation))
        }
        Ok(None) => Err((StatusCode::NOT_FOUND, error_response("Automation not found"))),
        Err(e) => {
            eprintln!("[automations] Toggle error: {}", e);
            Err((StatusCode::INTERNAL_SERVER_ERROR, error_response("Failed to toggle automation")))
        }
    }
}

/// POST /v1/automations/{id}/test - Dry-run test
pub async fn test_automation(
    State(app): State<AppState>,
    Path(automation_id): Path<String>,
) -> Result<Json<Value>, (StatusCode, Json<Value>)> {
    let automation = app.automations
        .get(&automation_id)
        .ok_or((StatusCode::NOT_FOUND, error_response("Automation not found")))?;

    // Check cooldown
    let cooldown_info = if automation.is_in_cooldown() {
        Some(json!({
            "in_cooldown": true,
            "remaining_seconds": automation.cooldown_remaining()
        }))
    } else {
        None
    };

    // Preview actions
    let actions_preview: Vec<String> = automation.actions.iter().map(|a| {
        match a {
            crate::automations::ActionDefinition::SendNotification { title, .. } =>
                format!("Send notification: {}", title),
            crate::automations::ActionDefinition::ForceMode { mode, .. } =>
                format!("Force mode: {}", mode),
            crate::automations::ActionDefinition::AgentCommand { agent_id, command_type, .. } =>
                format!("Agent {} command: {}", agent_id, command_type),
            crate::automations::ActionDefinition::Delay { seconds } =>
                format!("Delay: {}s", seconds),
            crate::automations::ActionDefinition::Custom { plugin_name, action_type, .. } =>
                format!("Plugin {} action: {}", plugin_name, action_type),
        }
    }).collect();

    Ok(Json(json!({
        "automation_id": automation_id,
        "automation_name": automation.name,
        "enabled": automation.enabled,
        "cooldown": cooldown_info,
        "would_execute": automation.enabled && !automation.is_in_cooldown(),
        "actions_preview": actions_preview,
        "conditions": automation.conditions,
        "trigger": automation.trigger
    })))
}
