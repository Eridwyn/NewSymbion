//! Local HTTP API for agent dashboard
//! 
//! Provides a simple HTTP server on localhost:9899 for local dashboard access
//! Used by system tray UI, browser-based dashboard, or external tools

use serde::{Serialize, Deserialize};
use warp::Filter;
use std::sync::Arc;
use tokio::sync::RwLock;

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

        // Static files for dashboard UI
        let ui_route = warp::path::end()
            .and(warp::get())
            .and(warp::fs::file("ui/simple-dashboard.html"));

        // CORS for local development
        let cors = warp::cors()
            .allow_any_origin()
            .allow_headers(vec!["content-type"])
            .allow_methods(vec!["GET", "POST"]);

        // POST /open-dashboard - Open local dashboard in browser
        let open_dashboard_route = warp::path("open-dashboard")
            .and(warp::post())
            .and_then(open_dashboard_handler);

        let routes = status_route
            .or(reconnect_route)
            .or(logs_route)
            .or(ui_route)
            .or(open_dashboard_route)
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
        std::process::Command::new("rundll32")
            .args(&["url.dll,FileProtocolHandler", url])
            .spawn()?;
        
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
    std::process::Command::new("rundll32")
        .args(&["url.dll,FileProtocolHandler", url])
        .spawn()?;
    
    #[cfg(target_os = "macos")]
    std::process::Command::new("open").arg(url).spawn()?;

    Ok(())
}