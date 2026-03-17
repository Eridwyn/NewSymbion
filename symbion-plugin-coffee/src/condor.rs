//! Philips Condor LAN Protocol Client
//!
//! Handles HTTPS communication with the coffee machine using
//! PHILIPS-Condor challenge-response authentication (SHA256).

use anyhow::{Context, Result};
use base64::engine::general_purpose::STANDARD as B64;
use base64::Engine;
use sha2::{Digest, Sha256};

/// Client for communicating with the Philips coffee machine
/// via the Condor LAN Protocol over HTTPS.
pub struct CondorClient {
    base_url: String,
    #[allow(dead_code)] // Kept for re-pairing flow
    client_id_b64: String,
    client_id_bytes: Vec<u8>,
    client_secret_bytes: Vec<u8>,
    http: reqwest::Client,
}

impl CondorClient {
    pub fn new(ip: &str, port: u16, client_id: &str, client_secret: &str) -> Self {
        let http = reqwest::Client::builder()
            .danger_accept_invalid_certs(true) // Self-signed CN=CoffeeOmnia
            .connect_timeout(std::time::Duration::from_secs(5))
            .timeout(std::time::Duration::from_secs(8))
            .build()
            .expect("Failed to create HTTP client");

        Self {
            base_url: format!("https://{}:{}/di/v1/products", ip, port),
            client_id_b64: client_id.to_string(),
            client_id_bytes: B64.decode(client_id).expect("Invalid client_id base64"),
            client_secret_bytes: B64.decode(client_secret).expect("Invalid client_secret base64"),
            http,
        }
    }

    /// Build Authorization header from a WWW-Authenticate challenge
    fn build_auth(&self, challenge_b64: &str) -> String {
        let challenge_bytes = B64.decode(challenge_b64).unwrap_or_default();

        // SHA256(challenge + client_id + client_secret)
        let mut hasher = Sha256::new();
        hasher.update(&challenge_bytes);
        hasher.update(&self.client_id_bytes);
        hasher.update(&self.client_secret_bytes);
        let hash = hasher.finalize();

        // Authorization = PHILIPS-Condor base64(client_id_bytes + hash)
        let mut auth_payload = self.client_id_bytes.clone();
        auth_payload.extend_from_slice(&hash);

        format!("PHILIPS-Condor {}", B64.encode(&auth_payload))
    }

    /// Extract challenge from WWW-Authenticate header
    fn extract_challenge(header: &str) -> Option<&str> {
        header
            .strip_prefix("PHILIPS-Condor ")
            .or_else(|| header.strip_prefix("PHILIPS-Condor\t"))
    }

    /// GET a port value from the machine
    pub async fn get(&self, port: &str) -> Result<serde_json::Value> {
        let url = format!("{}/{}", self.base_url, port);

        // First request to get challenge (will return 401)
        let resp = self
            .http
            .get(&url)
            .send()
            .await
            .with_context(|| format!("Connection failed to {}", url))?;

        if resp.status() == reqwest::StatusCode::UNAUTHORIZED {
            // Extract challenge from WWW-Authenticate
            let www_auth = resp
                .headers()
                .get("www-authenticate")
                .and_then(|v| v.to_str().ok())
                .with_context(|| "Missing WWW-Authenticate header")?;

            let challenge = Self::extract_challenge(www_auth)
                .with_context(|| "Invalid challenge format")?;

            let auth = self.build_auth(challenge);

            // Retry with auth
            let resp = self
                .http
                .get(&url)
                .header("Authorization", &auth)
                .send()
                .await
                .with_context(|| "Auth request failed")?;

            if resp.status().is_success() {
                let body = resp.text().await.unwrap_or_default();
                if body.is_empty() {
                    return Ok(serde_json::json!({}));
                }
                serde_json::from_str(&body).with_context(|| "Invalid JSON response")
            } else {
                anyhow::bail!("GET {} failed: HTTP {}", port, resp.status());
            }
        } else if resp.status().is_success() {
            // No auth needed (rare)
            let body = resp.text().await.unwrap_or_default();
            serde_json::from_str(&body).with_context(|| "Invalid JSON response")
        } else {
            anyhow::bail!("GET {} failed: HTTP {}", port, resp.status());
        }
    }

    /// PUT a value to a port on the machine
    pub async fn put(&self, port: &str, payload: &serde_json::Value) -> Result<serde_json::Value> {
        let url = format!("{}/{}", self.base_url, port);

        // First request to get challenge (will return 401)
        let resp = self
            .http
            .put(&url)
            .json(payload)
            .send()
            .await
            .with_context(|| format!("Connection failed to {}", url))?;

        if resp.status() == reqwest::StatusCode::UNAUTHORIZED {
            let www_auth = resp
                .headers()
                .get("www-authenticate")
                .and_then(|v| v.to_str().ok())
                .with_context(|| "Missing WWW-Authenticate header")?;

            let challenge = Self::extract_challenge(www_auth)
                .with_context(|| "Invalid challenge format")?;

            let auth = self.build_auth(challenge);

            let resp = self
                .http
                .put(&url)
                .header("Authorization", &auth)
                .json(payload)
                .send()
                .await
                .with_context(|| "Auth PUT request failed")?;

            if resp.status().is_success() {
                let body = resp.text().await.unwrap_or_default();
                if body.is_empty() {
                    return Ok(serde_json::json!({}));
                }
                serde_json::from_str(&body).with_context(|| "Invalid JSON response")
            } else {
                anyhow::bail!("PUT {} failed: HTTP {}", port, resp.status());
            }
        } else if resp.status().is_success() {
            let body = resp.text().await.unwrap_or_default();
            if body.is_empty() {
                return Ok(serde_json::json!({}));
            }
            serde_json::from_str(&body).with_context(|| "Invalid JSON response")
        } else {
            anyhow::bail!("PUT {} failed: HTTP {}", port, resp.status());
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_build_auth() {
        let client = CondorClient::new(
            "192.168.1.90",
            443,
            "VjVaLHrgarGVGuO1svdzBA==",
            "V+oG/9f0nV4wOlFIWyDgkw==",
        );

        let id_len = client.client_id_bytes.len();
        let secret_len = client.client_secret_bytes.len();
        assert!(id_len > 0);
        assert!(secret_len > 0);

        // Build auth with dummy challenge
        let challenge = B64.encode(b"test_challenge_16");
        let auth = client.build_auth(&challenge);
        assert!(auth.starts_with("PHILIPS-Condor "));

        // Auth payload should be client_id + sha256 (32) bytes
        let auth_b64 = auth.strip_prefix("PHILIPS-Condor ").unwrap();
        let auth_bytes = B64.decode(auth_b64).unwrap();
        assert_eq!(auth_bytes.len(), id_len + 32);

        // First bytes should be client_id
        assert_eq!(&auth_bytes[..id_len], &client.client_id_bytes);
    }

    #[test]
    fn test_extract_challenge() {
        assert_eq!(
            CondorClient::extract_challenge("PHILIPS-Condor abc123=="),
            Some("abc123==")
        );
        assert_eq!(CondorClient::extract_challenge("Basic abc"), None);
    }
}
