//! HTTP client for kernel file transfer operations
//!
//! Used by the agent to pull/push files from/to the kernel via HTTPS.
//! The kernel acts as a file hub — MQTT is only used for signaling.

use anyhow::{Context, Result};
use std::path::Path;
use tracing::info;

/// HTTP client for communicating with the kernel's file transfer endpoints
pub struct KernelClient {
    client: reqwest::Client,
    base_url: String,
}

impl KernelClient {
    /// Create a new kernel client.
    /// If `kernel_url` is None, derives from MQTT broker host.
    pub fn new(kernel_url: Option<&str>, mqtt_host: &str, tls_verify: bool) -> Self {
        let base_url = kernel_url
            .map(|u| u.to_string())
            .unwrap_or_else(|| format!("https://{}:8443", mqtt_host));

        let client = reqwest::Client::builder()
            .danger_accept_invalid_certs(!tls_verify)
            .timeout(std::time::Duration::from_secs(300)) // 5 min for large files
            .connect_timeout(std::time::Duration::from_secs(10))
            .build()
            .unwrap_or_else(|_| reqwest::Client::new());

        info!("[kernel-client] Initialized — base_url: {}, tls_verify: {}", base_url, tls_verify);

        Self { client, base_url }
    }

    /// Pull a file from the kernel (upload flow: kernel has file, agent pulls it)
    pub async fn pull_file(
        &self,
        transfer_id: &str,
        token: &str,
        dest: &Path,
        kernel_port: Option<u16>,
    ) -> Result<u64> {
        let url = self.build_transfer_url(transfer_id, token, kernel_port);

        info!("[kernel-client] Pulling file: transfer={}", transfer_id);

        let response = self
            .client
            .get(&url)
            .send()
            .await
            .context("Failed to connect to kernel")?;

        if !response.status().is_success() {
            let status = response.status();
            let body = response.text().await.unwrap_or_default();
            anyhow::bail!("Kernel returned {} : {}", status, body);
        }

        let bytes = response.bytes().await.context("Failed to read response body")?;
        let size = bytes.len() as u64;

        // Ensure parent directory exists
        if let Some(parent) = dest.parent() {
            tokio::fs::create_dir_all(parent).await.ok();
        }

        tokio::fs::write(dest, &bytes)
            .await
            .context("Failed to write file to disk")?;

        info!("[kernel-client] Pulled {} bytes → {}", size, dest.display());
        Ok(size)
    }

    /// Push a file to the kernel (download flow: agent has file, pushes to kernel)
    pub async fn push_file(
        &self,
        transfer_id: &str,
        token: &str,
        file_path: &Path,
        kernel_port: Option<u16>,
    ) -> Result<()> {
        let url = self.build_transfer_url(transfer_id, token, kernel_port);

        info!("[kernel-client] Pushing file: transfer={}, path={}", transfer_id, file_path.display());

        let data = tokio::fs::read(file_path)
            .await
            .context("Failed to read file")?;

        let size = data.len();

        let response = self
            .client
            .post(&url)
            .header("Content-Type", "application/octet-stream")
            .body(data)
            .send()
            .await
            .context("Failed to connect to kernel")?;

        if !response.status().is_success() {
            let status = response.status();
            let body = response.text().await.unwrap_or_default();
            anyhow::bail!("Kernel returned {} : {}", status, body);
        }

        info!("[kernel-client] Pushed {} bytes", size);
        Ok(())
    }

    fn build_transfer_url(&self, transfer_id: &str, token: &str, kernel_port: Option<u16>) -> String {
        let base = if let Some(port) = kernel_port {
            // Use the port specified by the kernel command (more reliable)
            let host = self.base_url
                .trim_start_matches("https://")
                .trim_start_matches("http://")
                .split(':')
                .next()
                .unwrap_or("127.0.0.1");
            format!("https://{}:{}", host, port)
        } else {
            self.base_url.clone()
        };

        format!("{}/v1/transfers/{}/data?token={}", base, transfer_id, token)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_kernel_client_url_construction() {
        let client = KernelClient::new(None, "192.168.1.14", false);
        assert_eq!(client.base_url, "https://192.168.1.14:8443");

        let url = client.build_transfer_url("abc-123", "tok-456", None);
        assert_eq!(url, "https://192.168.1.14:8443/v1/transfers/abc-123/data?token=tok-456");
    }

    #[test]
    fn test_kernel_client_explicit_url() {
        let client = KernelClient::new(Some("https://kernel.local:9443"), "unused", false);
        assert_eq!(client.base_url, "https://kernel.local:9443");
    }

    #[test]
    fn test_kernel_client_port_override() {
        let client = KernelClient::new(None, "192.168.1.14", false);
        let url = client.build_transfer_url("abc", "tok", Some(8443));
        assert_eq!(url, "https://192.168.1.14:8443/v1/transfers/abc/data?token=tok");
    }
}
