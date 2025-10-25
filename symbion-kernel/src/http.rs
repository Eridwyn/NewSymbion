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
use axum::response::Response;
use time::{Duration, OffsetDateTime, format_description::well_known::Rfc3339};
use axum::extract::Path;
use std::collections::HashMap;



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

    // Health check et auth routes toujours accessibles
    if path.starts_with("/health") || path.starts_with("/auth") {
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


#[derive(Clone)]
pub struct AppState {
    pub states: Shared<HostsMap>,
    pub cfg: Shared<HostsConfig>,
    pub contracts: crate::contracts::ContractRegistry,
    pub health_tracker: crate::health::HealthTracker,
    pub auth_manager: crate::auth::AuthManager,
    pub ports: Shared<crate::ports::PortRegistry>,
    pub plugins: Shared<crate::plugins::PluginManager>,
    pub notes_bridge: Option<SharedNotesBridge>,
    pub agents: crate::agents::SharedAgentRegistry,
    pub context_engine: std::sync::Arc<crate::context::ContextEngine>,
}

#[derive(Debug, Deserialize)]
struct WakeParams { host_id: String }

pub fn build_router(app_state: AppState) -> Router {
    Router::new()
        .route("/health", get(|| async { "ok" }))
        .route("/auth/login", post(auth_login))
        .route("/auth/verify", get(auth_verify))
        .route("/auth/session", get(auth_session))
        .route("/auth/logout", post(auth_logout))
        .route("/system/health", get(get_system_health))
        .route("/hosts", get(get_hosts))
        .route("/hosts/{id}", get(get_host))
        .route("/wake", post(wake))
        .route("/contracts", get(list_contracts))
        .route("/contracts/{name}", get(get_contract))
        .route("/ports", get(list_ports))
        .route("/ports/memo", get(handle_memo_list).post(handle_memo_create))
        .route("/ports/memo/{id}", axum::routing::delete(handle_memo_delete).put(handle_memo_update))
        .route("/ports/{port_name}", get(read_from_port).post(write_to_port))
        .route("/ports/{port_name}/{id}", axum::routing::delete(delete_from_port))
        .route("/plugins", get(list_plugins_endpoint))
        .route("/plugins/{name}/start", post(start_plugin_endpoint))
        .route("/plugins/{name}/stop", post(stop_plugin_endpoint))
        .route("/plugins/{name}/restart", post(restart_plugin_endpoint))
        .route("/agents", get(list_agents_endpoint))
        .route("/agents/{id}", get(get_agent_endpoint))
        .route("/agents/{id}/shutdown", post(agent_shutdown_endpoint))
        .route("/agents/{id}/reboot", post(agent_reboot_endpoint))
        .route("/agents/{id}/hibernate", post(agent_hibernate_endpoint))
        .route("/agents/{id}/processes", get(agent_processes_endpoint))
        .route("/agents/{id}/processes/{pid}/kill", post(agent_kill_process_endpoint))
        .route("/agents/{id}/command", post(agent_command_endpoint))
        .route("/agents/{id}/metrics", get(agent_metrics_endpoint))
        .route("/agents/{id}/commands", get(agent_commands_endpoint).post(agent_commands_post_endpoint))
        .route("/commands/{command_id}/cancel", post(cancel_command_endpoint))
        .route("/commands/{command_id}/status", get(command_status_endpoint))
        .route("/context/current", get(get_context_current))
        .route("/context/override", post(set_context_override))
        .route("/context/clear", post(clear_context_override))
        .with_state(app_state.clone())
        .layer(middleware::from_fn_with_state(app_state, require_auth))
        .layer(
            CorsLayer::new()
                .allow_origin(Any)
                .allow_methods([Method::GET, Method::POST, Method::DELETE, Method::PUT, Method::OPTIONS])
                .allow_headers(Any)
                .allow_credentials(false)
        )
        // Timeout de 30s pour toutes requêtes - Prévient blocages deadlock
        .layer(TimeoutLayer::new(std::time::Duration::from_secs(30)))
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
    Json(payload): Json<crate::auth::LoginRequest>,
) -> Result<Json<crate::auth::LoginResponse>, (StatusCode, Json<serde_json::Value>)> {
    match app.auth_manager.authenticate(&payload.username, &payload.password) {
        Ok(response) => Ok(Json(response)),
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