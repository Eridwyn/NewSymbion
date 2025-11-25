/**
 * SYMBION KERNEL - Mobile API Endpoints (F4)
 *
 * RÔLE : API REST pour application mobile (JWT auth uniquement, pas mTLS)
 * ARCHITECTURE : Séparé du serveur PWA pour contourner contrainte mTLS client certificates
 * UTILITÉ : Push notifications, validations humaines, contrôles rapides
 */

use axum::{
    extract::{Path, State},
    http::StatusCode,
    response::Json,
    routing::{get, post},
    Router,
};
use serde::{Deserialize, Serialize};

use crate::http::AppState;
use crate::notifications::{Notification, NotificationAction, NotificationPriority};

/// Router mobile API (port 8444)
pub fn mobile_router() -> Router<AppState> {
    Router::new()
        // Auth
        .route("/auth/login", post(mobile_login))
        // Notifications
        .route("/v1/notifications/register", post(mobile_register_fcm_token))
        .route("/v1/notifications/list", get(mobile_notifications_list))
        .route("/v1/notifications/{id}/acknowledge", post(mobile_notification_ack))
        .route("/v1/notifications/test", post(mobile_test_notification))
        // Validations (Decision Engine)
        .route("/v1/validations/pending", get(mobile_validations_pending))
        .route("/v1/validations/{id}/approve", post(mobile_validation_approve))
        .route("/v1/validations/{id}/reject", post(mobile_validation_reject))
        // Quick Actions
        .route("/v1/quick/lights/toggle", post(mobile_lights_toggle))
}

// ===== Auth =====

#[derive(Debug, Deserialize)]
struct LoginRequest {
    username: String,
    password: String,
}

#[derive(Debug, Serialize)]
struct LoginResponse {
    success: bool,
    token: Option<String>,
    message: String,
}

/// Login endpoint pour mobile (génère JWT)
async fn mobile_login(
    State(app): State<AppState>,
    Json(req): Json<LoginRequest>,
) -> (StatusCode, Json<LoginResponse>) {
    // Note: Mobile login does not support MFA for now (totp_code = None, trusted_device = false)
    match app.auth_manager.authenticate(&req.username, &req.password, None, false) {
        Ok(login_response) => (
            StatusCode::OK,
            Json(LoginResponse {
                success: true,
                token: Some(login_response.token),
                message: "Login successful".to_string(),
            }),
        ),
        Err(e) => (
            StatusCode::UNAUTHORIZED,
            Json(LoginResponse {
                success: false,
                token: None,
                message: format!("Login failed: {}", e),
            }),
        ),
    }
}

// ===== Notifications =====

#[derive(Debug, Deserialize)]
struct RegisterFcmTokenRequest {
    token: String,
    device_name: Option<String>,
}

#[derive(Debug, Serialize)]
struct RegisterFcmTokenResponse {
    success: bool,
    message: String,
}

/// Enregistre un token FCM pour l'utilisateur actuel
async fn mobile_register_fcm_token(
    State(app): State<AppState>,
    Json(req): Json<RegisterFcmTokenRequest>,
) -> (StatusCode, Json<RegisterFcmTokenResponse>) {
    // TODO: extraire user_id depuis JWT dans middleware
    let user_id = "default-user".to_string();

    app.notification_manager.register_fcm_token(user_id, req.token, req.device_name);

    (
        StatusCode::OK,
        Json(RegisterFcmTokenResponse {
            success: true,
            message: "FCM token registered successfully".to_string(),
        }),
    )
}

#[derive(Debug, Serialize)]
struct NotificationsListResponse {
    notifications: Vec<Notification>,
}

/// Liste toutes les notifications (actives + historique)
async fn mobile_notifications_list(
    State(app): State<AppState>,
) -> (StatusCode, Json<NotificationsListResponse>) {
    let notifications = app.notification_manager.list_all();

    (StatusCode::OK, Json(NotificationsListResponse { notifications }))
}

#[derive(Debug, Serialize)]
struct AcknowledgeResponse {
    success: bool,
    message: String,
}

/// Acquitte une notification
async fn mobile_notification_ack(
    State(app): State<AppState>,
    Path(id): Path<String>,
) -> (StatusCode, Json<AcknowledgeResponse>) {
    match app.notification_manager.acknowledge(&id) {
        Ok(_) => (
            StatusCode::OK,
            Json(AcknowledgeResponse {
                success: true,
                message: "Notification acknowledged".to_string(),
            }),
        ),
        Err(e) => (
            StatusCode::NOT_FOUND,
            Json(AcknowledgeResponse {
                success: false,
                message: format!("Failed to acknowledge: {}", e),
            }),
        ),
    }
}

