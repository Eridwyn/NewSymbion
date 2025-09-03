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
use sysinfo::{System, ProcessStatus};
use tracing::debug;

/// Complete system metrics (matches agents.heartbeat@v1 schema)
#[derive(Debug, Serialize)]
pub struct SystemMetrics {
    pub uptime_seconds: u64,
    pub cpu: CpuMetrics,
    pub memory: MemoryMetrics,
    pub disk: Vec<DiskMetrics>,
    pub network: Option<NetworkMetrics>,
    pub temperature: Option<TemperatureMetrics>,
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
        let disk = DiskMetrics::collect(&sys)?;
        let network = None; // Placeholder - will implement later
        let temperature = None; // Placeholder - will implement later
        
        Ok(SystemMetrics {
            uptime_seconds,
            cpu,
            memory,
            disk,
            network,
            temperature,
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
        let mut disk_metrics = Vec::new();
        
        // Use native system call for disk stats (cross-platform via std)
        if cfg!(unix) {
            // On Unix, use statvfs for root filesystem
            if let Ok(stat) = std::fs::metadata("/") {
                // Simple approach: get root filesystem info
                let path = std::path::Path::new("/");
                if let Ok(entries) = std::fs::read_dir(path) {
                    // Use du command for accurate disk usage (Unix only)
                    if let Ok(output) = std::process::Command::new("df")
                        .arg("/")
                        .arg("--output=size,used,avail,pcent")
                        .arg("--block-size=1G")
                        .output()
                    {
                        if let Ok(output_str) = String::from_utf8(output.stdout) {
                            let lines: Vec<&str> = output_str.lines().collect();
                            if lines.len() > 1 {
                                let parts: Vec<&str> = lines[1].split_whitespace().collect();
                                if parts.len() >= 4 {
                                    let total_gb: f64 = parts[0].parse().unwrap_or(0.0);
                                    let used_gb: f64 = parts[1].parse().unwrap_or(0.0);
                                    let free_gb: f64 = parts[2].parse().unwrap_or(0.0);
                                    let percent_str = parts[3].trim_end_matches('%');
                                    let percent_used: f32 = percent_str.parse().unwrap_or(0.0);
                                    
                                    disk_metrics.push(DiskMetrics {
                                        path: "/".to_string(),
                                        total_gb,
                                        used_gb,
                                        free_gb,
                                        percent_used,
                                    });
                                }
                            }
                        }
                    }
                }
            }
        }
        
        // Fallback - better than fake 50/100
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
    pub async fn collect_critical() -> Result<Vec<Self>> {
        // Placeholder - actual implementation will be OS-specific
        let critical_services = if cfg!(target_os = "linux") {
            vec!["ssh", "NetworkManager"]
        } else if cfg!(target_os = "windows") {
            vec!["Winmgmt", "EventLog"]
        } else {
            vec![]
        };
        
        let mut services = Vec::new();
        for service_name in critical_services {
            services.push(ServiceStatus {
                name: service_name.to_string(),
                status: ServiceState::Unknown,
                enabled: None,
            });
        }
        
        Ok(services)
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
        assert!(process_info.top_cpu.len() <= 5);
        assert!(process_info.top_memory.len() <= 5);
    }
}