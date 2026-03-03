//! Disk I/O throughput metrics

use serde::{Deserialize, Serialize};
use tracing::debug;

/// Disk I/O metrics for all block devices
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DiskIoMetrics {
    pub disks: Vec<DiskIoInfo>,
}

/// Per-device I/O statistics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DiskIoInfo {
    /// Device name (e.g. "sda", "nvme0n1")
    pub device: String,
    /// Read throughput in bytes per second
    pub read_bytes_per_sec: u64,
    /// Write throughput in bytes per second
    pub write_bytes_per_sec: u64,
    /// Read operations per second
    pub read_iops: u64,
    /// Write operations per second
    pub write_iops: u64,
}

/// Raw snapshot of a single device from /proc/diskstats
#[derive(Debug, Clone)]
struct DiskSnapshot {
    device: String,
    reads_completed: u64,
    sectors_read: u64,
    writes_completed: u64,
    sectors_written: u64,
}

/// Sector size in bytes (standard Linux block layer)
const SECTOR_SIZE: u64 = 512;

/// Check whether a device name represents a real block device
/// (not a partition, loop device, ramdisk, or device-mapper device).
fn is_real_device(name: &str) -> bool {
    if name.starts_with("loop") || name.starts_with("ram") || name.starts_with("dm-") {
        return false;
    }
    // nvme0n1 is a device, nvme0n1p1 is a partition
    if name.starts_with("nvme") {
        return !name.contains('p');
    }
    // sda is a device, sda1 is a partition
    !name.chars().last().is_some_and(|c| c.is_ascii_digit())
}

/// Parse the contents of /proc/diskstats into a list of device snapshots,
/// filtering to real block devices only.
fn parse_diskstats(content: &str) -> Vec<DiskSnapshot> {
    let mut snapshots = Vec::new();

    for line in content.lines() {
        let fields: Vec<&str> = line.split_whitespace().collect();
        if fields.len() < 10 {
            continue;
        }

        let device = fields[2];
        if !is_real_device(device) {
            continue;
        }

        let reads_completed = fields[3].parse::<u64>().unwrap_or(0);
        let sectors_read = fields[5].parse::<u64>().unwrap_or(0);
        let writes_completed = fields[7].parse::<u64>().unwrap_or(0);
        let sectors_written = fields[9].parse::<u64>().unwrap_or(0);

        snapshots.push(DiskSnapshot {
            device: device.to_string(),
            reads_completed,
            sectors_read,
            writes_completed,
            sectors_written,
        });
    }

    snapshots
}

impl DiskIoMetrics {
    /// Collect disk I/O metrics by taking two snapshots 1 second apart.
    ///
    /// Returns `None` if disk stats cannot be read (e.g. non-Linux OS).
    #[cfg(target_os = "linux")]
    pub async fn collect() -> Option<Self> {
        use tokio::time::{sleep, Duration, Instant};

        debug!("Collecting disk I/O metrics...");

        let content1 = tokio::fs::read_to_string("/proc/diskstats").await.ok()?;
        let snap1 = parse_diskstats(&content1);

        let start = Instant::now();
        sleep(Duration::from_secs(1)).await;
        let elapsed_secs = start.elapsed().as_secs_f64();

        let content2 = tokio::fs::read_to_string("/proc/diskstats").await.ok()?;
        let snap2 = parse_diskstats(&content2);

        let mut disks = Vec::new();

        for s2 in &snap2 {
            if let Some(s1) = snap1.iter().find(|s| s.device == s2.device) {
                let read_sectors_delta = s2.sectors_read.saturating_sub(s1.sectors_read);
                let write_sectors_delta = s2.sectors_written.saturating_sub(s1.sectors_written);
                let read_ops_delta = s2.reads_completed.saturating_sub(s1.reads_completed);
                let write_ops_delta = s2.writes_completed.saturating_sub(s1.writes_completed);

                let read_bytes_per_sec =
                    (read_sectors_delta as f64 * SECTOR_SIZE as f64 / elapsed_secs) as u64;
                let write_bytes_per_sec =
                    (write_sectors_delta as f64 * SECTOR_SIZE as f64 / elapsed_secs) as u64;
                let read_iops = (read_ops_delta as f64 / elapsed_secs) as u64;
                let write_iops = (write_ops_delta as f64 / elapsed_secs) as u64;

                disks.push(DiskIoInfo {
                    device: s2.device.clone(),
                    read_bytes_per_sec,
                    write_bytes_per_sec,
                    read_iops,
                    write_iops,
                });
            }
        }

        debug!("Disk I/O: {} devices collected", disks.len());

        if disks.is_empty() {
            None
        } else {
            Some(DiskIoMetrics { disks })
        }
    }

