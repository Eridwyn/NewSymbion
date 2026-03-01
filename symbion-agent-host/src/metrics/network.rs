//! Network interface statistics

use serde::Serialize;
use sysinfo::Networks;

/// Network interface statistics
#[derive(Debug, Serialize)]
pub struct NetworkMetrics {
    pub interfaces: Vec<NetworkInterfaceStats>,
}

/// Per-interface network statistics
#[derive(Debug, Serialize)]
pub struct NetworkInterfaceStats {
    pub name: String,
    pub bytes_sent: u64,
    pub bytes_recv: u64,
    pub packets_sent: u64,
    pub packets_recv: u64,
    pub is_up: bool,
}

impl NetworkMetrics {
    pub fn collect() -> Option<Self> {
        let networks = Networks::new_with_refreshed_list();
        let interfaces: Vec<NetworkInterfaceStats> = networks.iter()
            .map(|(name, data)| NetworkInterfaceStats {
                name: name.to_string(),
                bytes_sent: data.total_transmitted(),
                bytes_recv: data.total_received(),
                packets_sent: data.total_packets_transmitted(),
                packets_recv: data.total_packets_received(),
                is_up: data.total_received() > 0 || data.total_transmitted() > 0,
            })
            .collect();

        if interfaces.is_empty() {
            None
        } else {
            Some(NetworkMetrics { interfaces })
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_network_metrics() {
        let net = NetworkMetrics::collect();
        if let Some(net) = net {
            assert!(!net.interfaces.is_empty());
            assert!(net.interfaces.iter().any(|i| i.name == "lo" || i.name.starts_with("eth") || i.name.starts_with("en") || i.name.starts_with("wl")));
        }
    }
}
