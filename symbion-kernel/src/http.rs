use axum::{extract::{Query, State}, routing::{get, post}, Json, Router};
use axum::http::StatusCode;
use crate::models::{HostState, HostsMap};
use crate::state::Shared;
use crate::config::HostsConfig;
use crate::wol::trigger_wol_udp;
use serde::Deserialize;




#[derive(Clone)]
pub struct AppState {
    pub states: Shared<HostsMap>,
    pub cfg: Shared<HostsConfig>,
}

#[derive(Debug, Deserialize)]
struct WakeParams { host_id: String }

pub fn build_router(app_state: AppState) -> Router {
    Router::new()
        .route("/health", get(|| async { "ok" }))
        .route("/hosts", get(get_hosts))
        .route("/wake", post(wake))
        .with_state(app_state)
}

async fn get_hosts(
    State(app): State<AppState>,
) -> Json<Vec<HostState>> {
    Json(app.states.lock().values().cloned().collect())
}

async fn wake(
    State(app): State<AppState>,
    Query(params): Query<WakeParams>,
) -> (StatusCode, Json<serde_json::Value>) {
    let cfg = app.cfg.lock().clone();
    let (code, msg) = trigger_wol_udp(&cfg, &params.host_id).await;  // <— ici
    (code, Json(serde_json::json!({ "ok": code == StatusCode::OK, "msg": msg })))
}
