/**
 * PLUGIN REVERSE PROXY - Dynamic routing via Unix sockets
 *
 * Architecture:
 * - Kernel scans /tmp/symbion-plugin-*.sock at startup
 * - Plugins register routes dynamically
 * - Kernel proxies authenticated HTTPS requests to plugin Unix sockets
 * - Plugins don't handle TLS/JWT - kernel does it
 */

use axum::{
    body::Body,
    extract::{Request, State, Json},
    http::{StatusCode, Uri},
    response::{IntoResponse, Response},
};
use http_body_util::{BodyExt, Empty, Full};
use hyper::body::{Incoming, Bytes};
use hyper_util::rt::TokioIo;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::os::unix::fs::FileTypeExt;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use tokio::sync::RwLock;

/// Plugin registration request payload
#[derive(Debug, Deserialize)]
pub struct PluginRegistration {
    pub name: String,
    pub socket_path: String,
    pub routes: Vec<String>,
    #[serde(default)]
    pub version: Option<String>,
    #[serde(default)]
    pub description: Option<String>,
}

/// Plugin metadata stored in registry
#[derive(Debug, Clone, Serialize)]
pub struct PluginInfo {
    pub name: String,
    pub socket_path: PathBuf,
    pub routes: Vec<String>,
    pub version: Option<String>,
    pub description: Option<String>,
    pub registered_at: chrono::DateTime<chrono::Utc>,
}

/// Plugin route registry - maps path prefixes to Unix socket paths
#[derive(Clone)]
pub struct PluginRegistry {
    // Map: route_prefix -> PluginInfo
    plugins: Arc<RwLock<HashMap<String, PluginInfo>>>,
}

