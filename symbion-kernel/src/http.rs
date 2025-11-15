/**
 * API REST SYMBION - Serveur HTTP principal du kernel
 * 
 * RÔLE :
 * Ce module expose l'API REST sécurisée de Symbion pour interactions humaines.
 * Interface principale entre frontend/CLI et kernel backend.
 * 
 * FONCTIONNEMENT :
 * - Serveur Axum sur port 8080 avec middleware auth API key
 * - Routes organisées : /health, /system, /hosts, /contracts, /ports
 * - Sérialisation JSON automatique des réponses
 * - Gestion erreurs HTTP standardisée (404, 401, 500...)
 * 
 * UTILITÉ DANS SYMBION :
 * 🎯 Interface humaine : dashboard web, CLI, outils admin
 * 🎯 Intégration externe : webhooks, monitoring, scripts
 * 🎯 Debug/administration : inspection état système en temps réel
 * 🎯 Data Ports : CRUD unifiée des données persistantes
 * 
 * SÉCURITÉ :
 * - Header x-api-key obligatoire sur toutes routes sauf /health
 * - Validation côté middleware avant traitement métier
 * - Logs des tentatives d'accès non autorisé
 */

use axum::{extract::{Query, State}, routing::{get, post}, Json, Router};
use axum::http::{StatusCode, Method};
use tower_http::cors::{CorsLayer, Any};
use tower_http::timeout::TimeoutLayer;
use crate::models::{HostState, HostsMap};
use crate::state::Shared;
use crate::config::HostsConfig;
use crate::notes_bridge::{self, SharedNotesBridge};
use crate::wol::trigger_wol_udp;
use serde::Deserialize;
use axum::middleware::{self, Next};
use axum::extract::Request;
use axum::response::{Response, IntoResponse};
use time::{Duration, OffsetDateTime, format_description::well_known::Rfc3339};
use axum::extract::Path;
use std::collections::HashMap;
use sha2::Digest;
use base64::Engine;



#[derive(serde::Serialize)]
struct HostView {
    host_id: String,
    last_seen: String,       // format RFC3339 pour l’API
    stale: bool,             // true si > 90s
    stale_for_seconds: i64,  // âge en secondes
    cpu: Option<f32>,
    ram: Option<f32>,
    ip: Option<String>,
}

fn to_view(h: &HostState) -> HostView {
    let now = OffsetDateTime::now_utc();
    let age = now - h.last_seen;
    let secs = age.whole_seconds().max(0);
    HostView {
        host_id: h.host_id.clone(),
        last_seen: h.last_seen.format(&Rfc3339).unwrap_or_default(),
        stale: age > Duration::seconds(90),
        stale_for_seconds: secs,
        cpu: h.cpu,
        ram: h.ram,
        ip: h.ip.clone(),
    }
}

