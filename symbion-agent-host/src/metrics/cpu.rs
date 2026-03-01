//! CPU usage metrics

use anyhow::Result;
use serde::Serialize;
use sysinfo::System;

/// CPU usage metrics
#[derive(Debug, Serialize)]
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
