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

    /// Windows: collect advanced network metrics via PowerShell and built-in tools
    #[cfg(target_os = "windows")]
    pub async fn collect() -> Option<Self> {
        debug!("Collecting advanced network metrics (Windows)...");

        let (gateway_latency_ms, dns_latency_ms, active_connections, interfaces) = tokio::join!(
            get_gateway_latency_windows(),
            get_dns_latency_windows(),
            count_active_connections_windows(),
            get_interface_bandwidth_windows(),
        );

        // Return None only if we couldn't collect anything at all
        if gateway_latency_ms.is_none()
            && dns_latency_ms.is_none()
            && active_connections.is_none()
            && interfaces.is_empty()
        {
            debug!("No advanced network metrics could be collected (Windows)");
            return None;
        }

        Some(NetworkAdvancedMetrics {
            gateway_latency_ms,
            dns_latency_ms,
            active_connections,
            interfaces,
        })
    }

    /// Other platforms: not yet implemented
    #[cfg(not(any(target_os = "linux", target_os = "windows")))]
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
// Windows-specific helpers
// ---------------------------------------------------------------------------

/// Detect the default gateway IP via `route print` and ping it
#[cfg(target_os = "windows")]
async fn get_gateway_latency_windows() -> Option<f64> {
    use std::time::Duration;
    use tokio::process::Command;

    // Step 1: Detect gateway IP via `route print 0.0.0.0`
    let route_output = tokio::time::timeout(
        Duration::from_secs(3),
        Command::new("cmd")
            .args(["/C", "route print 0.0.0.0"])
            .output(),
    )
    .await
    .ok()?
    .ok()?;

    let route_stdout = String::from_utf8_lossy(&route_output.stdout);
    let gateway_ip = parse_windows_gateway_ip(&route_stdout)?;
    debug!("Detected gateway IP (Windows): {}", gateway_ip);

    // Step 2: Ping the gateway
    let ping_output = tokio::time::timeout(
        Duration::from_secs(3),
        Command::new("ping")
            .args(["-n", "1", "-w", "2000", &gateway_ip])
            .output(),
    )
    .await
    .ok()?
    .ok()?;

    let ping_stdout = String::from_utf8_lossy(&ping_output.stdout);
    // Windows ping uses "time=Xms" or "time<1ms"
    let rtt = parse_ping_rtt_windows(&ping_stdout);
    debug!("Gateway ping RTT (Windows): {:?} ms", rtt);
    rtt
}

/// Measure DNS latency by timing a TCP connect to 8.8.8.8:53 (cross-platform)
#[cfg(target_os = "windows")]
async fn get_dns_latency_windows() -> Option<f64> {
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
            debug!("DNS latency (TCP connect to 8.8.8.8:53, Windows): {:.2} ms", elapsed_ms);
            Some(elapsed_ms)
        }
        _ => {
            debug!("DNS latency measurement failed (Windows)");
            None
        }
    }
}

