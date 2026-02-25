use super::AppState;
use axum::extract::{Path, State};
use axum::http::StatusCode;
use axum::Json;
use axum::extract::Request;
use axum::response::IntoResponse;
use time::OffsetDateTime;
use base64::Engine;
use utoipa::ToSchema;

// =============== AUTH ENDPOINTS ===============

/// POST /auth/login — Authenticate user with username/password, return JWT token with optional MFA and device trust.
#[utoipa::path(
    post,
    path = "/auth/login",
    tag = "Authentication",
    responses(
        (status = 200, description = "Login successful", body = Object),
        (status = 401, description = "Invalid credentials or MFA required", body = Object)
    )
)]
pub(super) async fn auth_login(
    State(app): State<AppState>,
    req: Request,
) -> Result<impl IntoResponse, (StatusCode, Json<serde_json::Value>)> {
    // Extraire le body JSON
    let (parts, body) = req.into_parts();
    let bytes = axum::body::to_bytes(body, usize::MAX).await
        .map_err(|_| (
            StatusCode::BAD_REQUEST,
            Json(serde_json::json!({ "error": "Invalid request body" }))
        ))?;

    let payload: crate::auth::LoginRequest = serde_json::from_slice(&bytes)
        .map_err(|_| (
            StatusCode::BAD_REQUEST,
            Json(serde_json::json!({ "error": "Invalid JSON" }))
        ))?;

    // Extraire User-Agent pour device fingerprinting
    let user_agent = parts.headers
        .get("user-agent")
        .and_then(|v| v.to_str().ok())
        .unwrap_or("unknown");

    let device_fingerprint = crate::device_trust::DeviceTrustManager::generate_fingerprint(user_agent);

    // Vérifier si un device token existe dans le header X-Device-Token (localStorage)
    let device_token = parts.headers
        .get("x-device-token")
        .and_then(|v| v.to_str().ok())
        .map(|s| s.to_string());

    // Vérifier si le device est de confiance
    let trusted_device = if let Some(ref token) = device_token {
        let is_trusted = app.device_trust_manager.verify_device_token(
            token,
            &payload.username,
            &device_fingerprint
        );

        if is_trusted {
            println!("[device-trust] ✓ Device token valid - MFA bypassed");
        } else {
            println!("[device-trust] ✗ Device token invalid or expired");
        }

        is_trusted
    } else {
        false
    };

    // Authentifier l'utilisateur (avec bypass MFA si device trusted)
    match app.auth_manager.authenticate(
        &payload.username,
        &payload.password,
        payload.totp_code.as_deref(),
        trusted_device
    ) {
        Ok(mut response) => {
            // Si remember_device est activé et login réussi, créer un device token
            if payload.remember_device && !trusted_device {
                match app.device_trust_manager.create_device_token(&payload.username, &device_fingerprint) {
                    Ok(token) => {
                        // Retourner le device_token dans la réponse JSON (localStorage frontend)
                        response.device_token = Some(token.clone());
                        println!("[auth] Device token created (30 days)");
                    }
                    Err(e) => {
                        eprintln!("[auth] Failed to create device token: {}", e);
                        // Ne pas bloquer le login si la création du token échoue
                    }
                }
            }

            Ok(Json(response))
        }
        Err(e) => {
            let error_msg = e.to_string();
            eprintln!("[auth] login failed for '{}': {}", payload.username, error_msg);

            // Notification sécurité selon type d'échec
            let is_rate_limited = error_msg.contains("Too many login attempts");
            let (priority, title, body) = if is_rate_limited {
                // P0 : Attaque brute-force détectée
                (
                    crate::notifications::NotificationPriority::P0,
                    format!("🚨 Attaque bloquée - {}", payload.username),
                    format!("Trop de tentatives de connexion pour '{}'. L'accès a été temporairement bloqué. {}",
                        payload.username, error_msg),
                )
            } else {
                // P1 : Tentative échouée (credentials invalides)
                (
                    crate::notifications::NotificationPriority::P1,
                    format!("🔐 Échec login - {}", payload.username),
                    format!("Tentative de connexion échouée pour '{}': {}",
                        payload.username, error_msg),
                )
            };

            let notification = crate::notifications::Notification {
                id: String::new(),
                priority,
                title,
                body,
                source: "auth-security".to_string(),
                timestamp: OffsetDateTime::now_utc(),
                acknowledged: false,
                acknowledged_at: None,
                actions: vec![],
                data: None,
            };

            // Envoi async (ne bloque pas la réponse)
            let notif_manager = app.notifications_manager.clone();
            tokio::spawn(async move {
                let _ = notif_manager.send(notification).await;
            });

            Err((
                StatusCode::UNAUTHORIZED,
                Json(serde_json::json!({ "error": error_msg }))
            ))
        }
    }
}

/// GET /auth/verify — Verify JWT token validity and return decoded claims.
#[utoipa::path(
    get,
    path = "/auth/verify",
    tag = "Authentication",
    security(("bearer_auth" = [])),
    responses(
        (status = 200, description = "Token is valid", body = Object),
        (status = 401, description = "Invalid or expired token")
    )
)]
pub(super) async fn auth_verify(
    State(app): State<AppState>,
    req: Request,
) -> Result<Json<serde_json::Value>, StatusCode> {
    let token = req
        .headers()
        .get("authorization")
        .and_then(|v| v.to_str().ok())
        .and_then(|v| v.strip_prefix("Bearer "))
        .ok_or(StatusCode::UNAUTHORIZED)?;

    match app.auth_manager.verify_token(token) {
        Ok(claims) => Ok(Json(serde_json::json!({
            "valid": true,
            "username": claims.sub,
            "role": claims.role,
            "expires_at": claims.exp
        }))),
        Err(_) => Err(StatusCode::UNAUTHORIZED),
    }
}