async fn require_auth(
    State(app): State<AppState>,
    req: Request,
    next: Next
) -> Result<Response, StatusCode> {
    let path = req.uri().path();

    // Health check, auth routes et CA certificate toujours accessibles
    if path.starts_with("/health") || path.starts_with("/auth") || path.starts_with("/ca-certificate") {
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

#[derive(Clone)]
pub struct AppState {
    pub states: Shared<HostsMap>,
    pub cfg: Shared<HostsConfig>,
    pub contracts: crate::contracts::ContractRegistry,
    pub health_tracker: crate::health::HealthTracker,
    pub auth_manager: crate::auth::AuthManager,
    pub mfa_manager: std::sync::Arc<crate::mfa::MfaManager>,
    pub csrf_manager: std::sync::Arc<crate::csrf::CsrfManager>,
    pub device_trust_manager: std::sync::Arc<crate::device_trust::DeviceTrustManager>,
    pub webauthn_manager: std::sync::Arc<crate::webauthn::WebAuthnManager>,
    pub ports: Shared<crate::ports::PortRegistry>,
    pub plugins: Shared<crate::plugins::PluginManager>,
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
}

#[derive(Debug, Deserialize)]
struct WakeParams { host_id: String }

pub fn build_router(app_state: AppState) -> Router {
    // Routes publiques (sans version, sans auth, sans rate limit strict)
    let public_routes = Router::new()
        .route("/health", get(|| async { "ok" }))
        .route("/system/health", get(get_system_health))
        // Metrics API (PR4 - public for monitoring tools)
        .route("/metrics", get(prometheus_metrics_endpoint))
        .route("/v1/metrics/agents", get(get_metrics_agents))
        .route("/v1/metrics/system", get(get_metrics_system))
        .route("/ca-certificate", get(download_ca_certificate))
        .with_state(app_state.clone());

    // Route de login publique avec rate limiting strict (brute-force protection)
    // NOTE: Rate limiting désactivé pour localhost (tower_governor ne peut pas extraire l'IP)
    // Pour connexions réseau externes, le rate limiting dans auth.rs reste actif
    let login_route = Router::new()
        .route("/auth/login", post(auth_login))
        // WebAuthn passkey authentication (public - used for login)
        .route("/auth/webauthn/authenticate-start", post(webauthn_authenticate_start))
        .route("/auth/webauthn/authenticate-discoverable-start", post(webauthn_authenticate_discoverable_start))
        .route("/auth/webauthn/authenticate-finish", post(webauthn_authenticate_finish))
        .with_state(app_state.clone());

    // Routes d'authentification protégées (nécessitent JWT valide)
    let protected_auth_routes = Router::new()
        .route("/auth/verify", get(auth_verify))
        .route("/auth/session", get(auth_session))
        .route("/auth/logout", post(auth_logout))
        .route("/auth/mfa/status", get(mfa_status))
        .route("/auth/mfa/setup", post(mfa_setup))
        .route("/auth/mfa/verify", post(mfa_verify))
        .route("/auth/mfa/disable", post(mfa_disable))
        .route("/auth/csrf/nonce", get(csrf_generate_nonce))
        // WebAuthn passkey registration (protected - requires JWT)
        .route("/auth/webauthn/register-start", post(webauthn_register_start))
        .route("/auth/webauthn/register-finish", post(webauthn_register_finish))
        .route("/auth/webauthn/passkeys", get(webauthn_list_passkeys))
        .with_state(app_state.clone())
        .layer(middleware::from_fn_with_state(app_state.clone(), require_auth));

    // Routes destructrices nécessitant protection CSRF (POST/DELETE)
    let csrf_protected_routes = Router::new()
        .route("/agents/{id}/shutdown", post(agent_shutdown_endpoint))
        .route("/agents/{id}/reboot", post(agent_reboot_endpoint))
        .route("/agents/{id}/hibernate", post(agent_hibernate_endpoint))
        .route("/agents/{id}/processes/{pid}/kill", post(agent_kill_process_endpoint))
        .route("/context/override", post(set_context_override))
        .route("/context/clear", post(clear_context_override))
        .route("/auth/reload", post(auth_reload_users))
        .route("/v1/users", post(create_user))
        .route("/v1/users/{username}", axum::routing::delete(delete_user))
        .route("/plugins/{name}/start", post(start_plugin_endpoint))
        .route("/plugins/{name}/stop", post(stop_plugin_endpoint))
        .route("/plugins/{name}/restart", post(restart_plugin_endpoint))
        .route("/ports/memo/{id}", axum::routing::delete(handle_memo_delete).put(handle_memo_update))
        .route("/ports/{port_name}/{id}", axum::routing::delete(delete_from_port))
        .with_state(app_state.clone())
        .layer(middleware::from_fn_with_state(app_state.clone(), require_csrf));

    // Decision Engine routes (CSRF protection pour POST/DELETE)
    let decision_csrf_routes = Router::new()
        .route("/decision/evaluate", post(decision_evaluate))
        .route("/decision/validation/{id}/resolve", post(decision_resolve_validation))
        .route("/decision/validation/{id}", axum::routing::delete(decision_delete_validation))
        .route("/decision/validations/expired", axum::routing::delete(decision_delete_all_expired_validations))
        .route("/decision/override", post(decision_create_override))
        .route("/decision/override/{id}", axum::routing::delete(decision_revoke_override))
        .with_state(app_state.clone())
        .layer(middleware::from_fn_with_state(app_state.clone(), require_csrf));

    // Routes API standard avec rate limiting modéré
    let api_routes = Router::new()
        .route("/hosts", get(get_hosts))
        .route("/hosts/{id}", get(get_host))
        .route("/wake", post(wake))
        .route("/contracts", get(list_contracts))
        .route("/contracts/{name}", get(get_contract))
        .route("/ports", get(list_ports))
        .route("/ports/memo", get(handle_memo_list).post(handle_memo_create))
        .route("/ports/{port_name}", get(read_from_port).post(write_to_port))
        .route("/plugins", get(list_plugins_endpoint))
        .route("/agents", get(list_agents_endpoint))
        .route("/agents/{id}", get(get_agent_endpoint))
        .route("/agents/{id}/processes", get(agent_processes_endpoint))
        .route("/agents/{id}/command", post(agent_command_endpoint))
        .route("/agents/{id}/metrics", get(agent_metrics_endpoint))
        .route("/agents/{id}/commands", get(agent_commands_endpoint).post(agent_commands_post_endpoint))
        .route("/commands/{command_id}/cancel", post(cancel_command_endpoint))
        .route("/commands/{command_id}/status", get(command_status_endpoint))
        .route("/context/current", get(get_context_current))
        .route("/context/history", get(get_context_history))
        .route("/context/stats", get(get_context_stats))
        .route("/context/patterns", get(get_context_patterns))
        .route("/context/productivity", get(get_context_productivity))
        .route("/v1/users", get(list_users))
        // Decision Engine API (read-only endpoints)
        .route("/decision/audit", get(decision_get_audit))
        .route("/decision/metrics", get(decision_get_metrics))
        .route("/decision/validations/pending", get(decision_list_pending_validations))
        .route("/decision/validations/expired", get(decision_list_expired_validations))
        .route("/decision/overrides/active", get(decision_list_active_overrides))
        .route("/decision/config", get(decision_get_config))
        .route("/decision/agent-health", get(decision_get_agent_health))
        .route("/decision/stats", get(decision_get_stats))
        .with_state(app_state.clone())
        .layer(middleware::from_fn_with_state(app_state.clone(), require_auth));
        // NOTE: Rate limiting tower_governor désactivé (incompatibilité localhost)
        // .layer(GovernorLayer::new(api_rate_limit_config));

    // Routes WebSocket (auth via query parameter, pas de middleware require_auth)
    let websocket_routes = Router::new()
        .route("/ws/notes/stream", get(crate::notes_ws::notes_stream_handler))
        .with_state(app_state.clone());

    // Combine all v1 API routes
    let v1_api_routes = Router::new()
        .merge(login_route)
        .merge(protected_auth_routes)
        .merge(api_routes)
        .merge(csrf_protected_routes)
        .merge(decision_csrf_routes)
        .merge(websocket_routes);

    // Router principal avec versioning
    Router::new()
        // Routes publiques (toujours accessibles)
        .merge(public_routes)
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
                    "http://192.168.1.14:3000".parse().unwrap(),
                    "https://192.168.1.14:3000".parse().unwrap(),
                    "https://symbion.local:3000".parse().unwrap(),
                ])
                .allow_methods([Method::GET, Method::POST, Method::DELETE, Method::PUT, Method::OPTIONS])
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
        // Timeout de 30s pour toutes requêtes - Prévient blocages deadlock
        .layer(TimeoutLayer::new(std::time::Duration::from_secs(30)))
        // HSTS header - Force HTTPS, max-age 1 year
        .layer(middleware::from_fn(add_hsts_header))
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

// GET /hosts (liste)
async fn get_hosts(State(app): State<AppState>) -> Json<Vec<HostView>> {
    let list: Vec<HostView> = {
        let states = app.states.lock();
        states.values().map(to_view).collect()
    }; // Lock libéré immédiatement
    Json(list)
}

// GET /hosts/:id (détail)
async fn get_host(
    State(app): State<AppState>,
    Path(id): Path<String>,
) -> Result<Json<HostView>, StatusCode> {
    let host_view = {
        let map = app.states.lock();
        let Some(h) = map.get(&id) else { return Err(StatusCode::NOT_FOUND); };
        to_view(h)
    }; // Lock libéré immédiatement
    Ok(Json(host_view))
}


async fn wake(
    State(app): State<AppState>,
    Query(params): Query<WakeParams>,
) -> (StatusCode, Json<serde_json::Value>) {
    // D'abord essayer avec les agents (système moderne)
    let agents = app.agents.list_agents().await;
    for agent in agents.values() {
        if agent.agent_id == params.host_id {
            // Utiliser l'adresse MAC de l'agent pour WoL
            let mac_str = format!("{}:{}:{}:{}:{}:{}",
                &params.host_id[0..2], &params.host_id[2..4], &params.host_id[4..6],
                &params.host_id[6..8], &params.host_id[8..10], &params.host_id[10..12]
            );
            
            return send_magic_packet(&mac_str).await;
        }
    }
    
    // Fallback vers ancien système hosts
    let cfg = app.cfg.lock().clone();
    let (code, msg) = trigger_wol_udp(&cfg, &params.host_id).await;
    (code, Json(serde_json::json!({ "ok": code == StatusCode::OK, "msg": msg })))
}

/// Envoie un magic packet WoL pour l'adresse MAC donnée
async fn send_magic_packet(mac: &str) -> (StatusCode, Json<serde_json::Value>) {
    use std::net::{Ipv4Addr, SocketAddrV4, UdpSocket};
    
    // Parse MAC address
    let hex: String = mac.chars().filter(|c| c.is_ascii_hexdigit()).collect();
    if hex.len() != 12 {
        return (StatusCode::BAD_REQUEST, Json(serde_json::json!({"ok": false, "msg": "invalid mac length"})));
    }
    
    let mut mac_bytes = [0u8; 6];
    for i in 0..6 {
        match u8::from_str_radix(&hex[i*2..i*2+2], 16) {
            Ok(byte) => mac_bytes[i] = byte,
            Err(_) => return (StatusCode::BAD_REQUEST, Json(serde_json::json!({"ok": false, "msg": "invalid mac format"})))
        }
    }
    
    // Create magic packet (6 x 0xFF + 16 x MAC)
    let mut packet = [0u8; 102];
    for i in 0..6 { packet[i] = 0xFF; }
    for i in 0..16 {
        let base = 6 + i*6;
        packet[base..base+6].copy_from_slice(&mac_bytes);
    }
    
    // Send UDP broadcast on ports 9 and 7
    let sock = match UdpSocket::bind(("0.0.0.0", 0)) {
        Ok(s) => s,
        Err(_) => return (StatusCode::INTERNAL_SERVER_ERROR, Json(serde_json::json!({"ok": false, "msg": "failed to bind socket"})))
    };
    
    if sock.set_broadcast(true).is_err() {
        return (StatusCode::INTERNAL_SERVER_ERROR, Json(serde_json::json!({"ok": false, "msg": "failed to enable broadcast"})));
    }
    
    let broadcast = Ipv4Addr::new(255, 255, 255, 255);
    let mut success = false;
    
    for port in [9u16, 7u16] {
        let addr = SocketAddrV4::new(broadcast, port);
        if sock.send_to(&packet, addr).is_ok() {
            success = true;
        }
    }
    
    if success {
        (StatusCode::OK, Json(serde_json::json!({"ok": true, "msg": "magic packet sent"})))
    } else {
        (StatusCode::INTERNAL_SERVER_ERROR, Json(serde_json::json!({"ok": false, "msg": "failed to send magic packet"})))
    }
}

// GET /contracts (liste)
async fn list_contracts(State(app): State<AppState>) -> Json<Vec<String>> {
    Json(app.contracts.list_contracts())
}

// GET /contracts/{name} (détail)
async fn get_contract(
    State(app): State<AppState>,
    Path(name): Path<String>,
) -> Result<Json<crate::contracts::Contract>, StatusCode> {
    match app.contracts.get_contract(&name) {
        Some(contract) => Ok(Json(contract.clone())),
        None => Err(StatusCode::NOT_FOUND),
    }
}

// GET /system/health (état infrastructure)
async fn get_system_health(State(app): State<AppState>) -> Json<crate::health::KernelHealth> {
    let health = app.health_tracker.get_health(&app.contracts, &app.agents, &app.plugins);
    Json(health)
}

// GET /context/current (mode contextuel actuel)
async fn get_context_current(State(app): State<AppState>) -> Result<Json<crate::context::ContextState>, StatusCode> {
    match app.context_engine.get_state() {
        Some(state) => Ok(Json(state)),
        None => Err(StatusCode::INTERNAL_SERVER_ERROR),
    }
}

// POST /context/override (forcer manuellement un mode)
#[derive(serde::Deserialize)]
struct ContextOverrideRequest {
    mode: String,  // "cravate", "intime", "neutre"
    duration_minutes: i64,
    reason: Option<String>,
}

async fn set_context_override(
    State(app): State<AppState>,
    Json(req): Json<ContextOverrideRequest>,
) -> Result<Json<crate::context::ContextState>, StatusCode> {
    use crate::context::Mode;

    let mode = match req.mode.to_lowercase().as_str() {
        "cravate" => Mode::Cravate,
        "intime" => Mode::Intime,
        "neutre" => Mode::Neutre,
        _ => return Err(StatusCode::BAD_REQUEST),
    };

    let reason = req.reason.unwrap_or_else(|| "Override manuel".to_string());

    match app.context_engine.set_override(mode, req.duration_minutes, reason) {
        Some(state) => Ok(Json(state)),
        None => Err(StatusCode::INTERNAL_SERVER_ERROR),
    }
}

// POST /context/clear (annuler l'override manuel)
async fn clear_context_override(State(app): State<AppState>) -> Result<Json<crate::context::ContextState>, StatusCode> {
    let agents_map = app.agents.list_agents().await;
    let agents_list: Vec<crate::agents::Agent> = agents_map.values().cloned().collect();

    match app.context_engine.clear_override(&agents_list) {
        Some(state) => Ok(Json(state)),
        None => Err(StatusCode::NO_CONTENT),  // Pas d'override actif
    }
}

// GET /context/history (historique des changements de mode)
async fn get_context_history(State(app): State<AppState>) -> Json<Vec<crate::context::ModeHistoryEntry>> {
    Json(app.context_engine.get_history())
}

// GET /context/stats (statistiques par mode)
async fn get_context_stats(State(app): State<AppState>) -> Json<Vec<crate::context::ModeStats>> {
    Json(app.context_engine.calculate_stats())
}

// GET /context/patterns (patterns détectés)
async fn get_context_patterns(State(app): State<AppState>) -> Json<Vec<crate::context::DetectedPattern>> {
    Json(app.context_engine.detect_patterns())
}

// GET /context/productivity (métriques de productivité par mode)
async fn get_context_productivity(State(app): State<AppState>) -> Json<Vec<crate::context::ProductivityMetrics>> {
    Json(app.context_engine.calculate_productivity())
}

// GET /ports (liste des ports disponibles)
async fn list_ports(State(app): State<AppState>) -> Json<Vec<crate::ports::PortInfo>> {
    let port_info = {
        let ports = app.ports.lock();
        ports.list_port_info()
    }; // Lock libéré immédiatement
    Json(port_info)
}

// GET /ports/{port_name} (lecture depuis un port avec query optionnelle)
async fn read_from_port(
    State(app): State<AppState>,
    Path(port_name): Path<String>,
    Query(params): Query<HashMap<String, String>>,
) -> Result<Json<Vec<crate::ports::PortData>>, StatusCode> {
    // Construction de la query depuis les paramètres URL
    let mut query = crate::ports::PortQuery::default();

    // Parsing des filtres depuis query params
    for (key, value) in params {
        match key.as_str() {
            "limit" => {
                if let Ok(limit) = value.parse::<usize>() {
                    query.limit = Some(limit);
                }
            }
            "offset" => {
                if let Ok(offset) = value.parse::<usize>() {
                    query.offset = Some(offset);
                }
            }
            "order_by" => {
                query.order_by = Some(value);
            }
            _ => {
                // Autres paramètres = filtres
                let filter_value = if value == "true" {
                    serde_json::Value::Bool(true)
                } else if value == "false" {
                    serde_json::Value::Bool(false)
                } else {
                    serde_json::Value::String(value)
                };
                query.filters.insert(key, filter_value);
            }
        }
    }

    // Obtenir le port et exécuter la query - Lock minimal
    let data = {
        let ports = app.ports.lock();
        let port = ports.get(&port_name)
            .ok_or(StatusCode::NOT_FOUND)?;
        port.read(&query)
    }; // Lock libéré immédiatement

    match data {
        Ok(data) => Ok(Json(data)),
        Err(_) => Err(StatusCode::INTERNAL_SERVER_ERROR),
    }
}

// POST /ports/{port_name} (écriture vers un port)
async fn write_to_port(
    State(app): State<AppState>,
    Path(port_name): Path<String>,
    Json(data): Json<serde_json::Value>,
) -> Result<Json<serde_json::Value>, StatusCode> {
    // Construction d'un PortData depuis le JSON reçu
    let port_data = crate::ports::PortData {
        id: String::new(), // L'ID sera généré automatiquement
        timestamp: time::OffsetDateTime::now_utc(),
        data: data,
        metadata: HashMap::new(),
    };

    // Écriture - Lock minimal
    let write_result = {
        let ports = app.ports.lock();
        let port = ports.get(&port_name)
            .ok_or(StatusCode::NOT_FOUND)?;
        port.write(&port_data)
    }; // Lock libéré immédiatement

    match write_result {
        Ok(id) => Ok(Json(serde_json::json!({"id": id, "status": "created"}))),
        Err(_) => Err(StatusCode::INTERNAL_SERVER_ERROR),
    }
}

// DELETE /ports/{port_name}/{id} (suppression depuis un port)
async fn delete_from_port(
    State(app): State<AppState>,
    Path((port_name, id)): Path<(String, String)>,
) -> Result<Json<serde_json::Value>, StatusCode> {
    // Suppression - Lock minimal
    let delete_result = {
        let ports = app.ports.lock();
        let port = ports.get(&port_name)
            .ok_or(StatusCode::NOT_FOUND)?;
        port.delete(&id)
    }; // Lock libéré immédiatement

    match delete_result {
        Ok(_) => Ok(Json(serde_json::json!({"status": "deleted"}))),
        Err(_) => Err(StatusCode::NOT_FOUND),
    }
}

// GET /plugins (liste des plugins avec leur état)
async fn list_plugins_endpoint(State(app): State<AppState>) -> Result<Json<Vec<crate::plugins::PluginInfo>>, StatusCode> {
    // Utiliser try_lock pour éviter deadlock avec health publisher
    let plugin_info = {
        let plugins = match app.plugins.try_lock() {
            Some(plugins) => plugins,
            None => {
                eprintln!("[http] plugin manager busy, try again later");
                return Err(StatusCode::SERVICE_UNAVAILABLE);
            }
        };
        plugins.list_plugins()
    }; // Lock libéré immédiatement

    Ok(Json(plugin_info))
}

// POST /plugins/{name}/start (démarre un plugin)
async fn start_plugin_endpoint(
    State(app): State<AppState>,
    Path(name): Path<String>,
) -> Result<Json<serde_json::Value>, StatusCode> {
    // Tentative de verrou non-bloquant avec timeout via try_lock
    let result = {
        let mut plugins = match app.plugins.try_lock() {
            Some(plugins) => plugins,
            None => {
                eprintln!("[http] plugin manager busy, try again later");
                return Err(StatusCode::SERVICE_UNAVAILABLE);
            }
        };
        plugins.start_plugin(&name)
    }; // Verrou libéré immédiatement
    
    match result {
        Ok(()) => Ok(Json(serde_json::json!({
            "plugin": name,
            "action": "start",
            "status": "success"
        }))),
        Err(e) => {
            eprintln!("[http] failed to start plugin {}: {}", name, e);
            Err(StatusCode::INTERNAL_SERVER_ERROR)
        }
    }
}

// POST /plugins/{name}/stop (arrête un plugin)
async fn stop_plugin_endpoint(
    State(app): State<AppState>,
    Path(name): Path<String>,
) -> Result<Json<serde_json::Value>, StatusCode> {
    let result = {
        let mut plugins = match app.plugins.try_lock() {
            Some(plugins) => plugins,
            None => {
                eprintln!("[http] plugin manager busy, try again later");
                return Err(StatusCode::SERVICE_UNAVAILABLE);
            }
        };
        plugins.stop_plugin(&name)
    };
    
    match result {
        Ok(()) => Ok(Json(serde_json::json!({
            "plugin": name,
            "action": "stop",
            "status": "success"
        }))),
        Err(e) => {
            eprintln!("[http] failed to stop plugin {}: {}", name, e);
            Err(StatusCode::INTERNAL_SERVER_ERROR)
        }
    }
}

// POST /plugins/{name}/restart (redémarre un plugin)
async fn restart_plugin_endpoint(
    State(app): State<AppState>,
    Path(name): Path<String>,
) -> Result<Json<serde_json::Value>, StatusCode> {
    let result = {
        let mut plugins = match app.plugins.try_lock() {
            Some(plugins) => plugins,
            None => {
                eprintln!("[http] plugin manager busy, try again later");
                return Err(StatusCode::SERVICE_UNAVAILABLE);
            }
        };
        plugins.restart_plugin(&name)
    };
    
    match result {
        Ok(()) => Ok(Json(serde_json::json!({
            "plugin": name,
            "action": "restart", 
            "status": "success"
        }))),
        Err(e) => {
            eprintln!("[http] failed to restart plugin {}: {}", name, e);
            Err(StatusCode::INTERNAL_SERVER_ERROR)
        }
    }
}

// ============ MEMO HANDLERS (Plugin Bridge Only) ============

async fn handle_memo_list(
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

async fn handle_memo_create(
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
                // Injecter automatiquement le mode contextuel actuel
                app.context_engine.get_state()
                    .map(|state| format!("{:?}", state.mode).to_lowercase())
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

async fn handle_memo_delete(
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

async fn handle_memo_update(
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

// ====== AGENTS ENDPOINTS ======

#[derive(serde::Serialize)]
struct AgentView {
    agent_id: String,
    hostname: String,
    os: String,
    architecture: String,
    capabilities: Vec<String>,
    primary_mac: String,
    primary_ip: String,
    status: String,
    last_seen: String,
    registration_time: String,
    uptime_seconds: Option<u64>,
    cpu_percent: Option<f32>,
    memory_percent: Option<f32>,
}

#[derive(Deserialize)]
struct AgentCommandRequest {
    command: String,
    parameters: Option<serde_json::Value>,
}

#[derive(Deserialize)]
struct AgentCommandTrackingRequest {
    command_type: String,
    parameters: serde_json::Value,
}

fn agent_to_view(agent: &crate::agents::Agent) -> AgentView {
    // Prefer IPv4 over IPv6 for display (IPv6 are too long for UI)
    let primary_ip = agent.network.interfaces
        .iter()
        .find(|i| !i.ip.contains(':'))  // IPv4 doesn't contain ':'
        .map(|i| i.ip.clone())
        .or_else(|| agent.network.interfaces.first().map(|i| i.ip.clone()))  // Fallback to any IP
        .unwrap_or_else(|| "unknown".to_string());

    AgentView {
        agent_id: agent.agent_id.clone(),
        hostname: agent.hostname.clone(),
        os: agent.os.clone(),
        architecture: agent.architecture.clone(),
        capabilities: agent.capabilities.clone(),
        primary_mac: agent.network.primary_mac.clone(),
        primary_ip,
        status: agent.status.status.clone(),
        last_seen: agent.last_seen.format(&Rfc3339).unwrap_or_default(),
        registration_time: agent.registration_time.format(&Rfc3339).unwrap_or_default(),
        uptime_seconds: agent.status.system.as_ref().map(|s| s.uptime_seconds),
        cpu_percent: agent.status.system.as_ref().map(|s| s.cpu.percent),
        memory_percent: agent.status.system.as_ref().map(|s| s.memory.percent_used),
    }
}

// GET /agents - Liste des agents
async fn list_agents_endpoint(State(app): State<AppState>) -> Json<Vec<AgentView>> {
    let agents = app.agents.list_agents().await;
    let list: Vec<AgentView> = agents.values().map(agent_to_view).collect();
    Json(list)
}

// GET /agents/{id} - Détail d'un agent
async fn get_agent_endpoint(
    State(app): State<AppState>,
    Path(id): Path<String>,
) -> Result<Json<crate::agents::Agent>, StatusCode> {
    match app.agents.get_agent(&id).await {
        Some(agent) => Ok(Json(agent)),
        None => Err(StatusCode::NOT_FOUND),
    }
}

// POST /agents/{id}/shutdown - Extinction système
async fn agent_shutdown_endpoint(
    State(app): State<AppState>,
    Path(id): Path<String>,
) -> Result<Json<serde_json::Value>, StatusCode> {
    match app.agents.send_command(&id, "shutdown", None).await {
        Ok(command_id) => Ok(Json(serde_json::json!({
            "success": true,
            "command_id": command_id,
            "message": "Shutdown command sent"
        }))),
        Err(e) => {
            eprintln!("[http] failed to send shutdown command to agent {}: {}", id, e);
            Err(StatusCode::INTERNAL_SERVER_ERROR)
        }
    }
}

// POST /agents/{id}/reboot - Redémarrage système  
async fn agent_reboot_endpoint(
    State(app): State<AppState>,
    Path(id): Path<String>,
) -> Result<Json<serde_json::Value>, StatusCode> {
    match app.agents.send_command(&id, "reboot", None).await {
        Ok(command_id) => Ok(Json(serde_json::json!({
            "success": true,
            "command_id": command_id,
            "message": "Reboot command sent"
        }))),
        Err(e) => {
            eprintln!("[http] failed to send reboot command to agent {}: {}", id, e);
            Err(StatusCode::INTERNAL_SERVER_ERROR)
        }
    }
}

// POST /agents/{id}/hibernate - Mise en veille
async fn agent_hibernate_endpoint(
    State(app): State<AppState>,
    Path(id): Path<String>,
) -> Result<Json<serde_json::Value>, StatusCode> {
    match app.agents.send_command(&id, "hibernate", None).await {
        Ok(command_id) => Ok(Json(serde_json::json!({
            "success": true,
            "command_id": command_id,
            "message": "Hibernate command sent"
        }))),
        Err(e) => {
            eprintln!("[http] failed to send hibernate command to agent {}: {}", id, e);
            Err(StatusCode::INTERNAL_SERVER_ERROR)
        }
    }
}

// GET /agents/{id}/processes - Liste des processus
async fn agent_processes_endpoint(
    State(app): State<AppState>,
    Path(id): Path<String>,
) -> Result<Json<serde_json::Value>, StatusCode> {
    match app.agents.get_agent(&id).await {
        Some(agent) => {
            if let Some(processes) = &agent.status.processes {
                Ok(Json(serde_json::to_value(processes).unwrap()))
            } else {
                // Demander les processus via MQTT
                match app.agents.send_command(&id, "list_processes", None).await {
                    Ok(command_id) => Ok(Json(serde_json::json!({
                        "success": true,
                        "command_id": command_id,
                        "message": "Process list requested, check agent status for results"
                    }))),
                    Err(e) => {
                        eprintln!("[http] failed to request processes from agent {}: {}", id, e);
                        Err(StatusCode::INTERNAL_SERVER_ERROR)
                    }
                }
            }
        }
        None => Err(StatusCode::NOT_FOUND),
    }
}

// POST /agents/{id}/processes/{pid}/kill - Tuer un processus
async fn agent_kill_process_endpoint(
    State(app): State<AppState>,
    Path((id, pid)): Path<(String, u32)>,
) -> Result<Json<serde_json::Value>, StatusCode> {
    let params = serde_json::json!({ "pid": pid });
    
    match app.agents.send_command(&id, "kill_process", Some(params)).await {
        Ok(command_id) => Ok(Json(serde_json::json!({
            "success": true,
            "command_id": command_id,
            "message": format!("Kill process {} command sent", pid)
        }))),
        Err(e) => {
            eprintln!("[http] failed to send kill process command to agent {}: {}", id, e);
            Err(StatusCode::INTERNAL_SERVER_ERROR)
        }
    }
}

// POST /agents/{id}/command - Exécuter une commande shell
async fn agent_command_endpoint(
    State(app): State<AppState>,
    Path(id): Path<String>,
    Json(req): Json<AgentCommandRequest>,
) -> Result<Json<serde_json::Value>, StatusCode> {
    let params = serde_json::json!({ 
        "command": req.command,
        "parameters": req.parameters
    });
    
    match app.agents.send_command(&id, "run_command", Some(params)).await {
        Ok(command_id) => Ok(Json(serde_json::json!({
            "success": true,
            "command_id": command_id,
            "message": "Command execution requested"
        }))),
        Err(e) => {
            eprintln!("[http] failed to send command to agent {}: {}", id, e);
            Err(StatusCode::INTERNAL_SERVER_ERROR)
        }
    }
}

// GET /agents/{id}/metrics - Métriques système temps réel
async fn agent_metrics_endpoint(
    State(app): State<AppState>,
    Path(id): Path<String>,
) -> Result<Json<serde_json::Value>, StatusCode> {
    match app.agents.get_agent(&id).await {
        Some(agent) => {
            if let Some(system) = &agent.status.system {
                Ok(Json(serde_json::to_value(system).unwrap()))
            } else {
                // Demander les métriques via MQTT
                match app.agents.send_command(&id, "get_metrics", None).await {
                    Ok(command_id) => Ok(Json(serde_json::json!({
                        "success": true,
                        "command_id": command_id,
                        "message": "Metrics requested, check agent status for results"
                    }))),
                    Err(e) => {
                        eprintln!("[http] failed to request metrics from agent {}: {}", id, e);
                        Err(StatusCode::INTERNAL_SERVER_ERROR)
                    }
                }
            }
        }
        None => Err(StatusCode::NOT_FOUND),
    }
}
// Nouveaux endpoints à ajouter à la fin de http.rs

// GET /agents/{id}/commands - Liste des commandes en cours pour un agent
async fn agent_commands_endpoint(
    State(app): State<AppState>,
    Path(id): Path<String>,
) -> Result<Json<serde_json::Value>, StatusCode> {
    let commands = app.agents.get_agent_pending_commands(&id).await;
    Ok(Json(serde_json::json!({
        "agent_id": id,
        "pending_commands": commands
    })))
}

// POST /agents/{id}/commands - Nouvelle API avec tracking pour exécuter des commandes
async fn agent_commands_post_endpoint(
    State(app): State<AppState>,
    Path(id): Path<String>,
    Json(req): Json<AgentCommandTrackingRequest>,
) -> Result<Json<serde_json::Value>, StatusCode> {
    // Extract command from parameters for shell_command type
    if req.command_type == "shell_command" {
        if let Some(command) = req.parameters.get("command") {
            if let Some(command_str) = command.as_str() {
                match app.agents.send_command(&id, "run_command", Some(req.parameters)).await {
                    Ok(command_id) => Ok(Json(serde_json::json!({
                        "success": true,
                        "command_id": command_id,
                        "message": "Command execution requested with tracking"
                    }))),
                    Err(e) => {
                        eprintln!("[http] failed to send tracked command to agent {}: {}", id, e);
                        Err(StatusCode::INTERNAL_SERVER_ERROR)
                    }
                }
            } else {
                Err(StatusCode::BAD_REQUEST)
            }
        } else {
            Err(StatusCode::BAD_REQUEST)
        }
    } else {
        // Handle other command types in the future
        match app.agents.send_command(&id, &req.command_type, Some(req.parameters)).await {
            Ok(command_id) => Ok(Json(serde_json::json!({
                "success": true,
                "command_id": command_id,
                "message": "Command execution requested with tracking"
            }))),
            Err(e) => {
                eprintln!("[http] failed to send tracked command to agent {}: {}", id, e);
                Err(StatusCode::INTERNAL_SERVER_ERROR)
            }
        }
    }
}

// POST /commands/{command_id}/cancel - Annule une commande
async fn cancel_command_endpoint(
    State(app): State<AppState>,
    Path(command_id): Path<String>,
) -> Result<Json<serde_json::Value>, StatusCode> {
    match app.agents.cancel_command(&command_id).await {
        Ok(cancelled) => {
            if cancelled {
                Ok(Json(serde_json::json!({
                    "success": true,
                    "command_id": command_id,
                    "message": "Command cancelled successfully"
                })))
            } else {
                Ok(Json(serde_json::json!({
                    "success": false,
                    "command_id": command_id,
                    "message": "Command cannot be cancelled (already completed or failed)"
                })))
            }
        }
        Err(e) => {
            eprintln!("[http] failed to cancel command {}: {}", command_id, e);
            Err(StatusCode::NOT_FOUND)
        }
    }
}

// GET /commands/{command_id}/status - Statut d'une commande
async fn command_status_endpoint(
    State(app): State<AppState>,
    Path(command_id): Path<String>,
) -> Result<Json<serde_json::Value>, StatusCode> {
    match app.agents.get_command_status(&command_id).await {
        Some(command) => Ok(Json(serde_json::json!({
            "command_id": command_id,
            "status": command.status,
            "output": command.output,
            "error": command.error
        }))),
        None => Err(StatusCode::NOT_FOUND),
    }
}

// =============== AUTH ENDPOINTS ===============

// POST /auth/login - Authentification utilisateur
async fn auth_login(
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
        .map(|s| {
            println!("[device-trust] Header X-Device-Token found: {}...", &s[..std::cmp::min(8, s.len())]);
            s.to_string()
        });

    if device_token.is_none() {
        println!("[device-trust] No X-Device-Token header found in request");
    }

    // Vérifier si le device est de confiance
    let trusted_device = if let Some(ref token) = device_token {
        let is_trusted = app.device_trust_manager.verify_device_token(
            token,
            &payload.username,
            &device_fingerprint
        );

        if is_trusted {
            println!("[device-trust] ✓ Device token valid for user '{}' - MFA will be bypassed", payload.username);
        } else {
            println!("[device-trust] ✗ Device token invalid or expired for user '{}'", payload.username);
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
                        println!("[auth] Device token created for user '{}' (30 days) - sent in JSON response", payload.username);
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
            Err((
                StatusCode::UNAUTHORIZED,
                Json(serde_json::json!({ "error": error_msg }))
            ))
        }
    }
}

// GET /auth/verify - Vérifier validité token (depuis header Authorization)
async fn auth_verify(
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

// GET /auth/session - Informations session courante
async fn auth_session(
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

// POST /auth/logout - Déconnexion (pour l'instant juste un success)
async fn auth_logout() -> Json<serde_json::Value> {
    // JWT est stateless - le client doit juste supprimer le token
    // On pourrait implémenter une blacklist de tokens pour invalidation côté serveur
    Json(serde_json::json!({
        "success": true,
        "message": "Logged out successfully"
    }))
}

/// POST /auth/reload - Recharger les utilisateurs depuis users.json sans redémarrer le kernel
async fn auth_reload_users(
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

#[derive(Debug, serde::Deserialize)]
struct CreateUserRequest {
    username: String,
    password: String,
    role: String,
}

/// POST /v1/users - Créer un nouvel utilisateur (admin seulement)
async fn create_user(
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
async fn delete_user(
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
async fn list_users(
    State(app): State<AppState>,
) -> Json<Vec<serde_json::Value>> {
    let users = app.auth_manager.list_users();
    Json(users)
}

// ============================================================================
// MFA (Multi-Factor Authentication) Endpoints
// ============================================================================

/// GET /v1/auth/mfa/status - Vérifier si MFA est activé pour l'utilisateur courant
async fn mfa_status(
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

#[derive(serde::Deserialize)]
struct MfaSetupRequest {
    #[serde(default)]
    recovery_email: Option<String>,
}

#[derive(serde::Serialize)]
struct MfaSetupResponse {
    secret: String,
    qr_code: String,
    backup_codes: Vec<String>,
}

/// POST /v1/auth/mfa/setup - Initialiser la configuration MFA (génère secret + QR code)
async fn mfa_setup(
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
                    println!("[mfa] Previous MFA setup expired for user '{}', generating new secret", username);
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

#[derive(serde::Deserialize)]
struct MfaVerifyRequest {
    code: String,
}

/// POST /v1/auth/mfa/verify - Vérifier un code TOTP et activer MFA
async fn mfa_verify(
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
async fn mfa_disable(
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
async fn csrf_generate_nonce(
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

// GET /ca-certificate - Téléchargement du certificat CA
async fn download_ca_certificate() -> Result<impl IntoResponse, StatusCode> {
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

// ========================================================================
// DECISION ENGINE ENDPOINTS - Wrappers vers decision_http.rs
// ========================================================================

use crate::decision_http::{EvaluateRequest, AuditQueryParams, ResolveValidationRequest, CreateOverrideRequest, RevokeOverrideRequest};

async fn decision_evaluate(
    State(app): State<AppState>,
    Json(req): Json<EvaluateRequest>,
) -> Json<crate::decision::DecisionResult> {
    let state = crate::decision_http::DecisionEngineState {
        engine: app.decision_engine.clone(),
        validation_manager: app.decision_validation_manager.clone(),
        override_manager: app.decision_override_manager.clone(),
        audit_manager: app.decision_audit_manager.clone(),
        agent_health_manager: app.decision_agent_health_manager.clone(),
        metrics: app.decision_metrics.clone(),
    };
    crate::decision_http::evaluate_action(State(state), Json(req)).await
}

async fn decision_get_audit(
    State(app): State<AppState>,
    Query(params): Query<AuditQueryParams>,
) -> Json<serde_json::Value> {
    let state = crate::decision_http::DecisionEngineState {
        engine: app.decision_engine.clone(),
        validation_manager: app.decision_validation_manager.clone(),
        override_manager: app.decision_override_manager.clone(),
        audit_manager: app.decision_audit_manager.clone(),
        agent_health_manager: app.decision_agent_health_manager.clone(),
        metrics: app.decision_metrics.clone(),
    };
    crate::decision_http::get_audit_trail(State(state), Query(params)).await
}

async fn decision_get_metrics(
    State(app): State<AppState>,
) -> Result<String, StatusCode> {
    let state = crate::decision_http::DecisionEngineState {
        engine: app.decision_engine.clone(),
        validation_manager: app.decision_validation_manager.clone(),
        override_manager: app.decision_override_manager.clone(),
        audit_manager: app.decision_audit_manager.clone(),
        agent_health_manager: app.decision_agent_health_manager.clone(),
        metrics: app.decision_metrics.clone(),
    };
    crate::decision_http::get_metrics(State(state)).await
}

async fn decision_list_pending_validations(
    State(app): State<AppState>,
) -> Json<Vec<crate::decision::ValidationRequest>> {
    let state = crate::decision_http::DecisionEngineState {
        engine: app.decision_engine.clone(),
        validation_manager: app.decision_validation_manager.clone(),
        override_manager: app.decision_override_manager.clone(),
        audit_manager: app.decision_audit_manager.clone(),
        agent_health_manager: app.decision_agent_health_manager.clone(),
        metrics: app.decision_metrics.clone(),
    };
    crate::decision_http::list_pending_validations(State(state)).await
}

async fn decision_resolve_validation(
    State(app): State<AppState>,
    Path(validation_id): Path<String>,
    Json(req): Json<ResolveValidationRequest>,
) -> Result<Json<crate::decision::ValidationRequest>, StatusCode> {
    let state = crate::decision_http::DecisionEngineState {
        engine: app.decision_engine.clone(),
        validation_manager: app.decision_validation_manager.clone(),
        override_manager: app.decision_override_manager.clone(),
        audit_manager: app.decision_audit_manager.clone(),
        agent_health_manager: app.decision_agent_health_manager.clone(),
        metrics: app.decision_metrics.clone(),
    };
    crate::decision_http::resolve_validation(State(state), Path(validation_id), Json(req)).await
}

async fn decision_create_override(
    State(app): State<AppState>,
    Json(req): Json<CreateOverrideRequest>,
) -> Result<Json<crate::decision::MasterOverride>, StatusCode> {
    let state = crate::decision_http::DecisionEngineState {
        engine: app.decision_engine.clone(),
        validation_manager: app.decision_validation_manager.clone(),
        override_manager: app.decision_override_manager.clone(),
        audit_manager: app.decision_audit_manager.clone(),
        agent_health_manager: app.decision_agent_health_manager.clone(),
        metrics: app.decision_metrics.clone(),
    };
    crate::decision_http::create_override(State(state), Json(req)).await
}

async fn decision_list_active_overrides(
    State(app): State<AppState>,
) -> Json<Vec<crate::decision::MasterOverride>> {
    let state = crate::decision_http::DecisionEngineState {
        engine: app.decision_engine.clone(),
        validation_manager: app.decision_validation_manager.clone(),
        override_manager: app.decision_override_manager.clone(),
        audit_manager: app.decision_audit_manager.clone(),
        agent_health_manager: app.decision_agent_health_manager.clone(),
        metrics: app.decision_metrics.clone(),
    };
    crate::decision_http::list_active_overrides(State(state)).await
}

async fn decision_revoke_override(
    State(app): State<AppState>,
    Path(override_id): Path<String>,
    Json(req): Json<RevokeOverrideRequest>,
) -> Result<StatusCode, StatusCode> {
    let state = crate::decision_http::DecisionEngineState {
        engine: app.decision_engine.clone(),
        validation_manager: app.decision_validation_manager.clone(),
        override_manager: app.decision_override_manager.clone(),
        audit_manager: app.decision_audit_manager.clone(),
        agent_health_manager: app.decision_agent_health_manager.clone(),
        metrics: app.decision_metrics.clone(),
    };
    crate::decision_http::revoke_override(State(state), Path(override_id), Json(req)).await
}

async fn decision_get_config(
    State(app): State<AppState>,
) -> Json<crate::decision::DecisionConfig> {
    let state = crate::decision_http::DecisionEngineState {
        engine: app.decision_engine.clone(),
        validation_manager: app.decision_validation_manager.clone(),
        override_manager: app.decision_override_manager.clone(),
        audit_manager: app.decision_audit_manager.clone(),
        agent_health_manager: app.decision_agent_health_manager.clone(),
        metrics: app.decision_metrics.clone(),
    };
    crate::decision_http::get_config(State(state)).await
}

async fn decision_get_agent_health(
    State(app): State<AppState>,
) -> Json<serde_json::Value> {
    let state = crate::decision_http::DecisionEngineState {
        engine: app.decision_engine.clone(),
        validation_manager: app.decision_validation_manager.clone(),
        override_manager: app.decision_override_manager.clone(),
        audit_manager: app.decision_audit_manager.clone(),
        agent_health_manager: app.decision_agent_health_manager.clone(),
        metrics: app.decision_metrics.clone(),
    };
    crate::decision_http::get_agent_health(State(state)).await
}

async fn decision_get_stats(
    State(app): State<AppState>,
) -> Json<crate::decision_http::DecisionStats> {
    let state = crate::decision_http::DecisionEngineState {
        engine: app.decision_engine.clone(),
        validation_manager: app.decision_validation_manager.clone(),
        override_manager: app.decision_override_manager.clone(),
        audit_manager: app.decision_audit_manager.clone(),
        agent_health_manager: app.decision_agent_health_manager.clone(),
        metrics: app.decision_metrics.clone(),
    };
    crate::decision_http::get_stats(State(state)).await
}

async fn decision_list_expired_validations(
    State(app): State<AppState>,
) -> Json<Vec<crate::decision::ValidationRequest>> {
    let state = crate::decision_http::DecisionEngineState {
        engine: app.decision_engine.clone(),
        validation_manager: app.decision_validation_manager.clone(),
        override_manager: app.decision_override_manager.clone(),
        audit_manager: app.decision_audit_manager.clone(),
        agent_health_manager: app.decision_agent_health_manager.clone(),
        metrics: app.decision_metrics.clone(),
    };
    crate::decision_http::list_expired_validations(State(state)).await
}

async fn decision_delete_validation(
    State(app): State<AppState>,
    Path(validation_id): Path<String>,
) -> Result<StatusCode, StatusCode> {
    let state = crate::decision_http::DecisionEngineState {
        engine: app.decision_engine.clone(),
        validation_manager: app.decision_validation_manager.clone(),
        override_manager: app.decision_override_manager.clone(),
        audit_manager: app.decision_audit_manager.clone(),
        agent_health_manager: app.decision_agent_health_manager.clone(),
        metrics: app.decision_metrics.clone(),
    };
    crate::decision_http::delete_validation(State(state), Path(validation_id)).await
}

async fn decision_delete_all_expired_validations(
    State(app): State<AppState>,
) -> Json<serde_json::Value> {
    let state = crate::decision_http::DecisionEngineState {
        engine: app.decision_engine.clone(),
        validation_manager: app.decision_validation_manager.clone(),
        override_manager: app.decision_override_manager.clone(),
        audit_manager: app.decision_audit_manager.clone(),
        agent_health_manager: app.decision_agent_health_manager.clone(),
        metrics: app.decision_metrics.clone(),
    };
    crate::decision_http::delete_all_expired_validations(State(state)).await
}

// ============================================================================
// WebAuthn Biometric Authentication Endpoints
// ============================================================================

/// POST /auth/webauthn/register-start - Démarrer l'enregistrement d'une passkey
/// Protégé par JWT - l'utilisateur doit être authentifié pour ajouter une passkey
#[derive(serde::Deserialize)]
struct WebAuthnRegisterStartRequest {
    friendly_name: String, // Ex: "iPhone 15 Pro", "Windows Hello"
}

async fn webauthn_register_start(
    State(app): State<AppState>,
    req: Request,
) -> Result<Json<webauthn_rs::prelude::CreationChallengeResponse>, (StatusCode, Json<serde_json::Value>)> {
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

/// POST /auth/webauthn/register-finish - Terminer l'enregistrement d'une passkey
#[derive(serde::Deserialize)]
struct WebAuthnRegisterFinishRequest {
    friendly_name: String,
    credential: webauthn_rs::prelude::RegisterPublicKeyCredential,
}

async fn webauthn_register_finish(
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
async fn webauthn_list_passkeys(
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

/// POST /auth/webauthn/authenticate-start - Démarrer l'authentification avec passkey
#[derive(serde::Deserialize)]
struct WebAuthnAuthenticateStartRequest {
    username: String,
}

async fn webauthn_authenticate_start(
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

/// POST /auth/webauthn/authenticate-discoverable-start - Démarrer l'authentification sans username
/// Mode "discoverable credentials" : l'authenticator présente toutes les passkeys disponibles
async fn webauthn_authenticate_discoverable_start(
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

/// POST /auth/webauthn/authenticate-finish - Terminer l'authentification avec passkey
/// Retourne un JWT token si succès
#[derive(serde::Deserialize)]
struct WebAuthnAuthenticateFinishRequest {
    credential: webauthn_rs::prelude::PublicKeyCredential,
}

async fn webauthn_authenticate_finish(
    State(app): State<AppState>,
    req: Request,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<serde_json::Value>)> {
    // Extraire headers et body
    let (parts, body) = req.into_parts();

    // Extraire l'IP du client pour device trust
    let client_ip = parts
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

// ============================================================================
// Metrics API Endpoints (PR4 - P1)
// ============================================================================

/// GET /v1/metrics/agents - Per-agent metrics in JSON format
/// Returns detailed telemetry for each agent: CPU, RAM, disk, network, processes
#[derive(serde::Serialize)]
struct AgentMetrics {
    agent_id: String,
    hostname: String,
    status: String,
    last_seen: i64, // Unix timestamp
    uptime_seconds: u64,
    cpu: AgentCpuMetrics,
    memory: AgentMemoryMetrics,
    disk: Vec<AgentDiskMetrics>,
    network: Vec<AgentNetworkMetrics>,
    processes: AgentProcessMetrics,
}

#[derive(serde::Serialize)]
struct AgentCpuMetrics {
    percent: f32,
    load_avg: Vec<f32>,
    core_count: u32,
}

#[derive(serde::Serialize)]
struct AgentMemoryMetrics {
    total_mb: u64,
    used_mb: u64,
    available_mb: u64,
    percent_used: f32,
}

#[derive(serde::Serialize)]
struct AgentDiskMetrics {
    path: String,
    total_gb: f64,
    used_gb: f64,
    free_gb: f64,
    percent_used: f32,
}

#[derive(serde::Serialize)]
struct AgentNetworkMetrics {
    name: String,
    bytes_sent: u64,
    bytes_recv: u64,
    is_up: bool,
}

#[derive(serde::Serialize)]
struct AgentProcessMetrics {
    total_count: u32,
    running_count: u32,
}

async fn get_metrics_agents(
    State(app): State<AppState>,
) -> Json<Vec<AgentMetrics>> {
    let agents_map = app.agents.list_agents().await;

    let mut metrics = Vec::new();
    for (agent_id, agent) in agents_map.iter() {
        let agent_metric = AgentMetrics {
            agent_id: agent_id.clone(),
            hostname: agent.hostname.clone(),
            status: agent.status.status.clone(),
            last_seen: agent.last_seen.unix_timestamp(),
            uptime_seconds: agent.status.system.as_ref()
                .map(|s| s.uptime_seconds)
                .unwrap_or(0),
            cpu: AgentCpuMetrics {
                percent: agent.status.system.as_ref()
                    .map(|s| s.cpu.percent)
                    .unwrap_or(0.0),
                load_avg: agent.status.system.as_ref()
                    .and_then(|s| s.cpu.load_avg.map(|arr| arr.to_vec()))
                    .unwrap_or_default(),
                core_count: agent.status.system.as_ref()
                    .and_then(|s| s.cpu.core_count)
                    .unwrap_or(0),
            },
            memory: AgentMemoryMetrics {
                total_mb: agent.status.system.as_ref()
                    .map(|s| s.memory.total_mb)
                    .unwrap_or(0),
                used_mb: agent.status.system.as_ref()
                    .map(|s| s.memory.used_mb)
                    .unwrap_or(0),
                available_mb: agent.status.system.as_ref()
                    .and_then(|s| s.memory.available_mb)
                    .unwrap_or(0),
                percent_used: agent.status.system.as_ref()
                    .map(|s| s.memory.percent_used)
                    .unwrap_or(0.0),
            },
            disk: agent.status.system.as_ref()
                .and_then(|s| s.disk.as_ref())
                .map(|disks| disks.iter().map(|d| AgentDiskMetrics {
                    path: d.path.clone(),
                    total_gb: d.total_gb,
                    used_gb: d.used_gb,
                    free_gb: d.free_gb.unwrap_or(0.0),
                    percent_used: d.percent_used,
                }).collect())
                .unwrap_or_default(),
            network: agent.status.system.as_ref()
                .and_then(|s| s.network.as_ref())
                .map(|n| n.interfaces.iter().map(|i| AgentNetworkMetrics {
                    name: i.name.clone(),
                    bytes_sent: i.bytes_sent.unwrap_or(0),
                    bytes_recv: i.bytes_recv.unwrap_or(0),
                    is_up: i.is_up,
                }).collect())
                .unwrap_or_default(),
            processes: AgentProcessMetrics {
                total_count: agent.status.processes.as_ref()
                    .map(|p| p.total_count)
                    .unwrap_or(0),
                running_count: agent.status.processes.as_ref()
                    .map(|p| p.running_count)
                    .unwrap_or(0),
            },
        };
        metrics.push(agent_metric);
    }

    Json(metrics)
}

/// GET /v1/metrics/system - Kernel performance metrics in JSON format
/// Returns kernel runtime stats: uptime, memory, MQTT, plugins, context
#[derive(serde::Serialize)]
struct SystemMetrics {
    kernel: KernelRuntimeMetrics,
    mqtt: MqttMetrics,
    agents: AgentsSummaryMetrics,
    plugins: PluginsMetrics,
    context: ContextMetrics,
    decision_engine: DecisionEngineMetrics,
}

#[derive(serde::Serialize)]
struct KernelRuntimeMetrics {
    uptime_seconds: u64,
    memory_usage_mb: f32,
    contracts_loaded: u32,
}

#[derive(serde::Serialize)]
struct MqttMetrics {
    status: String, // "connected", "disconnected", "reconnecting"
    reconnects_total: u32,
    messages_per_minute: f32,
    messages_total: u64,
}

#[derive(serde::Serialize)]
struct AgentsSummaryMetrics {
    total: usize,
    online: usize,
    offline: usize,
}

#[derive(serde::Serialize)]
struct PluginsMetrics {
    total: u32,
    running: u32,
    failed: u32,
}

#[derive(serde::Serialize)]
struct ContextMetrics {
    current_mode: String, // "neutre", "cravate", "intime"
    confidence: f32,
}

#[derive(serde::Serialize)]
struct DecisionEngineMetrics {
    decisions_total: u64,
    decisions_approved: u64,
    decisions_blocked: u64,
    validations_pending: usize,
    overrides_active: usize,
}

async fn get_metrics_system(
    State(app): State<AppState>,
) -> Json<SystemMetrics> {
    // Get kernel health
    let kernel_health = app.health_tracker.get_health(&app.contracts, &app.agents, &app.plugins);

    // Get agent summary
    let agents_map = app.agents.list_agents().await;
    let total_agents = agents_map.len();
    let online_agents = agents_map.values()
        .filter(|a| a.status.status == "online")
        .count();

    // Get context state
    let context_state = app.context_engine.get_state();
    let (mode_name, mode_confidence) = if let Some(state) = context_state {
        use crate::context::Mode;
        let name = match state.mode {
            Mode::Neutre => "neutre",
            Mode::Cravate => "cravate",
            Mode::Intime => "intime",
        };
        (name.to_string(), state.confidence)
    } else {
        ("unknown".to_string(), 0.0)
    };

    // Get decision engine stats
    let validation_stats = app.decision_validation_manager.stats();
    let override_stats = app.decision_override_manager.stats();

    // Decision metrics counters
    let decisions_total = app.decision_metrics.get_decisions_total();
    let decisions_approved = app.decision_metrics.get_decisions_approved();
    let decisions_blocked = app.decision_metrics.get_decisions_blocked();

    let metrics = SystemMetrics {
        kernel: KernelRuntimeMetrics {
            uptime_seconds: kernel_health.uptime_seconds,
            memory_usage_mb: kernel_health.memory_usage_mb,
            contracts_loaded: kernel_health.contracts_loaded,
        },
        mqtt: MqttMetrics {
            status: kernel_health.mqtt_status,
            reconnects_total: kernel_health.mqtt_reconnects,
            messages_per_minute: kernel_health.mqtt_messages_per_minute,
            messages_total: kernel_health.mqtt_messages_total,
        },
        agents: AgentsSummaryMetrics {
            total: total_agents,
            online: online_agents,
            offline: total_agents - online_agents,
        },
        plugins: PluginsMetrics {
            total: kernel_health.plugins_total,
            running: kernel_health.plugins_active,
            failed: kernel_health.plugins_failed,
        },
        context: ContextMetrics {
            current_mode: mode_name,
            confidence: mode_confidence,
        },
        decision_engine: DecisionEngineMetrics {
            decisions_total,
            decisions_approved,
            decisions_blocked,
            validations_pending: validation_stats.pending,
            overrides_active: override_stats.active,
        },
    };

    Json(metrics)
}

// ============================================================================
// Prometheus Metrics Endpoint (PR4 - P0)
// ============================================================================

/// GET /metrics - Prometheus scraping endpoint (public, no auth required)
/// Exports all kernel metrics in Prometheus exposition format:
/// - Decision Engine metrics (decisions, guards, validations)
/// - Agent telemetry (count, online/offline status)
/// - System metrics (MQTT status, uptime)
/// - HTTP metrics (placeholder for future request counters)
async fn prometheus_metrics_endpoint(
    State(app): State<AppState>,
) -> Result<String, StatusCode> {
    let mut output = String::new();

    // ========== Decision Engine Metrics ==========
    let audit_stats = app.decision_audit_manager.stats();
    let validation_stats = app.decision_validation_manager.stats();
    let override_stats = app.decision_override_manager.stats();
    let agent_health_stats = app.decision_agent_health_manager.stats();

    output.push_str(&app.decision_metrics.export_prometheus(
        &audit_stats,
        &validation_stats,
        &override_stats,
        &agent_health_stats,
    ));

    // ========== System Metrics ==========
    // Get kernel health for MQTT status
    let kernel_health = app.health_tracker.get_health(&app.contracts, &app.agents, &app.plugins);

    // MQTT Connection Status
    let mqtt_connected = if kernel_health.mqtt_status == "connected" { 1 } else { 0 };
    output.push_str("# HELP symbion_mqtt_connected MQTT broker connection status (1=connected, 0=disconnected)\n");
    output.push_str("# TYPE symbion_mqtt_connected gauge\n");
    output.push_str(&format!("symbion_mqtt_connected {}\n", mqtt_connected));

    output.push_str("# HELP symbion_mqtt_reconnects_total Total number of MQTT reconnections since startup\n");
    output.push_str("# TYPE symbion_mqtt_reconnects_total counter\n");
    output.push_str(&format!("symbion_mqtt_reconnects_total {}\n", kernel_health.mqtt_reconnects));

    output.push_str("# HELP symbion_mqtt_messages_per_minute MQTT messages received per minute\n");
    output.push_str("# TYPE symbion_mqtt_messages_per_minute gauge\n");
    output.push_str(&format!("symbion_mqtt_messages_per_minute {:.2}\n", kernel_health.mqtt_messages_per_minute));

    output.push_str("# HELP symbion_mqtt_messages_total Total MQTT messages since startup\n");
    output.push_str("# TYPE symbion_mqtt_messages_total counter\n");
    output.push_str(&format!("symbion_mqtt_messages_total {}\n", kernel_health.mqtt_messages_total));

    // Agent Metrics
    let agents_map = app.agents.list_agents().await;
    let total_agents = agents_map.len();
    let online_agents = agents_map.values()
        .filter(|a| a.status.status == "online")
        .count();
    let offline_agents = total_agents - online_agents;

    output.push_str("# HELP symbion_kernel_agents_total Total number of registered agents\n");
    output.push_str("# TYPE symbion_kernel_agents_total gauge\n");
    output.push_str(&format!("symbion_kernel_agents_total {}\n", total_agents));

    output.push_str("# HELP symbion_kernel_agents_online Number of agents currently online\n");
    output.push_str("# TYPE symbion_kernel_agents_online gauge\n");
    output.push_str(&format!("symbion_kernel_agents_online {}\n", online_agents));

    output.push_str("# HELP symbion_kernel_agents_offline Number of agents currently offline\n");
    output.push_str("# TYPE symbion_kernel_agents_offline gauge\n");
    output.push_str(&format!("symbion_kernel_agents_offline {}\n", offline_agents));

    // Context Engine Metrics
    if let Some(context_state) = app.context_engine.get_state() {
        use crate::context::Mode;
        let mode_value = match context_state.mode {
            Mode::Neutre => 0,
            Mode::Cravate => 1,
            Mode::Intime => 2,
        };
        output.push_str("# HELP symbion_context_mode Current context mode (0=neutre, 1=cravate, 2=intime)\n");
        output.push_str("# TYPE symbion_context_mode gauge\n");
        output.push_str(&format!("symbion_context_mode {}\n", mode_value));

        output.push_str("# HELP symbion_context_confidence Context detection confidence (0.0-1.0)\n");
        output.push_str("# TYPE symbion_context_confidence gauge\n");
        output.push_str(&format!("symbion_context_confidence {:.2}\n", context_state.confidence));
    }

    // Plugin Metrics (already computed in kernel_health)
    output.push_str("# HELP symbion_plugins_total Total number of registered plugins\n");
    output.push_str("# TYPE symbion_plugins_total gauge\n");
    output.push_str(&format!("symbion_plugins_total {}\n", kernel_health.plugins_total));

    output.push_str("# HELP symbion_plugins_running Number of plugins currently running\n");
    output.push_str("# TYPE symbion_plugins_running gauge\n");
    output.push_str(&format!("symbion_plugins_running {}\n", kernel_health.plugins_active));

    output.push_str("# HELP symbion_plugins_failed Number of plugins in failed state\n");
    output.push_str("# TYPE symbion_plugins_failed gauge\n");
    output.push_str(&format!("symbion_plugins_failed {}\n", kernel_health.plugins_failed));

    // Kernel Runtime Metrics
    output.push_str("# HELP symbion_kernel_uptime_seconds Kernel uptime in seconds since startup\n");
    output.push_str("# TYPE symbion_kernel_uptime_seconds gauge\n");
    output.push_str(&format!("symbion_kernel_uptime_seconds {}\n", kernel_health.uptime_seconds));

    output.push_str("# HELP symbion_kernel_memory_usage_mb Kernel memory usage in megabytes\n");
    output.push_str("# TYPE symbion_kernel_memory_usage_mb gauge\n");
    output.push_str(&format!("symbion_kernel_memory_usage_mb {:.2}\n", kernel_health.memory_usage_mb));

    output.push_str("# HELP symbion_contracts_loaded Number of MQTT contracts loaded\n");
    output.push_str("# TYPE symbion_contracts_loaded gauge\n");
    output.push_str(&format!("symbion_contracts_loaded {}\n", kernel_health.contracts_loaded));

    // TODO: Add HTTP request metrics (counter, latency histogram)
    // Requires instrumentation with prometheus middleware

    Ok(output)
}