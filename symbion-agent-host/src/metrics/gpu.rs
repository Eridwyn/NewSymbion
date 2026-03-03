//! GPU metrics collection

use serde::{Serialize, Deserialize};
use tracing::debug;

/// GPU metrics for all detected GPUs
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GpuMetrics {
    pub gpus: Vec<GpuInfo>,
}

/// Individual GPU information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GpuInfo {
    pub name: String,
    pub vendor: String,
    pub temperature_celsius: Option<f32>,
    pub utilization_percent: Option<f32>,
    pub memory_used_mb: Option<u64>,
    pub memory_total_mb: Option<u64>,
    pub fan_speed_percent: Option<f32>,
    pub power_watts: Option<f32>,
}

impl GpuMetrics {
    /// Collect GPU metrics from available sources.
    ///
    /// Returns `Some(GpuMetrics)` always — with an empty `gpus` vec if no GPU is detected.
    pub async fn collect() -> Option<Self> {
        debug!("Collecting GPU metrics...");

        // Try NVIDIA first
        if let Some(gpus) = Self::collect_nvidia().await {
            if !gpus.is_empty() {
                debug!("Found {} NVIDIA GPU(s)", gpus.len());
                return Some(GpuMetrics { gpus });
            }
        }

        // Try AMD (Linux only)
        #[cfg(target_os = "linux")]
        {
            if let Some(gpus) = Self::collect_amd_linux().await {
                if !gpus.is_empty() {
                    debug!("Found {} AMD GPU(s)", gpus.len());
                    return Some(GpuMetrics { gpus });
                }
            }
        }

        // No GPU detected — return empty list explicitly
        debug!("No GPU detected");
        Some(GpuMetrics { gpus: vec![] })
    }

    /// Collect NVIDIA GPU metrics via nvidia-smi
    async fn collect_nvidia() -> Option<Vec<GpuInfo>> {
        use tokio::process::Command;
        use std::time::Duration;

        let output = tokio::time::timeout(
            Duration::from_secs(3),
            Command::new("nvidia-smi")
                .args([
                    "--query-gpu=name,temperature.gpu,utilization.gpu,memory.used,memory.total,fan.speed,power.draw",
                    "--format=csv,noheader,nounits",
                ])
                .output(),
        )
        .await
        .ok()?
        .ok()?;

        if !output.status.success() {
            debug!("nvidia-smi exited with status: {}", output.status);
            return None;
        }

        let stdout = String::from_utf8_lossy(&output.stdout);
        let gpus: Vec<GpuInfo> = stdout
            .lines()
            .filter(|line| !line.trim().is_empty())
            .filter_map(parse_nvidia_csv_line)
            .collect();

        if gpus.is_empty() {
            None
        } else {
            Some(gpus)
        }
    }

    /// Collect AMD GPU metrics from sysfs (Linux only)
    #[cfg(target_os = "linux")]
    async fn collect_amd_linux() -> Option<Vec<GpuInfo>> {
        use tokio::fs;
        use sysinfo::Components;

        let gpu_busy_path = "/sys/class/drm/card0/device/gpu_busy_percent";

        // Check if AMD GPU sysfs entries exist
        if fs::metadata(gpu_busy_path).await.is_err() {
            return None;
        }

        let utilization = fs::read_to_string(gpu_busy_path)
            .await
            .ok()
            .and_then(|s| s.trim().parse::<f32>().ok());

        let memory_used_mb = fs::read_to_string("/sys/class/drm/card0/device/mem_info_vram_used")
            .await
            .ok()
            .and_then(|s| s.trim().parse::<u64>().ok())
            .map(|bytes| bytes / (1024 * 1024));

        let memory_total_mb = fs::read_to_string("/sys/class/drm/card0/device/mem_info_vram_total")
            .await
            .ok()
            .and_then(|s| s.trim().parse::<u64>().ok())
            .map(|bytes| bytes / (1024 * 1024));

        // Temperature from sysinfo components with "amdgpu" label
        let temperature_celsius = {
            let components = Components::new_with_refreshed_list();
            components
                .iter()
                .find(|c| c.label().to_lowercase().contains("amdgpu"))
                .map(|c| c.temperature())
        };

        let gpu = GpuInfo {
            name: "AMD GPU".to_string(),
            vendor: "amd".to_string(),
            temperature_celsius,
            utilization_percent: utilization,
            memory_used_mb,
            memory_total_mb,
            fan_speed_percent: None,
            power_watts: None,
        };

        Some(vec![gpu])
    }
}