/// GET /auth/session — Retrieve current session information for the authenticated user.
#[utoipa::path(
    get,
    path = "/auth/session",
    tag = "Authentication",
    security(("bearer_auth" = [])),
    responses(
        (status = 200, description = "Current session info", body = Object),
        (status = 401, description = "Invalid or expired token")
    )
)]
pub(super) async fn auth_session(
    State(app): State<AppState>,
    req: Request,
) -> Result<Json<crate::auth::SessionInfo>, StatusCode> {
    let token = req
        .headers()
        .get("authorization")
        .and_then(|v| v.to_str().ok())
        .and_then(|v| v.strip_prefix("Bearer "))
        .ok_or(StatusCode::UNAUTHORIZED)?;

    match app.auth_manager.get_session_info(token) {
        Ok(session) => Ok(Json(session)),
        Err(_) => Err(StatusCode::UNAUTHORIZED),
    }
}

/// POST /auth/logout — Log out the current user (client-side token removal).
#[utoipa::path(
    post,
    path = "/auth/logout",
    tag = "Authentication",
    security(("bearer_auth" = [])),
    responses(
        (status = 200, description = "Logged out successfully", body = Object)
    )
)]
pub(super) async fn auth_logout() -> Json<serde_json::Value> {
    // JWT est stateless - le client doit juste supprimer le token
    // On pourrait implémenter une blacklist de tokens pour invalidation côté serveur
    Json(serde_json::json!({
        "success": true,
        "message": "Logged out successfully"
    }))
}

/// POST /auth/reload - Recharger les utilisateurs depuis users.json sans redémarrer le kernel
#[utoipa::path(
    post,
    path = "/auth/reload",
    tag = "Authentication",
    security(("bearer_auth" = [])),
    params(("X-CSRF-Token" = String, Header, description = "CSRF nonce")),
    responses(
        (status = 200, description = "Users reloaded successfully", body = Object),
        (status = 500, description = "Failed to reload users")
    )
)]
pub(super) async fn auth_reload_users(
    State(app): State<AppState>,
) -> Result<Json<serde_json::Value>, StatusCode> {
    app.auth_manager.reload_users()
        .map_err(|e| {
            eprintln!("[http] Failed to reload users: {}", e);
            StatusCode::INTERNAL_SERVER_ERROR
        })?;

    Ok(Json(serde_json::json!({
        "success": true,
        "message": "Users reloaded successfully"
    })))
}

// ============================================================================
// User Management Endpoints (Admin)
// ============================================================================

/// Request payload for creating a new user with username, password, and role.
#[derive(Debug, serde::Deserialize, ToSchema)]
pub(crate) struct CreateUserRequest {
    username: String,
    password: String,
    role: String,
}

/// Request payload for updating a user password with current and new password fields.
#[derive(Debug, serde::Deserialize, ToSchema)]
pub(crate) struct UpdatePasswordRequest {
    current_password: String,
    new_password: String,
}

/// POST /v1/users - Créer un nouvel utilisateur (admin seulement)
#[utoipa::path(
    post,
    path = "/v1/users",
    tag = "Authentication",
    security(("bearer_auth" = [])),
    params(("X-CSRF-Token" = String, Header, description = "CSRF nonce")),
    request_body = CreateUserRequest,
    responses(
        (status = 200, description = "User created successfully", body = Object),
        (status = 400, description = "Failed to create user")
    )
)]
pub(super) async fn create_user(
    State(app): State<AppState>,
    Json(req): Json<CreateUserRequest>,
) -> Result<Json<serde_json::Value>, StatusCode> {
    app.auth_manager.create_user(&req.username, &req.password, &req.role)
        .map_err(|e| {
            eprintln!("[http] Failed to create user '{}': {}", req.username, e);
            StatusCode::BAD_REQUEST
        })?;

    Ok(Json(serde_json::json!({
        "success": true,
        "message": format!("User '{}' created successfully", req.username)
    })))
}

/// DELETE /v1/users/{username} - Supprimer un utilisateur (admin seulement)
#[utoipa::path(
    delete,
    path = "/v1/users/{username}",
    tag = "Authentication",
    security(("bearer_auth" = [])),
    params(
        ("username" = String, Path, description = "Username to delete"),
        ("X-CSRF-Token" = String, Header, description = "CSRF nonce")
    ),
    responses(
        (status = 200, description = "User deleted successfully", body = Object),
        (status = 404, description = "User not found")
    )
)]
pub(super) async fn delete_user(
    State(app): State<AppState>,
    Path(username): Path<String>,
) -> Result<Json<serde_json::Value>, StatusCode> {
    app.auth_manager.delete_user(&username)
        .map_err(|e| {
            eprintln!("[http] Failed to delete user '{}': {}", username, e);
            StatusCode::NOT_FOUND
        })?;

    Ok(Json(serde_json::json!({
        "success": true,
        "message": format!("User '{}' deleted successfully", username)
    })))
}

