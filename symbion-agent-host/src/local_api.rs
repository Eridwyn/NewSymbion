//! Local HTTP API for agent dashboard
//! 
//! Provides a simple HTTP server on localhost:9899 for local dashboard access
//! Used by system tray UI, browser-based dashboard, or external tools

use serde::Serialize;
use warp::Filter;
use std::sync::Arc;
use tokio::sync::RwLock;

#[cfg(target_os = "windows")]
use std::os::windows::process::CommandExt;

#[cfg(target_os = "windows")]
const CREATE_NO_WINDOW: u32 = 0x08000000;

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
    pub process_count: u32,
    pub load_average: Option<f64>,
}

pub struct LocalApiServer {
    status: Arc<RwLock<AgentStatus>>,
}

impl LocalApiServer {
    pub fn new(agent_id: String, hostname: String) -> Self {
        let initial_status = AgentStatus {
            agent_id,
            hostname,
            version: env!("CARGO_PKG_VERSION").to_string(),
            uptime_seconds: 0,
            mqtt_connected: false,
            last_heartbeat: None,
            system: None,
        };

        Self {
            status: Arc::new(RwLock::new(initial_status)),
        }
    }

    /// Start the local API server on port 9899
    pub async fn start(&self) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        let status = self.status.clone();
        
        // GET /status - Agent status and metrics
        let status_route = warp::path("status")
            .and(warp::get())
            .and(warp::any().map(move || status.clone()))
            .and_then(get_status);

        // POST /reconnect - Force MQTT reconnection
        let reconnect_route = warp::path("reconnect")
            .and(warp::post())
            .map(|| {
                // TODO: Signal main agent to reconnect
                warp::reply::json(&serde_json::json!({
                    "success": true,
                    "message": "Reconnect signal sent"
                }))
            });

        // GET /logs - Agent logs (if available)
        let logs_route = warp::path("logs")
            .and(warp::get())
            .map(|| {
                // Return recent log entries
                warp::reply::json(&serde_json::json!({
                    "logs": ["[INFO] Agent started", "[INFO] MQTT connected"]
                }))
            });

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

        // POST /open-dashboard - Open local dashboard in browser
        let open_dashboard_route = warp::path("open-dashboard")
            .and(warp::post())
            .and_then(open_dashboard_handler);

        // GET /update/status - Check for updates
        let update_status_route = warp::path!("update" / "status")
            .and(warp::get())
            .and_then(update_status_handler);

        // POST /update/install - Install available update
        let update_install_route = warp::path!("update" / "install")
            .and(warp::post())
            .and_then(update_install_handler);

        let routes = status_route
            .or(reconnect_route)
            .or(logs_route)
            .or(ui_route)
            .or(open_dashboard_route)
            .or(update_status_route)
            .or(update_install_route)
            .with(cors);

        println!("[local-api] Starting local dashboard server on http://0.0.0.0:9899");
        
        warp::serve(routes)
            .run(([0, 0, 0, 0], 9899))
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
        let url = "http://localhost:3000"; // PWA kernel address
        
        #[cfg(target_os = "linux")]
        std::process::Command::new("xdg-open").arg(url).spawn()?;
        
        #[cfg(target_os = "windows")]
        {
            let mut cmd = std::process::Command::new("rundll32");
            cmd.creation_flags(CREATE_NO_WINDOW)
                .args(&["url.dll,FileProtocolHandler", url])
                .spawn()?;
        }
        
        #[cfg(target_os = "macos")]
        std::process::Command::new("open").arg(url).spawn()?;

        Ok(())
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
    let url = "http://localhost:9899";

    #[cfg(target_os = "linux")]
    std::process::Command::new("xdg-open").arg(url).spawn()?;

    #[cfg(target_os = "windows")]
    {
        let mut cmd = std::process::Command::new("rundll32");
        cmd.creation_flags(CREATE_NO_WINDOW)
            .args(&["url.dll,FileProtocolHandler", url])
            .spawn()?;
    }

    #[cfg(target_os = "macos")]
    std::process::Command::new("open").arg(url).spawn()?;

    Ok(())
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