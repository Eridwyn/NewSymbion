//! macOS Disk I/O metrics via iostat
//!
//! Provides disk I/O throughput on macOS via the `iostat` command.

use anyhow::Result;
use serde::{Deserialize, Serialize};

/// macOS disk I/O metrics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MacOsDiskIo {
    pub device: String,
    pub kb_per_transfer: f64,
    pub transfers_per_sec: f64,
    pub mb_per_sec: f64,
}

/// Collect macOS disk I/O metrics via iostat
#[cfg(target_os = "macos")]
pub async fn collect() -> Result<Vec<MacOsDiskIo>> {
    let output = tokio::process::Command::new("iostat")
        .args(["-d", "-c", "1"])
        .output()
        .await?;

    if !output.status.success() {
        return Ok(Vec::new());
    }

    let stdout = String::from_utf8_lossy(&output.stdout);
    let mut results = Vec::new();

    // Parse iostat output (skip header lines)
    let lines: Vec<&str> = stdout.lines().collect();
    for line in lines.iter().skip(2) {
        let parts: Vec<&str> = line.split_whitespace().collect();
        if parts.len() >= 3 {
            if let (Ok(kb_t), Ok(tps), Ok(mb_s)) = (
                parts[0].parse::<f64>(),
                parts[1].parse::<f64>(),
                parts[2].parse::<f64>(),
            ) {
                results.push(MacOsDiskIo {
                    device: format!("disk{}", results.len()),
                    kb_per_transfer: kb_t,
                    transfers_per_sec: tps,
                    mb_per_sec: mb_s,
                });
            }
        }
    }

    Ok(results)
}

#[cfg(not(target_os = "macos"))]
pub async fn collect() -> Result<Vec<MacOsDiskIo>> {
    Ok(Vec::new())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_disk_io_macos_collect() {
        let result = collect().await.unwrap();
        #[cfg(not(target_os = "macos"))]
        assert!(result.is_empty());
    }

    #[test]
    fn test_macos_disk_io_serialization() {
        let metrics = MacOsDiskIo {
            device: "disk0".to_string(),
            kb_per_transfer: 32.5,
            transfers_per_sec: 150.0,
            mb_per_sec: 4.7,
        };
        let json = serde_json::to_string(&metrics).unwrap();
        assert!(json.contains("disk0"));
        let parsed: MacOsDiskIo = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed.device, "disk0");
    }
}
