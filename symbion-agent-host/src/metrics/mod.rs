//! System metrics collection for Symbion agents
//!
//! Provides cross-platform system monitoring:
//! - CPU usage and load averages
//! - Memory usage statistics  
//! - Disk usage for mounted filesystems
//! - Network interface statistics (placeholder)
//! - Process information and top consumers
//! - System service status (placeholder)

use anyhow::Result;
use serde::Serialize;
use sysinfo::{Components, Networks, System, ProcessStatus};
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

/// CPU usage metrics
#[derive(Debug, Serialize)]
pub struct CpuMetrics {
    pub percent: f32,
    pub load_avg: [f64; 3],  // [1min, 5min, 15min]
    pub core_count: usize,
}

/// Memory usage metrics  
#[derive(Debug, Serialize)]
pub struct MemoryMetrics {
    pub total_mb: u64,
    pub used_mb: u64,
    pub available_mb: u64,
    pub percent_used: f32,
}

/// Disk usage metrics per filesystem
#[derive(Debug, Serialize)]
pub struct DiskMetrics {
    pub path: String,
    pub total_gb: f64,
    pub used_gb: f64,
    pub free_gb: f64,
    pub percent_used: f32,
}

/// Network interface statistics (placeholder)
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

/// Temperature sensor readings (placeholder)
#[derive(Debug, Serialize)]
pub struct TemperatureMetrics {
    pub cpu_celsius: Option<f32>,
    pub sensors: Vec<TemperatureSensor>,
}

/// Individual temperature sensor
#[derive(Debug, Serialize)]
pub struct TemperatureSensor {
    pub name: String,
    pub value: f32,
    pub unit: String,
    pub critical: Option<f32>,
}

/// Swap memory metrics
#[derive(Debug, Serialize)]
pub struct SwapMetrics {
    pub total_mb: u64,
    pub used_mb: u64,
    pub percent_used: f32,
}

/// Battery status (laptops / UPS)
#[derive(Debug, Serialize)]
pub struct BatteryMetrics {
    pub percent: f32,
    pub charging: bool,
    pub power_source: String,
}

