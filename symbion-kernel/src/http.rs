use axum::{extract::{Query, State}, routing::{get, post}, Json, Router};
use axum::http::StatusCode;
use crate::models::{HostState, HostsMap};
use crate::state::Shared;
use crate::config::HostsConfig;
use crate::wol::trigger_wol_udp;
use serde::Deserialize;
use axum::middleware::{self, Next};
use axum::extract::Request;
use axum::response::Response;
use time::{Duration, OffsetDateTime, format_description::well_known::Rfc3339};
use axum::extract::Path;



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
    let health = app.health_tracker.get_health(&app.contracts, &app.states);
    Json(health)
}