/// GET /v1/users - Lister tous les utilisateurs (admin seulement, sans mots de passe)
#[utoipa::path(
    get,
    path = "/v1/users",
    tag = "Authentication",
    security(("bearer_auth" = [])),
    responses(
        (status = 200, description = "List of users (without passwords)", body = Vec<Object>)
    )
)]
pub(super) async fn list_users(
    State(app): State<AppState>,
) -> Json<Vec<serde_json::Value>> {
    let users = app.auth_manager.list_users();
    Json(users)
}

/// PUT /v1/users/{username}/password - Changer le mot de passe d'un utilisateur
#[utoipa::path(
    put,
    path = "/v1/users/{username}/password",
    tag = "Authentication",
    security(("bearer_auth" = [])),
    params(
        ("username" = String, Path, description = "Username whose password to update"),
        ("X-CSRF-Token" = String, Header, description = "CSRF nonce")
    ),
    request_body = UpdatePasswordRequest,
    responses(
        (status = 200, description = "Password updated successfully", body = Object),
        (status = 401, description = "Current password incorrect"),
        (status = 403, description = "Cannot change another user's password"),
        (status = 500, description = "Internal server error")
    )
)]
pub(super) async fn update_user_password(
    State(app): State<AppState>,
    Path(username): Path<String>,
    headers: axum::http::HeaderMap,
    Json(body): Json<UpdatePasswordRequest>,
) -> Result<Json<serde_json::Value>, StatusCode> {
    // Extraire le token JWT pour vérifier que c'est bien l'utilisateur qui demande
    let token = headers
        .get("authorization")
        .and_then(|v| v.to_str().ok())
        .and_then(|v| v.strip_prefix("Bearer "))
        .ok_or(StatusCode::UNAUTHORIZED)?;

    // Vérifier et décoder le token
    let claims = app.auth_manager.verify_token(token)
        .map_err(|_| StatusCode::UNAUTHORIZED)?;

    // Vérifier que l'utilisateur ne peut changer que son propre mot de passe
    if claims.sub != username {
        return Err(StatusCode::FORBIDDEN);
    }

    // Vérifier le mot de passe actuel
    match app.auth_manager.verify_password(&username, &body.current_password) {
        Ok(true) => {
            // Mot de passe actuel correct, procéder au changement
            app.auth_manager.update_password(&username, &body.new_password)
                .map_err(|e| {
                    eprintln!("[http] Failed to update password for '{}': {}", username, e);
                    StatusCode::INTERNAL_SERVER_ERROR
                })?;

            Ok(Json(serde_json::json!({
                "success": true,
                "message": "Password updated successfully"
            })))
        },
        Ok(false) => {
            // Mot de passe actuel incorrect
            Err(StatusCode::UNAUTHORIZED)
        },
        Err(e) => {
            eprintln!("[http] Password verification failed for '{}': {}", username, e);
            Err(StatusCode::INTERNAL_SERVER_ERROR)
        }
    }
}

// ============================================================================
// MFA (Multi-Factor Authentication) Endpoints
// ============================================================================

/// GET /v1/auth/mfa/status - Vérifier si MFA est activé pour l'utilisateur courant
#[utoipa::path(
    get,
    path = "/auth/mfa/status",
    tag = "Authentication",
    security(("bearer_auth" = [])),
    responses(
        (status = 200, description = "MFA status for current user", body = Object),
        (status = 401, description = "Invalid or expired token"),
        (status = 404, description = "User not found")
    )
)]
pub(super) async fn mfa_status(
    State(app): State<AppState>,
    req: Request,
) -> Result<Json<serde_json::Value>, StatusCode> {
    // Extraire le token JWT
    let token = req
        .headers()
        .get("authorization")
        .and_then(|v| v.to_str().ok())
        .and_then(|v| v.strip_prefix("Bearer "))
        .ok_or(StatusCode::UNAUTHORIZED)?;

    // Vérifier et décoder le token
    let claims = app.auth_manager.verify_token(token)
        .map_err(|_| StatusCode::UNAUTHORIZED)?;

    // Récupérer l'utilisateur
    let user = app.auth_manager.get_user(&claims.sub)
        .ok_or(StatusCode::NOT_FOUND)?;

    // Retourner le status MFA
    match user.mfa_config {
        Some(config) => Ok(Json(serde_json::json!({
            "enabled": config.enabled,
            "setup_at": config.setup_at,
            "last_verified_at": config.last_verified_at,
            "backup_codes_count": config.backup_codes.len(),
            "recovery_email": config.recovery_email,
        }))),
        None => Ok(Json(serde_json::json!({
            "enabled": false,
            "setup_at": 0,
            "last_verified_at": 0,
            "backup_codes_count": 0,
            "recovery_email": null,
        }))),
    }
}

/// MFA setup request with optional recovery email address.
#[derive(serde::Deserialize, ToSchema)]
pub(crate) struct MfaSetupRequest {
    #[serde(default)]
    recovery_email: Option<String>,
}

/// MFA setup response containing the TOTP secret, QR code, and backup codes.
#[derive(serde::Serialize, ToSchema)]
pub(crate) struct MfaSetupResponse {
    secret: String,
    qr_code: String,
    backup_codes: Vec<String>,
}

