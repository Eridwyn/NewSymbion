/**
 * API REST SYMBION - Serveur HTTP principal du kernel
 *
 * RÔLE :
 * Ce module expose l'API REST sécurisée de Symbion pour interactions humaines.
 * Interface principale entre frontend/CLI et kernel backend.
 *
 * FONCTIONNEMENT :
 * - Serveur Axum sur port 8080 avec middleware auth API key
 * - Routes organisées en sous-modules : auth, agents, context, decision, notifications, system
 * - Sérialisation JSON automatique des réponses
 * - Gestion erreurs HTTP standardisée (404, 401, 500...)
 *
 * UTILITÉ DANS SYMBION :
 * - Interface humaine : dashboard web, CLI, outils admin
 * - Intégration externe : webhooks, monitoring, scripts
 * - Debug/administration : inspection état système en temps réel
 * - Data Ports : CRUD unifiée des données persistantes
 *
 * SÉCURITÉ :
 * - Header x-api-key obligatoire sur toutes routes sauf /health
 * - Validation côté middleware avant traitement métier
 * - Logs des tentatives d'accès non autorisé
 */

pub mod agents;
pub mod auth;
pub mod context;
pub mod decision;
pub mod files;
pub mod notifications;
pub mod system;

use axum::{routing::{get, post}, Json, Router};
use axum::http::{StatusCode, Method};
use tower_http::cors::CorsLayer;
use tower_http::timeout::TimeoutLayer;
use std::sync::Arc;
use crate::state::Shared;
use crate::notes_bridge::SharedNotesBridge;
use axum::middleware::{self, Next};
use axum::extract::{State, Request};
use axum::response::Response;
use utoipa::OpenApi;
use utoipa_swagger_ui::SwaggerUi;

/// Structured error response helper for consistent API error format.
/// Returns `(StatusCode, Json)` with fields: error (code), message (human-readable), status (number).
fn error_response(status: StatusCode, code: &str, message: impl std::fmt::Display) -> (StatusCode, Json<serde_json::Value>) {
    (status, Json(serde_json::json!({
        "error": code,
        "message": message.to_string(),
        "status": status.as_u16(),
    })))
}

#[derive(Clone)]
pub struct AppState {
    pub states: Shared<crate::models::HostsMap>,
    pub cfg: Shared<crate::config::HostsConfig>,
    pub contracts: crate::contracts::ContractRegistry,
    pub health_tracker: crate::health::HealthTracker,
    pub auth_manager: crate::auth::AuthManager,
    pub mfa_manager: std::sync::Arc<crate::mfa::MfaManager>,
    pub csrf_manager: std::sync::Arc<crate::csrf::CsrfManager>,
    pub device_trust_manager: std::sync::Arc<crate::device_trust::DeviceTrustManager>,
    pub webauthn_manager: std::sync::Arc<crate::webauthn::WebAuthnManager>,
    pub notes_bridge: Option<SharedNotesBridge>,
    pub agents: crate::agents::SharedAgentRegistry,
    pub context_engine: std::sync::Arc<crate::context::ContextEngine>,
    pub dashboard_events: crate::dashboard_events::DashboardEventPublisher,
    // Decision Engine PR3
    pub decision_engine: std::sync::Arc<crate::decision::DecisionEngine>,
    pub decision_validation_manager: std::sync::Arc<crate::decision::ValidationManager>,
    pub decision_override_manager: std::sync::Arc<crate::decision::OverrideManager>,
    pub decision_audit_manager: std::sync::Arc<crate::decision::AuditManager>,
    pub decision_agent_health_manager: std::sync::Arc<crate::decision::AgentHealthManager>,
    pub decision_metrics: std::sync::Arc<crate::decision::DecisionMetrics>,
    // F1: Environment Monitoring
    pub sensors: crate::sensors::SharedSensorRegistry,
    // Dynamic Plugin Routing
    pub plugin_registry: crate::plugin_proxy::PluginRegistry,
    // Automations Engine
    pub automations: Arc<crate::automations::AutomationStore>,
    pub automation_dispatcher: crate::automations::EventDispatcher,
    // Pending Action Registry for post-approval execution
    pub pending_action_registry: crate::automations::SharedPendingActionRegistry,
    // Dynamic Modes Registry
    pub mode_registry: crate::modes::SharedModeRegistry,
    // Schedule Registry for time-based mode changes
    pub schedule_registry: crate::schedule::SharedScheduleRegistry,
    // Notifications Manager (FCM, SMTP, MQTT→Telegram)
    pub notifications_manager: crate::notifications::SharedNotificationManager,
    // Notification Configuration Manager
    pub notification_config: crate::notification_config::SharedNotificationConfigManager,
    // Context Intelligence Engine
    pub context_intelligence: std::sync::Arc<crate::context_intelligence::ContextIntelligence>,
    // Feature Registry for data-driven intelligence (v2)
    pub feature_registry: crate::intelligence::SharedFeatureRegistry,
    // Inference Engine for case-based mode prediction (v2)
    pub inference_engine: crate::intelligence::SharedInferenceEngine,
    // Session Manager for hysteresis-based mode transitions (v2)
    pub session_manager: crate::intelligence::SharedSessionManager,
    // Trust Tracker for evolving action statistics
    pub trust_tracker: crate::decision::SharedTrustTracker,
    // Global IP-based rate limiter
    pub rate_limiter: crate::rate_limiter::RateLimitStore,
    // File Transfer Hub (HTTPS relay for agent file transfers)
    pub file_hub: Option<std::sync::Arc<crate::file_hub::FileHub>>,
}

