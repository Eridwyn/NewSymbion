//! Freebox API client
//!
//! Handles authentication and API calls to the Freebox.

use anyhow::{anyhow, Result};
use hmac::{Hmac, Mac};
use reqwest::Client;
use serde::{Deserialize, Serialize};
use sha1::Sha1;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;

type HmacSha1 = Hmac<Sha1>;

/// Freebox API client
pub struct FreeboxClient {
    client: Client,
    api_url: String,
    app_id: String,
    app_token: String,
    session_token: Arc<RwLock<Option<String>>>,
}

// ============================================================================
// API Response Types
// ============================================================================

#[derive(Debug, Deserialize)]
struct ApiResponse<T> {
    success: bool,
    result: Option<T>,
    #[serde(default)]
    error_code: Option<String>,
    #[serde(default)]
    msg: Option<String>,
}

#[derive(Debug, Deserialize)]
struct LoginResult {
    challenge: String,
}

#[derive(Debug, Deserialize)]
struct SessionResult {
    session_token: String,
}

/// Network device from LAN browser
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LanDevice {
    pub id: String,
    pub primary_name: String,
    #[serde(default)]
    pub default_name: String,
    pub host_type: String,
    pub active: bool,
    pub reachable: bool,
    pub last_time_reachable: i64,
    pub last_activity: i64,
    #[serde(default)]
    pub l3connectivities: Vec<L3Connectivity>,
    #[serde(default)]
    pub vendor_name: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct L3Connectivity {
    pub addr: String,
    pub af: String,
    pub active: bool,
    pub reachable: bool,
}

/// Internet connection status
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConnectionStatus {
    pub state: String,
    #[serde(rename = "type")]
    pub connection_type: String,
    pub media: String,
    #[serde(default)]
    pub ipv4: String,
    #[serde(default)]
    pub ipv6: String,
    pub rate_down: i64,
    pub rate_up: i64,
    pub bandwidth_down: i64,
    pub bandwidth_up: i64,
}

/// Download task
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Download {
    pub id: i64,
    pub name: String,
    pub status: String,
    pub size: i64,
    pub rx_bytes: i64,
    pub tx_bytes: i64,
    pub rx_rate: i64,
    pub tx_rate: i64,
    #[serde(default)]
    pub eta: i64,
    #[serde(rename = "type")]
    pub download_type: String,
}

// ============================================================================
// Client Implementation
// ============================================================================

impl FreeboxClient {
    /// Create a new Freebox client
    pub fn new(api_url: &str, app_id: &str, app_token: &str) -> Self {
        Self {
            client: Client::new(),
            api_url: api_url.to_string(),
            app_id: app_id.to_string(),
            app_token: app_token.to_string(),
            session_token: Arc::new(RwLock::new(None)),
        }
    }

    /// Authenticate and get a session token
    async fn authenticate(&self) -> Result<String> {
        // Get challenge
        let login_url = format!("{}/api/v8/login/", self.api_url);
        let resp: ApiResponse<LoginResult> = self.client
            .get(&login_url)
            .send()
            .await?
            .json()
            .await?;

        let challenge = resp.result
            .ok_or_else(|| anyhow!("No challenge in login response"))?
            .challenge;

        // Compute HMAC-SHA1 password
        let mut mac = HmacSha1::new_from_slice(self.app_token.as_bytes())
            .map_err(|e| anyhow!("HMAC error: {}", e))?;
        mac.update(challenge.as_bytes());
        let password = hex::encode(mac.finalize().into_bytes());

        // Open session
        let session_url = format!("{}/api/v8/login/session/", self.api_url);
        let body = serde_json::json!({
            "app_id": self.app_id,
            "password": password
        });

        let resp: ApiResponse<SessionResult> = self.client
            .post(&session_url)
            .json(&body)
            .send()
            .await?
            .json()
            .await?;

        if !resp.success {
            return Err(anyhow!(
                "Session failed: {}",
                resp.msg.unwrap_or_else(|| "Unknown error".to_string())
            ));
        }

        let token = resp.result
            .ok_or_else(|| anyhow!("No session token in response"))?
            .session_token;

        Ok(token)
    }

    /// Get a valid session token, re-authenticating if needed
    async fn get_session_token(&self) -> Result<String> {
        // Check if we have a valid token
        {
            let token = self.session_token.read().await;
            if let Some(ref t) = *token {
                return Ok(t.clone());
            }
        }

        // Need to authenticate
        let token = self.authenticate().await?;

        // Store the token
        {
            let mut stored = self.session_token.write().await;
            *stored = Some(token.clone());
        }

        Ok(token)
    }

    /// Make an authenticated API request with automatic token refresh
    async fn api_get<T: for<'de> Deserialize<'de>>(&self, endpoint: &str) -> Result<T> {
        self.api_get_with_retry(endpoint, true).await
    }

    /// Internal API request with optional retry on auth failure
    fn api_get_with_retry<'a, T: for<'de> Deserialize<'de> + 'a>(
        &'a self,
        endpoint: &'a str,
        allow_retry: bool
    ) -> std::pin::Pin<Box<dyn std::future::Future<Output = Result<T>> + Send + 'a>> {
        Box::pin(async move {
            let token = self.get_session_token().await?;
            let url = format!("{}/api/v8{}", self.api_url, endpoint);

            let response = self.client
                .get(&url)
                .header("X-Fbx-App-Auth", &token)
                .send()
                .await?;

            // Get raw text first to handle auth errors that can't be parsed as T
            let text = response.text().await?;

            // Try to detect auth errors in raw response before parsing as T
            if text.contains("auth_required") || text.contains("insufficient_rights") || text.contains("invalid_token") {
                if allow_retry {
                    tracing::warn!("Freebox auth error detected, refreshing session...");
                    // Clear cached token
                    {
                        let mut stored = self.session_token.write().await;
                        *stored = None;
                    }
                    // Retry with fresh token (no more retries)
                    return self.api_get_with_retry(endpoint, false).await;
                } else {
                    return Err(anyhow!("Freebox authentication failed after retry"));
                }
            }

            // Parse as ApiResponse<T>
            let resp: ApiResponse<T> = serde_json::from_str(&text)
                .map_err(|e| anyhow!("Failed to parse Freebox response: {} (raw: {}...)", e, &text[..text.len().min(200)]))?;

            if !resp.success {
                return Err(anyhow!(
                    "API error: {} (code: {:?})",
                    resp.msg.unwrap_or_else(|| "Unknown error".to_string()),
                    resp.error_code
                ));
            }

            resp.result.ok_or_else(|| anyhow!("No result in API response"))
        })
    }

    // ========================================================================
    // Public API Methods
    // ========================================================================

    /// Get all devices on the LAN
    pub async fn get_lan_devices(&self) -> Result<Vec<LanDevice>> {
        self.api_get("/lan/browser/pub/").await
    }

    /// Get devices matching specific names (for presence detection)
    pub async fn get_devices_by_names(&self, names: &[String]) -> Result<HashMap<String, LanDevice>> {
        let all_devices = self.get_lan_devices().await?;

        let mut result = HashMap::new();
        for device in all_devices {
            for name in names {
                if device.primary_name.to_lowercase() == name.to_lowercase() ||
                   device.default_name.to_lowercase().contains(&name.to_lowercase()) {
                    result.insert(name.clone(), device.clone());
                    break;
                }
            }
        }

        Ok(result)
    }

    /// Get internet connection status
    pub async fn get_connection_status(&self) -> Result<ConnectionStatus> {
        self.api_get("/connection/").await
    }

    /// Get active downloads
    pub async fn get_downloads(&self) -> Result<Vec<Download>> {
        self.api_get("/downloads/").await
    }

    /// Get download count summary
    pub async fn get_downloads_summary(&self) -> Result<DownloadsSummary> {
        let downloads = self.get_downloads().await?;

        let active = downloads.iter()
            .filter(|d| d.status == "downloading" || d.status == "seeding")
            .count();

        let total_rx_rate: i64 = downloads.iter().map(|d| d.rx_rate).sum();
        let total_tx_rate: i64 = downloads.iter().map(|d| d.tx_rate).sum();

        Ok(DownloadsSummary {
            total: downloads.len(),
            active,
            rx_rate: total_rx_rate,
            tx_rate: total_tx_rate,
            downloads,
        })
    }
}

/// Summary of downloads
#[derive(Debug, Clone, Serialize)]
pub struct DownloadsSummary {
    pub total: usize,
    pub active: usize,
    pub rx_rate: i64,
    pub tx_rate: i64,
    pub downloads: Vec<Download>,
}
