//! Symbion Coffee Plugin — Philips EP2520/10 Integration
//!
//! Controls the coffee machine via the Philips Condor LAN Protocol (HTTPS).
//! Features: brew control, power management, status monitoring, maintenance alerts.

mod condor;
mod mqtt;

use anyhow::{Context, Result};
use axum::extract::State;
use axum::http::StatusCode;
use axum::routing::{get, post};
use axum::{Json, Router};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use symbion_plugin_common::PluginHttpServer;
use tokio::sync::RwLock;
use tokio::time::{interval, Duration};
use tracing::{error, info, warn, Level};
use tracing_subscriber::EnvFilter;

use condor::CondorClient;
use mqtt::MqttPublisher;

const PLUGIN_ID: &str = "coffee";
const SOCKET_PATH: &str = "/run/symbion-plugins/coffee.sock";

// ============================================================================
// Configuration
// ============================================================================

#[derive(Debug, Clone, Deserialize)]
struct Config {
    machine: MachineConfig,
    #[serde(default)]
    mqtt: MqttConfig,
    #[serde(default)]
    polling: PollingConfig,
}

#[derive(Debug, Clone, Deserialize)]
struct MachineConfig {
    ip: String,
    #[serde(default = "default_port")]
    port: u16,
    client_id: String,
    client_secret: String,
}

fn default_port() -> u16 {
    443
}

#[derive(Debug, Clone, Deserialize)]
struct MqttConfig {
    #[serde(default = "default_mqtt_host")]
    host: String,
    #[serde(default = "default_mqtt_port")]
    port: u16,
}

fn default_mqtt_host() -> String {
    "127.0.0.1".to_string()
}
fn default_mqtt_port() -> u16 {
    1883
}

impl Default for MqttConfig {
    fn default() -> Self {
        Self {
            host: default_mqtt_host(),
            port: default_mqtt_port(),
        }
    }
}

#[derive(Debug, Clone, Deserialize)]
struct PollingConfig {
    #[serde(default = "default_status_interval")]
    status_interval_secs: u64,
    #[serde(default = "default_features_interval")]
    features_interval_secs: u64,
}

fn default_status_interval() -> u64 {
    10
}
fn default_features_interval() -> u64 {
    30
}

impl Default for PollingConfig {
    fn default() -> Self {
        Self {
            status_interval_secs: default_status_interval(),
            features_interval_secs: default_features_interval(),
        }
    }
}

impl Config {
    fn load(path: &str) -> Result<Self> {
        let content = std::fs::read_to_string(path)
            .with_context(|| format!("Cannot read config file: {}", path))?;
        toml::from_str(&content).with_context(|| "Invalid TOML config")
    }
}

// ============================================================================
// Machine State
// ============================================================================

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct MachineStatus {
    pub online: bool,
    pub mainstate: u8,
    pub mainstate_text: String,
    pub brewing: bool,
    pub brew_progress: u8,
    pub water_level: u8,
    pub bean_level: u8,
    pub waste_bean: u8,
    pub waste_water: u8,
    pub descale_status: u8,
    pub switch_stat: u8,
    pub maintenance_needed: bool,
    pub maintenance_reason: Option<String>,
    pub last_error: u8,
    pub aquaclean_installed: bool,
    pub aquaclean_remaining: Option<u8>,
    pub last_update: String,
    pub brew_count_today: u32,
    pub last_brew_at: Option<chrono::DateTime<chrono::Utc>>,
    pub brew_count_date: Option<chrono::NaiveDate>,
}

impl MachineStatus {
    fn mainstate_to_text(state: u8) -> String {
        match state {
            1 => "standby".to_string(),
            2 => "ready".to_string(),
            3 => "brewing".to_string(),
            5 => "maintenance".to_string(),
            _ => format!("unknown({})", state),
        }
    }

    fn check_maintenance(&mut self) {
        if self.mainstate == 5 {
            self.maintenance_needed = true;
            if self.switch_stat == 1 {
                self.maintenance_reason = Some("bac egouttage retire".to_string());
            } else if self.waste_bean > 12 {
                self.maintenance_reason = Some("bac marc plein".to_string());
            } else {
                self.maintenance_reason = Some("action maintenance requise".to_string());
            }
        } else if self.water_level == 0 {
            self.maintenance_needed = true;
            self.maintenance_reason = Some("reservoir eau vide".to_string());
        } else if self.descale_status < 5 {
            self.maintenance_needed = true;
            self.maintenance_reason = Some("detartrage necessaire".to_string());
        } else {
            self.maintenance_needed = false;
            self.maintenance_reason = None;
        }
    }
}

