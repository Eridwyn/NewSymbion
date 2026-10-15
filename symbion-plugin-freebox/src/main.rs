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
use axum::{extract::Path, http::StatusCode, routing::get, Json, Router};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
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
    /// Appareils suivis, MODIFIABLES À CHAUD depuis la PWA. Ce n'est plus la
    /// config TOML qui décide : elle n'en est que la graine.
    tracked: RwLock<HashMap<String, config::DeviceConfig>>,
    /// Dernière liste LAN connue, pour que la PWA puisse proposer un choix
    /// sans attendre le prochain tour de sonde (5 minutes).
    lan_cache: RwLock<Vec<freebox::LanDevice>>,
    /// Depuis quand personne n'est à la maison. Permet d'exiger une absence
    /// SOUTENUE avant d'agir : un Wi-Fi qui décroche trois secondes ne doit
    /// pas éteindre la maison.
    absent_since: RwLock<Option<chrono::DateTime<chrono::Utc>>>,
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

    // Test Freebox connection (non-fatal: polling loops will retry)
    info!("Testing Freebox connection...");
    match freebox.get_connection_status().await {
        Ok(status) => {
            info!("Freebox connected: {} ({})", status.state, status.connection_type);
        }
        Err(e) => {
            warn!("Freebox not reachable at startup (will retry in polling loops): {}", e);
        }
    }

    // Initialize MQTT publisher
    info!("Connecting to MQTT broker {}:{}...", config.mqtt.host, config.mqtt.port);
    let mqtt = MqttPublisher::connect(&config.mqtt)
        .await
        .context("Failed to connect to MQTT broker")?;

    info!("MQTT connected");

    // Les appareils suivis : le fichier de la PWA s'il existe, sinon la graine
    // du TOML. Jamais vide par accident — une liste vide déclare tout le monde
    // absent, et c'est le genre d'erreur qui éteint les lumières de la maison.
    let tracked = match config::TrackedDevices::load(&config.tracked_path) {
        Some(t) if !t.devices.is_empty() => {
            info!("{} appareils suivis chargés depuis {}", t.devices.len(), config.tracked_path);
            t.devices
        }
        _ => {
            info!(
                "{} appareils suivis depuis la graine TOML (aucun fichier {} exploitable)",
                config.devices.len(), config.tracked_path
            );
            config.devices.clone()
        }
    };

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
        tracked: RwLock::new(tracked),
        lan_cache: RwLock::new(Vec::new()),
        absent_since: RwLock::new(None),
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

    // Wait for any task to complete or graceful shutdown signal
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
        _ = tokio::signal::ctrl_c() => {
            info!("[freebox] Received SIGINT (Ctrl+C), shutting down gracefully...");
        }
        _ = async {
            #[cfg(unix)]
            {
                let mut sigterm = tokio::signal::unix::signal(tokio::signal::unix::SignalKind::terminate())
                    .expect("failed to install SIGTERM handler");
                sigterm.recv().await;
            }
            #[cfg(not(unix))]
            {
                // On non-Unix platforms, just wait forever (ctrl_c branch handles shutdown)
                std::future::pending::<()>().await;
            }
        } => {
            info!("[freebox] Received SIGTERM, shutting down gracefully...");
        }
    }

    info!("[freebox] Shutdown complete");

    Ok(())
}

/// Presence detection loop - checks tracked devices
async fn presence_loop(state: Arc<PluginState>) {
    // interval_at with Instant::now() ticks immediately on first call
    let mut ticker = interval_at(Instant::now(), Duration::from_secs(state.config.polling.presence_seconds));

    loop {
        ticker.tick().await;

        verifier_presence(&state).await;
    }
}

