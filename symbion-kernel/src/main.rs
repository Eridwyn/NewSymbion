mod models;
mod state;
mod mqtt;
mod http;
mod config;
mod wol;

use crate::models::HostsMap;
use crate::state::{new_state, Shared};
use crate::config::{load_config, HostsConfig};
use crate::http::AppState;

use std::collections::HashMap;
use std::net::SocketAddr;
use tokio::net::TcpListener;

#[tokio::main]
async fn main() {
    // maps et conf partagées
    let states = new_state::<HostsMap>(HashMap::new());
    let cfg_loaded: HostsConfig = load_config().await;
    let cfg: Shared<HostsConfig> = new_state(cfg_loaded);

    // MQTT remplit les states
    mqtt::spawn_mqtt_listener(states.clone());

    // fabrique l'état unique pour Axum
    let app_state = AppState { states, cfg };

    // HTTP
    let app = http::build_router(app_state);

    let addr = SocketAddr::from(([0,0,0,0], 8080));
    println!("[kernel] listening on http://{addr}");
    let listener = TcpListener::bind(addr).await.unwrap();
    axum::serve(listener, app).await.unwrap();
}