/// Count established TCP connections via `netstat -an`
#[cfg(target_os = "windows")]
async fn count_active_connections_windows() -> Option<u32> {
    use std::time::Duration;
    use tokio::process::Command;

    let output = tokio::time::timeout(
        Duration::from_secs(3),
        Command::new("cmd")
            .args(["/C", r#"netstat -an | find /c "ESTABLISHED""#])
            .output(),
    )
    .await
    .ok()?
    .ok()?;

    let stdout = String::from_utf8_lossy(&output.stdout);
    let count: u32 = stdout.trim().parse().ok()?;

    if count == 0 {
        debug!("No established connections found (Windows)");
        return None;
    }

    debug!("Active TCP connections (Windows): {}", count);
    Some(count)
}

/// Measure per-interface bandwidth via PowerShell Get-Counter
#[cfg(target_os = "windows")]
async fn get_interface_bandwidth_windows() -> Vec<InterfaceBandwidth> {
    use std::collections::HashMap;
    use std::time::Duration;
    use tokio::process::Command;

    let ps_script = r#"Get-Counter '\Network Interface(*)\Bytes Received/sec','\Network Interface(*)\Bytes Sent/sec' -SampleInterval 1 -MaxSamples 1 | ForEach-Object { $_.CounterSamples | ForEach-Object { '{0}|{1}' -f $_.Path,$_.CookedValue } }"#;

    let output = match tokio::time::timeout(
        Duration::from_secs(3),
        Command::new("powershell")
            .args(["-NoProfile", "-NonInteractive", "-Command", ps_script])
            .output(),
    )
    .await
    {
        Ok(Ok(o)) if o.status.success() => o,
        _ => {
            debug!("PowerShell Get-Counter for network failed");
            return Vec::new();
        }
    };

    let stdout = String::from_utf8_lossy(&output.stdout);

    struct IfAccum {
        rx_bytes_per_sec: u64,
        tx_bytes_per_sec: u64,
    }

    let mut map: HashMap<String, IfAccum> = HashMap::new();

    for line in stdout.lines() {
        let line = line.trim();
        if line.is_empty() {
            continue;
        }

        let Some((path, value_str)) = line.rsplit_once('|') else {
            continue;
        };

        let value: f64 = match value_str.trim().parse() {
            Ok(v) => v,
            Err(_) => continue,
        };

        let path_lower = path.to_lowercase();

        // Extract interface name from path like "\\server\network interface(intel...)\bytes received/sec"
        let iface_name = match extract_windows_counter_instance(&path_lower, "network interface") {
            Some(name) => name,
            None => continue,
        };

        let entry = map.entry(iface_name).or_insert(IfAccum {
            rx_bytes_per_sec: 0,
            tx_bytes_per_sec: 0,
        });

        if path_lower.contains("bytes received/sec") {
            entry.rx_bytes_per_sec = value as u64;
        } else if path_lower.contains("bytes sent/sec") {
            entry.tx_bytes_per_sec = value as u64;
        }
    }

    let result: Vec<InterfaceBandwidth> = map
        .into_iter()
        .map(|(name, accum)| InterfaceBandwidth {
            name,
            rx_bytes_per_sec: accum.rx_bytes_per_sec,
            tx_bytes_per_sec: accum.tx_bytes_per_sec,
        })
        .collect();

    debug!("Interface bandwidth (Windows) collected for {} interfaces", result.len());
    result
}

/// Extract instance name from a Windows performance counter path.
///
/// Given `\\server\network interface(intel gigabit)\bytes received/sec`
/// and object_name `"network interface"`, returns `Some("intel gigabit")`.
#[cfg(target_os = "windows")]
fn extract_windows_counter_instance(path: &str, object_name: &str) -> Option<String> {
    let obj_pos = path.find(object_name)?;
    let after_obj = &path[obj_pos + object_name.len()..];
    if !after_obj.starts_with('(') {
        return None;
    }
    let end = after_obj.find(')')?;
    let instance = &after_obj[1..end];
    if instance.is_empty() {
        return None;
    }
    Some(instance.to_string())
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
// Windows parse helpers (testable)
// ---------------------------------------------------------------------------

/// Parse the default gateway IP from `route print 0.0.0.0` output on Windows.
///
/// Looks for lines matching "0.0.0.0" in the route table and extracts the
/// gateway IP (third column in the IPv4 Route Table section).
fn parse_windows_gateway_ip(output: &str) -> Option<String> {
    for line in output.lines() {
        let parts: Vec<&str> = line.split_whitespace().collect();
        // Route table line: "0.0.0.0  0.0.0.0  192.168.1.1  192.168.1.100  25"
        if parts.len() >= 4 && parts[0] == "0.0.0.0" && parts[1] == "0.0.0.0" {
            let gw = parts[2];
            // Validate it looks like an IP
            if gw.contains('.') && gw != "0.0.0.0" {
                return Some(gw.to_string());
            }
        }
    }
    None
}

/// Parse ping RTT from Windows ping output.
///
/// Windows ping uses "time=Xms" (no space before ms) or "time<1ms".
fn parse_ping_rtt_windows(output: &str) -> Option<f64> {
    for line in output.lines() {
        // Try "time=Xms" pattern
        if let Some(idx) = line.find("time=") {
            let after = &line[idx + 5..];
            let num_str: String = after
                .chars()
                .take_while(|c| c.is_ascii_digit() || *c == '.')
                .collect();
            if let Ok(val) = num_str.parse::<f64>() {
                return Some(val);
            }
        }
        // Try "time<1ms" pattern (sub-millisecond)
        if let Some(idx) = line.find("time<") {
            let after = &line[idx + 5..];
            let num_str: String = after
                .chars()
                .take_while(|c| c.is_ascii_digit() || *c == '.')
                .collect();
            if let Ok(val) = num_str.parse::<f64>() {
                return Some(val);
            }
        }
    }
    None
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

    #[test]
    fn test_parse_windows_gateway_ip() {
        let output = "\
===========================================================================
Interface List
  6...00 15 5d 01 02 03 ......Hyper-V Virtual Ethernet Adapter
===========================================================================

IPv4 Route Table
===========================================================================
Active Routes:
Network Destination        Netmask          Gateway       Interface  Metric
          0.0.0.0          0.0.0.0      192.168.1.1    192.168.1.100     25
        127.0.0.0        255.0.0.0         On-link         127.0.0.1    331
===========================================================================";

        let result = parse_windows_gateway_ip(output);
        assert_eq!(result, Some("192.168.1.1".to_string()));
    }

    #[test]
    fn test_parse_windows_gateway_ip_no_gateway() {
        let output = "No routes found.";
        let result = parse_windows_gateway_ip(output);
        assert_eq!(result, None);
    }

    #[test]
    fn test_parse_ping_rtt_windows_normal() {
        let output = "\
Pinging 192.168.1.1 with 32 bytes of data:
Reply from 192.168.1.1: bytes=32 time=1ms TTL=64

Ping statistics for 192.168.1.1:
    Packets: Sent = 1, Received = 1, Lost = 0 (0% loss),
Approximate round trip times in milli-seconds:
    Minimum = 1ms, Maximum = 1ms, Average = 1ms";

        let result = parse_ping_rtt_windows(output);
        assert_eq!(result, Some(1.0));
    }

    #[test]
    fn test_parse_ping_rtt_windows_submillisecond() {
        let output = "\
Pinging 192.168.1.1 with 32 bytes of data:
Reply from 192.168.1.1: bytes=32 time<1ms TTL=64";

        let result = parse_ping_rtt_windows(output);
        assert_eq!(result, Some(1.0));
    }

    #[test]
    fn test_parse_ping_rtt_windows_decimal() {
        let output = "Reply from 192.168.1.1: bytes=32 time=0.5ms TTL=64";
        let result = parse_ping_rtt_windows(output);
        assert_eq!(result, Some(0.5));
    }
}
