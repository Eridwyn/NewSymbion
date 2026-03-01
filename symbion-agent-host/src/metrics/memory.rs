//! Memory and swap usage metrics

use anyhow::Result;
use serde::Serialize;
use sysinfo::System;

/// Memory usage metrics
#[derive(Debug, Serialize)]
pub struct MemoryMetrics {
    pub total_mb: u64,
    pub used_mb: u64,
    pub available_mb: u64,
    pub percent_used: f32,
}

impl MemoryMetrics {
    pub fn collect(sys: &System) -> Result<Self> {
        let total_bytes = sys.total_memory();
        let available_bytes = sys.available_memory();
        let used_bytes = total_bytes - available_bytes;

        let total_mb = (total_bytes / (1024 * 1024)) as u64;
        let used_mb = (used_bytes / (1024 * 1024)) as u64;
        let available_mb = (available_bytes / (1024 * 1024)) as u64;

        let percent_used = if total_bytes > 0 {
            (used_bytes as f32 / total_bytes as f32) * 100.0
        } else {
            0.0
        };

        Ok(MemoryMetrics {
            total_mb,
            used_mb,
            available_mb,
            percent_used,
        })
    }
}

/// Swap memory metrics
#[derive(Debug, Serialize)]
pub struct SwapMetrics {
    pub total_mb: u64,
    pub used_mb: u64,
    pub percent_used: f32,
}

impl SwapMetrics {
    pub fn collect(sys: &System) -> Self {
        let total = sys.total_swap();
        let used = sys.used_swap();
        let total_mb = (total / (1024 * 1024)) as u64;
        let used_mb = (used / (1024 * 1024)) as u64;
        let percent_used = if total > 0 {
            (used as f32 / total as f32) * 100.0
        } else {
            0.0
        };

        SwapMetrics {
            total_mb,
            used_mb,
            percent_used,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_swap_metrics() {
        let sys = System::new_all();
        let swap = SwapMetrics::collect(&sys);
        assert!(swap.percent_used >= 0.0 && swap.percent_used <= 100.0);
    }
}
