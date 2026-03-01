//! System metrics collection for Symbion agents
//!
//! Provides cross-platform system monitoring:
//! - CPU usage and load averages
//! - Memory usage statistics
//! - Disk usage for mounted filesystems
//! - Network interface statistics
//! - Temperature and battery monitoring
//! - Process information and top consumers
//! - System service status

mod cpu;
mod memory;
mod disk;
mod network;
mod thermal;
mod processes;
mod services;

pub use cpu::CpuMetrics;
pub use memory::{MemoryMetrics, SwapMetrics};
pub use disk::DiskMetrics;
pub use network::NetworkMetrics;
pub use thermal::{TemperatureMetrics, BatteryMetrics};
pub use processes::ProcessInfo;
pub use services::ServiceStatus;

use anyhow::Result;
use serde::Serialize;
use sysinfo::System;
use tracing::debug;

/// Complete system metrics (matches agents.heartbeat@v1 schema)
#[derive(Debug, Serialize)]
pub struct SystemMetrics {
    pub uptime_seconds: u64,
    pub cpu: CpuMetrics,
    pub memory: MemoryMetrics,
    pub swap: SwapMetrics,
    pub disk: Vec<DiskMetrics>,
    pub network: Option<NetworkMetrics>,
    pub temperature: Option<TemperatureMetrics>,
    pub battery: Option<BatteryMetrics>,
}

impl SystemMetrics {
    /// Collect complete system metrics
    pub async fn collect() -> Result<Self> {
        debug!("Collecting system metrics...");

        let mut sys = System::new_all();
        sys.refresh_all();

        // Wait a moment for accurate CPU readings
        tokio::time::sleep(tokio::time::Duration::from_millis(200)).await;
        sys.refresh_cpu_usage();

        let uptime_seconds = System::uptime();

        let cpu = CpuMetrics::collect(&sys)?;
        let memory = MemoryMetrics::collect(&sys)?;
        let swap = SwapMetrics::collect(&sys);
        let disk = DiskMetrics::collect(&sys)?;
        let network = NetworkMetrics::collect();
        let temperature = TemperatureMetrics::collect();
        let battery = BatteryMetrics::collect().await;

        Ok(SystemMetrics {
            uptime_seconds,
            cpu,
            memory,
            swap,
            disk,
            network,
            temperature,
            battery,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_metrics_collection() {
        let metrics = SystemMetrics::collect().await.unwrap();
        assert!(metrics.uptime_seconds > 0);
        assert!(metrics.cpu.core_count > 0);
        assert!(metrics.memory.total_mb > 0);
        assert!(!metrics.disk.is_empty());
    }
}
