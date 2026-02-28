//! Local HTTP API for agent dashboard
//! 
//! Provides a simple HTTP server on localhost:9899 for local dashboard access
//! Used by system tray UI, browser-based dashboard, or external tools

use serde::Serialize;
use warp::Filter;
use std::collections::VecDeque;
use std::sync::Arc;
use tokio::sync::{RwLock, mpsc};

use crate::windows_utils;

#[derive(Debug, Clone, Serialize)]
pub struct AgentStatus {
    pub agent_id: String,
    pub hostname: String,
    pub version: String,
    pub uptime_seconds: u64,
    pub mqtt_connected: bool,
    pub last_heartbeat: Option<String>,
    pub system: Option<SystemStatus>,
}

#[derive(Debug, Clone, Serialize)]
pub struct SystemStatus {
    pub cpu_percent: f64,
    pub memory_used_mb: u64,
    pub memory_total_mb: u64,
    pub disk_used_gb: Option<f64>,
    pub disk_total_gb: Option<f64>,
    pub process_count: u32,
    pub load_average: Option<f64>,
    pub temperature: Option<f64>,
    pub swap_used_mb: Option<u64>,
    pub swap_total_mb: Option<u64>,
    pub network_rx_bytes: Option<u64>,
    pub network_tx_bytes: Option<u64>,
    pub cpu_cores: Option<usize>,
}

/// A single log entry stored in the ring buffer
#[derive(Debug, Clone, Serialize)]
pub struct LogEntry {
    pub level: String,
    pub message: String,
    pub timestamp: String,
}

/// Maximum number of log entries kept in the ring buffer
const LOG_BUFFER_CAPACITY: usize = 200;

pub struct LocalApiServer {
    status: Arc<RwLock<AgentStatus>>,
    logs: Arc<RwLock<VecDeque<LogEntry>>>,
    reconnect_tx: mpsc::Sender<()>,
    api_token: Option<String>,
}

impl LocalApiServer {
    pub fn new(agent_id: String, hostname: String, reconnect_tx: mpsc::Sender<()>) -> Self {
        let initial_status = AgentStatus {
            agent_id,
            hostname,
            version: env!("CARGO_PKG_VERSION").to_string(),
            uptime_seconds: 0,
            mqtt_connected: false,
            last_heartbeat: None,
            system: None,
        };

        // Read optional API token from env
        let api_token = std::env::var("SYMBION_AGENT_API_TOKEN").ok()
            .filter(|t| !t.is_empty());

        Self {
            status: Arc::new(RwLock::new(initial_status)),
            logs: Arc::new(RwLock::new(VecDeque::with_capacity(LOG_BUFFER_CAPACITY))),
            reconnect_tx,
            api_token,
        }
    }

    /// Start the local API server on port 9899 (localhost only)
    pub async fn start(&self) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        let status = self.status.clone();
        let api_token = self.api_token.clone();

        // GET /status - Agent status and metrics (no auth required)
        let status_route = warp::path("status")
            .and(warp::get())
            .and(warp::any().map(move || status.clone()))
            .and_then(get_status);

        // POST /reconnect - Force MQTT reconnection (auth required)
        let reconnect_tx = self.reconnect_tx.clone();
        let token_for_reconnect = api_token.clone();
        let reconnect_auth = warp::header::optional::<String>("authorization")
            .and(warp::any().map(move || token_for_reconnect.clone()))
            .and_then(|auth_header: Option<String>, expected_token: Option<String>| async move {
                match expected_token {
                    None => Ok::<_, warp::Rejection>(()),
                    Some(expected) => {
                        let provided = auth_header
                            .as_deref()
                            .and_then(|h| h.strip_prefix("Bearer "));
                        if provided == Some(&expected) {
                            Ok(())
                        } else {
                            Err(warp::reject::reject())
                        }
                    }
                }
            })
            .untuple_one();

        let reconnect_route = warp::path("reconnect")
            .and(warp::post())
            .and(reconnect_auth)
            .and(warp::any().map(move || reconnect_tx.clone()))
            .and_then(|tx: mpsc::Sender<()>| async move {
                match tx.try_send(()) {
                    Ok(_) => Ok::<_, warp::Rejection>(warp::reply::json(&serde_json::json!({
                        "success": true,
                        "message": "Reconnect signal sent to agent"
                    }))),
                    Err(_) => Ok(warp::reply::json(&serde_json::json!({
                        "success": false,
                        "message": "Reconnect already in progress"
                    }))),
                }
            });

