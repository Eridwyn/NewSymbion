use super::AppState;
use axum::extract::{Path, State};
use axum::http::StatusCode;
use axum::Json;
use serde::Deserialize;

// ============================================================================
// Notifications Endpoints
// ============================================================================

/// GET /notifications — List all notifications from history.
pub(super) async fn list_notifications(
    State(app): State<AppState>,
) -> Json<Vec<crate::notifications::Notification>> {
    Json(app.notifications_manager.list_all())
}

/// GET /notifications/active — List all unacknowledged notifications.
pub(super) async fn list_active_notifications(
    State(app): State<AppState>,
) -> Json<Vec<crate::notifications::Notification>> {
    Json(app.notifications_manager.list_active())
}

/// GET /notifications/tokens — List all registered FCM tokens.
pub(super) async fn list_fcm_tokens(
    State(app): State<AppState>,
) -> Json<Vec<crate::notifications::FcmToken>> {
    Json(app.notifications_manager.list_fcm_tokens())
}

/// Request body for sending a new notification.
#[derive(Debug, Deserialize)]
pub(super) struct SendNotificationRequest {
    title: String,
    body: String,
    #[serde(default)]
    priority: Option<String>,
    #[serde(default)]
    source: Option<String>,
    #[serde(default)]
    actions: Vec<crate::notifications::NotificationAction>,
    #[serde(default)]
    data: Option<serde_json::Value>,
}

/// POST /notifications — Send a new notification with optional priority and actions.
pub(super) async fn send_notification(
    State(app): State<AppState>,
    Json(request): Json<SendNotificationRequest>,
) -> Result<Json<serde_json::Value>, (StatusCode, String)> {
    let priority = match request.priority.as_deref() {
        Some("P0") | Some("p0") => crate::notifications::NotificationPriority::P0,
        Some("P1") | Some("p1") => crate::notifications::NotificationPriority::P1,
        _ => crate::notifications::NotificationPriority::P2,
    };

    let notification = crate::notifications::Notification {
        id: String::new(), // Will be assigned by manager
        priority,
        title: request.title,
        body: request.body,
        source: request.source.unwrap_or_else(|| "api".to_string()),
        timestamp: time::OffsetDateTime::now_utc(),
        acknowledged: false,
        acknowledged_at: None,
        actions: request.actions,
        data: request.data,
    };

    app.notifications_manager.send(notification).await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    Ok(Json(serde_json::json!({
        "success": true,
        "message": "Notification sent"
    })))
}

/// POST /notifications/{id}/acknowledge — Acknowledge a notification by ID.
pub(super) async fn acknowledge_notification(
    State(app): State<AppState>,
    Path(id): Path<String>,
) -> Result<Json<serde_json::Value>, (StatusCode, String)> {
    app.notifications_manager.acknowledge(&id)
        .map_err(|e| (StatusCode::NOT_FOUND, e.to_string()))?;

    Ok(Json(serde_json::json!({
        "success": true,
        "message": "Notification acknowledged"
    })))
}

/// DELETE /notifications/{id} — Delete a notification by ID.
pub(super) async fn delete_notification(
    State(app): State<AppState>,
    Path(id): Path<String>,
) -> Result<Json<serde_json::Value>, (StatusCode, String)> {
    app.notifications_manager.delete(&id)
        .map_err(|e| (StatusCode::NOT_FOUND, e.to_string()))?;

    Ok(Json(serde_json::json!({
        "success": true,
        "message": "Notification deleted"
    })))
}

/// Request body for registering an FCM push token.
#[derive(Debug, Deserialize)]
pub(super) struct RegisterFcmTokenRequest {
    user_id: String,
    token: String,
    #[serde(default)]
    device_name: Option<String>,
}

/// POST /notifications/tokens — Register an FCM push token for a user/device.
pub(super) async fn register_fcm_token(
    State(app): State<AppState>,
    Json(request): Json<RegisterFcmTokenRequest>,
) -> Json<serde_json::Value> {
    app.notifications_manager.register_fcm_token(
        request.user_id,
        request.token,
        request.device_name,
    );

    Json(serde_json::json!({
        "success": true,
        "message": "FCM token registered"
    }))
}

// =============================================================================
// Notification Config API
// =============================================================================

/// GET /notifications/config — List all notification type configurations.
pub(super) async fn list_notification_configs(
    State(app): State<AppState>,
) -> Json<Vec<crate::notification_config::NotificationTypeConfig>> {
    Json(app.notification_config.list_all())
}

/// GET /notifications/config/{type_id} — Retrieve a specific notification type configuration.
pub(super) async fn get_notification_config(
    State(app): State<AppState>,
    Path(type_id): Path<String>,
) -> Result<Json<crate::notification_config::NotificationTypeConfig>, (StatusCode, String)> {
    app.notification_config
        .get(&type_id)
        .map(Json)
        .ok_or_else(|| (StatusCode::NOT_FOUND, format!("Notification type '{}' not found", type_id)))
}

/// PUT /notifications/config/{type_id} — Update a notification type configuration.
pub(super) async fn update_notification_config(
    State(app): State<AppState>,
    Path(type_id): Path<String>,
    Json(update): Json<crate::notification_config::NotificationConfigUpdate>,
) -> Result<Json<crate::notification_config::NotificationTypeConfig>, (StatusCode, String)> {
    app.notification_config
        .update(&type_id, update)
        .map(Json)
        .map_err(|e| (StatusCode::NOT_FOUND, e))
}