/// Un tour de vérification de présence.
///
/// Extrait de la boucle pour être rejouable À LA DEMANDE : quand la PWA
/// déclare un appareil, attendre trente secondes que la sonde repasse donnerait
/// l'impression que l'ajout n'a pas pris.
async fn verifier_presence(state: &Arc<PluginState>) {
    let suivis = state.tracked.read().await.clone();
    let names: Vec<String> = suivis.values().map(|d| d.freebox_name.clone()).collect();

    match state.freebox.get_devices_by_names(&names).await {
        Ok(devices) => {
            if let Err(e) = state.mqtt.publish_presence_batch(&devices, &suivis).await {
                warn!("Failed to publish presence: {}", e);
            }

            // Durée d'absence : on horodate le passage à « personne », et on
            // publie les minutes écoulées. Une règle peut alors exiger une
            // absence soutenue au lieu de réagir au premier trou de Wi-Fi.
            let quelquun = suivis.values().any(|c| {
                devices
                    .get(&c.freebox_name)
                    .map(|d| d.active && d.reachable)
                    .unwrap_or(false)
            });
            let minutes = {
                let mut depuis = state.absent_since.write().await;
                if quelquun {
                    *depuis = None;
                    0
                } else {
                    let t0 = *depuis.get_or_insert_with(chrono::Utc::now);
                    (chrono::Utc::now() - t0).num_minutes().max(0)
                }
            };
            if let Err(e) = state
                .mqtt
                .publish_feature("presence.away_minutes", serde_json::json!(minutes), 900)
                .await
            {
                warn!("Failed to publish away_minutes: {}", e);
            }

            // Health update: short write lock, safe since RwLock serializes writes
            let mut health = state.health.write().await;
            health.last_presence_check = Some(chrono::Utc::now().to_rfc3339());
            health.freebox_connected = true;
            health.error = None;
        }
        Err(e) => {
            error!("Presence check failed: {}", e);
            let mut health = state.health.write().await;
            health.freebox_connected = false;
            health.error = Some(format!("Presence: {}", e));
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
                *state.lan_cache.write().await = devices.clone();
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
        // Lecture : ce que la présence vaut, et ce que la Freebox voit
        .route("/presence", get(presence_handler))
        .route("/devices", get(devices_handler))
        // Écriture : les appareils suivis, pilotés depuis la PWA
        .route("/tracked", get(tracked_list_handler).post(tracked_add_handler))
        .route("/tracked/{id}", axum::routing::delete(tracked_remove_handler))
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


// ============================================================================
// Présence : lecture et configuration depuis la PWA
// ============================================================================

#[derive(Debug, Clone, Serialize)]
struct AppareilSuivi {
    id: String,
    freebox_name: String,
    device_type: String,
    friendly_name: String,
    present: bool,
    /// Vu pour la dernière fois, horodatage Unix de la Freebox (0 = jamais).
    last_seen: i64,
    ip_address: Option<String>,
    /// Faux si la Freebox ne connaît aucun appareil de ce nom : c'est le cas
    /// d'une faute de frappe, et il faut le DIRE — sinon l'appareil compte
    /// comme absent pour toujours et personne ne comprend pourquoi.
    known_to_freebox: bool,
}

#[derive(Debug, Clone, Serialize)]
struct ReponsePresence {
    anyone_home: bool,
    present: usize,
    absent: usize,
    away_minutes: i64,
    devices: Vec<AppareilSuivi>,
}

/// GET /presence — l'état de présence, appareil par appareil.
async fn presence_handler(
    axum::extract::State(state): axum::extract::State<Arc<PluginState>>,
) -> Json<ReponsePresence> {
    let suivis = state.tracked.read().await.clone();
    let lan = state.lan_cache.read().await.clone();

    let mut devices = Vec::new();
    for (id, conf) in &suivis {
        let trouve = lan
            .iter()
            .find(|d| d.primary_name.eq_ignore_ascii_case(&conf.freebox_name));
        devices.push(AppareilSuivi {
            id: id.clone(),
            freebox_name: conf.freebox_name.clone(),
            device_type: conf.device_type.clone(),
            friendly_name: conf.friendly_name.clone().unwrap_or_else(|| id.clone()),
            present: trouve.map(|d| d.active && d.reachable).unwrap_or(false),
            last_seen: trouve.map(|d| d.last_time_reachable).unwrap_or(0),
            ip_address: trouve.and_then(|d| {
                d.l3connectivities
                    .iter()
                    .find(|c| c.af == "ipv4" && c.active)
                    .map(|c| c.addr.clone())
            }),
            known_to_freebox: trouve.is_some(),
        });
    }
    devices.sort_by(|a, b| a.friendly_name.cmp(&b.friendly_name));

    let present = devices.iter().filter(|d| d.present).count();
    let away_minutes = match *state.absent_since.read().await {
        Some(t0) => (chrono::Utc::now() - t0).num_minutes().max(0),
        None => 0,
    };

    Json(ReponsePresence {
        anyone_home: present > 0,
        present,
        absent: devices.len() - present,
        away_minutes,
        devices,
    })
}

#[derive(Debug, Clone, Serialize)]
struct AppareilLan {
    name: String,
    host_type: String,
    active: bool,
    vendor: String,
    ip_address: Option<String>,
    /// Déjà suivi pour la présence ?
    tracked: bool,
}

/// GET /devices — tout ce que la Freebox voit, pour choisir dans la PWA.
///
/// Sert le cache, et ne sonde la Freebox que s'il est vide : la PWA s'ouvre
/// vite, et la liste complète ne change pas à la seconde.
async fn devices_handler(
    axum::extract::State(state): axum::extract::State<Arc<PluginState>>,
) -> Json<serde_json::Value> {
    let mut lan = state.lan_cache.read().await.clone();
    if lan.is_empty() {
        if let Ok(frais) = state.freebox.get_lan_devices().await {
            *state.lan_cache.write().await = frais.clone();
            lan = frais;
        }
    }
    let suivis = state.tracked.read().await.clone();
    let deja: Vec<String> = suivis
        .values()
        .map(|c| c.freebox_name.to_lowercase())
        .collect();

    let mut devices: Vec<AppareilLan> = lan
        .iter()
        .filter(|d| !d.primary_name.trim().is_empty())
        .map(|d| AppareilLan {
            name: d.primary_name.clone(),
            host_type: d.host_type.clone(),
            active: d.active && d.reachable,
            vendor: d.vendor_name.clone(),
            ip_address: d
                .l3connectivities
                .iter()
                .find(|c| c.af == "ipv4" && c.active)
                .map(|c| c.addr.clone()),
            tracked: deja.contains(&d.primary_name.to_lowercase()),
        })
        .collect();
    // Les présents d'abord, puis par nom : on choisit presque toujours un
    // appareil allumé, autant ne pas le faire chercher.
    devices.sort_by(|a, b| b.active.cmp(&a.active).then(a.name.cmp(&b.name)));

    Json(serde_json::json!({ "devices": devices, "total": devices.len() }))
}

/// GET /tracked — la liste déclarée, brute.
async fn tracked_list_handler(
    axum::extract::State(state): axum::extract::State<Arc<PluginState>>,
) -> Json<serde_json::Value> {
    let suivis = state.tracked.read().await.clone();
    Json(serde_json::json!({ "tracked": suivis }))
}

#[derive(Debug, Deserialize)]
struct DemandeSuivi {
    /// Nom de l'appareil tel que la Freebox l'affiche. Obligatoire.
    freebox_name: String,
    /// Identifiant logique (défaut : dérivé du nom).
    #[serde(default)]
    id: Option<String>,
    #[serde(default)]
    device_type: Option<String>,
    #[serde(default)]
    friendly_name: Option<String>,
}

/// POST /tracked — déclarer un appareil à suivre.
async fn tracked_add_handler(
    axum::extract::State(state): axum::extract::State<Arc<PluginState>>,
    Json(req): Json<DemandeSuivi>,
) -> (StatusCode, Json<serde_json::Value>) {
    let nom = req.freebox_name.trim().to_string();
    if nom.is_empty() {
        return (
            StatusCode::BAD_REQUEST,
            Json(serde_json::json!({ "error": "freebox_name est vide" })),
        );
    }

    // La Freebox connaît-elle ce nom ? On REFUSE une faute de frappe plutôt
    // que d'enregistrer un appareil éternellement absent — qui ferait
    // basculer « personne à la maison » sans jamais l'expliquer.
    let lan = state.lan_cache.read().await.clone();
    let connu = lan
        .iter()
        .any(|d| d.primary_name.eq_ignore_ascii_case(&nom));
    if !connu && !lan.is_empty() {
        return (
            StatusCode::NOT_FOUND,
            Json(serde_json::json!({
                "error": format!("la Freebox ne connaît aucun appareil nommé « {} »", nom),
                "hint": "voir GET /devices pour la liste exacte"
            })),
        );
    }

    let id = req
        .id
        .map(|i| i.trim().to_string())
        .filter(|i| !i.is_empty())
        .unwrap_or_else(|| {
            nom.to_lowercase()
                .chars()
                .map(|c| if c.is_alphanumeric() { c } else { '_' })
                .collect()
        });

    let conf = config::DeviceConfig {
        freebox_name: nom.clone(),
        device_type: req
            .device_type
            .unwrap_or_else(|| "phone".to_string())
            .trim()
            .to_lowercase(),
        friendly_name: req.friendly_name.filter(|f| !f.trim().is_empty()),
    };

    {
        let mut suivis = state.tracked.write().await;
        suivis.insert(id.clone(), conf.clone());
        let a_ecrire = config::TrackedDevices { devices: suivis.clone() };
        if let Err(e) = a_ecrire.save(&state.config.tracked_path) {
            error!("écriture de {} impossible : {}", state.config.tracked_path, e);
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(serde_json::json!({ "error": format!("écriture impossible : {}", e) })),
            );
        }
    }

    // Recalcul immédiat : sans ça la présence ne bougerait qu'au prochain tour
    // de sonde, et l'ajout aurait l'air de n'avoir rien fait.
    verifier_presence(&state).await;
    info!("appareil suivi ajouté : {} ({})", id, nom);

    (
        StatusCode::OK,
        Json(serde_json::json!({ "status": "suivi", "id": id, "freebox_name": nom })),
    )
}

/// DELETE /tracked/{id} — ne plus suivre un appareil.
async fn tracked_remove_handler(
    axum::extract::State(state): axum::extract::State<Arc<PluginState>>,
    Path(id): Path<String>,
) -> (StatusCode, Json<serde_json::Value>) {
    {
        let mut suivis = state.tracked.write().await;
        if suivis.remove(&id).is_none() {
            return (
                StatusCode::NOT_FOUND,
                Json(serde_json::json!({ "error": format!("appareil « {} » non suivi", id) })),
            );
        }
        let a_ecrire = config::TrackedDevices { devices: suivis.clone() };
        if let Err(e) = a_ecrire.save(&state.config.tracked_path) {
            error!("écriture de {} impossible : {}", state.config.tracked_path, e);
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(serde_json::json!({ "error": format!("écriture impossible : {}", e) })),
            );
        }
    }
    verifier_presence(&state).await;
    info!("appareil retiré du suivi : {}", id);
    (StatusCode::OK, Json(serde_json::json!({ "status": "retiré", "id": id })))
}
