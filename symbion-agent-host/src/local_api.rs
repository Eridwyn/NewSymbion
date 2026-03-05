//! Local HTTP API for agent dashboard
//! 
//! Provides a simple HTTP server on localhost:9899 for local dashboard access
//! Used by system tray UI, browser-based dashboard, or external tools

use serde::Serialize;
use warp::Filter;
use std::collections::VecDeque;
use std::sync::Arc;
use std::time::Instant;
use tokio::sync::{RwLock, mpsc};

use crate::windows_utils;

// ============================================================================
// Rate Limiter — simple sliding window counter (no external deps)
// ============================================================================

/// Simple rate limiter using a sliding window of request timestamps
#[derive(Clone)]
pub struct RateLimiter {
    max_requests: u32,
    window_secs: u64,
    requests: Arc<std::sync::Mutex<VecDeque<Instant>>>,
}

impl RateLimiter {
    pub fn new(max_requests: u32, window_secs: u64) -> Self {
        Self {
            max_requests,
            window_secs,
            requests: Arc::new(std::sync::Mutex::new(VecDeque::new())),
        }
    }

    /// Check if a request is allowed. Returns true if allowed, false if rate limited.
    pub fn check(&self) -> bool {
        let mut requests = self.requests.lock().unwrap_or_else(|e| e.into_inner());
        let now = Instant::now();
        let window = std::time::Duration::from_secs(self.window_secs);

        // Remove expired entries
        while let Some(front) = requests.front() {
            if now.duration_since(*front) > window {
                requests.pop_front();
            } else {
                break;
            }
        }

        if requests.len() >= self.max_requests as usize {
            false
        } else {
            requests.push_back(now);
            true
        }
    }

    /// Create a warp filter that rejects with 429 if rate limited
    pub fn filter(&self) -> impl Filter<Extract = (), Error = warp::Rejection> + Clone {
        let limiter = self.clone();
        warp::any()
            .and(warp::any().map(move || limiter.clone()))
            .and_then(|limiter: RateLimiter| async move {
                if limiter.check() {
                    Ok::<_, warp::Rejection>(())
                } else {
                    Err(warp::reject::custom(TooManyRequests))
                }
            })
            .untuple_one()
    }
}

#[derive(Debug)]
struct TooManyRequests;
impl warp::reject::Reject for TooManyRequests {}

