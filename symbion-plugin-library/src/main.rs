// Symbion Knowledge Library Plugin
// Centralized knowledge base with graph navigation and study desk

use anyhow::Result;
use std::sync::Arc;
use tracing::Level;
use tracing_subscriber::EnvFilter;

use symbion_plugin_library::{config, database, mqtt, routes, PluginState};
use config::Config;
use database::Database;
use mqtt::MqttPublisher;

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(
            EnvFilter::from_default_env()
                .add_directive(Level::INFO.into())
                .add_directive("symbion_plugin_library=debug".parse().unwrap()),
        )
        .init();

    // Load config
    let config_path = std::env::var("LIBRARY_CONFIG")
        .unwrap_or_else(|_| "symbion-plugin-library/config/library.toml".to_string());
    let config = Config::load(&config_path)?;
    tracing::info!("[library] config loaded from {}", config_path);

    // Open database + migrate
    let db = Database::open(&config.database.path)?;
    db.migrate().await?;
    tracing::info!("[library] database ready at {}", config.database.path);

    // Seed initial data
    if db.seed_if_empty().await? {
        tracing::info!("[library] seeded initial data (Fiche Epice + Marc de Cafe)");
    }

    // Connect MQTT
    let mqtt = MqttPublisher::connect(
        &config.mqtt.host,
        config.mqtt.port,
        &config.mqtt.client_id,
    ).await?;
    tracing::info!("[library] MQTT connected");

    // Publish manifest
    let manifest = include_str!("../manifest.json");
    mqtt.publish_manifest(manifest).await?;
    mqtt.publish_health(true, "Plugin started (v0.1)").await?;

    // Publish initial features
    let (nodes, sections, pending) = db.stats().await?;
    mqtt.publish_features(nodes, sections, pending).await?;

    let db = Arc::new(db);
    let mqtt = Arc::new(mqtt);

    let state = Arc::new(PluginState {
        db: Arc::clone(&db),
        mqtt: Arc::clone(&mqtt),
        config: config.clone(),
        started_at: std::time::Instant::now(),
    });

    // Spawn API server
    let api_state = Arc::clone(&state);
    let api_handle = tokio::spawn(api_server(api_state));

    // Spawn periodic feature publisher
    let feat_db = Arc::clone(&db);
    let feat_mqtt = Arc::clone(&mqtt);
    let feature_handle = tokio::spawn(async move {
        loop {
            tokio::time::sleep(tokio::time::Duration::from_secs(60)).await;
            if let Ok((n, s, p)) = feat_db.stats().await {
                let _ = feat_mqtt.publish_features(n, s, p).await;
            }
        }
    });

    tracing::info!("[library] plugin ready - {} nodes, {} sections", nodes, sections);

    // Graceful shutdown
    tokio::select! {
        r = api_handle => {
            if let Err(e) = r { tracing::error!("[library] API server exited: {:?}", e); }
        }
        r = feature_handle => {
            if let Err(e) = r { tracing::error!("[library] feature publisher exited: {:?}", e); }
        }
        _ = tokio::signal::ctrl_c() => {
            tracing::info!("[library] SIGINT received, shutting down");
        }
        _ = async {
            #[cfg(unix)]
            {
                let mut sigterm = tokio::signal::unix::signal(tokio::signal::unix::SignalKind::terminate())
                    .expect("Failed to register SIGTERM");
                sigterm.recv().await;
            }
            #[cfg(not(unix))]
            std::future::pending::<()>().await;
        } => {
            tracing::info!("[library] SIGTERM received, shutting down");
        }
    }

    mqtt.publish_health(false, "Plugin shutting down").await.ok();
    tracing::info!("[library] shutdown complete");
    Ok(())
}

async fn api_server(state: Arc<PluginState>) -> Result<()> {
    let socket_path = state.config.http.socket_path.clone();

    if let Some(parent) = std::path::Path::new(&socket_path).parent() {
        std::fs::create_dir_all(parent)?;
    }

    let app = routes::build_router(state);
    let server = symbion_plugin_common::PluginHttpServer::new(&socket_path, app);
    tracing::info!("[library] API listening on {}", socket_path);
    server.serve().await.map_err(|e| anyhow::anyhow!("API server error: {}", e))?;
    Ok(())
}