        // GET /logs - Agent logs from ring buffer
        let logs = self.logs.clone();
        let logs_route = warp::path("logs")
            .and(warp::get())
            .and(warp::any().map(move || logs.clone()))
            .and_then(get_logs);

        // POST /open-config - Open config file in editor
        let open_config_route = warp::path("open-config")
            .and(warp::post())
            .and_then(open_config_handler);

        // Static files for dashboard UI - fallback to embedded HTML
        let ui_route = warp::path::end()
            .and(warp::get())
            .map(|| {
                warp::reply::html(include_str!("../ui/simple-dashboard.html"))
            });

        // CORS for local development
        let cors = warp::cors()
            .allow_any_origin()
            .allow_headers(vec!["content-type"])
            .allow_methods(vec!["GET", "POST"]);

        // POST /open-dashboard - Open local dashboard in browser (auth required)
        let token_for_dashboard = api_token.clone();
        let dashboard_auth = warp::header::optional::<String>("authorization")
            .and(warp::any().map(move || token_for_dashboard.clone()))
            .and_then(|auth_header: Option<String>, expected_token: Option<String>| async move {
                match expected_token {
                    None => Ok::<_, warp::Rejection>(()),
                    Some(expected) => {
                        let provided = auth_header.as_deref().and_then(|h| h.strip_prefix("Bearer "));
                        if provided == Some(&expected) { Ok(()) } else { Err(warp::reject::reject()) }
                    }
                }
            })
            .untuple_one();

        let open_dashboard_route = warp::path("open-dashboard")
            .and(warp::post())
            .and(dashboard_auth)
            .and_then(open_dashboard_handler);

        // GET /update/status - Check for updates (no auth required)
        let update_status_route = warp::path!("update" / "status")
            .and(warp::get())
            .and_then(update_status_handler);

        // POST /update/install - Install available update (auth required)
        let token_for_update = api_token;
        let update_auth = warp::header::optional::<String>("authorization")
            .and(warp::any().map(move || token_for_update.clone()))
            .and_then(|auth_header: Option<String>, expected_token: Option<String>| async move {
                match expected_token {
                    None => Ok::<_, warp::Rejection>(()),
                    Some(expected) => {
                        let provided = auth_header.as_deref().and_then(|h| h.strip_prefix("Bearer "));
                        if provided == Some(&expected) { Ok(()) } else { Err(warp::reject::reject()) }
                    }
                }
            })
            .untuple_one();

        let update_install_route = warp::path!("update" / "install")
            .and(warp::post())
            .and(update_auth)
            .and_then(update_install_handler);

        let routes = status_route
            .or(reconnect_route)
            .or(logs_route)
            .or(ui_route)
            .or(open_dashboard_route)
            .or(open_config_route)
            .or(update_status_route)
            .or(update_install_route)
            .with(cors);

        println!("[local-api] Starting local dashboard server on http://127.0.0.1:9899");

        warp::serve(routes)
            .run(([127, 0, 0, 1], 9899))
            .await;

        Ok(())
    }

    /// Update agent status (called from main agent loop)
    pub async fn update_status(&self, mqtt_connected: bool, system: Option<SystemStatus>) {
        let mut status = self.status.write().await;
        status.mqtt_connected = mqtt_connected;
        status.system = system;
        status.uptime_seconds += 5; // Approximation, updated every 5s

        if mqtt_connected {
            status.last_heartbeat = Some(chrono::Utc::now().to_rfc3339());
        }
    }

    /// Push a log entry into the ring buffer
    pub async fn push_log(&self, level: &str, message: &str) {
        let entry = LogEntry {
            level: level.to_string(),
            message: message.to_string(),
            timestamp: chrono::Utc::now().to_rfc3339(),
        };
        let mut logs = self.logs.write().await;
        if logs.len() >= LOG_BUFFER_CAPACITY {
            logs.pop_front();
        }
        logs.push_back(entry);
    }

    /// Get current status (for external access)
    pub async fn get_current_status(&self) -> AgentStatus {
        self.status.read().await.clone()
    }

    /// Show notification (if feature enabled)
    #[cfg(feature = "notifications")]
    pub fn notify(&self, title: &str, body: &str) {
        use notify_rust::Notification;
        
        if let Err(e) = Notification::new()
            .summary(title)
            .body(body)
            .icon("symbion-agent")
            .timeout(5000)
            .show()
        {
            eprintln!("[local-api] Notification failed: {}", e);
        }
    }

    #[cfg(not(feature = "notifications"))]
    pub fn notify(&self, title: &str, body: &str) {
        println!("[notify] {}: {}", title, body);
    }

    /// Open main PWA in browser
    pub fn open_main_pwa(&self) -> Result<(), std::io::Error> {
        windows_utils::open_url("http://localhost:3000")
    }
}

