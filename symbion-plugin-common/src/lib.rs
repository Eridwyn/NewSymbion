/**
 * SYMBION PLUGIN COMMON - Shared utilities for plugins
 *
 * Provides:
 * - Unix socket HTTP server helper for plugins to expose routes
 * - Service Discovery client for auto-registration with kernel
 */

use axum::Router;
use hyper::body::Incoming;
use hyper_util::rt::TokioIo;
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use tokio::fs;
use tower::Service;

/// Unix socket HTTP server for plugins
pub struct PluginHttpServer {
    socket_path: PathBuf,
    router: Router,
}

impl PluginHttpServer {
    /// Create new plugin HTTP server
    pub fn new<P: AsRef<Path>>(socket_path: P, router: Router) -> Self {
        Self {
            socket_path: socket_path.as_ref().to_path_buf(),
            router,
        }
    }

    /// Start serving on Unix socket
    pub async fn serve(self) -> Result<(), Box<dyn std::error::Error>> {
        // Remove existing socket if present
        if self.socket_path.exists() {
            fs::remove_file(&self.socket_path).await?;
        }

        println!("[plugin-common] serving on Unix socket: {:?}", self.socket_path);

        // Bind to Unix socket
        let listener = tokio::net::UnixListener::bind(&self.socket_path)?;

        // Set permissions (readable/writable by owner and group)
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let mut perms = tokio::fs::metadata(&self.socket_path).await?.permissions();
            perms.set_mode(0o770); // rwxrwx---
            tokio::fs::set_permissions(&self.socket_path, perms).await?;
        }

        println!("[plugin-common] Unix socket ready, accepting connections");

        // Convert router into service
        let mut make_service = self.router.into_make_service();

        // Accept connections loop
        loop {
            let (stream, _addr) = listener.accept().await?;
            let tower_service = make_service.call(&stream).await?;

            tokio::spawn(async move {
                let stream = TokioIo::new(stream);
                let hyper_service = hyper::service::service_fn(move |request: hyper::Request<Incoming>| {
                    tower_service.clone().call(request)
                });

                if let Err(err) = hyper_util::server::conn::auto::Builder::new(hyper_util::rt::TokioExecutor::new())
                    .serve_connection(stream, hyper_service)
                    .await
                {
                    eprintln!("[plugin-common] Error serving connection: {}", err);
                }
            });
        }
    }
}

// ============================================================================
// SERVICE DISCOVERY - Auto-registration with kernel
// ============================================================================

/// Plugin registration request - matches kernel's PluginRegistration struct
#[derive(Debug, Serialize, Deserialize)]
pub struct PluginRegistration {
    pub name: String,
    pub socket_path: String,
    pub routes: Vec<String>,
    #[serde(default)]
    pub version: Option<String>,
    #[serde(default)]
    pub description: Option<String>,
}

/// Plugin registration response from kernel
#[derive(Debug, Deserialize)]
pub struct PluginRegistrationResponse {
    pub status: String,
    pub message: String,
}

/// Register plugin with kernel via Service Discovery
///
/// # Arguments
/// * `kernel_url` - Base URL of kernel (e.g., "https://localhost:8443")
/// * `plugin_name` - Name of plugin (e.g., "notifications")
/// * `socket_path` - Path to Unix socket (e.g., "/tmp/symbion-plugin-notifications.sock")
/// * `routes` - List of routes to register (e.g., ["/notifications", "/notifications/send"])
/// * `version` - Optional plugin version
/// * `description` - Optional plugin description
///
/// # Example
/// ```no_run
/// use symbion_plugin_common::register_with_kernel;
///
/// #[tokio::main]
/// async fn main() -> anyhow::Result<()> {
///     register_with_kernel(
///         "https://localhost:8443",
///         "notifications",
///         "/tmp/symbion-plugin-notifications.sock",
///         vec!["/notifications".to_string(), "/notifications/send".to_string()],
///         Some("1.0.0".to_string()),
///         Some("Push notifications plugin".to_string()),
///     ).await?;
///     Ok(())
/// }
/// ```
pub async fn register_with_kernel(
    kernel_url: &str,
    plugin_name: &str,
    socket_path: &str,
    routes: Vec<String>,
    version: Option<String>,
    description: Option<String>,
) -> anyhow::Result<()> {
    let registration = PluginRegistration {
        name: plugin_name.to_string(),
        socket_path: socket_path.to_string(),
        routes,
        version,
        description,
    };

    let endpoint = format!("{}/v1/plugins/register", kernel_url);

    println!("[plugin-common] Registering plugin '{}' with kernel at {}", plugin_name, kernel_url);

    // Build HTTP client that accepts self-signed certs (for dev TLS)
    let client = reqwest::Client::builder()
        .danger_accept_invalid_certs(true)
        .timeout(std::time::Duration::from_secs(5))
        .build()?;

    let response = client
        .post(&endpoint)
        .json(&registration)
        .send()
        .await?;

    let status = response.status();
    let body: PluginRegistrationResponse = response.json().await?;

    if status.is_success() {
        println!("[plugin-common] ✅ Registration successful: {}", body.message);
        Ok(())
    } else {
        anyhow::bail!("Registration failed (HTTP {}): {}", status, body.message);
    }
}

