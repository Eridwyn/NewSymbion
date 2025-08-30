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
use axum::http::StatusCode;
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

async fn require_api_key(req: Request, next: Next) -> Result<Response, StatusCode> {
    let path = req.uri().path();
    
    // Health check toujours accessible
    if path.starts_with("/health") {
        return Ok(next.run(req).await);
    }

    let expected = std::env::var("SYMBION_API_KEY").unwrap_or_default();
    if expected.is_empty() {
        eprintln!("SECURITY: SYMBION_API_KEY not set - API access denied");
        return Err(StatusCode::UNAUTHORIZED);
    }

    let ok = req.headers()
        .get("x-api-key")
        .and_then(|v| v.to_str().ok())
        .map(|v| v == expected)
        .unwrap_or(false);

    if !ok {
        return Err(StatusCode::UNAUTHORIZED);
    }
    Ok(next.run(req).await)
}


#[derive(Clone)]
pub struct AppState {
    pub states: Shared<HostsMap>,
    pub cfg: Shared<HostsConfig>,
    pub contracts: crate::contracts::ContractRegistry,
    pub health_tracker: crate::health::HealthTracker,
    pub ports: Shared<crate::ports::PortRegistry>,
    pub plugins: Shared<crate::plugins::PluginManager>,
    pub notes_bridge: Option<SharedNotesBridge>,
}

#[derive(Debug, Deserialize)]
struct WakeParams { host_id: String }

pub fn build_router(app_state: AppState) -> Router {
    Router::new()
        .route("/health", get(|| async { "ok" }))
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
        .with_state(app_state)
        .layer(middleware::from_fn(require_api_key))
}


// GET /hosts (liste)
async fn get_hosts(State(app): State<AppState>) -> Json<Vec<HostView>> {
    let list: Vec<HostView> = app.states.lock().values().map(to_view).collect();
    Json(list)
}

// GET /hosts/:id (détail)
async fn get_host(
    State(app): State<AppState>,
    Path(id): Path<String>,
) -> Result<Json<HostView>, StatusCode> {
    let map = app.states.lock();
    let Some(h) = map.get(&id) else { return Err(StatusCode::NOT_FOUND); };
    Ok(Json(to_view(h)))
}


async fn wake(
    State(app): State<AppState>,
    Query(params): Query<WakeParams>,
) -> (StatusCode, Json<serde_json::Value>) {
    let cfg = app.cfg.lock().clone();
    let (code, msg) = trigger_wol_udp(&cfg, &params.host_id).await;  // <— ici
    (code, Json(serde_json::json!({ "ok": code == StatusCode::OK, "msg": msg })))
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
    let health = app.health_tracker.get_health(&app.contracts, &app.states, &app.plugins);
    Json(health)
}

// GET /ports (liste des ports disponibles)
async fn list_ports(State(app): State<AppState>) -> Json<Vec<crate::ports::PortInfo>> {
    let ports = app.ports.lock();
    let port_info = ports.list_port_info();
    Json(port_info)
}