async fn get_status(
    status: Arc<RwLock<AgentStatus>>,
) -> Result<impl warp::Reply, warp::Rejection> {
    let status = status.read().await;
    Ok(warp::reply::json(&*status))
}

async fn open_dashboard_handler() -> Result<impl warp::Reply, warp::Rejection> {
    // Open local dashboard
    if let Err(e) = open_local_dashboard() {
        return Ok(warp::reply::json(&serde_json::json!({
            "success": false,
            "error": format!("Failed to open dashboard: {}", e)
        })));
    }

    Ok(warp::reply::json(&serde_json::json!({
        "success": true,
        "message": "Dashboard opened in browser"
    })))
}

fn open_local_dashboard() -> Result<(), std::io::Error> {
    windows_utils::open_url("http://localhost:9899")
}

async fn update_status_handler() -> Result<impl warp::Reply, warp::Rejection> {
    use crate::config::AgentConfig;
    use crate::updater::AgentUpdater;

    // Load config and check for updates
    let config = match AgentConfig::load().await {
        Ok(c) => c,
        Err(_) => {
            return Ok(warp::reply::json(&serde_json::json!({
                "update_available": false,
                "error": "Failed to load config"
            })));
        }
    };

    let updater = AgentUpdater::new(config);
    match updater.check_update().await {
        Ok(update_info) => {
            Ok(warp::reply::json(&serde_json::json!({
                "update_available": update_info.is_update_available,
                "current_version": update_info.current_version,
                "latest_version": update_info.latest_version,
                "release_notes": update_info.release_notes,
                "is_critical": update_info.is_critical
            })))
        }
        Err(e) => {
            Ok(warp::reply::json(&serde_json::json!({
                "update_available": false,
                "error": format!("Update check failed: {}", e)
            })))
        }
    }
}

async fn update_install_handler() -> Result<impl warp::Reply, warp::Rejection> {
    use crate::config::AgentConfig;
    use crate::updater::AgentUpdater;

    // Load config and perform update
    let config = match AgentConfig::load().await {
        Ok(c) => c,
        Err(_) => {
            return Ok(warp::reply::json(&serde_json::json!({
                "success": false,
                "error": "Failed to load config"
            })));
        }
    };

    let updater = AgentUpdater::new(config);

    // Check for updates first
    let update_info = match updater.check_update().await {
        Ok(info) => info,
        Err(e) => {
            return Ok(warp::reply::json(&serde_json::json!({
                "success": false,
                "error": format!("Update check failed: {}", e)
            })));
        }
    };

    if !update_info.is_update_available {
        return Ok(warp::reply::json(&serde_json::json!({
            "success": false,
            "error": "No update available"
        })));
    }

    // Perform update in background (will restart agent)
    tokio::spawn(async move {
        if let Err(e) = updater.perform_update(&update_info).await {
            eprintln!("Update failed: {}", e);
        }
    });

    Ok(warp::reply::json(&serde_json::json!({
        "success": true,
        "message": "Update started, agent will restart shortly"
    })))
}

async fn get_logs(
    logs: Arc<RwLock<VecDeque<LogEntry>>>,
) -> Result<impl warp::Reply, warp::Rejection> {
    let logs = logs.read().await;
    let entries: Vec<&LogEntry> = logs.iter().collect();
    Ok(warp::reply::json(&serde_json::json!({ "logs": entries })))
}

async fn open_config_handler() -> Result<impl warp::Reply, warp::Rejection> {
    if let Err(e) = windows_utils::open_config() {
        return Ok(warp::reply::json(&serde_json::json!({
            "success": false,
            "error": format!("Failed to open config: {}", e)
        })));
    }
    Ok(warp::reply::json(&serde_json::json!({
        "success": true,
        "message": "Config file opened"
    })))
}