/// Shared plugin state
struct PluginState {
    condor: CondorClient,
    mqtt: MqttPublisher,
    status: RwLock<MachineStatus>,
    config: Config,
    started_at: std::time::Instant,
    randnr: RwLock<u32>,
}

impl PluginState {
    fn new(condor: CondorClient, mqtt: MqttPublisher, config: Config) -> Self {
        Self {
            condor,
            mqtt,
            status: RwLock::new(MachineStatus::default()),
            config,
            started_at: std::time::Instant::now(),
            randnr: RwLock::new(1),
        }
    }

    async fn next_randnr(&self) -> u32 {
        let mut r = self.randnr.write().await;
        let val = *r;
        *r = r.wrapping_add(1);
        val
    }
}

// ============================================================================
// Drink types
// ============================================================================

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
enum DrinkType {
    Espresso,
    Coffee,
    HotWater,
}

impl DrinkType {
    fn recipe_book_id(&self) -> u8 {
        match self {
            DrinkType::Espresso => 2,
            DrinkType::Coffee => 6,
            DrinkType::HotWater => 21,
        }
    }

    fn default_gr_dose(&self) -> u8 {
        match self {
            DrinkType::HotWater => 0,
            _ => 3, // STRONG par defaut
        }
    }

    fn label(&self) -> &str {
        match self {
            DrinkType::Espresso => "espresso",
            DrinkType::Coffee => "cafe long",
            DrinkType::HotWater => "eau chaude",
        }
    }
}

impl std::str::FromStr for DrinkType {
    type Err = anyhow::Error;

    fn from_str(s: &str) -> Result<Self> {
        match s {
            "espresso" => Ok(DrinkType::Espresso),
            "coffee" | "cafe" | "cafe_long" => Ok(DrinkType::Coffee),
            "hot_water" | "eau_chaude" | "water" => Ok(DrinkType::HotWater),
            _ => anyhow::bail!("Unknown drink type: {}. Valid: espresso, coffee, hot_water", s),
        }
    }
}

// ============================================================================
// HTTP Handlers
// ============================================================================

#[derive(Debug, Deserialize)]
struct BrewRequest {
    drink: String,
    #[serde(default = "default_temp")]
    temperature: u8,
    #[serde(default = "default_cups")]
    cups: u8,
}

fn default_temp() -> u8 {
    2
}
fn default_cups() -> u8 {
    1
}

#[derive(Debug, Deserialize)]
struct PowerRequest {
    on: bool,
}

async fn health_handler(State(state): State<Arc<PluginState>>) -> Json<serde_json::Value> {
    let status = state.status.read().await;
    Json(serde_json::json!({
        "plugin_id": PLUGIN_ID,
        "spec_version": "1.0",
        "status": if status.online { "healthy" } else { "degraded" },
        "uptime_seconds": state.started_at.elapsed().as_secs(),
        "machine_online": status.online,
    }))
}

async fn status_handler(State(state): State<Arc<PluginState>>) -> Json<MachineStatus> {
    Json(state.status.read().await.clone())
}