/// POST /v1/auth/mfa/setup - Initialiser la configuration MFA (génère secret + QR code)
#[utoipa::path(
    post,
    path = "/auth/mfa/setup",
    tag = "Authentication",
    security(("bearer_auth" = [])),
    request_body = MfaSetupRequest,
    responses(
        (status = 200, description = "MFA setup initiated with secret and QR code", body = MfaSetupResponse),
        (status = 400, description = "MFA already enabled"),
        (status = 401, description = "Invalid or expired token"),
        (status = 404, description = "User not found"),
        (status = 500, description = "Failed to generate secret or QR code")
    )
)]
pub(super) async fn mfa_setup(
    State(app): State<AppState>,
    headers: axum::http::HeaderMap,
    Json(payload): Json<MfaSetupRequest>,
) -> Result<Json<MfaSetupResponse>, (StatusCode, Json<serde_json::Value>)> {
    // Extraire le token JWT depuis le header
    let token = headers
        .get("authorization")
        .and_then(|v| v.to_str().ok())
        .and_then(|v| v.strip_prefix("Bearer "))
        .ok_or_else(|| (
            StatusCode::UNAUTHORIZED,
            Json(serde_json::json!({"error": "Missing or invalid authorization token"}))
        ))?;

    // Vérifier le token
    let claims = app.auth_manager.verify_token(token)
        .map_err(|e| (
            StatusCode::UNAUTHORIZED,
            Json(serde_json::json!({"error": format!("Invalid token: {}", e)}))
        ))?;

    let username = &claims.sub;

    // Durée de persistance du secret TOTP non-activé : 20 minutes
    const MFA_SETUP_EXPIRY_SECS: i64 = 1200;

    // Vérifier si un secret MFA existe déjà (non encore activé)
    let existing_user = app.auth_manager.get_user(username);
    let (secret, backup_codes, should_save) = if let Some(user) = existing_user {
        if let Some(ref mfa) = user.mfa_config {
            if !mfa.enabled {
                // Vérifier si le secret est encore valide (< 20 minutes)
                let now = time::OffsetDateTime::now_utc().unix_timestamp();
                let elapsed = now - mfa.setup_at;

                if elapsed < MFA_SETUP_EXPIRY_SECS {
                    // Réutiliser le secret existant (encore dans la fenêtre de 20 min)
                    let remaining_mins = (MFA_SETUP_EXPIRY_SECS - elapsed) / 60;
                    println!("[mfa] Reusing existing MFA setup for user '{}' (valid for {} more minutes)", username, remaining_mins);
                    (mfa.secret_base32.clone(), mfa.backup_codes.clone(), false)
                } else {
                    // Secret expiré, générer un nouveau
                    println!("[mfa] Previous MFA setup expired, generating new secret");
                    let new_secret = app.mfa_manager.generate_secret()
                        .map_err(|e| (
                            StatusCode::INTERNAL_SERVER_ERROR,
                            Json(serde_json::json!({"error": format!("Failed to generate secret: {}", e)}))
                        ))?;
                    let new_backup_codes = app.mfa_manager.generate_backup_codes(10);
                    (new_secret, new_backup_codes, true)
                }
            } else {
                // MFA déjà activé, on ne peut pas régénérer (sécurité)
                return Err((
                    StatusCode::BAD_REQUEST,
                    Json(serde_json::json!({"error": "MFA already enabled. Disable it first to reconfigure."}))
                ));
            }
        } else {
            // Pas de config MFA existante, générer un nouveau secret
            let new_secret = app.mfa_manager.generate_secret()
                .map_err(|e| (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    Json(serde_json::json!({"error": format!("Failed to generate secret: {}", e)}))
                ))?;
            let new_backup_codes = app.mfa_manager.generate_backup_codes(10);
            (new_secret, new_backup_codes, true)
        }
    } else {
        return Err((
            StatusCode::NOT_FOUND,
            Json(serde_json::json!({"error": "User not found"}))
        ));
    };

    // Générer le QR code (même secret = même QR code)
    let qr_code = app.mfa_manager.generate_qr_code(username, &secret)
        .map_err(|e| (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(serde_json::json!({"error": format!("Failed to generate QR code: {}", e)}))
        ))?;

    // Si c'est un nouveau secret ou expiré, sauvegarder la configuration
    if should_save {
        let mfa_config = crate::mfa::MfaConfig {
            enabled: false,  // Sera activé après vérification du premier code
            secret_base32: secret.clone(),
            backup_codes: backup_codes.clone(),
            recovery_email: payload.recovery_email,
            setup_at: time::OffsetDateTime::now_utc().unix_timestamp(),
            last_verified_at: 0,
        };

        // Sauvegarder la configuration (mais pas encore activée)
        app.auth_manager.update_user_mfa(username, Some(mfa_config))
            .map_err(|e| (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(serde_json::json!({"error": format!("Failed to save MFA config: {}", e)}))
            ))?;

        println!("[mfa] Setup initiated for user '{}' (not yet enabled)", username);
    }

    Ok(Json(MfaSetupResponse {
        secret,
        qr_code,
        backup_codes,
    }))
}

/// MFA verification request containing the TOTP code to validate.
#[derive(serde::Deserialize, ToSchema)]
pub(crate) struct MfaVerifyRequest {
    code: String,
}

