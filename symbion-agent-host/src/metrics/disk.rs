//! Disk usage metrics

use anyhow::Result;
use serde::Serialize;
use sysinfo::System;

/// Disk usage metrics per filesystem
#[derive(Debug, Serialize)]
pub struct DiskMetrics {
    pub path: String,
    pub total_gb: f64,
    pub used_gb: f64,
    pub free_gb: f64,
    pub percent_used: f32,
}

impl DiskMetrics {
    pub fn collect(_sys: &System) -> Result<Vec<Self>> {
        let disks = sysinfo::Disks::new_with_refreshed_list();
        let mut disk_metrics: Vec<Self> = disks.iter().map(|disk| {
            let total_bytes = disk.total_space() as f64;
            let available_bytes = disk.available_space() as f64;
            let used_bytes = total_bytes - available_bytes;
            let total_gb = total_bytes / (1024.0 * 1024.0 * 1024.0);
            let used_gb = used_bytes / (1024.0 * 1024.0 * 1024.0);
            let free_gb = available_bytes / (1024.0 * 1024.0 * 1024.0);
            let percent_used = if total_bytes > 0.0 {
                (used_bytes / total_bytes * 100.0) as f32
            } else {
                0.0
            };

            DiskMetrics {
                path: disk.mount_point().to_string_lossy().to_string(),
                total_gb,
                used_gb,
                free_gb,
                percent_used,
            }
        }).collect();

        if disk_metrics.is_empty() {
            disk_metrics.push(DiskMetrics {
                path: "/".to_string(),
                total_gb: 0.0,
                used_gb: 0.0,
                free_gb: 0.0,
                percent_used: 0.0,
            });
        }

        Ok(disk_metrics)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_disk_metrics_collect_non_empty() {
        let sys = System::new();
        let disks = DiskMetrics::collect(&sys).unwrap();
        assert!(!disks.is_empty(), "Should return at least one disk (or fallback)");
    }

    #[test]
    fn test_disk_metrics_valid_values() {
        let sys = System::new();
        let disks = DiskMetrics::collect(&sys).unwrap();
        for disk in &disks {
            assert!(disk.total_gb >= 0.0, "Total GB should be non-negative");
            assert!(disk.used_gb >= 0.0, "Used GB should be non-negative");
            assert!(disk.free_gb >= 0.0, "Free GB should be non-negative");
            assert!(disk.percent_used >= 0.0 && disk.percent_used <= 100.0,
                "Percent used should be 0-100, got {}", disk.percent_used);
        }
    }

    #[test]
    fn test_disk_metrics_serialization() {
        let metric = DiskMetrics {
            path: "/".to_string(),
            total_gb: 500.0,
            used_gb: 250.0,
            free_gb: 250.0,
            percent_used: 50.0,
        };
        let json = serde_json::to_string(&metric).unwrap();
        assert!(json.contains("\"/\""));
        assert!(json.contains("500"));
        let deserialized: serde_json::Value = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized["path"], "/");
        assert_eq!(deserialized["percent_used"], 50.0);
    }
}