async fn brew_handler(
    State(state): State<Arc<PluginState>>,
    Json(req): Json<BrewRequest>,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<serde_json::Value>)> {
    let drink: DrinkType = req.drink.parse().map_err(|e: anyhow::Error| {
        (
            StatusCode::BAD_REQUEST,
            Json(serde_json::json!({"error": e.to_string()})),
        )
    })?;

    if req.temperature < 1 || req.temperature > 3 {
        return Err((
            StatusCode::BAD_REQUEST,
            Json(serde_json::json!({"error": "temperature must be 1-3"})),
        ));
    }

    if req.cups < 1 || req.cups > 2 {
        return Err((
            StatusCode::BAD_REQUEST,
            Json(serde_json::json!({"error": "cups must be 1 or 2"})),
        ));
    }

    // Check machine is ready
    {
        let s = state.status.read().await;
        if !s.online {
            return Err((
                StatusCode::SERVICE_UNAVAILABLE,
                Json(serde_json::json!({"error": "machine offline"})),
            ));
        }
        if s.brewing {
            return Err((
                StatusCode::CONFLICT,
                Json(serde_json::json!({"error": "already brewing"})),
            ));
        }
        if s.mainstate != 2 {
            return Err((
                StatusCode::CONFLICT,
                Json(serde_json::json!({"error": format!("machine not ready (state: {})", s.mainstate_text)})),
            ));
        }
    }

    let randnr = state.next_randnr().await;
    let nr_of_brews = req.cups - 1; // 0-indexed

    // Step 1: Write BasicRecipe
    let recipe = serde_json::json!({
        "RecipeBookId": drink.recipe_book_id(),
        "GrDose": drink.default_gr_dose(),
        "NrOfBrews": nr_of_brews,
        "Temperature": req.temperature,
        "PrimDose": 0,
        "SecDose": 0,
        "randnr": randnr,
    });

    state
        .condor
        .put("1/command/BasicRecipe", &recipe)
        .await
        .map_err(|e| {
            error!("BasicRecipe write failed: {}", e);
            (
                StatusCode::BAD_GATEWAY,
                Json(serde_json::json!({"error": format!("machine communication error: {}", e)})),
            )
        })?;

    // Step 2: Start brewing
    state
        .condor
        .put("1/command", &serde_json::json!({"processctrl": 5}))
        .await
        .map_err(|e| {
            error!("processctrl write failed: {}", e);
            (
                StatusCode::BAD_GATEWAY,
                Json(serde_json::json!({"error": format!("machine communication error: {}", e)})),
            )
        })?;

    info!(
        "Brewing started: {} (temp={}, cups={})",
        drink.label(),
        req.temperature,
        req.cups
    );

    // Publish MQTT event
    state
        .mqtt
        .publish_event(
            "brewing/started",
            &serde_json::json!({
                "drink": drink.label(),
                "temperature": req.temperature,
                "cups": req.cups,
            }),
        )
        .await;

    Ok(Json(serde_json::json!({
        "status": "brewing",
        "drink": drink.label(),
        "temperature": req.temperature,
        "cups": req.cups,
    })))
}

async fn stop_handler(
    State(state): State<Arc<PluginState>>,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<serde_json::Value>)> {
    state
        .condor
        .put("1/command", &serde_json::json!({"processctrl": 2}))
        .await
        .map_err(|e| {
            (
                StatusCode::BAD_GATEWAY,
                Json(serde_json::json!({"error": format!("machine communication error: {}", e)})),
            )
        })?;

    info!("Brew stopped");
    Ok(Json(serde_json::json!({"status": "stopped"})))
}

async fn info_handler(
    State(state): State<Arc<PluginState>>,
) -> Json<serde_json::Value> {
    // Fetch device info, firmware, config from machine
    let mut device_info = serde_json::json!({ "model": "Philips EP2520/10", "protocol": "Condor LAN" });
    let mut firmware_info = serde_json::json!({});
    let mut replenishment = serde_json::json!({});

    // Fetch in parallel
    let (dev_res, fw_res, rep_res, cfg_res) = tokio::join!(
        state.condor.get("1/deviceinfo"),
        state.condor.get("1/firmwareinfo"),
        state.condor.get("1/replenishment"),
        state.condor.get("1/configuration"),
    );

    if let Ok(d) = dev_res { device_info = d; }
    if let Ok(f) = fw_res { firmware_info = f; }
    if let Ok(r) = rep_res { replenishment = r; }
    let config_data = cfg_res.unwrap_or(serde_json::json!({}));

    let status = state.status.read().await;

    Json(serde_json::json!({
        "machine": {
            "model": "Philips EP2520/10",
            "ip": state.config.machine.ip,
            "port": state.config.machine.port,
            "online": status.online,
            "mainstate": status.mainstate_text,
        },
        "device_info": device_info,
        "firmware": firmware_info,
        "configuration": config_data,
        "replenishment": replenishment,
        "uptime_seconds": state.started_at.elapsed().as_secs(),
        "drinks_available": ["espresso", "coffee", "hot_water"],
    }))
}

