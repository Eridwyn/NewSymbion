//! Advanced network metrics (latency, bandwidth, connections)

use serde::{Deserialize, Serialize};
use tracing::debug;

/// Advanced network metrics including latency, bandwidth, and connection counts
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NetworkAdvancedMetrics {
    pub gateway_latency_ms: Option<f64>,
    pub dns_latency_ms: Option<f64>,
    pub active_connections: Option<u32>,
    pub interfaces: Vec<InterfaceBandwidth>,
}

/// Per-interface bandwidth measurement (bytes per second)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InterfaceBandwidth {
    pub name: String,
    pub rx_bytes_per_sec: u64,
    pub tx_bytes_per_sec: u64,
}

impl NetworkAdvancedMetrics {
    /// Collect advanced network metrics (Linux only for now)
    #[cfg(target_os = "linux")]
    pub async fn collect() -> Option<Self> {
        debug!("Collecting advanced network metrics...");

        let (gateway_latency_ms, dns_latency_ms, active_connections, interfaces) = tokio::join!(
            get_gateway_latency(),
            get_dns_latency(),
            count_active_connections(),
            get_interface_bandwidth(),
        );

        // Return None only if we couldn't collect anything at all
        if gateway_latency_ms.is_none()
            && dns_latency_ms.is_none()
            && active_connections.is_none()
            && interfaces.is_empty()
        {
            debug!("No advanced network metrics could be collected");
            return None;
        }

        Some(NetworkAdvancedMetrics {
            gateway_latency_ms,
            dns_latency_ms,
            active_connections,
            interfaces,
        })
    }

    /// Non-Linux platforms: not yet implemented
    #[cfg(not(target_os = "linux"))]
    pub async fn collect() -> Option<Self> {
        None
    }
}

// ---------------------------------------------------------------------------
// Linux-specific helpers
// ---------------------------------------------------------------------------

/// Detect the default gateway IP and measure ping latency to it
#[cfg(target_os = "linux")]
async fn get_gateway_latency() -> Option<f64> {
    use std::time::Duration;
    use tokio::process::Command;

    // Step 1: Detect gateway IP
    let route_output = tokio::time::timeout(
        Duration::from_secs(3),
        Command::new("ip")
            .args(["route", "show", "default"])
            .output(),
    )
    .await
    .ok()?
    .ok()?;

    let route_stdout = String::from_utf8_lossy(&route_output.stdout);
    let gateway_ip = parse_gateway_ip(&route_stdout)?;
    debug!("Detected gateway IP: {}", gateway_ip);

    // Step 2: Ping the gateway
    let ping_output = tokio::time::timeout(
        Duration::from_secs(3),
        Command::new("ping")
            .args(["-c", "1", "-W", "2", &gateway_ip])
            .output(),
    )
    .await
    .ok()?
    .ok()?;

    let ping_stdout = String::from_utf8_lossy(&ping_output.stdout);
    let rtt = parse_ping_rtt(&ping_stdout);
    debug!("Gateway ping RTT: {:?} ms", rtt);
    rtt
}

/// Measure DNS latency by timing a TCP connect to 8.8.8.8:53
#[cfg(target_os = "linux")]
async fn get_dns_latency() -> Option<f64> {
    use std::time::{Duration, Instant};
    use tokio::net::TcpStream;

    let start = Instant::now();
    let result = tokio::time::timeout(
        Duration::from_secs(2),
        TcpStream::connect("8.8.8.8:53"),
    )
    .await;

    match result {
        Ok(Ok(_stream)) => {
            let elapsed_ms = start.elapsed().as_secs_f64() * 1000.0;
            debug!("DNS latency (TCP connect to 8.8.8.8:53): {:.2} ms", elapsed_ms);
            Some(elapsed_ms)
        }
        _ => {
            debug!("DNS latency measurement failed (timeout or connection error)");
            None
        }
    }
}