    /// Windows: collect disk I/O metrics via PowerShell Get-Counter.
    #[cfg(target_os = "windows")]
    pub async fn collect() -> Option<Self> {
        use std::time::Duration;

        debug!("Collecting disk I/O metrics (Windows)...");

        let ps_script = r#"Get-Counter '\PhysicalDisk(*)\Disk Read Bytes/sec','\PhysicalDisk(*)\Disk Write Bytes/sec','\PhysicalDisk(*)\Disk Reads/sec','\PhysicalDisk(*)\Disk Writes/sec' -SampleInterval 1 -MaxSamples 1 | ForEach-Object { $_.CounterSamples | ForEach-Object { '{0}|{1}' -f $_.Path,$_.CookedValue } }"#;

        let output = tokio::time::timeout(
            Duration::from_secs(3),
            crate::windows_utils::silent_tokio_command("powershell")
                .args(["-NoProfile", "-NonInteractive", "-Command", ps_script])
                .output(),
        )
        .await
        .ok()?
        .ok()?;

        if !output.status.success() {
            debug!("PowerShell Get-Counter failed: {}", output.status);
            return None;
        }

        let stdout = String::from_utf8_lossy(&output.stdout);
        let disks = parse_windows_disk_counters(&stdout);

        debug!("Disk I/O (Windows): {} devices collected", disks.len());

        if disks.is_empty() {
            None
        } else {
            Some(DiskIoMetrics { disks })
        }
    }

    /// Other platforms: disk I/O collection not yet implemented.
    #[cfg(not(any(target_os = "linux", target_os = "windows")))]
    pub async fn collect() -> Option<Self> {
        None
    }
}

// ---------------------------------------------------------------------------
// Windows parse helpers
// ---------------------------------------------------------------------------

/// Parse PowerShell Get-Counter output lines for disk I/O metrics.
///
/// Each line has the format:
///   \\server\physicaldisk(N X:)\counter name|value
///
/// The disk instance looks like "0 c:" or "_total".
/// We skip the "_total" aggregate instance.
#[cfg(target_os = "windows")]
fn parse_windows_disk_counters(output: &str) -> Vec<DiskIoInfo> {
    use std::collections::HashMap;

    // Accumulate per-disk values
    struct DiskAccum {
        read_bytes_per_sec: u64,
        write_bytes_per_sec: u64,
        read_iops: u64,
        write_iops: u64,
    }

    let mut map: HashMap<String, DiskAccum> = HashMap::new();

    for line in output.lines() {
        let line = line.trim();
        if line.is_empty() {
            continue;
        }

        // Split on '|' to get path and value
        let Some((path, value_str)) = line.rsplit_once('|') else {
            continue;
        };

        let value: f64 = match value_str.trim().parse() {
            Ok(v) => v,
            Err(_) => continue,
        };

        let path_lower = path.to_lowercase();

        // Skip _total aggregate
        if path_lower.contains("(_total)") {
            continue;
        }

        // Extract disk instance name from path, e.g. "0 c:" from "physicaldisk(0 c:)"
        let disk_name = match extract_counter_instance(&path_lower, "physicaldisk") {
            Some(name) => name,
            None => continue,
        };

        let entry = map.entry(disk_name).or_insert(DiskAccum {
            read_bytes_per_sec: 0,
            write_bytes_per_sec: 0,
            read_iops: 0,
            write_iops: 0,
        });

        if path_lower.contains("disk read bytes/sec") {
            entry.read_bytes_per_sec = value as u64;
        } else if path_lower.contains("disk write bytes/sec") {
            entry.write_bytes_per_sec = value as u64;
        } else if path_lower.contains("disk reads/sec") {
            entry.read_iops = value as u64;
        } else if path_lower.contains("disk writes/sec") {
            entry.write_iops = value as u64;
        }
    }

    map.into_iter()
        .map(|(device, accum)| DiskIoInfo {
            device,
            read_bytes_per_sec: accum.read_bytes_per_sec,
            write_bytes_per_sec: accum.write_bytes_per_sec,
            read_iops: accum.read_iops,
            write_iops: accum.write_iops,
        })
        .collect()
}