async fn configuration_handler(
    State(state): State<Arc<PluginState>>,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<serde_json::Value>)> {
    state
        .condor
        .get("1/configuration")
        .await
        .map(Json)
        .map_err(|e| {
            (
                StatusCode::BAD_GATEWAY,
                Json(serde_json::json!({"error": format!("machine communication error: {}", e)})),
            )
        })
}

async fn power_handler(
    State(state): State<Arc<PluginState>>,
    Json(req): Json<PowerRequest>,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<serde_json::Value>)> {
    let power_val = if req.on { 2 } else { 1 };
    state
        .condor
        .put("1/command", &serde_json::json!({"power": power_val}))
        .await
        .map_err(|e| {
            (
                StatusCode::BAD_GATEWAY,
                Json(serde_json::json!({"error": format!("machine communication error: {}", e)})),
            )
        })?;

    let action = if req.on { "on" } else { "off" };
    info!("Power {}", action);

    state
        .mqtt
        .publish_event("power", &serde_json::json!({"power": action}))
        .await;

    Ok(Json(serde_json::json!({"status": format!("power_{}", action)})))
}

// ============================================================================
// Polling loop
// ============================================================================

async fn poll_status(state: Arc<PluginState>) {
    let mut status_interval = interval(Duration::from_secs(state.config.polling.status_interval_secs));
    let mut features_interval = interval(Duration::from_secs(state.config.polling.features_interval_secs));
    let mut prev_brewing = false;
    let mut prev_mainstate: u8 = 0;

    loop {
        tokio::select! {
            _ = status_interval.tick() => {
                match state.condor.get("1/machinestatus").await {
                    Ok(data) => {
                        let mut s = state.status.write().await;
                        s.online = true;
                        s.mainstate = data["mainstate"].as_u64().unwrap_or(0) as u8;
                        s.mainstate_text = MachineStatus::mainstate_to_text(s.mainstate);
                        s.brewing = s.mainstate == 3;
                        s.brew_progress = data["Progress"].as_u64().unwrap_or(0) as u8;
                        s.water_level = data["waterlevel"].as_u64().unwrap_or(0) as u8;
                        s.bean_level = data["beanlevel"].as_u64().unwrap_or(0) as u8;
                        s.waste_bean = data["wastebean"].as_u64().unwrap_or(0) as u8;
                        s.waste_water = data["wastewater"].as_u64().unwrap_or(0) as u8;
                        s.descale_status = data["Descalestat"].as_u64().unwrap_or(0) as u8;
                        s.switch_stat = data["switchstat"].as_u64().unwrap_or(0) as u8;
                        s.last_error = data["lasterror"].as_u64().unwrap_or(0) as u8;
                        let filternr = data["Filternr"].as_u64().unwrap_or(0) as u8;
                        s.aquaclean_installed = filternr > 0;
                        s.aquaclean_remaining = if filternr > 0 {
                            Some(data["Filterstat"].as_u64().unwrap_or(0) as u8)
                        } else {
                            None
                        };
                        s.last_update = chrono::Utc::now().to_rfc3339();
                        s.check_maintenance();

                        // Detect state transitions
                        let current_brewing = s.brewing;
                        let current_mainstate = s.mainstate;
                        let maintenance = s.maintenance_needed;
                        let reason = s.maintenance_reason.clone();
                        drop(s);

                        // Brewing completed
                        if prev_brewing && !current_brewing {
                            info!("Brewing completed");
                            {
                                let mut s = state.status.write().await;
                                let today = chrono::Utc::now().date_naive();
                                if s.brew_count_date != Some(today) {
                                    s.brew_count_today = 0;
                                    s.brew_count_date = Some(today);
                                }
                                s.brew_count_today = s.brew_count_today.saturating_add(1);
                                s.last_brew_at = Some(chrono::Utc::now());
                            }
                            state.mqtt.publish_event("brewing/completed", &serde_json::json!({})).await;
                        }

                        // Maintenance alert
                        if maintenance && current_mainstate != prev_mainstate {
                            if let Some(r) = &reason {
                                warn!("Maintenance alert: {}", r);
                                state.mqtt.publish_event("maintenance/alert", &serde_json::json!({
                                    "reason": r,
                                })).await;
                            }
                        }

                        // Publish status to MQTT for PWA widget
                        let s = state.status.read().await;
                        state.mqtt.publish_event("status", &serde_json::json!({
                            "online": s.online,
                            "mainstate": s.mainstate,
                            "mainstate_text": &s.mainstate_text,
                            "brewing": s.brewing,
                            "brew_progress": s.brew_progress,
                            "water_level": s.water_level,
                            "bean_level": s.bean_level,
                            "waste_bean": s.waste_bean,
                            "descale_status": s.descale_status,
                            "maintenance_needed": s.maintenance_needed,
                            "maintenance_reason": &s.maintenance_reason,
                            "aquaclean_installed": s.aquaclean_installed,
                            "aquaclean_remaining": s.aquaclean_remaining,
                            "last_update": &s.last_update,
                        })).await;
                        drop(s);

                        prev_brewing = current_brewing;
                        prev_mainstate = current_mainstate;
                    }
                    Err(e) => {
                        let mut s = state.status.write().await;
                        if s.online {
                            warn!("Machine offline: {}", e);
                            s.online = false;
                            s.last_update = chrono::Utc::now().to_rfc3339();
                        }
                    }
                }
            }
            _ = features_interval.tick() => {
                let s = state.status.read().await;
                state.mqtt.publish_features(&s).await;
            }
        }
    }
}