// GET /ports/{port_name} (lecture depuis un port avec query optionnelle)
async fn read_from_port(
    State(app): State<AppState>,
    Path(port_name): Path<String>,
    Query(params): Query<HashMap<String, String>>,
) -> Result<Json<Vec<crate::ports::PortData>>, StatusCode> {
    let ports = app.ports.lock();
    let port = ports.get(&port_name)
        .ok_or(StatusCode::NOT_FOUND)?;
    
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
    
    match port.read(&query) {
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
    let ports = app.ports.lock();
    let port = ports.get(&port_name)
        .ok_or(StatusCode::NOT_FOUND)?;
    
    // Construction d'un PortData depuis le JSON reçu
    let port_data = crate::ports::PortData {
        id: String::new(), // L'ID sera généré automatiquement
        timestamp: time::OffsetDateTime::now_utc(),
        data: data,
        metadata: HashMap::new(),
    };
    
    match port.write(&port_data) {
        Ok(id) => Ok(Json(serde_json::json!({"id": id, "status": "created"}))),
        Err(_) => Err(StatusCode::INTERNAL_SERVER_ERROR),
    }
}

// DELETE /ports/{port_name}/{id} (suppression depuis un port)
async fn delete_from_port(
    State(app): State<AppState>,
    Path((port_name, id)): Path<(String, String)>,
) -> Result<Json<serde_json::Value>, StatusCode> {
    let ports = app.ports.lock();
    let port = ports.get(&port_name)
        .ok_or(StatusCode::NOT_FOUND)?;
    
    match port.delete(&id) {
        Ok(_) => Ok(Json(serde_json::json!({"status": "deleted"}))),
        Err(_) => Err(StatusCode::NOT_FOUND),
    }
}

// GET /plugins (liste des plugins avec leur état)
async fn list_plugins_endpoint(State(app): State<AppState>) -> Json<Vec<crate::plugins::PluginInfo>> {
    let plugins = app.plugins.lock();
    let plugin_info = plugins.list_plugins();
    Json(plugin_info)
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

// ============ MEMO HANDLERS (Bridge + Fallback) ============

async fn handle_memo_list(
    State(app): State<AppState>,
    Query(params): Query<HashMap<String, String>>,
) -> Result<Json<serde_json::Value>, StatusCode> {
    // Si le bridge notes est disponible, l'utiliser
    if let Some(ref bridge) = app.notes_bridge {
        return notes_bridge::list_notes_endpoint(
            axum::extract::State(bridge.clone()),
            axum::extract::Query(params)
        ).await;
    }
    
    // Sinon, utiliser le port memo classique
    let ports = app.ports.lock();
    let port = ports.get("memo")
        .ok_or(StatusCode::NOT_FOUND)?;
    
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
    
    match port.read(&query) {
        Ok(data) => Ok(Json(serde_json::Value::Array(
            data.into_iter().map(|d| serde_json::json!({
                "id": d.id,
                "timestamp": d.timestamp.format(&time::format_description::well_known::Rfc3339).unwrap_or_default(),
                "data": d.data,
                "metadata": d.metadata
            })).collect()
        ))),
        Err(_) => Err(StatusCode::INTERNAL_SERVER_ERROR),
    }
}

async fn handle_memo_create(
    State(app): State<AppState>,
    Json(note_data): Json<serde_json::Value>,
) -> Result<Json<serde_json::Value>, StatusCode> {
    // Si le bridge notes est disponible, l'utiliser
    if let Some(ref bridge) = app.notes_bridge {
        // Convertir les données en format CreateNoteRequest
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
        
        return notes_bridge::create_note_endpoint(
            axum::extract::State(bridge.clone()),
            axum::extract::Json(create_request)
        ).await;
    }
    
    // Sinon, utiliser le port memo classique
    let ports = app.ports.lock();
    let port = ports.get("memo")
        .ok_or(StatusCode::NOT_FOUND)?;
    
    let port_data = crate::ports::PortData {
        id: String::new(),
        timestamp: time::OffsetDateTime::now_utc(),
        data: note_data,
        metadata: HashMap::new(),
    };
    
    match port.write(&port_data) {
        Ok(id) => Ok(Json(serde_json::json!({"id": id, "status": "created"}))),
        Err(_) => Err(StatusCode::INTERNAL_SERVER_ERROR),
    }
}

async fn handle_memo_delete(
    State(app): State<AppState>,
    Path(id): Path<String>,
) -> Result<Json<serde_json::Value>, StatusCode> {
    // Si le bridge notes est disponible, l'utiliser
    if let Some(ref bridge) = app.notes_bridge {
        return notes_bridge::delete_note_endpoint(
            axum::extract::State(bridge.clone()),
            axum::extract::Path(id)
        ).await;
    }
    
    // Sinon, utiliser le port memo classique
    let ports = app.ports.lock();
    let port = ports.get("memo")
        .ok_or(StatusCode::NOT_FOUND)?;
    
    match port.delete(&id) {
        Ok(_) => Ok(Json(serde_json::json!({"status": "deleted"}))),
        Err(_) => Err(StatusCode::NOT_FOUND),
    }
}

async fn handle_memo_update(
    State(app): State<AppState>,
    Path(id): Path<String>,
    Json(note_data): Json<serde_json::Value>,
) -> Result<Json<serde_json::Value>, StatusCode> {
    // Si le bridge notes est disponible, l'utiliser
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
    
    // Port memo classique ne supporte pas la mise à jour
    Err(StatusCode::NOT_IMPLEMENTED)
}