/// Test notification (debug)
async fn mobile_test_notification(
    State(app): State<AppState>,
) -> (StatusCode, Json<RegisterFcmTokenResponse>) {
    let test_notif = Notification {
        id: uuid::Uuid::new_v4().to_string(),
        priority: NotificationPriority::P2,
        title: "Test Notification".to_string(),
        body: "This is a test notification from Symbion".to_string(),
        source: "mobile-api-test".to_string(),
        timestamp: time::OffsetDateTime::now_utc(),
        acknowledged: false,
        acknowledged_at: None,
        actions: vec![],
        data: None,
    };

    match app.notification_manager.send(test_notif).await {
        Ok(_) => (
            StatusCode::OK,
            Json(RegisterFcmTokenResponse {
                success: true,
                message: "Test notification sent".to_string(),
            }),
        ),
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(RegisterFcmTokenResponse {
                success: false,
                message: format!("Failed to send: {}", e),
            }),
        ),
    }
}

// ===== Validations (Decision Engine) =====

#[derive(Debug, Serialize)]
struct ValidationsPendingResponse {
    validations: Vec<ValidationItem>,
}

#[derive(Debug, Serialize)]
struct ValidationItem {
    id: String,
    action_name: String,
    reason: String,
    expires_at: i64,
}

/// Liste toutes les validations humaines en attente
async fn mobile_validations_pending(
    State(app): State<AppState>,
) -> (StatusCode, Json<ValidationsPendingResponse>) {
    let pending = app.decision_validation_manager.list_pending();

    let validations: Vec<ValidationItem> = pending
        .into_iter()
        .map(|v| ValidationItem {
            id: v.validation_id.clone(),
            action_name: v.action.action_type.clone(),
            reason: format!("Action: {} (agent: {})", v.action.action_type, v.action.agent_id),
            expires_at: v.expires_at.unix_timestamp(),
        })
        .collect();

    (StatusCode::OK, Json(ValidationsPendingResponse { validations }))
}

#[derive(Debug, Serialize)]
struct ValidationActionResponse {
    success: bool,
    message: String,
}

/// Approuve une validation humaine
async fn mobile_validation_approve(
    State(app): State<AppState>,
    Path(id): Path<String>,
) -> (StatusCode, Json<ValidationActionResponse>) {
    // TODO: extraire username depuis JWT token
    let username = "mobile-user";

    match app.decision_validation_manager.resolve_validation(&id, true, username) {
        Ok(_) => {
            println!("[mobile] validation approved: {}", id);
            (
                StatusCode::OK,
                Json(ValidationActionResponse {
                    success: true,
                    message: "Validation approved".to_string(),
                }),
            )
        }
        Err(e) => (
            StatusCode::NOT_FOUND,
            Json(ValidationActionResponse {
                success: false,
                message: format!("Failed to approve: {}", e),
            }),
        ),
    }
}

/// Rejette une validation humaine
async fn mobile_validation_reject(
    State(app): State<AppState>,
    Path(id): Path<String>,
) -> (StatusCode, Json<ValidationActionResponse>) {
    // TODO: extraire username depuis JWT token
    let username = "mobile-user";

    match app.decision_validation_manager.resolve_validation(&id, false, username) {
        Ok(_) => {
            println!("[mobile] validation rejected: {}", id);
            (
                StatusCode::OK,
                Json(ValidationActionResponse {
                    success: true,
                    message: "Validation rejected".to_string(),
                }),
            )
        }
        Err(e) => (
            StatusCode::NOT_FOUND,
            Json(ValidationActionResponse {
                success: false,
                message: format!("Failed to reject: {}", e),
            }),
        ),
    }
}

// ===== Quick Actions =====

#[derive(Debug, Serialize)]
struct QuickActionResponse {
    success: bool,
    message: String,
}

/// Toggle lumières (action rapide)
async fn mobile_lights_toggle(
    State(_app): State<AppState>,
) -> (StatusCode, Json<QuickActionResponse>) {
    // TODO: intégration F5 LightActuator
    println!("[mobile] quick action: lights toggle requested");

    (
        StatusCode::OK,
        Json(QuickActionResponse {
            success: true,
            message: "Lights toggle command sent (F5 not implemented yet)".to_string(),
        }),
    )
}