/// Count active TCP connections from /proc/net/tcp and /proc/net/tcp6
#[cfg(target_os = "linux")]
async fn count_active_connections() -> Option<u32> {
    use tokio::fs;

    let mut count: u32 = 0;

    if let Ok(content) = fs::read_to_string("/proc/net/tcp").await {
        // Skip the header line
        count += content.lines().skip(1).count() as u32;
    }

    if let Ok(content) = fs::read_to_string("/proc/net/tcp6").await {
        // Skip the header line
        count += content.lines().skip(1).count() as u32;
    }

    if count == 0 {
        debug!("No active TCP connections found (or /proc/net/tcp unreadable)");
        return None;
    }

    debug!("Active TCP connections: {}", count);
    Some(count)
}

/// Read /proc/net/dev twice (1 second apart) and compute per-second bandwidth
#[cfg(target_os = "linux")]
async fn get_interface_bandwidth() -> Vec<InterfaceBandwidth> {
    use tokio::fs;

    let first = match fs::read_to_string("/proc/net/dev").await {
        Ok(content) => parse_proc_net_dev(&content),
        Err(_) => return Vec::new(),
    };

    tokio::time::sleep(tokio::time::Duration::from_secs(1)).await;

    let second = match fs::read_to_string("/proc/net/dev").await {
        Ok(content) => parse_proc_net_dev(&content),
        Err(_) => return Vec::new(),
    };

    // Match interfaces and compute delta
    let mut result = Vec::new();
    for (name2, rx2, tx2) in &second {
        if let Some((_, rx1, tx1)) = first.iter().find(|(n, _, _)| n == name2) {
            let rx_per_sec = rx2.saturating_sub(*rx1);
            let tx_per_sec = tx2.saturating_sub(*tx1);
            result.push(InterfaceBandwidth {
                name: name2.clone(),
                rx_bytes_per_sec: rx_per_sec,
                tx_bytes_per_sec: tx_per_sec,
            });
        }
    }

    debug!("Interface bandwidth collected for {} interfaces", result.len());
    result
}

// ---------------------------------------------------------------------------
// Parse helpers (testable, platform-independent)
// ---------------------------------------------------------------------------

/// Parse gateway IP from `ip route show default` output
fn parse_gateway_ip(output: &str) -> Option<String> {
    // Look for "default via X.X.X.X"
    for line in output.lines() {
        let parts: Vec<&str> = line.split_whitespace().collect();
        if parts.len() >= 3 && parts[0] == "default" && parts[1] == "via" {
            return Some(parts[2].to_string());
        }
    }
    None
}

/// Parse ping RTT from ping output (looks for "time=X.XX ms")
fn parse_ping_rtt(output: &str) -> Option<f64> {
    for line in output.lines() {
        if let Some(idx) = line.find("time=") {
            let after = &line[idx + 5..];
            let num_str: String = after
                .chars()
                .take_while(|c| c.is_ascii_digit() || *c == '.')
                .collect();
            return num_str.parse::<f64>().ok();
        }
    }
    None
}