/// Extract the instance name from a performance counter path.
///
/// Given a path like `\\server\physicaldisk(0 c:)\disk read bytes/sec`
/// and object_name `"physicaldisk"`, returns `Some("0 c:")`.
#[cfg(target_os = "windows")]
fn extract_counter_instance(path: &str, object_name: &str) -> Option<String> {
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

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_disk_io_collect() {
        // Should return Some on Linux with /proc/diskstats, None on other OS.
        // Must not panic regardless.
        let result = DiskIoMetrics::collect().await;
        if cfg!(target_os = "linux") {
            // On Linux CI the file exists, but we tolerate None if running in a
            // minimal container without block devices.
            if let Some(metrics) = result {
                assert!(!metrics.disks.is_empty());
                for d in &metrics.disks {
                    assert!(!d.device.is_empty());
                }
            }
        } else {
            assert!(result.is_none());
        }
    }

    #[test]
    fn test_disk_io_serialization() {
        let metrics = DiskIoMetrics {
            disks: vec![
                DiskIoInfo {
                    device: "sda".to_string(),
                    read_bytes_per_sec: 1_048_576,
                    write_bytes_per_sec: 524_288,
                    read_iops: 120,
                    write_iops: 85,
                },
                DiskIoInfo {
                    device: "nvme0n1".to_string(),
                    read_bytes_per_sec: 2_097_152,
                    write_bytes_per_sec: 1_048_576,
                    read_iops: 350,
                    write_iops: 200,
                },
            ],
        };

        let json = serde_json::to_string(&metrics).expect("serialize");
        let roundtrip: DiskIoMetrics = serde_json::from_str(&json).expect("deserialize");

        assert_eq!(roundtrip.disks.len(), 2);
        assert_eq!(roundtrip.disks[0].device, "sda");
        assert_eq!(roundtrip.disks[0].read_bytes_per_sec, 1_048_576);
        assert_eq!(roundtrip.disks[0].write_bytes_per_sec, 524_288);
        assert_eq!(roundtrip.disks[0].read_iops, 120);
        assert_eq!(roundtrip.disks[0].write_iops, 85);
        assert_eq!(roundtrip.disks[1].device, "nvme0n1");
        assert_eq!(roundtrip.disks[1].read_bytes_per_sec, 2_097_152);
        assert_eq!(roundtrip.disks[1].write_bytes_per_sec, 1_048_576);
        assert_eq!(roundtrip.disks[1].read_iops, 350);
        assert_eq!(roundtrip.disks[1].write_iops, 200);
    }

    #[test]
    fn test_parse_proc_diskstats() {
        let mock_content = "\
   8       0 sda 51445 1116 2293354 17604 38291 8123 1234567 9876 0 15432 27480
   8       1 sda1 51200 1100 2290000 17500 38000 8100 1230000 9800 0 15000 27000
   7       0 loop0 100 0 800 50 0 0 0 0 0 50 50
 259       0 nvme0n1 123456 5678 9876543 45678 98765 4321 8765432 34567 0 56789 80245
 259       1 nvme0n1p1 123000 5600 9870000 45000 98000 4300 8760000 34000 0 56000 79000";

        let snapshots = parse_diskstats(mock_content);

        // Only real devices: sda and nvme0n1
        assert_eq!(snapshots.len(), 2, "Expected 2 real devices, got {}", snapshots.len());

        let devices: Vec<&str> = snapshots.iter().map(|s| s.device.as_str()).collect();
        assert!(devices.contains(&"sda"), "Missing sda");
        assert!(devices.contains(&"nvme0n1"), "Missing nvme0n1");
        assert!(!devices.contains(&"sda1"), "sda1 should be filtered");
        assert!(!devices.contains(&"loop0"), "loop0 should be filtered");
        assert!(!devices.contains(&"nvme0n1p1"), "nvme0n1p1 should be filtered");

        // Verify sda fields
        let sda = snapshots.iter().find(|s| s.device == "sda").unwrap();
        assert_eq!(sda.reads_completed, 51445);
        assert_eq!(sda.sectors_read, 2293354);
        assert_eq!(sda.writes_completed, 38291);
        assert_eq!(sda.sectors_written, 1234567);

        // Verify nvme0n1 fields
        let nvme = snapshots.iter().find(|s| s.device == "nvme0n1").unwrap();
        assert_eq!(nvme.reads_completed, 123456);
        assert_eq!(nvme.sectors_read, 9876543);
        assert_eq!(nvme.writes_completed, 98765);
        assert_eq!(nvme.sectors_written, 8765432);
    }
}