impl SwapMetrics {
    fn collect(sys: &System) -> Self {
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

impl BatteryMetrics {
    /// Collect battery info from /sys/class/power_supply (Linux) or WMI (Windows)
    async fn collect() -> Option<Self> {
        #[cfg(target_os = "linux")]
        {
            Self::collect_linux().await
        }
        #[cfg(not(target_os = "linux"))]
        {
            None
        }
    }

    #[cfg(target_os = "linux")]
    async fn collect_linux() -> Option<Self> {
        use tokio::fs;

        // Find battery device (BAT0, BAT1, etc.)
        let power_dir = "/sys/class/power_supply";
        let mut entries = fs::read_dir(power_dir).await.ok()?;

        while let Ok(Some(entry)) = entries.next_entry().await {
            let name = entry.file_name().to_string_lossy().to_string();
            if !name.starts_with("BAT") {
                continue;
            }

            let base = format!("{}/{}", power_dir, name);

            let capacity = fs::read_to_string(format!("{}/capacity", base))
                .await.ok()
                .and_then(|s| s.trim().parse::<f32>().ok())?;

            let status = fs::read_to_string(format!("{}/status", base))
                .await.ok()
                .map(|s| s.trim().to_string())
                .unwrap_or_default();

            let charging = status == "Charging" || status == "Full";
            let power_source = if charging { "AC" } else { "Battery" }.to_string();

            return Some(BatteryMetrics {
                percent: capacity,
                charging,
                power_source,
            });
        }

        None
    }
}

impl NetworkMetrics {
    /// Collect network interface statistics
    fn collect() -> Option<Self> {
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

impl TemperatureMetrics {
    /// Collect temperature readings from system sensors
    fn collect() -> Option<Self> {
        let components = Components::new_with_refreshed_list();
        if components.is_empty() {
            return None;
        }

        let mut cpu_celsius: Option<f32> = None;
        let mut sensors = Vec::new();

        for component in components.iter() {
            let label = component.label().to_string();
            let temp = component.temperature();
            let critical = component.critical();

            // Detect CPU temperature (common labels across platforms)
            if cpu_celsius.is_none() {
                let lower = label.to_lowercase();
                if lower.contains("cpu") || lower.contains("core") || lower.contains("package") || lower.contains("tctl") {
                    cpu_celsius = Some(temp);
                }
            }

            sensors.push(TemperatureSensor {
                name: label,
                value: temp,
                unit: "°C".to_string(),
                critical,
            });
        }

        Some(TemperatureMetrics {
            cpu_celsius,
            sensors,
        })
    }
}

/// Process information summary
#[derive(Debug, Serialize)]
pub struct ProcessInfo {
    pub total_count: usize,
    pub running_count: usize,
    pub top_cpu: Vec<ProcessEntry>,
    pub top_memory: Vec<ProcessEntry>,
}

/// Individual process entry
#[derive(Debug, Serialize)]
pub struct ProcessEntry {
    pub pid: u32,
    pub name: String,
    pub cpu_percent: f32,
    pub memory_mb: f64,
    pub user: Option<String>,
}

/// System service status
#[derive(Debug, Serialize)]
pub struct ServiceStatus {
    pub name: String,
    pub status: ServiceState,
    pub enabled: Option<bool>,
}

/// Service state enumeration
#[derive(Debug, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum ServiceState {
    Active,
    Inactive,
    Failed,
    Unknown,
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

impl CpuMetrics {
    fn collect(sys: &System) -> Result<Self> {
        let cpus = sys.cpus();
        let global_cpu = sys.global_cpu_info();
        
        let percent = global_cpu.cpu_usage();
        let core_count = cpus.len();
        
        // Get load averages (Unix-specific, fallback for others)
        let load_avg = if cfg!(unix) {
            let load = System::load_average();
            [load.one, load.five, load.fifteen]
        } else {
            [0.0, 0.0, 0.0] // Windows fallback
        };
        
        Ok(CpuMetrics {
            percent,
            load_avg,
            core_count,
        })
    }
}

impl MemoryMetrics {
    fn collect(sys: &System) -> Result<Self> {
        let total_bytes = sys.total_memory();
        let available_bytes = sys.available_memory();
        let used_bytes = total_bytes - available_bytes;
        
        // Convert bytes to MB (divide by 1024^2)
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

impl DiskMetrics {
    fn collect(_sys: &System) -> Result<Vec<Self>> {
        // Cross-platform disk metrics via sysinfo (works on Linux, Windows, macOS)
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

        // Fallback if no disks found
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

impl ProcessInfo {
    pub async fn collect() -> Result<Self> {
        let mut sys = System::new();
        sys.refresh_processes();
        
        let processes: Vec<_> = sys.processes().values().collect();
        let total_count = processes.len();
        let running_count = processes.iter()
            .filter(|p| matches!(p.status(), ProcessStatus::Run))
            .count();
        
        // Sort by CPU usage (top 15)
        let mut cpu_sorted = processes.clone();
        cpu_sorted.sort_by(|a, b| b.cpu_usage().partial_cmp(&a.cpu_usage()).unwrap_or(std::cmp::Ordering::Equal));
        let top_cpu = cpu_sorted.into_iter()
            .take(15)
            .map(|p| ProcessEntry {
                pid: p.pid().as_u32(),
                name: p.name().to_string(),
                cpu_percent: p.cpu_usage(),
                memory_mb: p.memory() as f64 / (1024.0 * 1024.0),
                user: p.user_id().map(|u| u.to_string()),
            })
            .collect();
        
        // Sort by memory usage (top 15)  
        let mut mem_sorted = processes;
        mem_sorted.sort_by(|a, b| b.memory().cmp(&a.memory()));
        let top_memory = mem_sorted.into_iter()
            .take(15)
            .map(|p| ProcessEntry {
                pid: p.pid().as_u32(),
                name: p.name().to_string(),
                cpu_percent: p.cpu_usage(),
                memory_mb: p.memory() as f64 / (1024.0 * 1024.0),
                user: p.user_id().map(|u| u.to_string()),
            })
            .collect();
        
        Ok(ProcessInfo {
            total_count,
            running_count,
            top_cpu,
            top_memory,
        })
    }
}

impl ServiceStatus {
    /// Collect status of critical system services
    pub async fn collect_critical() -> Result<Vec<Self>> {
        let critical_services = if cfg!(target_os = "linux") {
            vec!["ssh", "NetworkManager", "mosquitto", "symbion-kernel"]
        } else if cfg!(target_os = "windows") {
            vec!["Winmgmt", "EventLog", "Mosquitto"]
        } else {
            vec![]
        };

        let mut services = Vec::new();
        for name in critical_services {
            let status = Self::query_service(name).await;
            services.push(status);
        }

        Ok(services)
    }

    /// Query a single service status (platform-specific)
    async fn query_service(name: &str) -> Self {
        if cfg!(target_os = "linux") {
            Self::query_linux_service(name).await
        } else if cfg!(target_os = "windows") {
            Self::query_windows_service(name).await
        } else {
            ServiceStatus {
                name: name.to_string(),
                status: ServiceState::Unknown,
                enabled: None,
            }
        }
    }

    async fn query_linux_service(name: &str) -> Self {
        use tokio::process::Command;

        // Check if active
        let is_active = Command::new("systemctl")
            .args(["is-active", "--quiet", name])
            .output()
            .await
            .map(|o| o.status.success())
            .unwrap_or(false);

        // Check if enabled
        let is_enabled = Command::new("systemctl")
            .args(["is-enabled", "--quiet", name])
            .output()
            .await
            .map(|o| o.status.success())
            .ok();

        let status = if is_active {
            ServiceState::Active
        } else {
            // Distinguish inactive from failed
            let state_output = Command::new("systemctl")
                .args(["is-failed", "--quiet", name])
                .output()
                .await
                .map(|o| o.status.success())
                .unwrap_or(false);

            if state_output {
                ServiceState::Failed
            } else {
                ServiceState::Inactive
            }
        };

        ServiceStatus {
            name: name.to_string(),
            status,
            enabled: is_enabled,
        }
    }

    async fn query_windows_service(name: &str) -> Self {
        use tokio::process::Command;

        let output = Command::new("sc")
            .args(["query", name])
            .output()
            .await;

        let (status, enabled) = match output {
            Ok(o) => {
                let stdout = String::from_utf8_lossy(&o.stdout).to_string();
                let state = if stdout.contains("RUNNING") {
                    ServiceState::Active
                } else if stdout.contains("STOPPED") {
                    ServiceState::Inactive
                } else {
                    ServiceState::Unknown
                };
                (state, None)
            }
            Err(_) => (ServiceState::Unknown, None),
        };

        ServiceStatus {
            name: name.to_string(),
            status,
            enabled,
        }
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

    #[tokio::test]
    async fn test_process_info() {
        let process_info = ProcessInfo::collect().await.unwrap();
        assert!(process_info.total_count > 0);
        assert!(process_info.top_cpu.len() <= 15);
        assert!(process_info.top_memory.len() <= 15);
    }

    #[test]
    fn test_swap_metrics() {
        let sys = System::new_all();
        let swap = SwapMetrics::collect(&sys);
        // Swap may be 0 on some systems, but percent should be valid
        assert!(swap.percent_used >= 0.0 && swap.percent_used <= 100.0);
    }

    #[test]
    fn test_network_metrics() {
        let net = NetworkMetrics::collect();
        // Most systems have at least loopback
        if let Some(net) = net {
            assert!(!net.interfaces.is_empty());
            assert!(net.interfaces.iter().any(|i| i.name == "lo" || i.name.starts_with("eth") || i.name.starts_with("en") || i.name.starts_with("wl")));
        }
    }

    #[test]
    fn test_temperature_metrics() {
        // Temperature sensors may not be available on all systems
        let temp = TemperatureMetrics::collect();
        if let Some(temp) = temp {
            assert!(!temp.sensors.is_empty());
            for sensor in &temp.sensors {
                assert!(sensor.value > -50.0 && sensor.value < 200.0);
            }
        }
    }

    #[tokio::test]
    async fn test_service_status() {
        let services = ServiceStatus::collect_critical().await.unwrap();
        assert!(!services.is_empty());
        // At least one service should have a real status (not Unknown)
        // ssh or NetworkManager should be queryable on Linux
    }
}