// ============================================================================
// Main
// ============================================================================

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(
            EnvFilter::from_default_env()
                .add_directive(Level::INFO.into())
                .add_directive("symbion_plugin_coffee=debug".parse().unwrap()),
        )
        .init();

    info!("Starting Symbion Coffee Plugin v{}", env!("CARGO_PKG_VERSION"));

    // Load config
    let config_path = std::env::var("COFFEE_CONFIG")
        .unwrap_or_else(|_| "/opt/symbion/config/coffee.toml".to_string());

    let config = Config::load(&config_path)
        .with_context(|| format!("Failed to load config from {}", config_path))?;

    info!(
        "Machine: {}:{} (Philips Condor)",
        config.machine.ip, config.machine.port
    );

    // Initialize Condor client
    let condor = CondorClient::new(
        &config.machine.ip,
        config.machine.port,
        &config.machine.client_id,
        &config.machine.client_secret,
    )?;

    // Test connectivity
    match condor.get("1/machinestatus").await {
        Ok(data) => {
            let state = data["mainstate"].as_u64().unwrap_or(0);
            info!("Machine connected (mainstate={})", state);
        }
        Err(e) => {
            warn!("Machine not reachable at startup (will retry): {}", e);
        }
    }

    // Initialize MQTT
    let mqtt = MqttPublisher::new(&config.mqtt.host, config.mqtt.port).await?;
    info!("MQTT connected to {}:{}", config.mqtt.host, config.mqtt.port);

    // Shared state
    let state = Arc::new(PluginState::new(condor, mqtt, config));

    // HTTP routes
    let router = Router::new()
        .route("/health", get(health_handler))
        .route("/status", get(status_handler))
        .route("/info", get(info_handler))
        .route("/configuration", get(configuration_handler))
        .route("/brew", post(brew_handler))
        .route("/stop", post(stop_handler))
        .route("/power", post(power_handler))
        .with_state(state.clone());

    // Start polling
    let poll_state = state.clone();
    tokio::spawn(async move {
        poll_status(poll_state).await;
    });

    // Register with kernel
    tokio::spawn(async {
        tokio::time::sleep(Duration::from_secs(2)).await;
        if let Err(e) = symbion_plugin_common::PluginRegistrationBuilder::new(PLUGIN_ID, SOCKET_PATH)
            .route("/status")
            .route("/info")
            .route("/configuration")
            .route("/brew")
            .route("/stop")
            .route("/power")
            .route("/health")
            .version(env!("CARGO_PKG_VERSION"))
            .description("Philips EP2520 coffee machine integration")
            .register()
            .await
        {
            warn!("Kernel registration failed (will work standalone): {}", e);
        }
    });

    // Serve on Unix socket
    info!("Serving on {}", SOCKET_PATH);
    PluginHttpServer::new(SOCKET_PATH, router)
        .serve()
        .await
        .map_err(|e| anyhow::anyhow!("{}", e))?;

    Ok(())
}
