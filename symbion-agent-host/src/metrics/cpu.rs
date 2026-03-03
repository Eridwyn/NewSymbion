//! CPU usage metrics

use anyhow::Result;
use serde::Serialize;
use sysinfo::System;

/// CPU usage metrics
#[derive(Debug, Clone, Serialize)]
pub struct CpuMetrics {
    pub percent: f32,
    pub load_avg: [f64; 3],  // [1min, 5min, 15min]
    pub core_count: usize,
}

impl CpuMetrics {
    pub fn collect(sys: &System) -> Result<Self> {
        let cpus = sys.cpus();
        let global_cpu = sys.global_cpu_info();

        let percent = global_cpu.cpu_usage();
        let core_count = cpus.len();

        let load_avg = if cfg!(unix) {
            let load = System::load_average();
            [load.one, load.five, load.fifteen]
        } else {
            [0.0, 0.0, 0.0]
        };

        Ok(CpuMetrics {
            percent,
            load_avg,
            core_count,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cpu_metrics_collect() {
        let mut sys = System::new();
        sys.refresh_cpu();
        // Need a small delay for CPU usage to register
        std::thread::sleep(std::time::Duration::from_millis(200));
        sys.refresh_cpu();

        let metrics = CpuMetrics::collect(&sys).unwrap();
        assert!(metrics.percent >= 0.0);
        assert!(metrics.percent <= 100.0);
        assert!(metrics.core_count > 0);
    }

    #[test]
    fn test_cpu_metrics_serialization() {
        let metrics = CpuMetrics {
            percent: 42.5,
            load_avg: [1.0, 0.5, 0.25],
            core_count: 8,
        };
        let json = serde_json::to_string(&metrics).unwrap();
        assert!(json.contains("42.5"));
        assert!(json.contains("\"core_count\":8"));
    }
}