/// Parse a single CSV line from nvidia-smi output into a GpuInfo.
///
/// nvidia-smi uses ", " as the field separator.
/// Fields that are unavailable are reported as "[N/A]" or empty.
fn parse_nvidia_csv_line(line: &str) -> Option<GpuInfo> {
    let fields: Vec<&str> = line.split(", ").collect();
    if fields.len() < 7 {
        debug!("nvidia-smi CSV line has {} fields, expected 7: {:?}", fields.len(), line);
        return None;
    }

    let parse_f32 = |s: &str| -> Option<f32> {
        let trimmed = s.trim();
        if trimmed.is_empty() || trimmed == "[N/A]" {
            None
        } else {
            trimmed.parse().ok()
        }
    };

    let parse_u64 = |s: &str| -> Option<u64> {
        let trimmed = s.trim();
        if trimmed.is_empty() || trimmed == "[N/A]" {
            None
        } else {
            trimmed.parse().ok()
        }
    };

    let name = fields[0].trim().to_string();

    Some(GpuInfo {
        name,
        vendor: "nvidia".to_string(),
        temperature_celsius: parse_f32(fields[1]),
        utilization_percent: parse_f32(fields[2]),
        memory_used_mb: parse_u64(fields[3]),
        memory_total_mb: parse_u64(fields[4]),
        fan_speed_percent: parse_f32(fields[5]),
        power_watts: parse_f32(fields[6]),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_gpu_metrics_collect() {
        let metrics = GpuMetrics::collect().await;
        assert!(metrics.is_some(), "collect() should always return Some");
    }

    #[test]
    fn test_gpu_metrics_serialization() {
        let metrics = GpuMetrics {
            gpus: vec![GpuInfo {
                name: "NVIDIA GeForce RTX 3080".to_string(),
                vendor: "nvidia".to_string(),
                temperature_celsius: Some(45.0),
                utilization_percent: Some(12.0),
                memory_used_mb: Some(2048),
                memory_total_mb: Some(10240),
                fan_speed_percent: Some(65.0),
                power_watts: Some(220.5),
            }],
        };

        let json = serde_json::to_string(&metrics).expect("serialize should succeed");
        let deserialized: GpuMetrics =
            serde_json::from_str(&json).expect("deserialize should succeed");

        assert_eq!(deserialized.gpus.len(), 1);
        let gpu = &deserialized.gpus[0];
        assert_eq!(gpu.name, "NVIDIA GeForce RTX 3080");
        assert_eq!(gpu.vendor, "nvidia");
        assert_eq!(gpu.temperature_celsius, Some(45.0));
        assert_eq!(gpu.utilization_percent, Some(12.0));
        assert_eq!(gpu.memory_used_mb, Some(2048));
        assert_eq!(gpu.memory_total_mb, Some(10240));
        assert_eq!(gpu.fan_speed_percent, Some(65.0));
        assert_eq!(gpu.power_watts, Some(220.5));
    }

    #[test]
    fn test_parse_nvidia_smi_output() {
        let line = "NVIDIA GeForce RTX 3080, 45, 12, 2048, 10240, 65, 220.5";
        let gpu = parse_nvidia_csv_line(line).expect("should parse valid CSV line");

        assert_eq!(gpu.name, "NVIDIA GeForce RTX 3080");
        assert_eq!(gpu.vendor, "nvidia");
        assert_eq!(gpu.temperature_celsius, Some(45.0));
        assert_eq!(gpu.utilization_percent, Some(12.0));
        assert_eq!(gpu.memory_used_mb, Some(2048));
        assert_eq!(gpu.memory_total_mb, Some(10240));
        assert_eq!(gpu.fan_speed_percent, Some(65.0));
        assert_eq!(gpu.power_watts, Some(220.5));

        // Test with [N/A] fields
        let line_na = "NVIDIA Tesla T4, 38, 0, 512, 16384, [N/A], [N/A]";
        let gpu_na = parse_nvidia_csv_line(line_na).expect("should parse line with [N/A]");

        assert_eq!(gpu_na.name, "NVIDIA Tesla T4");
        assert_eq!(gpu_na.temperature_celsius, Some(38.0));
        assert_eq!(gpu_na.fan_speed_percent, None);
        assert_eq!(gpu_na.power_watts, None);
    }

    #[test]
    fn test_parse_no_gpu() {
        let metrics = GpuMetrics { gpus: vec![] };
        let json = serde_json::to_string(&metrics).expect("serialize empty gpus should succeed");
        let deserialized: GpuMetrics =
            serde_json::from_str(&json).expect("deserialize empty gpus should succeed");

        assert!(deserialized.gpus.is_empty());
        assert_eq!(json, r#"{"gpus":[]}"#);
    }
}
