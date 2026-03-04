//! Cross-platform capabilities detection for Symbion agents
//!
//! Detects and reports available system control capabilities:
//! - Power management (shutdown, reboot, hibernate)
//! - Process control (list, kill, monitor)
//! - Command execution (shell commands with timeout)
//! - Service management (systemd, Windows services)
//! - File operations (future extension)

use serde::{Deserialize, Serialize};
use tracing::debug;

use crate::windows_utils;

/// Supported capability types
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CapabilityType {
    PowerManagement,
    ProcessControl,
    CommandExecution,
    SystemMetrics,
    ServiceManagement,
    FileOperations,
    Notifications,
}

/// Capability detection result
#[derive(Debug)]
#[allow(dead_code)] // reason used for debug logging context
pub struct CapabilityInfo {
    pub capability_type: CapabilityType,
    pub available: bool,
    pub reason: Option<String>,
}

/// Cross-platform capability detector
pub struct CapabilityDetector;

impl CapabilityDetector {
    /// Detect all available capabilities on current platform
    pub async fn detect_all() -> Vec<CapabilityInfo> {
        debug!("Detecting system capabilities...");
        
        let mut capabilities = Vec::new();
        
        capabilities.push(Self::detect_power_management().await);
        capabilities.push(Self::detect_process_control().await);
        capabilities.push(Self::detect_command_execution().await);
        capabilities.push(Self::detect_system_metrics().await);
        capabilities.push(Self::detect_service_management().await);
        capabilities.push(Self::detect_file_operations().await);
        capabilities.push(Self::detect_notifications().await);

        let available_count = capabilities.iter().filter(|c| c.available).count();
        debug!("Detected {}/{} capabilities available", available_count, capabilities.len());
        
        capabilities
    }
    
    /// Get list of available capability names for registration
    pub async fn get_available_capabilities() -> Vec<String> {
        Self::detect_all().await
            .into_iter()
            .filter(|c| c.available)
            .map(|c| match c.capability_type {
                CapabilityType::PowerManagement => "power_management",
                CapabilityType::ProcessControl => "process_control", 
                CapabilityType::CommandExecution => "command_execution",
                CapabilityType::SystemMetrics => "system_metrics",
                CapabilityType::ServiceManagement => "service_management",
                CapabilityType::FileOperations => "file_operations",
                CapabilityType::Notifications => "notifications",
            })
            .map(String::from)
            .collect()
    }
    
    /// Detect power management capabilities
    async fn detect_power_management() -> CapabilityInfo {
        let available = if cfg!(target_os = "linux") {
            // Check for shutdown command and systemctl
            Self::command_exists("shutdown").await || Self::command_exists("systemctl").await
        } else if cfg!(target_os = "windows") {
            // Windows has built-in shutdown command
            Self::command_exists("shutdown").await
        } else if cfg!(target_os = "android") {
            // Android in Termux might have limited power control
            false
        } else {
            false
        };
        
        let reason = if !available {
            Some("No power management commands found".to_string())
        } else {
            None
        };
        
        CapabilityInfo {
            capability_type: CapabilityType::PowerManagement,
            available,
            reason,
        }
    }
    
    /// Detect process control capabilities
    async fn detect_process_control() -> CapabilityInfo {
        let available = if cfg!(target_os = "linux") {
            Self::command_exists("ps").await && Self::command_exists("kill").await
        } else if cfg!(target_os = "windows") {
            Self::command_exists("tasklist").await && Self::command_exists("taskkill").await
        } else if cfg!(target_os = "android") {
            Self::command_exists("ps").await && Self::command_exists("kill").await
        } else {
            false
        };
        
        let reason = if !available {
            Some("Process control commands not found".to_string())
        } else {
            None  
        };
        
        CapabilityInfo {
            capability_type: CapabilityType::ProcessControl,
            available,
            reason,
        }
    }
    
    /// Detect command execution capabilities
    async fn detect_command_execution() -> CapabilityInfo {
        let available = if cfg!(target_os = "linux") {
            Self::command_exists("bash").await || Self::command_exists("sh").await
        } else if cfg!(target_os = "windows") {
            Self::command_exists("cmd").await || Self::command_exists("powershell").await
        } else if cfg!(target_os = "android") {
            Self::command_exists("sh").await
        } else {
            false
        };
        
        let reason = if !available {
            Some("No shell interpreters found".to_string())
        } else {
            None
        };
        
        CapabilityInfo {
            capability_type: CapabilityType::CommandExecution,
            available,
            reason,
        }
    }
    
    /// System metrics are always available (using sysinfo crate)
    async fn detect_system_metrics() -> CapabilityInfo {
        CapabilityInfo {
            capability_type: CapabilityType::SystemMetrics,
            available: true,
            reason: None,
        }
    }
    
    /// Detect service management capabilities
    async fn detect_service_management() -> CapabilityInfo {
        let available = if cfg!(target_os = "linux") {
            Self::command_exists("systemctl").await
        } else if cfg!(target_os = "windows") {
            Self::command_exists("sc").await || Self::command_exists("net").await
        } else {
            false
        };
        
        let reason = if !available {
            Some("Service management tools not found".to_string())
        } else {
            None
        };
        
        CapabilityInfo {
            capability_type: CapabilityType::ServiceManagement,
            available,
            reason,
        }
    }
    
    /// File operations (file transfer over MQTT)
    async fn detect_file_operations() -> CapabilityInfo {
        CapabilityInfo {
            capability_type: CapabilityType::FileOperations,
            available: true,
            reason: None,
        }
    }
    
    /// Detect notification capability (compile-time feature check)
    async fn detect_notifications() -> CapabilityInfo {
        let available = cfg!(feature = "notifications");
        let reason = if !available {
            Some("Notifications feature not compiled".to_string())
        } else {
            None
        };
        CapabilityInfo {
            capability_type: CapabilityType::Notifications,
            available,
            reason,
        }
    }

    /// Check if a command exists in PATH
    async fn command_exists(command: &str) -> bool {
        #[cfg(target_os = "windows")]
        let check_cmd = "where";
        #[cfg(not(target_os = "windows"))]
        let check_cmd = "which";

        match windows_utils::silent_command(check_cmd).arg(command).output() {
            Ok(output) => output.status.success(),
            Err(_) => false,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[tokio::test]
    async fn test_capability_detection() {
        let capabilities = CapabilityDetector::detect_all().await;
        assert!(!capabilities.is_empty());
        
        // System metrics should always be available
        assert!(capabilities.iter().any(|c| 
            matches!(c.capability_type, CapabilityType::SystemMetrics) && c.available
        ));
    }
    
    #[tokio::test]
    async fn test_available_capabilities_list() {
        let available = CapabilityDetector::get_available_capabilities().await;
        assert!(available.contains(&"system_metrics".to_string()));
    }
}