async fn require_auth(
    State(app): State<AppState>,
    req: Request,
    next: Next
) -> Result<Response, StatusCode> {
    let path = req.uri().path();

    // Health check, auth routes, CA certificate, Swagger UI et routes publiques toujours accessibles
    if path.starts_with("/health") || path.starts_with("/auth") || path.starts_with("/ca-certificate")
        || path.starts_with("/swagger-ui") || path.starts_with("/api-docs")
        || path.starts_with("/public/") {
        return Ok(next.run(req).await);
    }

    // Vérifier 1: Authorization: Bearer {token}
    if let Some(auth_header) = req.headers().get("authorization") {
        if let Ok(auth_str) = auth_header.to_str() {
            if let Some(token) = auth_str.strip_prefix("Bearer ") {
                // Valider le JWT token
                if app.auth_manager.verify_token(token).is_ok() {
                    return Ok(next.run(req).await);
                }
            }
        }
    }

    // Vérifier 2: x-api-key (fallback pour compatibilité)
    let expected = std::env::var("SYMBION_API_KEY").unwrap_or_default();
    if !expected.is_empty() {
        let ok = req.headers()
            .get("x-api-key")
            .and_then(|v| v.to_str().ok())
            .map(|v| v == expected)
            .unwrap_or(false);

        if ok {
            return Ok(next.run(req).await);
        }
    }

    // Aucune authentification valide trouvée
    eprintln!("SECURITY: No valid authentication (JWT or API key) for {}", path);
    Err(StatusCode::UNAUTHORIZED)
}

/// Middleware pour vérifier le nonce CSRF sur les routes destructrices
async fn require_csrf(
    State(app): State<AppState>,
    req: Request,
    next: Next
) -> Result<Response, StatusCode> {
    // API key auth bypasses CSRF (not browser-sent, not vulnerable to CSRF)
    let expected_api_key = std::env::var("SYMBION_API_KEY").unwrap_or_default();
    if !expected_api_key.is_empty() {
        if let Some(key) = req.headers().get("x-api-key").and_then(|v| v.to_str().ok()) {
            if key == expected_api_key {
                return Ok(next.run(req).await);
            }
        }
    }

    // Extraire le header X-CSRF-Token
    let csrf_token = req
        .headers()
        .get("x-csrf-token")
        .and_then(|v| v.to_str().ok())
        .ok_or_else(|| {
            eprintln!("SECURITY: Missing X-CSRF-Token header for {}", req.uri().path());
            StatusCode::FORBIDDEN
        })?;

    // Extraire le username depuis le JWT token
    let username = req
        .headers()
        .get("authorization")
        .and_then(|v| v.to_str().ok())
        .and_then(|v| v.strip_prefix("Bearer "))
        .and_then(|token| app.auth_manager.verify_token(token).ok())
        .map(|claims| claims.sub)
        .ok_or_else(|| {
            eprintln!("SECURITY: No valid JWT token for CSRF verification");
            StatusCode::UNAUTHORIZED
        })?;

    // Vérifier et consommer le nonce CSRF
    match app.csrf_manager.verify_and_consume(csrf_token, &username) {
        Ok(true) => {
            println!("[csrf] Valid nonce consumed for user '{}'", username);
            Ok(next.run(req).await)
        }
        Ok(false) | Err(_) => {
            eprintln!("SECURITY: Invalid or expired CSRF token for user '{}'", username);
            Err(StatusCode::FORBIDDEN)
        }
    }
}