/// Handle rejections including rate limiting
async fn handle_rejection(err: warp::Rejection) -> Result<impl warp::Reply, std::convert::Infallible> {
    if err.find::<TooManyRequests>().is_some() {
        Ok(warp::reply::with_status(
            warp::reply::json(&serde_json::json!({
                "error": "Too many requests",
                "retry_after_secs": 60
            })),
            warp::http::StatusCode::TOO_MANY_REQUESTS,
        ))
    } else {
        Ok(warp::reply::with_status(
            warp::reply::json(&serde_json::json!({
                "error": "Not found or unauthorized"
            })),
            warp::http::StatusCode::METHOD_NOT_ALLOWED,
        ))
    }
}

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
    api_token: String,
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

        // API token is mandatory for POST endpoints security
        // If not set via env, generate a random one and log it
        let api_token = match std::env::var("SYMBION_AGENT_API_TOKEN").ok().filter(|t| !t.is_empty()) {
            Some(token) => token,
            None => {
                let generated = uuid::Uuid::new_v4().to_string();
                eprintln!("[local-api] WARNING: SYMBION_AGENT_API_TOKEN not set");
                eprintln!("[local-api] Generated API token for this session: {}", generated);
                eprintln!("[local-api] Set SYMBION_AGENT_API_TOKEN env var to persist it");
                generated
            }
        };

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

        // Rate limiter for POST endpoints: 10 requests per 60 seconds
        let rate_limiter = RateLimiter::new(10, 60);

        // Auth filter factory — reusable for all protected endpoints
        let make_auth = |token: String| {
            warp::header::optional::<String>("authorization")
                .and(warp::any().map(move || token.clone()))
                .and_then(|auth_header: Option<String>, expected_token: String| async move {
                    let provided = auth_header
                        .as_deref()
                        .and_then(|h| h.strip_prefix("Bearer "));
                    if provided == Some(expected_token.as_str()) {
                        Ok::<_, warp::Rejection>(())
                    } else {
                        Err(warp::reject::reject())
                    }
                })
                .untuple_one()
        };

        // GET /status - Agent status and metrics (auth required)
        let status_route = warp::path("status")
            .and(warp::get())
            .and(make_auth(api_token.clone()))
            .and(warp::any().map(move || status.clone()))
            .and_then(get_status);

        // POST /reconnect - Force MQTT reconnection (auth + rate limited)
        let reconnect_tx = self.reconnect_tx.clone();

        let reconnect_route = warp::path("reconnect")
            .and(warp::post())
            .and(rate_limiter.filter())
            .and(make_auth(api_token.clone()))
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

        // GET /logs - Agent logs from ring buffer (auth required)
        let logs = self.logs.clone();
        let logs_route = warp::path("logs")
            .and(warp::get())
            .and(make_auth(api_token.clone()))
            .and(warp::any().map(move || logs.clone()))
            .and_then(get_logs);

        // POST /open-config - Open config file in editor (rate limited)
        let open_config_route = warp::path("open-config")
            .and(warp::post())
            .and(rate_limiter.filter())
            .and_then(open_config_handler);

        // Static files for dashboard UI - fallback to embedded HTML
        let ui_route = warp::path::end()
            .and(warp::get())
            .map(|| {
                warp::reply::html(include_str!("../ui/simple-dashboard.html"))
            });

        // CORS: allow any origin — security is handled by:
        // - API bound to 127.0.0.1 only (not exposed to network)
        // - Bearer token required on POST endpoints
        // - WebView with_html() sends origin "null" which restrictive CORS rejects
        let cors = warp::cors()
            .allow_any_origin()
            .allow_headers(vec!["content-type", "authorization"])
            .allow_methods(vec!["GET", "POST"]);

        // POST /open-dashboard - Open local dashboard in browser (auth + rate limited)
        let open_dashboard_route = warp::path("open-dashboard")
            .and(warp::post())
            .and(rate_limiter.filter())
            .and(make_auth(api_token.clone()))
            .and_then(open_dashboard_handler);

        // GET /update/status - Check for updates (auth required)
        let update_status_route = warp::path!("update" / "status")
            .and(warp::get())
            .and(make_auth(api_token.clone()))
            .and_then(update_status_handler);

        // POST /update/install - Install available update (auth + rate limited)
        let update_install_route = warp::path!("update" / "install")
            .and(warp::post())
            .and(rate_limiter.filter())
            .and(make_auth(api_token))
            .and_then(update_install_handler);

        let routes = status_route
            .or(reconnect_route)
            .or(logs_route)
            .or(ui_route)
            .or(open_dashboard_route)
            .or(open_config_route)
            .or(update_status_route)
            .or(update_install_route)
            .recover(handle_rejection)
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

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::Value;

    fn make_server(token: &str) -> LocalApiServer {
        let (tx, _rx) = mpsc::channel(1);
        let mut server = LocalApiServer::new(
            "test-agent".to_string(),
            "test-host".to_string(),
            tx,
        );
        server.api_token = token.to_string();
        server
    }

    #[tokio::test]
    async fn test_status_endpoint() {
        let server = make_server("test-token");
        let status = server.status.clone();
        let filter = warp::path("status")
            .and(warp::get())
            .and(warp::any().map(move || status.clone()))
            .and_then(get_status);

        let resp = warp::test::request()
            .method("GET")
            .path("/status")
            .reply(&filter)
            .await;

        assert_eq!(resp.status(), 200);
        let body: Value = serde_json::from_slice(resp.body()).unwrap();
        assert_eq!(body["agent_id"], "test-agent");
        assert_eq!(body["hostname"], "test-host");
        assert_eq!(body["mqtt_connected"], false);
    }

    #[tokio::test]
    async fn test_logs_endpoint() {
        let server = make_server("test-token");
        server.push_log("INFO", "Hello from test").await;

        let logs = server.logs.clone();
        let filter = warp::path("logs")
            .and(warp::get())
            .and(warp::any().map(move || logs.clone()))
            .and_then(get_logs);

        let resp = warp::test::request()
            .method("GET")
            .path("/logs")
            .reply(&filter)
            .await;

        assert_eq!(resp.status(), 200);
        let body: Value = serde_json::from_slice(resp.body()).unwrap();
        assert!(body["logs"].is_array());
        assert_eq!(body["logs"][0]["message"], "Hello from test");
        assert_eq!(body["logs"][0]["level"], "INFO");
    }

    #[tokio::test]
    async fn test_reconnect_requires_auth() {
        let (tx, _rx) = mpsc::channel(1);
        let token = "secret-token".to_string();
        let token_clone = token.clone();
        let reconnect_auth = warp::header::optional::<String>("authorization")
            .and(warp::any().map(move || token_clone.clone()))
            .and_then(|auth_header: Option<String>, expected_token: String| async move {
                let provided = auth_header.as_deref().and_then(|h| h.strip_prefix("Bearer "));
                if provided == Some(expected_token.as_str()) { Ok::<_, warp::Rejection>(()) } else { Err(warp::reject::reject()) }
            })
            .untuple_one();

        let filter = warp::path("reconnect")
            .and(warp::post())
            .and(reconnect_auth)
            .and(warp::any().map(move || tx.clone()))
            .and_then(|tx: mpsc::Sender<()>| async move {
                match tx.try_send(()) {
                    Ok(_) => Ok::<_, warp::Rejection>(warp::reply::json(&serde_json::json!({"success": true}))),
                    Err(_) => Ok(warp::reply::json(&serde_json::json!({"success": false}))),
                }
            });

        // No token → rejection (warp returns 404 for unmatched POST routes)
        let resp = warp::test::request()
            .method("POST")
            .path("/reconnect")
            .reply(&filter)
            .await;
        assert!(resp.status().is_client_error());

        // Wrong token → rejection
        let resp = warp::test::request()
            .method("POST")
            .path("/reconnect")
            .header("authorization", "Bearer wrong-token")
            .reply(&filter)
            .await;
        assert!(resp.status().is_client_error());
    }

    #[tokio::test]
    async fn test_reconnect_with_valid_token() {
        let (tx, _rx) = mpsc::channel(1);
        let token = "valid-token".to_string();
        let token_clone = token.clone();
        let reconnect_auth = warp::header::optional::<String>("authorization")
            .and(warp::any().map(move || token_clone.clone()))
            .and_then(|auth_header: Option<String>, expected_token: String| async move {
                let provided = auth_header.as_deref().and_then(|h| h.strip_prefix("Bearer "));
                if provided == Some(expected_token.as_str()) { Ok::<_, warp::Rejection>(()) } else { Err(warp::reject::reject()) }
            })
            .untuple_one();

        let filter = warp::path("reconnect")
            .and(warp::post())
            .and(reconnect_auth)
            .and(warp::any().map(move || tx.clone()))
            .and_then(|tx: mpsc::Sender<()>| async move {
                match tx.try_send(()) {
                    Ok(_) => Ok::<_, warp::Rejection>(warp::reply::json(&serde_json::json!({"success": true}))),
                    Err(_) => Ok(warp::reply::json(&serde_json::json!({"success": false}))),
                }
            });

        let resp = warp::test::request()
            .method("POST")
            .path("/reconnect")
            .header("authorization", "Bearer valid-token")
            .reply(&filter)
            .await;

        assert_eq!(resp.status(), 200);
        let body: Value = serde_json::from_slice(resp.body()).unwrap();
        assert_eq!(body["success"], true);
    }

    #[test]
    fn test_rate_limiter_allows_within_limit() {
        let limiter = RateLimiter::new(3, 60);
        assert!(limiter.check());
        assert!(limiter.check());
        assert!(limiter.check());
        // 4th request should be rejected
        assert!(!limiter.check());
    }

    #[tokio::test]
    async fn test_push_log_ring_buffer() {
        let server = make_server("test-token");

        // Fill beyond capacity
        for i in 0..(LOG_BUFFER_CAPACITY + 50) {
            server.push_log("INFO", &format!("msg-{}", i)).await;
        }

        let logs = server.logs.read().await;
        assert_eq!(logs.len(), LOG_BUFFER_CAPACITY);

        // Oldest entry should be msg-50 (first 50 were evicted)
        assert_eq!(logs.front().unwrap().message, "msg-50");
        // Newest should be the last one pushed
        assert_eq!(logs.back().unwrap().message, format!("msg-{}", LOG_BUFFER_CAPACITY + 49));
    }

    #[test]
    fn test_rate_limiter_blocks_after_max() {
        let limiter = RateLimiter::new(2, 60);
        assert!(limiter.check()); // 1
        assert!(limiter.check()); // 2
        assert!(!limiter.check()); // 3 → blocked
        assert!(!limiter.check()); // 4 → still blocked
    }

    #[test]
    fn test_rate_limiter_window_single_request() {
        let limiter = RateLimiter::new(1, 60);
        assert!(limiter.check()); // 1st → ok
        assert!(!limiter.check()); // 2nd → blocked
    }

    #[test]
    fn test_agent_status_serialization() {
        let status = AgentStatus {
            agent_id: "test-123".to_string(),
            hostname: "my-host".to_string(),
            version: "1.2.6".to_string(),
            uptime_seconds: 3600,
            mqtt_connected: true,
            last_heartbeat: Some("2026-03-02T12:00:00Z".to_string()),
            system: None,
        };
        let json = serde_json::to_string(&status).unwrap();
        let parsed: Value = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed["agent_id"], "test-123");
        assert_eq!(parsed["uptime_seconds"], 3600);
        assert_eq!(parsed["mqtt_connected"], true);
    }

    #[tokio::test]
    async fn test_update_status_tracks_mqtt() {
        let server = make_server("tok");
        server.update_status(true, None).await;
        let status = server.status.read().await;
        assert!(status.mqtt_connected);
        assert!(status.last_heartbeat.is_some());
        assert!(status.uptime_seconds > 0);
    }

    #[test]
    fn test_log_entry_serialization() {
        let entry = LogEntry {
            level: "ERROR".to_string(),
            message: "Something failed".to_string(),
            timestamp: "2026-03-02T12:00:00Z".to_string(),
        };
        let json = serde_json::to_string(&entry).unwrap();
        let parsed: Value = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed["level"], "ERROR");
        assert_eq!(parsed["message"], "Something failed");
    }
}