/// POST /v1/auth/mfa/verify - Vérifier un code TOTP et activer MFA
#[utoipa::path(
    post,
    path = "/auth/mfa/verify",
    tag = "Authentication",
    security(("bearer_auth" = [])),
    request_body = MfaVerifyRequest,
    responses(
        (status = 200, description = "MFA enabled successfully", body = Object),
        (status = 400, description = "MFA not configured"),
        (status = 401, description = "Invalid TOTP code or token"),
        (status = 404, description = "User not found"),
        (status = 500, description = "Internal server error")
    )
)]
pub(super) async fn mfa_verify(
    State(app): State<AppState>,
    headers: axum::http::HeaderMap,
    Json(payload): Json<MfaVerifyRequest>,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<serde_json::Value>)> {
    // Extraire le token JWT
    let token = headers
        .get("authorization")
        .and_then(|v| v.to_str().ok())
        .and_then(|v| v.strip_prefix("Bearer "))
        .ok_or_else(|| (
            StatusCode::UNAUTHORIZED,
            Json(serde_json::json!({"error": "Missing authorization token"}))
        ))?;

    // Vérifier le token
    let claims = app.auth_manager.verify_token(token)
        .map_err(|_| (
            StatusCode::UNAUTHORIZED,
            Json(serde_json::json!({"error": "Invalid token"}))
        ))?;

    let username = &claims.sub;

    // Récupérer l'utilisateur
    let user = app.auth_manager.get_user(username)
        .ok_or_else(|| (
            StatusCode::NOT_FOUND,
            Json(serde_json::json!({"error": "User not found"}))
        ))?;

    // Vérifier que MFA est configuré
    let mut mfa_config = user.mfa_config
        .ok_or_else(|| (
            StatusCode::BAD_REQUEST,
            Json(serde_json::json!({"error": "MFA not configured. Please run /mfa/setup first"}))
        ))?;

    // Vérifier le code TOTP
    let totp = totp_rs::TOTP::new(
        totp_rs::Algorithm::SHA1,
        6,
        1,
        30,
        totp_rs::Secret::Encoded(mfa_config.secret_base32.clone()).to_bytes()
            .map_err(|e| (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(serde_json::json!({"error": format!("Failed to decode secret: {}", e)}))
            ))?,
    ).map_err(|e| (
        StatusCode::INTERNAL_SERVER_ERROR,
        Json(serde_json::json!({"error": format!("Failed to create TOTP: {}", e)}))
    ))?;

    let is_valid = totp.check_current(&payload.code)
        .map_err(|e| (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(serde_json::json!({"error": format!("Failed to verify code: {}", e)}))
        ))?;

    if !is_valid {
        println!("[mfa] Invalid code for user '{}'", username);
        return Err((
            StatusCode::UNAUTHORIZED,
            Json(serde_json::json!({"error": "Invalid TOTP code"}))
        ));
    }

    // Code valide : activer MFA
    mfa_config.enabled = true;
    mfa_config.last_verified_at = time::OffsetDateTime::now_utc().unix_timestamp();

    app.auth_manager.update_user_mfa(username, Some(mfa_config))
        .map_err(|e| (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(serde_json::json!({"error": format!("Failed to enable MFA: {}", e)}))
        ))?;

    println!("[mfa] MFA enabled for user '{}'", username);

    Ok(Json(serde_json::json!({
        "success": true,
        "message": "MFA enabled successfully"
    })))
}

/// POST /v1/auth/mfa/disable - Désactiver MFA pour l'utilisateur
#[utoipa::path(
    post,
    path = "/auth/mfa/disable",
    tag = "Authentication",
    security(("bearer_auth" = [])),
    responses(
        (status = 200, description = "MFA disabled successfully", body = Object),
        (status = 401, description = "Invalid or expired token"),
        (status = 500, description = "Failed to disable MFA")
    )
)]
pub(super) async fn mfa_disable(
    State(app): State<AppState>,
    req: Request,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<serde_json::Value>)> {
    // Extraire le token JWT
    let token = req
        .headers()
        .get("authorization")
        .and_then(|v| v.to_str().ok())
        .and_then(|v| v.strip_prefix("Bearer "))
        .ok_or_else(|| (
            StatusCode::UNAUTHORIZED,
            Json(serde_json::json!({"error": "Missing authorization token"}))
        ))?;

    // Vérifier le token
    let claims = app.auth_manager.verify_token(token)
        .map_err(|_| (
            StatusCode::UNAUTHORIZED,
            Json(serde_json::json!({"error": "Invalid token"}))
        ))?;

    let username = &claims.sub;

    // Supprimer la configuration MFA
    app.auth_manager.update_user_mfa(username, None)
        .map_err(|e| (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(serde_json::json!({"error": format!("Failed to disable MFA: {}", e)}))
        ))?;

    println!("[mfa] MFA disabled for user '{}'", username);

    Ok(Json(serde_json::json!({
        "success": true,
        "message": "MFA disabled successfully"
    })))
}

// ============================================================================
// CSRF Protection Endpoints
// ============================================================================

/// GET /v1/auth/csrf/nonce - Générer un nonce CSRF pour l'utilisateur courant
#[utoipa::path(
    get,
    path = "/auth/csrf/nonce",
    tag = "Authentication",
    security(("bearer_auth" = [])),
    responses(
        (status = 200, description = "CSRF nonce generated", body = Object),
        (status = 401, description = "Invalid or expired token")
    )
)]
pub(super) async fn csrf_generate_nonce(
    State(app): State<AppState>,
    req: Request,
) -> Result<Json<serde_json::Value>, StatusCode> {
    // Extraire le token JWT
    let token = req
        .headers()
        .get("authorization")
        .and_then(|v| v.to_str().ok())
        .and_then(|v| v.strip_prefix("Bearer "))
        .ok_or(StatusCode::UNAUTHORIZED)?;

    // Vérifier et décoder le token
    let claims = app.auth_manager.verify_token(token)
        .map_err(|_| StatusCode::UNAUTHORIZED)?;

    // Générer un nonce CSRF pour cet utilisateur
    let nonce = app.csrf_manager.generate_nonce(claims.sub.clone());

    println!("[csrf] Generated nonce for user '{}'", claims.sub);

    Ok(Json(serde_json::json!({
        "nonce": nonce,
        "expires_in_seconds": 300  // 5 minutes TTL
    })))
}