/// Builder pattern for plugin registration with sensible defaults
pub struct PluginRegistrationBuilder {
    kernel_url: String,
    plugin_name: String,
    socket_path: String,
    routes: Vec<String>,
    version: Option<String>,
    description: Option<String>,
}

impl PluginRegistrationBuilder {
    /// Create new registration builder
    pub fn new(plugin_name: impl Into<String>, socket_path: impl Into<String>) -> Self {
        Self {
            kernel_url: "https://localhost:8443".to_string(),
            plugin_name: plugin_name.into(),
            socket_path: socket_path.into(),
            routes: vec![],
            version: None,
            description: None,
        }
    }

    /// Set kernel URL (default: https://localhost:8443)
    pub fn kernel_url(mut self, url: impl Into<String>) -> Self {
        self.kernel_url = url.into();
        self
    }

    /// Add a route
    pub fn route(mut self, route: impl Into<String>) -> Self {
        self.routes.push(route.into());
        self
    }

    /// Add multiple routes
    pub fn routes(mut self, routes: Vec<String>) -> Self {
        self.routes.extend(routes);
        self
    }

    /// Set version
    pub fn version(mut self, version: impl Into<String>) -> Self {
        self.version = Some(version.into());
        self
    }

    /// Set description
    pub fn description(mut self, description: impl Into<String>) -> Self {
        self.description = Some(description.into());
        self
    }

    /// Execute registration
    pub async fn register(self) -> anyhow::Result<()> {
        register_with_kernel(
            &self.kernel_url,
            &self.plugin_name,
            &self.socket_path,
            self.routes,
            self.version,
            self.description,
        ).await
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::{routing::get, Json};
    use serde_json::json;

    #[tokio::test]
    async fn test_plugin_http_server() {
        let router = Router::new()
            .route("/health", get(|| async { Json(json!({"status": "ok"})) }));

        let _server = PluginHttpServer::new("/tmp/test-plugin.sock", router);

        // Would serve here in real test
        // _server.serve().await.unwrap();
    }

    #[test]
    fn test_plugin_registration_serialization() {
        let registration = PluginRegistration {
            name: "test-plugin".to_string(),
            socket_path: "/tmp/test.sock".to_string(),
            routes: vec!["/test".to_string(), "/test/status".to_string()],
            version: Some("1.0.0".to_string()),
            description: Some("A test plugin".to_string()),
        };

        let json = serde_json::to_string(&registration).unwrap();
        let deserialized: PluginRegistration = serde_json::from_str(&json).unwrap();

        assert_eq!(deserialized.name, "test-plugin");
        assert_eq!(deserialized.socket_path, "/tmp/test.sock");
        assert_eq!(deserialized.routes.len(), 2);
        assert_eq!(deserialized.version, Some("1.0.0".to_string()));
        assert_eq!(deserialized.description, Some("A test plugin".to_string()));
    }

    #[test]
    fn test_plugin_registration_builder() {
        let builder = PluginRegistrationBuilder::new("notifications", "/tmp/notifications.sock")
            .route("/notifications")
            .route("/notifications/send")
            .version("2.1.0")
            .description("Push notifications plugin");

        assert_eq!(builder.plugin_name, "notifications");
        assert_eq!(builder.socket_path, "/tmp/notifications.sock");
        assert_eq!(builder.routes.len(), 2);
        assert_eq!(builder.version, Some("2.1.0".to_string()));
        assert_eq!(builder.description, Some("Push notifications plugin".to_string()));
    }

    #[test]
    fn test_plugin_registration_builder_defaults() {
        let builder = PluginRegistrationBuilder::new("test", "/tmp/test.sock");

        assert_eq!(builder.kernel_url, "https://localhost:8443");
        assert_eq!(builder.routes.len(), 0);
        assert_eq!(builder.version, None);
        assert_eq!(builder.description, None);
    }

    #[test]
    fn test_plugin_registration_response_deserialize() {
        let json = r#"{
            "status": "success",
            "message": "Plugin registered successfully"
        }"#;

        let response: PluginRegistrationResponse = serde_json::from_str(json).unwrap();

        assert_eq!(response.status, "success");
        assert_eq!(response.message, "Plugin registered successfully");
    }
}