impl PluginRegistry {
    pub fn new() -> Self {
        Self {
            plugins: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// Register a plugin dynamically (hot reload)
    pub async fn register_plugin(&self, registration: PluginRegistration) -> anyhow::Result<()> {
        let mut plugins = self.plugins.write().await;

        let plugin_info = PluginInfo {
            name: registration.name.clone(),
            socket_path: PathBuf::from(&registration.socket_path),
            routes: registration.routes.clone(),
            version: registration.version,
            description: registration.description,
            registered_at: chrono::Utc::now(),
        };

        // Validate socket exists
        if !plugin_info.socket_path.exists() {
            anyhow::bail!("Socket path does not exist: {}", plugin_info.socket_path.display());
        }

        // Register each route for this plugin
        for route in &plugin_info.routes {
            let full_route = format!("/v1/plugin-api/{}{}", registration.name, route);
            plugins.insert(full_route.clone(), plugin_info.clone());
            println!("[plugin-proxy] Registered route: {} -> {}",
                     full_route, plugin_info.socket_path.display());
        }

        println!("[plugin-proxy] Plugin '{}' registered successfully with {} routes",
                 registration.name, registration.routes.len());
        Ok(())
    }

    /// Unregister a plugin (for cleanup)
    pub async fn unregister_plugin(&self, plugin_name: &str) -> anyhow::Result<()> {
        let mut plugins = self.plugins.write().await;
        plugins.retain(|_, info| info.name != plugin_name);
        println!("[plugin-proxy] Plugin '{}' unregistered", plugin_name);
        Ok(())
    }

    /// List all registered plugins
    pub async fn list_plugins(&self) -> Vec<PluginInfo> {
        let plugins = self.plugins.read().await;
        let mut unique_plugins: HashMap<String, PluginInfo> = HashMap::new();

        for (_, info) in plugins.iter() {
            unique_plugins.insert(info.name.clone(), info.clone());
        }

        unique_plugins.into_values().collect()
    }

    /// Scan /run/symbion-plugins/ for Unix sockets and auto-register plugins
    pub async fn discover_plugins(&self) -> anyhow::Result<()> {
        println!("[plugin-proxy] Starting plugin discovery in /run/symbion-plugins/...");

        let socket_dir = Path::new("/run/symbion-plugins");

        if !socket_dir.exists() {
            println!("[plugin-proxy] Plugin directory does not exist, skipping discovery");
            return Ok(());
        }

        let mut discovered_count = 0;

        // Scan directory for symbion-plugin-*.sock files
        let entries = match std::fs::read_dir(socket_dir) {
            Ok(entries) => entries,
            Err(e) => {
                eprintln!("[plugin-proxy] Failed to read plugin directory: {}", e);
                return Ok(());
            }
        };

        for entry in entries.flatten() {
            let path = entry.path();

            // Check if file is a socket matching pattern symbion-plugin-*.sock
            let is_socket = match path.metadata() {
                Ok(metadata) => metadata.file_type().is_socket(),
                Err(_) => false,
            };

            if !is_socket {
                continue;
            }

            let filename = match path.file_name().and_then(|n| n.to_str()) {
                Some(name) => name,
                None => continue,
            };

            // Accept both patterns: symbion-plugin-NAME.sock OR NAME.sock
            if !filename.ends_with(".sock") {
                continue;
            }

            // Extract plugin name from filename
            let plugin_name = if filename.starts_with("symbion-plugin-") {
                // symbion-plugin-NAME.sock → NAME
                filename
                    .trim_start_matches("symbion-plugin-")
                    .trim_end_matches(".sock")
            } else {
                // NAME.sock → NAME
                filename.trim_end_matches(".sock")
            };

            println!("[plugin-proxy] Discovered socket: {} (plugin: {})", filename, plugin_name);

            // Query /health endpoint to get plugin info
            match self.query_plugin_health(&path, plugin_name).await {
                Ok((version, description)) => {
                    // Auto-register with standard route convention
                    let routes = vec![
                        format!("/{}", plugin_name),  // /notes, /notifications, /sensors
                    ];

                    let registration = PluginRegistration {
                        name: plugin_name.to_string(),
                        socket_path: path.to_string_lossy().to_string(),
                        routes,
                        version: Some(version),
                        description,
                    };

                    if let Err(e) = self.register_plugin(registration).await {
                        eprintln!("[plugin-proxy] Failed to register {}: {}", plugin_name, e);
                    } else {
                        discovered_count += 1;
                    }
                }
                Err(e) => {
                    eprintln!("[plugin-proxy] Failed to query health for {}: {}", plugin_name, e);
                }
            }
        }

        println!("[plugin-proxy] Discovery complete: {} plugins registered", discovered_count);
        Ok(())
    }

    /// Query plugin /health endpoint via Unix socket
    async fn query_plugin_health(&self, socket_path: &Path, plugin_name: &str) -> anyhow::Result<(String, Option<String>)> {
        use hyper::body::Incoming;
        use hyper_util::client::legacy::connect::HttpConnector;
        use tower::ServiceExt;

        // Connect to Unix socket
        let stream = tokio::net::UnixStream::connect(socket_path).await?;
        let io = TokioIo::new(stream);

        // Build HTTP request
        let (mut sender, conn) = hyper::client::conn::http1::handshake(io).await?;

        tokio::spawn(async move {
            if let Err(e) = conn.await {
                eprintln!("[plugin-proxy] Connection error: {}", e);
            }
        });

        let req = hyper::Request::builder()
            .uri("/health")
            .header("Host", "localhost")
            .body(Empty::<Bytes>::new())?;

        let res = sender.send_request(req).await?;

        if res.status() != StatusCode::OK {
            anyhow::bail!("Health endpoint returned {}", res.status());
        }

        // Read response body
        let body_bytes = res.into_body().collect().await?.to_bytes();

        // Parse JSON response (Contract v1.0 compatible)
        #[derive(serde::Deserialize)]
        struct HealthResponse {
            #[serde(alias = "plugin")]
            plugin_id: String,
            #[serde(alias = "version")]
            spec_version: String,
            #[serde(default)]
            description: Option<String>,
        }

        let health: HealthResponse = serde_json::from_slice(&body_bytes)?;

        println!("[plugin-proxy] Health check OK: {} v{}", health.plugin_id, health.spec_version);

        Ok((health.spec_version, health.description))
    }

    /// Find Unix socket for a given path
    pub async fn find_socket(&self, path: &str) -> Option<PathBuf> {
        let plugins = self.plugins.read().await;

        println!("[plugin-proxy] DEBUG - find_socket searching for: {}", path);
        println!("[plugin-proxy] DEBUG - Registered routes:");
        for (route, info) in plugins.iter() {
            println!("  - {} -> {}", route, info.socket_path.display());
        }

        // Find longest matching route (supports parameterized routes like :id)
        let mut best_match: Option<(&String, &PluginInfo)> = None;
        for (route_pattern, plugin_info) in plugins.iter() {
            if Self::route_matches(route_pattern, path) {
                if let Some((best_prefix, _)) = best_match {
                    if route_pattern.len() > best_prefix.len() {
                        best_match = Some((route_pattern, plugin_info));
                    }
                } else {
                    best_match = Some((route_pattern, plugin_info));
                }
            }
        }

        if let Some((matched_route, _)) = best_match {
            println!("[plugin-proxy] DEBUG - Matched route: {}", matched_route);
        } else {
            println!("[plugin-proxy] DEBUG - No matching route found");
        }

        best_match.map(|(_, info)| info.socket_path.clone())
    }

    /// Check if a path matches a route pattern (supports :param segments and prefix matching)
    ///
    /// Examples:
    /// - `/v1/plugin-api/notes/notes` matches `/v1/plugin-api/notes/notes`
    /// - `/v1/plugin-api/notes/123` matches `/v1/plugin-api/notes/:id`
    /// - `/v1/plugin-api/sensors/environment/chambre` matches `/v1/plugin-api/sensors/environment/:room_id`
    /// - `/v1/plugin-api/notes/notes/uuid-123` matches `/v1/plugin-api/notes/notes` (prefix match for REST path parameters)
    fn route_matches(pattern: &str, path: &str) -> bool {
        let pattern_segments: Vec<&str> = pattern.split('/').collect();
        let path_segments: Vec<&str> = path.split('/').collect();

        // Path must have at least as many segments as pattern
        // Allow more segments for REST path parameters (e.g., /notes/notes/:id)
        if path_segments.len() < pattern_segments.len() {
            return false;
        }

        // Check each segment in pattern against path
        for (i, pattern_seg) in pattern_segments.iter().enumerate() {
            // If pattern segment starts with ':', it's a parameter - matches anything
            if pattern_seg.starts_with(':') {
                continue;
            }
            // Otherwise must be exact match
            if pattern_seg != &path_segments[i] {
                return false;
            }
        }

        // Pattern matches! Extra path segments are treated as path parameters
        true
    }
}

/// Proxy handler - forwards authenticated requests to plugin Unix sockets
pub async fn proxy_to_plugin(
    State(app_state): State<crate::http::AppState>,
    req: Request,
) -> Response {
    let path = req.uri().path();

    // Only handle plugin-related paths - return 404 for other paths
    // This prevents the fallback from catching unrelated routes like /auth/*
    if !path.starts_with("/plugin-api/")
        && !path.starts_with("/v1/plugin-api/")
        && !path.starts_with("/plugins/")
    {
        return (
            StatusCode::NOT_FOUND,
            axum::Json(serde_json::json!({
                "error": "Not found",
                "path": path
            })),
        ).into_response();
    }

    // When merged (not nested), the full path is preserved minus the /v1 prefix
    // So we receive paths like /plugin-api/notifications/notifications
    // We need to add back the /v1 prefix for registry lookup
    let full_path = if path.starts_with("/plugin-api/") {
        format!("/v1{}", path)
    } else if path.starts_with("/v1/plugin-api/") {
        path.to_string()
    } else if path.starts_with("/plugins/") {
        // Handle legacy /v1/plugins/* routes by converting to /v1/plugin-api/*
        path.replace("/plugins/", "/v1/plugin-api/")
    } else {
        // This branch should never be reached due to the guard above
        return (
            StatusCode::NOT_FOUND,
            axum::Json(serde_json::json!({
                "error": "Not found",
                "path": path
            })),
        ).into_response();
    };

    // Find plugin socket for this path
    let socket_path = match app_state.plugin_registry.find_socket(&full_path).await {
        Some(s) => s,
        None => {
            return (
                StatusCode::NOT_FOUND,
                format!("No plugin registered for path: {}", full_path),
            ).into_response();
        }
    };

    // Strip /plugin-api/{plugin_name} prefix from path before forwarding to plugin
    // Router sends: /plugin-api/notifications/notifications
    // We need to forward: /notifications (strip first 2 segments)
    let forwarded_path = path
        .strip_prefix("/plugin-api/")
        .and_then(|remaining| {
            // remaining is "notifications/notifications"
            // Split on first '/' to get plugin_name and actual_path
            remaining.split_once('/').map(|(_, rest)| format!("/{}", rest))
        })
        .unwrap_or_else(|| path.to_string());

    println!("[plugin-proxy] DEBUG - Forwarded path to plugin: {}", forwarded_path);
    println!("[plugin-proxy] Proxying {} -> {} (socket: {})",
             full_path, forwarded_path, socket_path.display());

    // Connect to Unix socket
    let stream = match tokio::net::UnixStream::connect(&socket_path).await {
        Ok(s) => s,
        Err(e) => {
            eprintln!("[plugin-proxy] Failed to connect to socket {}: {}", socket_path.display(), e);
            return (
                StatusCode::BAD_GATEWAY,
                format!("Plugin unreachable: {}", e),
            ).into_response();
        }
    };

    // Build new request with forwarded path
    let (parts, body) = req.into_parts();
    let mut new_uri_parts = parts.uri.into_parts();
    new_uri_parts.path_and_query = Some(
        forwarded_path.parse().unwrap_or_else(|_| "/".parse().expect("static / path"))
    );
    // [SECURITY] P0-4: Handle URI parsing errors gracefully
    let new_uri = match Uri::from_parts(new_uri_parts) {
        Ok(uri) => uri,
        Err(e) => {
            eprintln!("[plugin-proxy] Failed to build forwarded URI: {}", e);
            return (StatusCode::BAD_REQUEST, "Invalid request URI").into_response();
        }
    };

    let forwarded_req = hyper::Request::builder()
        .method(parts.method)
        .uri(new_uri)
        .version(parts.version);

    // Copy headers
    let mut forwarded_req = parts.headers.iter().fold(
        forwarded_req,
        |builder, (name, value)| builder.header(name, value)
    );

    // Attach body
    let body_bytes = match body.collect().await {
        Ok(collected) => collected.to_bytes(),
        Err(e) => {
            eprintln!("[plugin-proxy] Failed to read request body: {}", e);
            return (StatusCode::BAD_REQUEST, "Failed to read request body").into_response();
        }
    };

    // [SECURITY] P0-4: Handle request building errors gracefully
    let forwarded_req = match forwarded_req.body(Full::new(body_bytes)) {
        Ok(req) => req,
        Err(e) => {
            eprintln!("[plugin-proxy] Failed to build forwarded request: {}", e);
            return (StatusCode::INTERNAL_SERVER_ERROR, "Failed to build request").into_response();
        }
    };

    // Send request via Unix socket using hyper client
    let io = TokioIo::new(stream);

    let (mut sender, conn) = match hyper::client::conn::http1::handshake(io).await {
        Ok(c) => c,
        Err(e) => {
            eprintln!("[plugin-proxy] HTTP handshake failed: {}", e);
            return (StatusCode::BAD_GATEWAY, "Plugin handshake failed").into_response();
        }
    };

    // Spawn connection task
    tokio::spawn(async move {
        if let Err(e) = conn.await {
            eprintln!("[plugin-proxy] Connection error: {}", e);
        }
    });

    // Send request to plugin
    let plugin_response = match sender.send_request(forwarded_req).await {
        Ok(r) => r,
        Err(e) => {
            eprintln!("[plugin-proxy] Failed to send request to plugin: {}", e);
            return (StatusCode::BAD_GATEWAY, "Plugin request failed").into_response();
        }
    };

    // Convert plugin response to axum response
    let (parts, body) = plugin_response.into_parts();
    let body_bytes = match body.collect().await {
        Ok(collected) => collected.to_bytes(),
        Err(e) => {
            eprintln!("[plugin-proxy] Failed to read plugin response: {}", e);
            return (StatusCode::BAD_GATEWAY, "Failed to read plugin response").into_response();
        }
    };

    let mut response = Response::builder()
        .status(parts.status)
        .version(parts.version);

    // Copy response headers
    for (name, value) in parts.headers.iter() {
        response = response.header(name, value);
    }

    // [SECURITY] P0-4: Handle response building errors gracefully
    match response.body(Body::from(body_bytes)) {
        Ok(resp) => resp,
        Err(e) => {
            eprintln!("[plugin-proxy] Failed to build response: {}", e);
            (StatusCode::INTERNAL_SERVER_ERROR, "Failed to build response").into_response()
        }
    }
}

/// HTTP handler for plugin registration endpoint
/// POST /v1/plugins/register
pub async fn handle_plugin_registration(
    State(app_state): State<crate::http::AppState>,
    Json(registration): Json<PluginRegistration>,
) -> impl IntoResponse {
    println!("[plugin-proxy] Registration request received for plugin: {}", registration.name);

    match app_state.plugin_registry.register_plugin(registration).await {
        Ok(_) => {
            (StatusCode::OK, Json(serde_json::json!({
                "status": "registered",
                "message": "Plugin registered successfully"
            }))).into_response()
        }
        Err(e) => {
            eprintln!("[plugin-proxy] Registration failed: {}", e);
            (StatusCode::BAD_REQUEST, Json(serde_json::json!({
                "status": "error",
                "message": format!("Registration failed: {}", e)
            }))).into_response()
        }
    }
}

/// HTTP handler for listing all registered plugins
/// GET /v1/plugins
/// Adds dynamic "status" field based on socket existence (Running/Stopped)
pub async fn handle_list_plugins(
    State(app_state): State<crate::http::AppState>,
) -> impl IntoResponse {
    let plugins = app_state.plugin_registry.list_plugins().await;

    // Add dynamic status based on socket existence
    let plugins_with_status: Vec<serde_json::Value> = plugins.into_iter().map(|plugin| {
        let socket_exists = plugin.socket_path.exists();
        let status = if socket_exists { "Running" } else { "Stopped" };

        serde_json::json!({
            "name": plugin.name,
            "socket_path": plugin.socket_path,
            "routes": plugin.routes,
            "version": plugin.version,
            "description": plugin.description,
            "registered_at": plugin.registered_at,
            "status": status
        })
    }).collect();

    (StatusCode::OK, Json(serde_json::json!({
        "plugins": plugins_with_status
    }))).into_response()
}
