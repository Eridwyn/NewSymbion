//! Symbion Synology Plugin
//!
//! Monitors Synology NAS via NUT (Network UPS Tools):
//! - UPS status, battery level, runtime, load
//! - Publishes features for automations (Intelligence v2)
//! - Exposes /health and /ups over a Unix socket (kernel service discovery)

mod config;
mod mqtt;
mod nut;

use anyhow::{Context, Result};
use axum::{extract::State, http::StatusCode, routing::get, Json, Router};
use serde::Serialize;
use std::sync::Arc;
use symbion_plugin_common::PluginHttpServer;
use tokio::sync::RwLock;
use tokio::time::{interval, Duration};
use tracing::{error, info, warn};
use tracing_subscriber::EnvFilter;

use config::Config;
use mqtt::MqttPublisher;
use nut::{NutClient, UpsStatus};

const PLUGIN_ID: &str = "synology";
const SPEC_VERSION: &str = "1.0";

struct PluginState {
    config: Config,
    mqtt: MqttPublisher,
    ups_status: RwLock<Option<UpsStatus>>,
    health: RwLock<HealthState>,
    started_at: std::time::Instant,
}

#[derive(Debug, Clone, Default)]
struct HealthState {
    mqtt_connected: bool,
    last_poll: Option<String>,
    error: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
struct HealthResponse {
    plugin_id: String,
    spec_version: String,
    status: String,
    uptime_seconds: u64,
    mqtt_connected: bool,
    last_poll: Option<String>,
    error: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
struct UpsResponse {
    status: String,
    battery_charge: f64,
    battery_runtime_seconds: u64,
    load_percent: f64,
    on_battery: bool,
    battery_low: bool,
    output_voltage: f64,
    model: String,
    manufacturer: String,
}

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(EnvFilter::from_default_env().add_directive("info".parse()?))
        .init();

    let config_path =
        std::env::var("SYNOLOGY_CONFIG").unwrap_or_else(|_| "/opt/symbion/config/synology.toml".to_string());

    let config = Config::load(&config_path)
        .with_context(|| format!("Failed to load config from {}", config_path))?;
    info!(
        "Synology plugin starting — NUT: {}:{}, UPS '{}'",
        config.nut.host, config.nut.port, config.nut.ups_name
    );

    let mqtt = MqttPublisher::new(&config.mqtt).await?;

    // Manifest + health initiale (retained)
    let manifest = include_str!("../manifest.json");
    mqtt.publish_manifest(manifest).await?;
    mqtt.publish_health(true, "Plugin started").await?;
    info!("Manifest published to symbion/plugins/synology/manifest");

    let state = Arc::new(PluginState {
        config: config.clone(),
        mqtt,
        ups_status: RwLock::new(None),
        health: RwLock::new(HealthState::default()),
        started_at: std::time::Instant::now(),
    });

    // Tâche de polling NUT
    let poll_state = state.clone();
    let poll_handle = tokio::spawn(async move {
        let mut ticker = interval(Duration::from_secs(poll_state.config.poll_interval_seconds));
        loop {
            ticker.tick().await;
            match NutClient::query(&poll_state.config.nut).await {
                Ok(status) => {
                    let now = chrono::Utc::now().to_rfc3339();
                    if let Err(e) = poll_state.mqtt.publish_ups(&status).await {
                        error!("MQTT publish error: {}", e);
                    }
                    {
                        let mut h = poll_state.health.write().await;
                        h.last_poll = Some(now);
                        h.error = None;
                        h.mqtt_connected = true;
                    }
                    *poll_state.ups_status.write().await = Some(status);
                }
                Err(e) => {
                    warn!("NUT poll error: {}", e);
                    let mut h = poll_state.health.write().await;
                    h.error = Some(e.to_string());
                    let _ = poll_state.mqtt.publish_health(false, &e.to_string()).await;
                }
            }
        }
    });

    // Serveur HTTP (Unix socket)
    let api_handle = tokio::spawn(api_server(state.clone()));

    // Enregistrement auprès du kernel (service discovery)
    let socket_str = config.http.socket_path.clone();
    tokio::spawn(async move {
        use symbion_plugin_common::PluginRegistrationBuilder;

        // Laisse le socket se créer avant de s'annoncer.
        tokio::time::sleep(Duration::from_secs(2)).await;

        if let Err(e) = PluginRegistrationBuilder::new(PLUGIN_ID, &socket_str)
            .route("/health")
            .route("/ups")
            .version(env!("CARGO_PKG_VERSION"))
            .description("Synology NAS UPS monitoring via NUT")
            .register()
            .await
        {
            warn!("Failed to register with kernel: {}", e);
        } else {
            info!("Registered with kernel");
        }
    });

    info!("All tasks started, plugin running");

    // Attente d'un arrêt propre ou d'une tâche qui meurt.
    tokio::select! {
        r = poll_handle => error!("Polling loop exited: {:?}", r),
        r = api_handle => error!("API server exited: {:?}", r),
        _ = tokio::signal::ctrl_c() => info!("[synology] SIGINT received, shutting down"),
        _ = async {
            #[cfg(unix)]
            {
                let mut sigterm = tokio::signal::unix::signal(
                    tokio::signal::unix::SignalKind::terminate(),
                ).expect("failed to install SIGTERM handler");
                sigterm.recv().await;
            }
            #[cfg(not(unix))]
            { std::future::pending::<()>().await; }
        } => info!("[synology] SIGTERM received, shutting down"),
    }

    let _ = state.mqtt.publish_health(false, "Plugin stopped").await;
    info!("[synology] Shutdown complete");
    Ok(())
}

/// Serveur HTTP sur Unix socket (health + état UPS).
async fn api_server(state: Arc<PluginState>) -> Result<()> {
    let socket_path = state.config.http.socket_path.clone();

    if let Some(parent) = std::path::Path::new(&socket_path).parent() {
        std::fs::create_dir_all(parent)
            .with_context(|| format!("Failed to create socket dir {:?}", parent))?;
    }

    let app = Router::new()
        .route("/health", get(handle_health))
        .route("/ups", get(handle_ups))
        .with_state(state);

    info!("API server listening on {}", socket_path);
    PluginHttpServer::new(&socket_path, app)
        .serve()
        .await
        .map_err(|e| anyhow::anyhow!("API server error: {}", e))?;

    Ok(())
}

async fn handle_health(State(state): State<Arc<PluginState>>) -> Json<HealthResponse> {
    let h = state.health.read().await;
    Json(HealthResponse {
        plugin_id: PLUGIN_ID.to_string(),
        spec_version: SPEC_VERSION.to_string(),
        status: if h.error.is_none() { "ok".to_string() } else { "degraded".to_string() },
        uptime_seconds: state.started_at.elapsed().as_secs(),
        mqtt_connected: h.mqtt_connected,
        last_poll: h.last_poll.clone(),
        error: h.error.clone(),
    })
}

async fn handle_ups(State(state): State<Arc<PluginState>>) -> Result<Json<UpsResponse>, StatusCode> {
    match state.ups_status.read().await.clone() {
        Some(s) => Ok(Json(UpsResponse {
            status: s.status.clone(),
            battery_charge: s.battery_charge,
            battery_runtime_seconds: s.battery_runtime_seconds,
            load_percent: s.load_percent,
            on_battery: s.on_battery(),
            battery_low: s.battery_low(),
            output_voltage: s.output_voltage,
            model: s.model.clone(),
            manufacturer: s.manufacturer.clone(),
        })),
        None => Err(StatusCode::SERVICE_UNAVAILABLE),
    }
}