pub fn build_router(app_state: AppState) -> Router {
    // Routes publiques (sans version, sans auth, sans rate limit strict)
    let public_routes = Router::new()
        .route("/health", get(|| async { "ok" }))
        .route("/health/live", get(|| async { "ok" }))
        .route("/health/ready", get(system::health_readiness_check))
        .route("/system/health", get(system::get_system_health))
        // Metrics API (PR4 - public for monitoring tools)
        .route("/metrics", get(system::prometheus_metrics_endpoint))
        .route("/v1/metrics/agents", get(system::get_metrics_agents))
        .route("/v1/metrics/system", get(system::get_metrics_system))
        .route("/ca-certificate", get(auth::download_ca_certificate))
        .with_state(app_state.clone());

    // Route de login publique avec rate limiting strict (brute-force protection)
    // NOTE: Rate limiting désactivé pour localhost (tower_governor ne peut pas extraire l'IP)
    // Pour connexions réseau externes, le rate limiting dans auth.rs reste actif
    let login_route = Router::new()
        .route("/auth/login", post(auth::auth_login))
        // WebAuthn passkey authentication (public - used for login)
        .route("/auth/webauthn/authenticate-start", post(auth::webauthn_authenticate_start))
        .route("/auth/webauthn/authenticate-discoverable-start", post(auth::webauthn_authenticate_discoverable_start))
        .route("/auth/webauthn/authenticate-finish", post(auth::webauthn_authenticate_finish))
        .with_state(app_state.clone());

    // Route publique pour plugin service discovery (no auth - plugins register at startup)
    let plugin_registration_route = Router::new()
        .route("/plugins/register", post(crate::plugin_proxy::handle_plugin_registration))
        .with_state(app_state.clone());

    // File transfer data routes (token-authenticated, no JWT required — agents use transfer tokens)
    let file_transfer_data_routes = Router::new()
        .route("/v1/transfers/{id}/data", get(files::download_transfer_data).post(files::upload_transfer_data))
        .with_state(app_state.clone());

    // Routes d'authentification protégées (nécessitent JWT valide)
    let protected_auth_routes = Router::new()
        .route("/auth/verify", get(auth::auth_verify))
        .route("/auth/session", get(auth::auth_session))
        .route("/auth/logout", post(auth::auth_logout))
        .route("/auth/mfa/status", get(auth::mfa_status))
        .route("/auth/mfa/setup", post(auth::mfa_setup))
        .route("/auth/mfa/verify", post(auth::mfa_verify))
        .route("/auth/mfa/disable", post(auth::mfa_disable))
        .route("/auth/csrf/nonce", get(auth::csrf_generate_nonce))
        // WebAuthn passkey registration (protected - requires JWT)
        .route("/auth/webauthn/register-start", post(auth::webauthn_register_start))
        .route("/auth/webauthn/register-finish", post(auth::webauthn_register_finish))
        .route("/auth/webauthn/passkeys", get(auth::webauthn_list_passkeys))
        .route("/auth/webauthn/passkeys/{credential_id}", axum::routing::delete(auth::webauthn_delete_passkey))
        .with_state(app_state.clone())
        .layer(middleware::from_fn_with_state(app_state.clone(), require_auth));

    // Routes destructrices nécessitant protection CSRF (POST/DELETE)
    let csrf_protected_routes = Router::new()
        .route("/agents/{id}/shutdown", post(agents::agent_shutdown_endpoint))
        .route("/agents/{id}/reboot", post(agents::agent_reboot_endpoint))
        .route("/agents/{id}/hibernate", post(agents::agent_hibernate_endpoint))
        .route("/v1/agents/{id}/reconnect", post(agents::agent_reconnect_endpoint))
        .route("/agents/{id}/processes/{pid}/kill", post(agents::agent_kill_process_endpoint))
        .route("/agents/{id}/command", post(agents::agent_command_endpoint))
        .route("/agents/{id}/commands", post(agents::agent_commands_post_endpoint))
        .route("/commands/{command_id}/cancel", post(agents::cancel_command_endpoint))
        .route("/agents/{id}/services/{name}/{action}", post(agents::agent_service_control_endpoint))
        // Agent v2.5 feature endpoints (CSRF protected)
        .route("/agents/{id}/notify", post(agents::agent_notify_endpoint))
        .route("/agents/{id}/screenshot", post(agents::agent_screenshot_endpoint))
        .route("/agents/{id}/scheduled-tasks", post(agents::agent_create_scheduled_task_endpoint))
        .route("/agents/{id}/scheduled-tasks/{name}", axum::routing::delete(agents::agent_delete_scheduled_task_endpoint))
        .route("/agents/{id}/plugins/{plugin_id}/command", post(agents::agent_plugin_command_endpoint))
        .route("/v1/agents/{id}", axum::routing::delete(agents::delete_agent_endpoint))
        .route("/context/override", post(context::set_context_override))
        .route("/context/clear", post(context::clear_context_override))
        // Dynamic Modes API (write operations)
        .route("/modes", post(context::create_mode))
        .route("/modes/{id}", axum::routing::put(context::update_mode).delete(context::delete_mode))
        // Schedule API (write operations)
        .route("/schedule/rules", post(context::create_schedule_rule))
        .route("/schedule/rules/{id}", axum::routing::put(context::update_schedule_rule).delete(context::delete_schedule_rule))
        .route("/schedule/default", axum::routing::put(context::set_schedule_default_mode))
        // Notifications API (CSRF protected write operations)
        .route("/notifications", post(notifications::send_notification).delete(notifications::delete_all_notifications))
        .route("/notifications/{id}/acknowledge", post(notifications::acknowledge_notification))
        .route("/notifications/{id}", axum::routing::delete(notifications::delete_notification))
        .route("/notifications/tokens", post(notifications::register_fcm_token))
        // Notification Config API (CSRF protected)
        .route("/notification-types/{type_id}", axum::routing::put(notifications::update_notification_config))
        .route("/auth/reload", post(auth::auth_reload_users))
        .route("/v1/users", post(auth::create_user))
        .route("/v1/users/{username}", axum::routing::delete(auth::delete_user))
        .route("/v1/users/{username}/password", axum::routing::put(auth::update_user_password))
        // Plugin systemctl control routes
        .route("/v1/plugins/{name}/start", post(agents::start_plugin_systemctl))
        .route("/v1/plugins/{name}/stop", post(agents::stop_plugin_systemctl))
        .route("/v1/plugins/{name}/restart", post(agents::restart_plugin_systemctl))
        // File Transfer (POST upload, CSRF protected)
        .route("/agents/{id}/files/upload", post(files::upload_file_to_agent))
        .route("/agents/{id}/files/download", post(files::request_file_download))
        .route("/agents/{id}/files/{filename}", axum::routing::delete(files::delete_agent_file))
        // Sensor delete (soft delete, CSRF protected)
        .route("/environment/sensors/{sensor_id}", axum::routing::delete(agents::delete_sensor_endpoint))
        // Automations CRUD (CSRF protected)
        .route("/automations", post(crate::automations_http::create_automation))
        .route("/automations/{automation_id}", axum::routing::put(crate::automations_http::update_automation))
        .route("/automations/{automation_id}", axum::routing::delete(crate::automations_http::delete_automation))
        .route("/automations/{automation_id}/enable", axum::routing::patch(crate::automations_http::toggle_automation))
        .route("/automations/{automation_id}/test", post(crate::automations_http::test_automation))
        .route("/automations/{automation_id}/run", post(crate::automations_http::run_automation))
        .with_state(app_state.clone())
        .layer(middleware::from_fn_with_state(app_state.clone(), require_csrf));

    // Decision Engine routes (CSRF protection pour POST/DELETE)
    let decision_csrf_routes = Router::new()
        .route("/decision/evaluate", post(decision::decision_evaluate))
        .route("/decision/validation/{id}/resolve", post(decision::decision_resolve_validation))
        .route("/decision/validation/{id}", axum::routing::delete(decision::decision_delete_validation))
        .route("/decision/validations/expired", axum::routing::delete(decision::decision_delete_all_expired_validations))
        .route("/decision/override", post(decision::decision_create_override))
        .route("/decision/override/{id}", axum::routing::delete(decision::decision_revoke_override))
        .with_state(app_state.clone())
        .layer(middleware::from_fn_with_state(app_state.clone(), require_csrf));

    // Routes API standard avec rate limiting modéré
    let api_routes = Router::new()
        .route("/hosts", get(system::get_hosts))
        .route("/hosts/{id}", get(system::get_host))
        .route("/wake", post(system::wake))
        .route("/contracts", get(system::list_contracts))
        .route("/contracts/{name}", get(system::get_contract))
        .route("/plugins", get(crate::plugin_proxy::handle_list_plugins))
        .route("/v1/plugins/{name}/status", get(agents::get_plugin_systemctl_status))
        .route("/agents", get(agents::list_agents_endpoint))
        .route("/agents/latest-version", get(agents::agents_latest_version))
        .route("/agents/{id}", get(agents::get_agent_endpoint))
        .route("/agents/{id}/processes", get(agents::agent_processes_endpoint))
        .route("/agents/{id}/metrics", get(agents::agent_metrics_endpoint))
        .route("/agents/{id}/commands", get(agents::agent_commands_endpoint))
        .route("/commands/{command_id}/status", get(agents::command_status_endpoint))
        .route("/agents/{id}/services", get(agents::agent_services_endpoint))
        .route("/agents/{id}/commands/history", get(agents::agent_command_history_endpoint))
        .route("/agents/{id}/logs", get(agents::agent_logs_endpoint))
        // Agent v2.5 feature endpoints (read-only)
        .route("/agents/{id}/watchdog", get(agents::agent_watchdog_endpoint))
        .route("/agents/{id}/plugins", get(agents::agent_plugins_endpoint))
        .route("/agents/{id}/scheduled-tasks", get(agents::agent_scheduled_tasks_endpoint))
        .route("/context/current", get(context::get_context_current))
        .route("/context/history", get(context::get_context_history))
        .route("/context/stats", get(context::get_context_stats))
        // Note: /context/patterns removed - use /intelligence/patterns instead
        .route("/context/productivity", get(context::get_context_productivity))
        // Dynamic Modes API
        .route("/modes", get(context::list_modes))
        .route("/modes/{id}", get(context::get_mode))
        // Schedule API (read-only)
        .route("/schedule", get(context::get_schedule))
        .route("/schedule/rules", get(context::list_schedule_rules))
        .route("/schedule/current", get(context::get_current_schedule_mode))
        // Notifications API (read-only)
        .route("/notifications", get(notifications::list_notifications))
        .route("/notifications/active", get(notifications::list_active_notifications))
        .route("/notifications/tokens", get(notifications::list_fcm_tokens))
        // Notification Config API (read-only)
        .route("/notification-types", get(notifications::list_notification_configs))
        .route("/notification-types/{type_id}", get(notifications::get_notification_config))
        .route("/v1/users", get(auth::list_users))
        // Decision Engine API (read-only endpoints)
        .route("/decision/audit", get(decision::decision_get_audit))
        .route("/decision/metrics", get(decision::decision_get_metrics))
        .route("/decision/validations/pending", get(decision::decision_list_pending_validations))
        .route("/decision/validations/expired", get(decision::decision_list_expired_validations))
        .route("/decision/overrides/active", get(decision::decision_list_active_overrides))
        .route("/decision/config", get(decision::decision_get_config))
        .route("/decision/agent-health", get(decision::decision_get_agent_health))
        .route("/decision/stats", get(decision::decision_get_stats))
        // Automations API (read-only endpoints)
        .route("/automations", get(crate::automations_http::list_automations))
        .route("/automations/schema", get(crate::automations_http::get_automations_schema))
        .route("/automations/history", get(crate::automations_http::get_automations_history))
        .route("/automations/{automation_id}", get(crate::automations_http::get_automation))
        // File Transfer (read-only, JWT auth only)
        .route("/agents/{id}/files", get(files::list_agent_files))
        .route("/transfers/{id}/status", get(files::get_transfer_status))
        // Logs API
        .route("/logs", get(system::get_logs))
        .with_state(app_state.clone())
        .layer(middleware::from_fn_with_state(app_state.clone(), require_auth));
        // NOTE: Rate limiting tower_governor désactivé (incompatibilité localhost)
        // .layer(GovernorLayer::new(api_rate_limit_config));

    // Routes WebSocket (auth via query parameter, pas de middleware require_auth)
    let websocket_routes = Router::new()
        .route("/ws/notes/stream", get(crate::notes_ws::notes_stream_handler))
        .with_state(app_state.clone());

    // F1: Environment monitoring routes (protected by auth)
    let environment_routes = crate::environment_http::build_environment_routes(app_state.clone())
        .layer(middleware::from_fn_with_state(app_state.clone(), require_auth));

    // Context Intelligence routes (protected by auth)
    let intelligence_routes = crate::intelligence_http::intelligence_routes()
        .layer(middleware::from_fn_with_state(app_state.clone(), require_auth))
        .with_state(app_state.clone());

    // Dynamic Plugin Routing - fallback handler with auth middleware
    let plugin_router = Router::new()
        .fallback(crate::plugin_proxy::proxy_to_plugin)
        .layer(middleware::from_fn_with_state(app_state.clone(), require_auth))
        .with_state(app_state.clone());

    // Combine all v1 API routes
    let v1_api_routes = Router::new()
        .merge(login_route)
        .merge(plugin_registration_route)
        .merge(protected_auth_routes)
        .merge(api_routes)
        .merge(csrf_protected_routes)
        .merge(decision_csrf_routes)
        .merge(websocket_routes)
        .nest("/environment", environment_routes)
        .nest("/intelligence", intelligence_routes)
        .merge(plugin_router);

    // K8: Generate OpenAPI spec on a dedicated thread with 4 MiB stack
    // (utoipa derive generates deeply nested code that exceeds tokio's 2 MiB default)
    let openapi_spec = std::thread::Builder::new()
        .name("openapi-init".into())
        .stack_size(8 * 1024 * 1024)
        .spawn(|| crate::openapi::ApiDoc::openapi())
        .expect("failed to spawn openapi thread")
        .join()
        .expect("openapi thread panicked");

    // Public read-only library access (no auth, GET only)
    let public_library_routes = Router::new()
        .fallback(crate::plugin_proxy::public_library_proxy)
        .with_state(app_state.clone());

    // Router principal avec versioning
    Router::new()
        // K8: Swagger UI + OpenAPI JSON (public, no auth)
        .merge(SwaggerUi::new("/swagger-ui").url("/api-docs/openapi.json", openapi_spec))
        // Routes publiques (toujours accessibles)
        .merge(public_routes)
        // Public library (read-only, no auth) — must be before v1 nest to avoid plugin_router fallback
        .nest("/v1/public/library", public_library_routes)
        // File transfer data (token-authenticated, bypasses JWT for agent access)
        .merge(file_transfer_data_routes)
        // API v1 sous namespace /v1/
        .nest("/v1", v1_api_routes.clone())
        // Backward compatibility: routes à la racine (DEPRECATED, à supprimer en v0.3.0)
        .merge(v1_api_routes)
        // Middlewares globaux
        .layer(
            CorsLayer::new()
                .allow_origin([
                    "http://localhost:3000".parse().unwrap(),
                    "https://localhost:3000".parse().unwrap(),
                    "http://localhost:3002".parse().unwrap(),
                    "https://localhost:3002".parse().unwrap(),
                    "http://192.168.1.14:3000".parse().unwrap(),
                    "https://192.168.1.14:3000".parse().unwrap(),
                    "https://symbion.local:3000".parse().unwrap(),
                    "https://symbion.markcha.fr".parse().unwrap(), // Production domain
                    "https://192.168.1.14".parse().unwrap(), // Via Nginx reverse proxy (local)
                    "https://localhost".parse().unwrap(), // Via Nginx reverse proxy (localhost)
                ])
                .allow_methods([Method::GET, Method::POST, Method::DELETE, Method::PUT, Method::PATCH, Method::OPTIONS])
                .allow_headers([
                    axum::http::header::CONTENT_TYPE,
                    axum::http::header::AUTHORIZATION,
                    axum::http::header::ACCEPT,
                    axum::http::header::USER_AGENT,
                    axum::http::header::HeaderName::from_static("x-api-key"),
                    axum::http::header::HeaderName::from_static("x-csrf-token"),
                    axum::http::header::HeaderName::from_static("x-device-token"),
                ])
                .allow_credentials(true) // Requis pour cookies (historique, device_token maintenant via header)
        )
        // Rate limiting global IP-based (120 req/min, Cloudflare/nginx aware)
        .layer(middleware::from_fn_with_state(app_state.clone(), crate::rate_limiter::rate_limit_middleware))
        // Timeout de 30s pour toutes requêtes - Prévient blocages deadlock
        .layer(TimeoutLayer::new(std::time::Duration::from_secs(30)))
        // HSTS header - Force HTTPS, max-age 1 year
        .layer(middleware::from_fn(add_hsts_header))
        // CSP header - Content Security Policy (prévention XSS)
        .layer(middleware::from_fn(add_csp_header))
}

