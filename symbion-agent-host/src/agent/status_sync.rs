//! Local API status synchronization

use crate::local_api;
use crate::metrics;

use super::Agent;

impl Agent {
    /// Push a log entry to the local API ring buffer and log collector
    pub(crate) async fn log(&self, level: &str, message: &str) {
        if let Some(ref api) = self.local_api {
            api.push_log(level, message).await;
        }
        // Forward to log collector (it handles level filtering and immediate publish)
        self.log_collector.push(level, message, None, None).await;
    }

    /// Update local API status. Reuses metrics from heartbeat to avoid double collection.
    pub(crate) async fn update_local_api_status(&self, mqtt_connected: bool, cached_metrics: Option<metrics::SystemMetrics>) {
        if let Some(ref api) = self.local_api {
            let system_status = if let Some(m) = cached_metrics {
                let process_count = metrics::ProcessInfo::collect().await
                    .map(|p| p.total_count as u32)
                    .unwrap_or(0);

                // Aggregate disk: use root "/" or first disk
                let (disk_used, disk_total) = m.disk.first()
                    .map(|d| (Some(d.used_gb), Some(d.total_gb)))
                    .unwrap_or((None, None));

                // Temperature: use cpu_celsius from temperature metrics
                let temperature = m.temperature.as_ref()
                    .and_then(|t| t.cpu_celsius)
                    .map(|c| c as f64);

                // Network: aggregate all interfaces
                let (net_rx, net_tx) = m.network.as_ref()
                    .map(|n| {
                        let rx: u64 = n.interfaces.iter().map(|i| i.bytes_recv).sum();
                        let tx: u64 = n.interfaces.iter().map(|i| i.bytes_sent).sum();
                        (Some(rx), Some(tx))
                    })
                    .unwrap_or((None, None));

                Some(local_api::SystemStatus {
                    cpu_percent: m.cpu.percent as f64,
                    memory_used_mb: m.memory.used_mb,
                    memory_total_mb: m.memory.total_mb,
                    disk_used_gb: disk_used,
                    disk_total_gb: disk_total,
                    process_count,
                    load_average: Some(m.cpu.load_avg[0]),
                    temperature,
                    swap_used_mb: Some(m.swap.used_mb),
                    swap_total_mb: Some(m.swap.total_mb),
                    network_rx_bytes: net_rx,
                    network_tx_bytes: net_tx,
                    cpu_cores: Some(m.cpu.core_count),
                })
            } else {
                None
            };
            api.update_status(mqtt_connected, system_status).await;
        }
    }
}
