//! Symbion Freebox Plugin
//!
//! Integrates Freebox API with Symbion for:
//! - Presence detection (phone on network)
//! - Device list monitoring
//! - Internet connection status
//! - Downloads manager status

mod config;
mod freebox;
mod mqtt;

use anyhow::{Context, Result};
use axum::{routing::get, Json, Router};
use serde::Serialize;
use std::sync::Arc;
use symbion_plugin_common::PluginHttpServer;
use tokio::sync::RwLock;
use tokio::time::{interval_at, Duration, Instant};
use tracing::{error, info, warn, Level};
use tracing_subscriber::EnvFilter;

use config::Config;
use freebox::FreeboxClient;
use mqtt::MqttPublisher;

/// Plugin constants
const PLUGIN_ID: &str = "freebox";
const SPEC_VERSION: &str = "1.0";

/// Plugin state shared across tasks
struct PluginState {
    config: Config,
    freebox: FreeboxClient,
    mqtt: MqttPublisher,
    health: RwLock<InternalHealth>,
    started_at: std::time::Instant,
}

/// Internal health tracking
#[derive(Debug, Clone, Default)]
struct InternalHealth {
    freebox_connected: bool,
    mqtt_connected: bool,
    last_presence_check: Option<String>,
    last_connection_check: Option<String>,
    last_devices_refresh: Option<String>,
    last_downloads_check: Option<String>,
    error: Option<String>,
}

/// Health response for kernel discovery (standard format)
#[derive(Debug, Clone, Serialize)]
struct HealthResponse {
    plugin_id: String,
    spec_version: String,
    status: String,
    uptime_seconds: u64,
}


#[tokio::main]
async fn main() -> Result<()> {
    // Initialize logging
    tracing_subscriber::fmt()
        .with_env_filter(
            EnvFilter::from_default_env()
                .add_directive(Level::INFO.into())
                .add_directive("symbion_plugin_freebox=debug".parse().unwrap()),
        )
        .init();

    info!("Starting Symbion Freebox Plugin v{}", env!("CARGO_PKG_VERSION"));

    // Load configuration
    let config_path = std::env::var("FREEBOX_CONFIG")
        .unwrap_or_else(|_| "/opt/symbion/config/freebox.toml".to_string());

    let config = Config::load(&config_path)
        .with_context(|| format!("Failed to load config from {}", config_path))?;

    info!("Loaded configuration from {}", config_path);
    info!("Tracking {} devices for presence", config.devices.len());

    // Initialize Freebox client
    let freebox = FreeboxClient::new(
        &config.freebox.api_url,
        &config.freebox.app_id,
        &config.freebox.app_token,
    );

    // Test Freebox connection
    info!("Testing Freebox connection...");
    match freebox.get_connection_status().await {
        Ok(status) => {
            info!("Freebox connected: {} ({})", status.state, status.connection_type);
        }
        Err(e) => {
            error!("Failed to connect to Freebox: {}", e);
            return Err(e);
        }
    }

    // Initialize MQTT publisher
    info!("Connecting to MQTT broker {}:{}...", config.mqtt.host, config.mqtt.port);
    let mqtt = MqttPublisher::connect(&config.mqtt)
        .await
        .context("Failed to connect to MQTT broker")?;

    info!("MQTT connected");

    // Create shared state
    let state = Arc::new(PluginState {
        config: config.clone(),
        freebox,
        mqtt,
        health: RwLock::new(InternalHealth {
            freebox_connected: true,
            mqtt_connected: true,
            ..Default::default()
        }),
        started_at: std::time::Instant::now(),
    });

    // Publish manifest (plugin discovery)
    let manifest = include_str!("../manifest.json");
    state.mqtt.publish_manifest(manifest).await?;
    info!("Manifest published to symbion/plugins/freebox/manifest");

    // Publish initial health
    state.mqtt.publish_health(true, "Plugin started").await?;

    // Start polling tasks
    let presence_handle = tokio::spawn(presence_loop(Arc::clone(&state)));
    let connection_handle = tokio::spawn(connection_loop(Arc::clone(&state)));
    let downloads_handle = tokio::spawn(downloads_loop(Arc::clone(&state)));
    let devices_handle = tokio::spawn(devices_loop(Arc::clone(&state)));

    // Start health endpoint
    let health_handle = tokio::spawn(health_server(Arc::clone(&state)));

    info!("All tasks started, plugin running");

    // Wait for any task to complete (shouldn't happen normally)
    tokio::select! {
        r = presence_handle => {
            error!("Presence loop exited: {:?}", r);
        }
        r = connection_handle => {
            error!("Connection loop exited: {:?}", r);
        }
        r = downloads_handle => {
            error!("Downloads loop exited: {:?}", r);
        }
        r = devices_handle => {
            error!("Devices loop exited: {:?}", r);
        }
        r = health_handle => {
            error!("Health server exited: {:?}", r);
        }
    }

    Ok(())
}