/// GET /ca-certificate — Download the CA certificate as a PEM file.
#[utoipa::path(
    get,
    path = "/ca-certificate",
    tag = "Authentication",
    responses(
        (status = 200, description = "CA certificate PEM file", content_type = "application/x-pem-file"),
        (status = 500, description = "Failed to read CA certificate")
    )
)]
pub(super) async fn download_ca_certificate() -> Result<impl IntoResponse, StatusCode> {
    use axum::http::header;

    // Construire le chemin vers le certificat CA (toujours dans certs/ca/)
    let ca_cert_path = std::env::var("SYMBION_CA_CERT_PATH")
        .unwrap_or_else(|_| "symbion-kernel/certs/ca/symbion-ca.crt".to_string());

    println!("[http] Attempting to read CA certificate from: {}", ca_cert_path);

    match tokio::fs::read(&ca_cert_path).await {
        Ok(contents) => {
            println!("[http] CA certificate read successfully ({} bytes)", contents.len());
            Ok((
                [
                    (header::CONTENT_TYPE, "application/x-pem-file"),
                    (header::CONTENT_DISPOSITION, "attachment; filename=\"symbion-ca.crt\""),
                ],
                contents
            ))
        }
        Err(e) => {
            eprintln!("[http] Failed to read CA certificate from {}: {}", ca_cert_path, e);
            Err(StatusCode::INTERNAL_SERVER_ERROR)
        }
    }
}

// ============================================================================
// WebAuthn Biometric Authentication Endpoints
// ============================================================================

/// Registration start request containing a friendly name for the new passkey.
#[derive(serde::Deserialize, ToSchema)]
pub(crate) struct WebAuthnRegisterStartRequest {
    friendly_name: String, // Ex: "iPhone 15 Pro", "Windows Hello"
}

/// POST /auth/webauthn/register-start — Start passkey registration for the authenticated user.
#[utoipa::path(
    post,
    path = "/auth/webauthn/register-start",
    tag = "Authentication",
    security(("bearer_auth" = [])),
    responses(
        (status = 200, description = "Registration challenge created", body = Object),
        (status = 401, description = "Invalid or expired token"),
        (status = 500, description = "Failed to start registration")
    )
)]
pub(super) async fn webauthn_register_start(
    State(app): State<AppState>,
    req: Request,
) -> Result<Json<webauthn_rs::prelude::CreationChallengeResponse>, (StatusCode, Json<serde_json::Value>)> {
    // Extraire headers et body
    let (parts, _body) = req.into_parts();

    // Extraire le token JWT pour identifier l'utilisateur
    let token = parts
        .headers
        .get("authorization")
        .and_then(|h| h.to_str().ok())
        .and_then(|s| s.strip_prefix("Bearer "))
        .ok_or_else(|| (
            StatusCode::UNAUTHORIZED,
            Json(serde_json::json!({
                "error": "Missing or invalid Authorization header"
            }))
        ))?;

    let session = app.auth_manager.get_session_info(token)
        .map_err(|_| (
            StatusCode::UNAUTHORIZED,
            Json(serde_json::json!({
                "error": "Invalid or expired token"
            }))
        ))?;

    // Démarrer l'enregistrement WebAuthn
    let ccr = app.webauthn_manager.start_registration(
        &session.username,
        &session.username, // display_name = username par défaut
    ).map_err(|e| (
        StatusCode::INTERNAL_SERVER_ERROR,
        Json(serde_json::json!({
            "error": format!("Failed to start registration: {}", e)
        }))
    ))?;

    println!("[webauthn] Started registration for user '{}'", session.username);
    Ok(Json(ccr))
}

/// Registration finish request containing the friendly name and the public key credential.
#[derive(serde::Deserialize)]
pub(super) struct WebAuthnRegisterFinishRequest {
    friendly_name: String,
    credential: webauthn_rs::prelude::RegisterPublicKeyCredential,
}

