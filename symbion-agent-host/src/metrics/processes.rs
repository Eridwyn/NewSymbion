//! Process information and top consumers

use anyhow::Result;
use serde::Serialize;
use sysinfo::{System, ProcessStatus};

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

impl ProcessInfo {
    pub async fn collect() -> Result<Self> {
        let mut sys = System::new();
        sys.refresh_processes();

        let processes: Vec<_> = sys.processes().values().collect();
        let total_count = processes.len();
        let running_count = processes.iter()
            .filter(|p| matches!(p.status(), ProcessStatus::Run))
            .count();

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

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_process_info() {
        let process_info = ProcessInfo::collect().await.unwrap();
        assert!(process_info.total_count > 0);
        assert!(process_info.top_cpu.len() <= 15);
        assert!(process_info.top_memory.len() <= 15);
    }
}
