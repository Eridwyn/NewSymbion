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
 * - POST   /v1/automations/{id}/run     → Execute automation manually
 */

use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    Json,
};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use time::OffsetDateTime;

use crate::automations::{
    Automation, AutomationEvent, AutomationRequest, AutomationSchema, AutomationsListResponse,
    ExecutionContext, ExecutionRecord, AutomationEngine,
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

    // Collect modes from mode registry (dynamic modes)
    let modes: Vec<(String, String, String)> = app
        .mode_registry
        .list_all()
        .into_iter()
        .map(|m| (m.slug, format!("{} {}", m.icon, m.name), m.name))
        .collect();

    // Use SchemaRegistry to build typed schema
    Json(SchemaRegistry::get_schema(&agents, &rooms, &sensors, &modes))
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
            crate::automations::ActionDefinition::SetFeature { feature_id, value, .. } =>
                format!("Set feature '{}' = {}", feature_id, value),
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

/// POST /v1/automations/{id}/run - Execute automation manually
pub async fn run_automation(
    State(app): State<AppState>,
    Path(automation_id): Path<String>,
) -> Result<Json<Value>, (StatusCode, Json<Value>)> {
    let automation = app.automations
        .get(&automation_id)
        .ok_or((StatusCode::NOT_FOUND, error_response("Automation not found")))?;

    // Check if automation is enabled
    if !automation.enabled {
        return Err((StatusCode::BAD_REQUEST, error_response("Automation is disabled")));
    }

    // Check cooldown (but allow manual override with warning)
    let was_in_cooldown = automation.is_in_cooldown();
    if was_in_cooldown {
        eprintln!(
            "[automations] ⚠️  Manual run of '{}' ignoring cooldown",
            automation.name
        );
    }

    eprintln!(
        "[automations] 🚀 Manual execution of '{}' (id: {})",
        automation.name, automation_id
    );

    // Create execution context for manual trigger
    let ctx = ExecutionContext {
        context_engine: app.context_engine.clone(),
        agents: app.agents.clone(),
        sensors: app.sensors.clone(),
        notifications_manager: app.notifications_manager.clone(),
        event: AutomationEvent::Manual {
            automation_id: automation_id.clone(),
            triggered_by: None, // Manual via API
            timestamp: OffsetDateTime::now_utc(),
        },
        decision_engine: Some(app.decision_engine.clone()),
        trust_tracker: None, // Not available in HTTP context
        validation_manager: Some(app.decision_validation_manager.clone()),
        pending_action_registry: Some(app.pending_action_registry.clone()),
        context_intelligence: Some(app.context_intelligence.clone()),
        mode_registry: Some(app.mode_registry.clone()),
        feature_registry: Some(app.feature_registry.clone()),
    };

    // Execute actions
    let action_results = AutomationEngine::execute_actions(&automation, &ctx).await;

    // Check overall success
    let all_success = action_results.iter().all(|r| r.success);
    let error_msg = if all_success {
        None
    } else {
        Some(action_results.iter()
            .filter(|r| !r.success)
            .filter_map(|r| r.error.as_ref())
            .cloned()
            .collect::<Vec<_>>()
            .join("; "))
    };

    // Extract trust info
    let (overall_trust_score, overall_decision_outcome) = action_results.iter()
        .find(|r| r.trust_score.is_some())
        .map(|r| (r.trust_score, r.decision_outcome.clone()))
        .unwrap_or((None, None));

    // Check if all actions pending validation
    let all_pending_validation = action_results.iter().all(|r| {
        r.decision_outcome.as_ref().map(|o| o == "require_validation").unwrap_or(false)
    });

    // Record execution in history (unless all pending)
    if !all_pending_validation {
        let record = ExecutionRecord {
            automation_id: automation_id.clone(),
            automation_name: automation.name.clone(),
            executed_at: OffsetDateTime::now_utc(),
            trigger_event: "manual".to_string(),
            conditions_met: true,
            actions_executed: action_results.clone(),
            success: all_success,
            error: error_msg.clone(),
            trust_score: overall_trust_score,
            decision_outcome: overall_decision_outcome.clone(),
        };
        let _ = app.automations.add_history(record);

        // Update execution tracking (for cooldown)
        if let Err(e) = app.automations.record_execution(&automation_id) {
            eprintln!(
                "[automations] Failed to record execution for '{}': {}",
                automation.name, e
            );
        }
    }

    // Build response
    let actions_summary: Vec<Value> = action_results.iter().map(|r| {
        json!({
            "action_type": r.action_type,
            "success": r.success,
            "error": r.error,
            "duration_ms": r.duration_ms,
            "decision_outcome": r.decision_outcome
        })
    }).collect();

    Ok(Json(json!({
        "automation_id": automation_id,
        "automation_name": automation.name,
        "executed": true,
        "success": all_success,
        "error": error_msg,
        "actions_count": action_results.len(),
        "actions": actions_summary,
        "trust_score": overall_trust_score,
        "decision_outcome": overall_decision_outcome,
        "was_in_cooldown": was_in_cooldown,
        "pending_validation": all_pending_validation
    })))
}