/// Presence detection loop - checks tracked devices
async fn presence_loop(state: Arc<PluginState>) {
    // interval_at with Instant::now() ticks immediately on first call
    let mut ticker = interval_at(Instant::now(), Duration::from_secs(state.config.polling.presence_seconds));

    loop {
        ticker.tick().await;

        let names: Vec<String> = state.config.devices
            .values()
            .map(|d| d.freebox_name.clone())
            .collect();

        match state.freebox.get_devices_by_names(&names).await {
            Ok(devices) => {
                if let Err(e) = state.mqtt
                    .publish_presence_batch(&devices, &state.config.devices)
                    .await
                {
                    warn!("Failed to publish presence: {}", e);
                }

                let mut health = state.health.write().await;
                health.last_presence_check = Some(chrono::Utc::now().to_rfc3339());
                health.freebox_connected = true;
            }
            Err(e) => {
                error!("Presence check failed: {}", e);
                let mut health = state.health.write().await;
                health.freebox_connected = false;
                health.error = Some(format!("Presence: {}", e));
            }
        }
    }
}

/// Connection status loop
async fn connection_loop(state: Arc<PluginState>) {
    let mut ticker = interval_at(Instant::now(), Duration::from_secs(state.config.polling.connection_seconds));

    loop {
        ticker.tick().await;

        match state.freebox.get_connection_status().await {
            Ok(status) => {
                if let Err(e) = state.mqtt.publish_connection(&status).await {
                    warn!("Failed to publish connection status: {}", e);
                }

                let mut health = state.health.write().await;
                health.last_connection_check = Some(chrono::Utc::now().to_rfc3339());
            }
            Err(e) => {
                error!("Connection status check failed: {}", e);
                let mut health = state.health.write().await;
                health.error = Some(format!("Connection: {}", e));
            }
        }
    }
}

/// Downloads monitoring loop
async fn downloads_loop(state: Arc<PluginState>) {
    let mut ticker = interval_at(Instant::now(), Duration::from_secs(state.config.polling.downloads_seconds));

    loop {
        ticker.tick().await;

        match state.freebox.get_downloads_summary().await {
            Ok(summary) => {
                if let Err(e) = state.mqtt.publish_downloads(&summary).await {
                    warn!("Failed to publish downloads: {}", e);
                }

                let mut health = state.health.write().await;
                health.last_downloads_check = Some(chrono::Utc::now().to_rfc3339());
            }
            Err(e) => {
                // Downloads API might not be available (no downloads app)
                warn!("Downloads check failed: {}", e);
            }
        }
    }
}

/// Full device list refresh loop (less frequent)
async fn devices_loop(state: Arc<PluginState>) {
    let mut ticker = interval_at(Instant::now(), Duration::from_secs(state.config.polling.devices_seconds));

    loop {
        ticker.tick().await;

        match state.freebox.get_lan_devices().await {
            Ok(devices) => {
                if let Err(e) = state.mqtt.publish_devices(&devices).await {
                    warn!("Failed to publish devices: {}", e);
                }

                let mut health = state.health.write().await;
                health.last_devices_refresh = Some(chrono::Utc::now().to_rfc3339());
            }
            Err(e) => {
                error!("Devices refresh failed: {}", e);
            }
        }
    }
}

/// Health endpoint server (Unix socket)
async fn health_server(state: Arc<PluginState>) -> Result<()> {
    let socket_path = state.config.http.socket_path.clone();

    // Create parent directory if needed
    if let Some(parent) = std::path::Path::new(&socket_path).parent() {
        std::fs::create_dir_all(parent)?;
    }

    info!("Health endpoint listening on {}", socket_path);

    let app = Router::new()
        .route("/health", get(health_handler))
        .with_state(state);

    let server = PluginHttpServer::new(&socket_path, app);
    server.serve().await.map_err(|e| anyhow::anyhow!("Health server error: {}", e))?;

    Ok(())
}

async fn health_handler(
    axum::extract::State(state): axum::extract::State<Arc<PluginState>>,
) -> Json<HealthResponse> {
    let internal = state.health.read().await;
    let status = if internal.freebox_connected && internal.mqtt_connected && internal.error.is_none() {
        "healthy"
    } else {
        "degraded"
    };

    Json(HealthResponse {
        plugin_id: PLUGIN_ID.to_string(),
        spec_version: SPEC_VERSION.to_string(),
        status: status.to_string(),
        uptime_seconds: state.started_at.elapsed().as_secs(),
    })
}