/// Middleware pour ajouter le header HSTS (HTTP Strict Transport Security)
/// Force les navigateurs à toujours utiliser HTTPS pour ce domaine pendant 1 an
async fn add_hsts_header(
    req: Request,
    next: Next,
) -> Response {
    let mut response = next.run(req).await;
    response.headers_mut().insert(
        axum::http::header::STRICT_TRANSPORT_SECURITY,
        "max-age=31536000; includeSubDomains".parse().unwrap()
    );
    response
}

/// Middleware pour ajouter le header CSP (Content Security Policy)
/// Prévient les attaques XSS en restreignant les sources de contenu autorisées
async fn add_csp_header(
    req: Request,
    next: Next,
) -> Response {
    let mut response = next.run(req).await;

    let csp_policy = "default-src 'none'; \
                      script-src 'self'; \
                      style-src 'self' 'unsafe-inline'; \
                      img-src 'self' data:; \
                      font-src 'self'; \
                      connect-src 'self' http: https: ws: wss:; \
                      manifest-src 'self'; \
                      base-uri 'self'; \
                      form-action 'self'; \
                      frame-ancestors 'none'";

    response.headers_mut().insert(
        axum::http::header::CONTENT_SECURITY_POLICY,
        csp_policy.parse().unwrap()
    );
    response
}

/// Router HTTP simple qui redirige toutes les requêtes vers HTTPS
/// Utilisé sur port 8080 pour rediriger vers port 8443 (HTTPS)
pub fn build_redirect_router(https_port: u16) -> Router {
    Router::new()
        .fallback(move |req: Request| async move {
            let host = req.headers()
                .get(axum::http::header::HOST)
                .and_then(|h| h.to_str().ok())
                .unwrap_or("localhost");

            // Retirer le port HTTP s'il est présent dans le host header
            let host_without_port = host.split(':').next().unwrap_or(host);

            let uri = req.uri();
            let path_and_query = uri.path_and_query()
                .map(|pq| pq.as_str())
                .unwrap_or("/");

            let https_url = format!("https://{}:{}{}", host_without_port, https_port, path_and_query);

            (
                StatusCode::MOVED_PERMANENTLY,
                [(axum::http::header::LOCATION, https_url)]
            )
        })
}