/// Parse /proc/net/dev content into (interface_name, rx_bytes, tx_bytes) tuples.
/// Skips the loopback interface ("lo") and the two header lines.
fn parse_proc_net_dev(content: &str) -> Vec<(String, u64, u64)> {
    let mut result = Vec::new();

    // Skip first 2 lines (headers)
    for line in content.lines().skip(2) {
        let line = line.trim();
        if line.is_empty() {
            continue;
        }

        // Format: "iface: rx_bytes rx_packets rx_errs rx_drop rx_fifo rx_frame rx_compressed rx_multicast tx_bytes ..."
        let Some((iface_part, stats_part)) = line.split_once(':') else {
            continue;
        };

        let name = iface_part.trim().to_string();

        // Skip loopback
        if name == "lo" {
            continue;
        }

        let fields: Vec<&str> = stats_part.split_whitespace().collect();
        // rx_bytes is field 0, tx_bytes is field 8
        if fields.len() < 9 {
            continue;
        }

        let rx_bytes = fields[0].parse::<u64>().unwrap_or(0);
        let tx_bytes = fields[8].parse::<u64>().unwrap_or(0);

        result.push((name, rx_bytes, tx_bytes));
    }

    result
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_network_advanced_collect() {
        // Should return Some on Linux with network, or None — but never panic
        let result = NetworkAdvancedMetrics::collect().await;
        if let Some(metrics) = &result {
            // If we got metrics, at least one field should be populated
            assert!(
                metrics.gateway_latency_ms.is_some()
                    || metrics.dns_latency_ms.is_some()
                    || metrics.active_connections.is_some()
                    || !metrics.interfaces.is_empty()
            );
        }
    }

    #[test]
    fn test_parse_gateway_ip() {
        let output = "\
default via 192.168.1.1 dev enp0s3 proto dhcp src 192.168.1.100 metric 100
192.168.1.0/24 dev enp0s3 proto kernel scope link src 192.168.1.100";

        let result = parse_gateway_ip(output);
        assert_eq!(result, Some("192.168.1.1".to_string()));
    }

    #[test]
    fn test_parse_ping_rtt() {
        let output = "\
PING 192.168.1.1 (192.168.1.1) 56(84) bytes of data.
64 bytes from 192.168.1.1: icmp_seq=1 ttl=64 time=0.543 ms

--- 192.168.1.1 ping statistics ---
1 packets transmitted, 1 received, 0% packet loss, time 0ms
rtt min/avg/max/mdev = 0.543/0.543/0.543/0.000 ms";

        let result = parse_ping_rtt(output);
        assert_eq!(result, Some(0.543));
    }

    #[test]
    fn test_parse_proc_net_dev() {
        let content = "\
Inter-|   Receive                                                |  Transmit
 face |bytes    packets errs drop fifo frame compressed multicast|bytes    packets errs drop fifo colls carrier compressed
    lo: 1234567   12345    0    0    0     0          0         0  1234567   12345    0    0    0     0       0          0
  eth0: 9876543   98765    0    0    0     0          0         0  5432109   54321    0    0    0     0       0          0";

        let result = parse_proc_net_dev(content);

        // lo should be skipped
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].0, "eth0");
        assert_eq!(result[0].1, 9876543); // rx_bytes
        assert_eq!(result[0].2, 5432109); // tx_bytes
    }

    #[test]
    fn test_network_advanced_serialization() {
        let metrics = NetworkAdvancedMetrics {
            gateway_latency_ms: Some(1.234),
            dns_latency_ms: Some(12.5),
            active_connections: Some(42),
            interfaces: vec![
                InterfaceBandwidth {
                    name: "eth0".to_string(),
                    rx_bytes_per_sec: 1024,
                    tx_bytes_per_sec: 512,
                },
                InterfaceBandwidth {
                    name: "wlan0".to_string(),
                    rx_bytes_per_sec: 2048,
                    tx_bytes_per_sec: 256,
                },
            ],
        };

        // Serialize to JSON
        let json = serde_json::to_string(&metrics).expect("serialization failed");
        assert!(json.contains("gateway_latency_ms"));
        assert!(json.contains("1.234"));
        assert!(json.contains("eth0"));

        // Deserialize back
        let roundtrip: NetworkAdvancedMetrics =
            serde_json::from_str(&json).expect("deserialization failed");
        assert_eq!(roundtrip.gateway_latency_ms, Some(1.234));
        assert_eq!(roundtrip.dns_latency_ms, Some(12.5));
        assert_eq!(roundtrip.active_connections, Some(42));
        assert_eq!(roundtrip.interfaces.len(), 2);
        assert_eq!(roundtrip.interfaces[0].name, "eth0");
        assert_eq!(roundtrip.interfaces[0].rx_bytes_per_sec, 1024);
        assert_eq!(roundtrip.interfaces[1].name, "wlan0");
        assert_eq!(roundtrip.interfaces[1].tx_bytes_per_sec, 256);
    }
}
