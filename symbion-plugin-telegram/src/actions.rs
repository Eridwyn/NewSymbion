use axum::extract::State;
use axum::http::StatusCode;
use axum::response::Json;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::time::Instant;
use teloxide::prelude::*;
use uuid::Uuid;

use crate::state::AppState;

const SPEC_VERSION: &str = "1.0";

// ── Contract v1.0 types ──

#[derive(Debug, Deserialize)]
#[allow(dead_code)]
pub struct ActionRequest {
    pub spec_version: String,
    pub action_id: Uuid,
    pub action_type: String,
    pub payload: Value,
    #[serde(default)]
    pub metadata: Value,
}

#[derive(Debug, Serialize)]
pub struct ActionResponse {
    pub spec_version: String,
    pub action_id: Uuid,
    pub status: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub result: Option<Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<ActionError>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub execution_time_ms: Option<u64>,
}

#[derive(Debug, Serialize)]
pub struct ActionError {
    pub code: String,
    pub message: String,
    pub retryable: bool,
}

impl ActionResponse {
    fn success(action_id: Uuid, result: Value, elapsed_ms: u64) -> Self {
        Self {
            spec_version: SPEC_VERSION.into(),
            action_id,
            status: "success".into(),
            result: Some(result),
            error: None,
            execution_time_ms: Some(elapsed_ms),
        }
    }

    fn error(action_id: Uuid, code: &str, message: &str, retryable: bool, elapsed_ms: u64) -> Self {
        Self {
            spec_version: SPEC_VERSION.into(),
            action_id,
            status: "error".into(),
            result: None,
            error: Some(ActionError {
                code: code.into(),
                message: message.into(),
                retryable,
            }),
            execution_time_ms: Some(elapsed_ms),
        }
    }

    fn rejected(action_id: Uuid, code: &str, message: &str, elapsed_ms: u64) -> Self {
        Self {
            spec_version: SPEC_VERSION.into(),
            action_id,
            status: "rejected".into(),
            result: None,
            error: Some(ActionError {
                code: code.into(),
                message: message.into(),
                retryable: false,
            }),
            execution_time_ms: Some(elapsed_ms),
        }
    }
}

// ── Handler ──

pub async fn handle_action(
    State(state): State<AppState>,
    Json(req): Json<ActionRequest>,
) -> (StatusCode, Json<ActionResponse>) {
    let start = Instant::now();
    let elapsed = || start.elapsed().as_millis() as u64;

    match req.action_type.as_str() {
        "send_message" => handle_send_message(&state, &req, elapsed).await,
        "send_notification" => handle_send_notification(&state, &req, elapsed).await,
        _ => (
            StatusCode::OK,
            Json(ActionResponse::rejected(
                req.action_id,
                "unknown_action",
                &format!("Unknown action type: {}", req.action_type),
                elapsed(),
            )),
        ),
    }
}

async fn handle_send_message(
    state: &AppState,
    req: &ActionRequest,
    elapsed: impl Fn() -> u64,
) -> (StatusCode, Json<ActionResponse>) {
    let chat_id = match req.payload.get("chat_id").and_then(|v| v.as_i64()) {
        Some(id) => id,
        None => {
            return (
                StatusCode::OK,
                Json(ActionResponse::error(
                    req.action_id,
                    "invalid_payload",
                    "Missing or invalid chat_id",
                    false,
                    elapsed(),
                )),
            );
        }
    };

    let text = match req.payload.get("text").and_then(|v| v.as_str()) {
        Some(t) => t,
        None => {
            return (
                StatusCode::OK,
                Json(ActionResponse::error(
                    req.action_id,
                    "invalid_payload",
                    "Missing text field",
                    false,
                    elapsed(),
                )),
            );
        }
    };

    let parse_mode = req
        .payload
        .get("parse_mode")
        .and_then(|v| v.as_str())
        .unwrap_or("");

    let mut msg = state.bot.send_message(ChatId(chat_id), text);
    if parse_mode.eq_ignore_ascii_case("html") {
        msg = msg.parse_mode(teloxide::types::ParseMode::Html);
    } else if parse_mode.eq_ignore_ascii_case("markdown") {
        msg = msg.parse_mode(teloxide::types::ParseMode::MarkdownV2);
    }

    match msg.await {
        Ok(sent) => (
            StatusCode::OK,
            Json(ActionResponse::success(
                req.action_id,
                json!({ "message_id": sent.id.0 }),
                elapsed(),
            )),
        ),
        Err(e) => (
            StatusCode::OK,
            Json(ActionResponse::error(
                req.action_id,
                "telegram_error",
                &e.to_string(),
                true,
                elapsed(),
            )),
        ),
    }
}

async fn handle_send_notification(
    state: &AppState,
    req: &ActionRequest,
    elapsed: impl Fn() -> u64,
) -> (StatusCode, Json<ActionResponse>) {
    let text = match req.payload.get("text").and_then(|v| v.as_str()) {
        Some(t) => t,
        None => {
            return (
                StatusCode::OK,
                Json(ActionResponse::error(
                    req.action_id,
                    "invalid_payload",
                    "Missing text field",
                    false,
                    elapsed(),
                )),
            );
        }
    };

    let level = req
        .payload
        .get("level")
        .and_then(|v| v.as_str())
        .unwrap_or("info");

    let icon = match level {
        "error" | "critical" => "🚨",
        "warning" => "⚠️",
        "success" => "✅",
        _ => "ℹ️",
    };

    let formatted = format!("{} [Symbion] {}", icon, text);
    let mut sent_count = 0;
    let mut errors = Vec::new();

    for &user_id in &state.config.allowed_user_ids {
        match state
            .bot
            .send_message(ChatId(user_id), &formatted)
            .await
        {
            Ok(_) => sent_count += 1,
            Err(e) => errors.push(format!("user {}: {}", user_id, e)),
        }
    }

    if sent_count > 0 {
        (
            StatusCode::OK,
            Json(ActionResponse::success(
                req.action_id,
                json!({
                    "sent_to": sent_count,
                    "errors": errors,
                }),
                elapsed(),
            )),
        )
    } else {
        (
            StatusCode::OK,
            Json(ActionResponse::error(
                req.action_id,
                "delivery_failed",
                "Failed to send to any user",
                true,
                elapsed(),
            )),
        )
    }
}

// ── Health endpoint ──

pub async fn health_handler(State(state): State<AppState>) -> Json<Value> {
    let uptime = state.start_time.elapsed().as_secs();
    Json(json!({
        "status": "healthy",
        "plugin_id": "telegram",
        "uptime_seconds": uptime,
        "active_tasks": state.active_tasks.len(),
        "sessions": state.user_sessions.len(),
    }))
}