/// POST /auth/webauthn/register-finish — Complete passkey registration and persist the credential.
#[utoipa::path(
    post,
    path = "/auth/webauthn/register-finish",
    tag = "Authentication",
    security(("bearer_auth" = [])),
    request_body = Object,
    responses(
        (status = 200, description = "Passkey registered successfully", body = Object),
        (status = 400, description = "Invalid credential or JSON body"),
        (status = 401, description = "Invalid or expired token")
    )
)]
pub(super) async fn webauthn_register_finish(
    State(app): State<AppState>,
    req: Request,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<serde_json::Value>)> {
    // Extraire headers et body
    let (parts, body) = req.into_parts();

    // Extraire le token JWT pour identifier l'utilisateur
    let token = parts
        .headers
        .get("authorization")
        .and_then(|h| h.to_str().ok())
        .and_then(|s| s.strip_prefix("Bearer "))
        .ok_or_else(|| (
            StatusCode::UNAUTHORIZED,
            Json(serde_json::json!({
                "error": "Missing or invalid Authorization header"
            }))
        ))?;

    let session = app.auth_manager.get_session_info(token)
        .map_err(|_| (
            StatusCode::UNAUTHORIZED,
            Json(serde_json::json!({
                "error": "Invalid or expired token"
            }))
        ))?;

    // Parser le body JSON
    let bytes = axum::body::to_bytes(body, usize::MAX).await
        .map_err(|_| (
            StatusCode::BAD_REQUEST,
            Json(serde_json::json!({ "error": "Invalid request body" }))
        ))?;

    // Log du JSON brut pour debugging
    if let Ok(json_str) = std::str::from_utf8(&bytes) {
        println!("[webauthn] Received JSON (first 500 chars): {}", &json_str.chars().take(500).collect::<String>());
    }

    let payload: WebAuthnRegisterFinishRequest = serde_json::from_slice(&bytes)
        .map_err(|e| {
            eprintln!("[webauthn] JSON parsing error: {}", e);
            (
                StatusCode::BAD_REQUEST,
                Json(serde_json::json!({ "error": format!("Invalid JSON body: {}", e) }))
            )
        })?;

    // Terminer l'enregistrement WebAuthn
    app.webauthn_manager.finish_registration(
        &session.username,
        &payload.credential,
        payload.friendly_name.clone(),
    ).map_err(|e| (
        StatusCode::BAD_REQUEST,
        Json(serde_json::json!({
            "error": format!("Failed to finish registration: {}", e)
        }))
    ))?;

    println!("[webauthn] ✅ Registered passkey '{}' for user '{}'", payload.friendly_name, session.username);
    Ok(Json(serde_json::json!({
        "success": true,
        "message": "Passkey registered successfully"
    })))
}

/// GET /auth/webauthn/passkeys - Lister les passkeys de l'utilisateur connecté
#[utoipa::path(
    get,
    path = "/auth/webauthn/passkeys",
    tag = "Authentication",
    security(("bearer_auth" = [])),
    responses(
        (status = 200, description = "List of user passkeys", body = Vec<Object>),
        (status = 401, description = "Invalid or expired token")
    )
)]
pub(super) async fn webauthn_list_passkeys(
    State(app): State<AppState>,
    req: Request,
) -> Result<Json<Vec<serde_json::Value>>, (StatusCode, Json<serde_json::Value>)> {
    let (parts, _body) = req.into_parts();

    // Extraire le token JWT pour identifier l'utilisateur
    let token = parts
        .headers
        .get("authorization")
        .and_then(|h| h.to_str().ok())
        .and_then(|s| s.strip_prefix("Bearer "))
        .ok_or_else(|| (
            StatusCode::UNAUTHORIZED,
            Json(serde_json::json!({
                "error": "Missing or invalid Authorization header"
            }))
        ))?;

    let session = app.auth_manager.get_session_info(token)
        .map_err(|_| (
            StatusCode::UNAUTHORIZED,
            Json(serde_json::json!({
                "error": "Invalid or expired token"
            }))
        ))?;

    // Récupérer la liste des passkeys pour cet utilisateur
    let passkeys = app.webauthn_manager.list_user_passkeys(&session.username);

    // Transformer en format JSON simplifié
    let passkeys_json: Vec<serde_json::Value> = passkeys.into_iter().map(|pk| {
        serde_json::json!({
            "credential_id": base64::engine::general_purpose::URL_SAFE_NO_PAD.encode(&pk.credential_id),
            "friendly_name": pk.friendly_name,
            "created_at": pk.created_at,
            "last_used_at": pk.last_used_at
        })
    }).collect();

    println!("[webauthn] Listed {} passkeys for user '{}'", passkeys_json.len(), session.username);
    Ok(Json(passkeys_json))
}

/// DELETE /auth/webauthn/passkeys/:credential_id - Supprimer une passkey
#[utoipa::path(
    delete,
    path = "/auth/webauthn/passkeys/{credential_id}",
    tag = "Authentication",
    security(("bearer_auth" = [])),
    params(("credential_id" = String, Path, description = "Base64 URL-safe encoded credential ID")),
    responses(
        (status = 200, description = "Passkey deleted successfully", body = Object),
        (status = 400, description = "Invalid credential_id format"),
        (status = 401, description = "Invalid or expired token"),
        (status = 404, description = "Passkey not found")
    )
)]
pub(super) async fn webauthn_delete_passkey(
    State(app): State<AppState>,
    Path(credential_id): Path<String>,
    req: Request,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<serde_json::Value>)> {
    let (parts, _body) = req.into_parts();

    // Extraire le token JWT pour identifier l'utilisateur
    let token = parts
        .headers
        .get("authorization")
        .and_then(|h| h.to_str().ok())
        .and_then(|s| s.strip_prefix("Bearer "))
        .ok_or_else(|| (
            StatusCode::UNAUTHORIZED,
            Json(serde_json::json!({
                "error": "Missing or invalid Authorization header"
            }))
        ))?;

    let session = app.auth_manager.get_session_info(token)
        .map_err(|_| (
            StatusCode::UNAUTHORIZED,
            Json(serde_json::json!({
                "error": "Invalid or expired token"
            }))
        ))?;

    // Décoder le credential_id depuis base64 URL-safe
    let credential_id_bytes = base64::engine::general_purpose::URL_SAFE_NO_PAD
        .decode(&credential_id)
        .map_err(|_| (
            StatusCode::BAD_REQUEST,
            Json(serde_json::json!({
                "error": "Invalid credential_id format"
            }))
        ))?;

    // Supprimer la passkey
    app.webauthn_manager.delete_passkey(&session.username, &credential_id_bytes)
        .map_err(|e| (
            StatusCode::NOT_FOUND,
            Json(serde_json::json!({
                "error": format!("Failed to delete passkey: {}", e)
            }))
        ))?;

    println!("[webauthn] 🗑️ Deleted passkey for user '{}'", session.username);
    Ok(Json(serde_json::json!({
        "success": true,
        "message": "Passkey deleted successfully"
    })))
}

