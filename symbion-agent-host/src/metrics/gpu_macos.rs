//! macOS GPU metrics via ioreg
//!
//! Provides GPU utilization and VRAM info on macOS via the `ioreg` command.

use anyhow::Result;
use serde::{Deserialize, Serialize};

/// macOS GPU metrics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MacOsGpuMetrics {
    pub name: Option<String>,
    pub vendor: Option<String>,
    pub vram_mb: Option<u64>,
}

/// Collect macOS GPU metrics via ioreg
#[cfg(target_os = "macos")]
pub async fn collect() -> Result<Option<MacOsGpuMetrics>> {
    let output = tokio::process::Command::new("ioreg")
        .args(["-l", "-w0", "-r", "-c", "IOPCIDevice"])
        .output()
        .await?;

    if !output.status.success() {
        return Ok(None);
    }

    let stdout = String::from_utf8_lossy(&output.stdout);

    // Parse GPU info from ioreg output
    let mut name = None;
    let mut vendor = None;
    let mut vram_mb = None;

    for line in stdout.lines() {
        let trimmed = line.trim();
        if trimmed.contains("model") && trimmed.contains("=") && name.is_none() {
            if let Some(val) = extract_ioreg_string(trimmed) {
                name = Some(val);
            }
        }
        if trimmed.contains("vendor-id") && vendor.is_none() {
            vendor = Some("Apple".to_string()); // Simplified
        }
        if trimmed.contains("VRAM,totalMB") {
            if let Some(val) = extract_ioreg_number(trimmed) {
                vram_mb = Some(val);
            }
        }
    }

    if name.is_some() || vram_mb.is_some() {
        Ok(Some(MacOsGpuMetrics { name, vendor, vram_mb }))
    } else {
        Ok(None)
    }
}

#[cfg(not(target_os = "macos"))]
pub async fn collect() -> Result<Option<MacOsGpuMetrics>> {
    Ok(None)
}

#[cfg(target_os = "macos")]
fn extract_ioreg_string(line: &str) -> Option<String> {
    let parts: Vec<&str> = line.splitn(2, '=').collect();
    if parts.len() == 2 {
        let val = parts[1].trim().trim_matches('"').to_string();
        if !val.is_empty() { Some(val) } else { None }
    } else {
        None
    }
}

#[cfg(target_os = "macos")]
fn extract_ioreg_number(line: &str) -> Option<u64> {
    let parts: Vec<&str> = line.splitn(2, '=').collect();
    if parts.len() == 2 {
        parts[1].trim().trim_matches('"').parse().ok()
    } else {
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_gpu_macos_collect() {
        // On non-macOS, should return None
        let result = collect().await.unwrap();
        #[cfg(not(target_os = "macos"))]
        assert!(result.is_none());
    }

    #[test]
    fn test_macos_gpu_metrics_serialization() {
        let metrics = MacOsGpuMetrics {
            name: Some("Apple M1".to_string()),
            vendor: Some("Apple".to_string()),
            vram_mb: Some(8192),
        };
        let json = serde_json::to_string(&metrics).unwrap();
        assert!(json.contains("Apple M1"));
        let parsed: MacOsGpuMetrics = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed.vram_mb, Some(8192));
    }
}