/// Authentication start request containing the username to authenticate.
#[derive(serde::Deserialize, ToSchema)]
pub(crate) struct WebAuthnAuthenticateStartRequest {
    username: String,
}

/// POST /auth/webauthn/authenticate-start — Start passkey authentication for a given username.
#[utoipa::path(
    post,
    path = "/auth/webauthn/authenticate-start",
    tag = "Authentication",
    request_body = WebAuthnAuthenticateStartRequest,
    responses(
        (status = 200, description = "Authentication challenge created", body = Object),
        (status = 400, description = "Failed to start authentication")
    )
)]
pub(super) async fn webauthn_authenticate_start(
    State(app): State<AppState>,
    Json(payload): Json<WebAuthnAuthenticateStartRequest>,
) -> Result<Json<webauthn_rs::prelude::RequestChallengeResponse>, (StatusCode, Json<serde_json::Value>)> {
    // Démarrer l'authentification WebAuthn
    let rcr = app.webauthn_manager.start_authentication(&payload.username)
        .map_err(|e| (
            StatusCode::BAD_REQUEST,
            Json(serde_json::json!({
                "error": format!("Failed to start authentication: {}", e)
            }))
        ))?;

    println!("[webauthn] Started authentication for user '{}'", payload.username);
    Ok(Json(rcr))
}

/// POST /auth/webauthn/authenticate-discoverable-start — Start passwordless authentication using discoverable credentials.
#[utoipa::path(
    post,
    path = "/auth/webauthn/authenticate-discoverable-start",
    tag = "Authentication",
    responses(
        (status = 200, description = "Discoverable authentication challenge created", body = Object),
        (status = 400, description = "Failed to start discoverable authentication")
    )
)]
pub(super) async fn webauthn_authenticate_discoverable_start(
    State(app): State<AppState>,
) -> Result<Json<webauthn_rs::prelude::RequestChallengeResponse>, (StatusCode, Json<serde_json::Value>)> {
    // Démarrer l'authentification WebAuthn en mode découvrable (sans username)
    let rcr = app.webauthn_manager.start_discoverable_authentication()
        .map_err(|e| (
            StatusCode::BAD_REQUEST,
            Json(serde_json::json!({
                "error": format!("Failed to start discoverable authentication: {}", e)
            }))
        ))?;

    println!("[webauthn] Started discoverable authentication (passwordless)");
    Ok(Json(rcr))
}

/// Authentication finish request containing the signed public key credential.
#[derive(serde::Deserialize)]
pub(super) struct WebAuthnAuthenticateFinishRequest {
    credential: webauthn_rs::prelude::PublicKeyCredential,
}

/// POST /auth/webauthn/authenticate-finish — Complete passkey authentication and return a JWT token.
#[utoipa::path(
    post,
    path = "/auth/webauthn/authenticate-finish",
    tag = "Authentication",
    request_body = Object,
    responses(
        (status = 200, description = "Authentication successful, JWT token returned", body = Object),
        (status = 400, description = "Invalid request body"),
        (status = 401, description = "Authentication failed"),
        (status = 500, description = "Failed to create token")
    )
)]
pub(super) async fn webauthn_authenticate_finish(
    State(app): State<AppState>,
    req: Request,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<serde_json::Value>)> {
    // Extraire headers et body
    let (parts, body) = req.into_parts();

    // Extraire l'IP du client pour device trust (reserved for future device fingerprinting)
    let _client_ip = parts
        .headers
        .get("x-forwarded-for")
        .and_then(|v| v.to_str().ok())
        .and_then(|s| s.split(',').next())
        .unwrap_or("127.0.0.1")
        .to_string();

    // Parser le body JSON
    let bytes = axum::body::to_bytes(body, usize::MAX).await
        .map_err(|_| (
            StatusCode::BAD_REQUEST,
            Json(serde_json::json!({ "error": "Invalid request body" }))
        ))?;

    let payload: WebAuthnAuthenticateFinishRequest = serde_json::from_slice(&bytes)
        .map_err(|_| (
            StatusCode::BAD_REQUEST,
            Json(serde_json::json!({ "error": "Invalid JSON body" }))
        ))?;

    // Terminer l'authentification WebAuthn
    let username = app.webauthn_manager.finish_authentication(&payload.credential)
        .map_err(|e| (
            StatusCode::UNAUTHORIZED,
            Json(serde_json::json!({
                "error": format!("Authentication failed: {}", e)
            }))
        ))?;

    // Générer JWT token
    // Note: WebAuthn passkey est déjà le facteur de confiance biométrique,
    // pas besoin de device trust supplémentaire
    let token_data = app.auth_manager.create_token_for_user(&username)
        .map_err(|e| (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(serde_json::json!({
                "error": format!("Failed to create token: {}", e)
            }))
        ))?;

    // Récupérer le rôle de l'utilisateur
    let role = app.auth_manager.get_user(&username)
        .map(|user| user.role.clone())
        .unwrap_or_else(|| "user".to_string());

    println!("[webauthn] ✅ Authenticated user '{}' via passkey", username);
    Ok(Json(serde_json::json!({
        "token": token_data,
        "username": username,
        "role": role,
        "expires_at": OffsetDateTime::now_utc().unix_timestamp() + 86400 // 24h
    })))
}
