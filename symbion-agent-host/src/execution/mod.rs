//! Command execution module for Symbion agents
//!
//! Handles secure execution of system commands:
//! - Power management commands (shutdown, reboot, hibernate)
//! - Process control (list, kill by PID)  
//! - Shell command execution with timeout
//! - Service management (start/stop/status)
//! - Cross-platform implementation

use anyhow::{Result, Context, anyhow};
use serde::Serialize;
use std::process::Stdio;
use std::time::{Duration, Instant};
use tokio::process::Command as AsyncCommand;
use tracing::{info, debug};

/// Command execution result
#[derive(Debug, Serialize)]
pub struct ExecutionResult {
    pub success: bool,
    pub output: String,
    pub error: Option<String>,
    pub exit_code: Option<i32>,
    pub execution_time_ms: u128,
}

/// Process information for listing
#[derive(Debug, Serialize)]
pub struct ProcessInfo {
    pub pid: u32,
    pub name: String,
    pub cpu_percent: f32,
    pub memory_mb: f64,
    pub status: String,
    pub user: Option<String>,
}

/// Cross-platform command executor
pub struct CommandExecutor;

impl CommandExecutor {
    /// Execute power management command
    pub async fn execute_power_command(command_type: &str, delay_secs: Option<u32>) -> Result<ExecutionResult> {
        let start_time = Instant::now();
        info!("Executing power command: {} (delay: {:?}s)", command_type, delay_secs);
        
        let result = match command_type {
            "shutdown" => Self::shutdown(delay_secs.unwrap_or(0)).await,
            "reboot" => Self::reboot(delay_secs.unwrap_or(0)).await,
            "hibernate" => Self::hibernate().await,
            _ => Err(anyhow!("Unknown power command: {}", command_type)),
        };
        
        let execution_time = start_time.elapsed().as_millis();
        
        match result {
            Ok(output) => Ok(ExecutionResult {
                success: true,
                output,
                error: None,
                exit_code: Some(0),
                execution_time_ms: execution_time,
            }),
            Err(e) => Ok(ExecutionResult {
                success: false,
                output: String::new(),
                error: Some(e.to_string()),
                exit_code: Some(1),
                execution_time_ms: execution_time,
            }),
        }
    }
    
    /// Execute shell command with timeout
    pub async fn execute_shell_command(command: &str, timeout_secs: u32) -> Result<ExecutionResult> {
        let start_time = Instant::now();
        debug!("Executing shell command: {} (timeout: {}s)", command, timeout_secs);
        
        let result = if cfg!(target_os = "windows") {
            Self::execute_windows_command(command, timeout_secs).await
        } else {
            Self::execute_unix_command(command, timeout_secs).await
        };
        
        let execution_time = start_time.elapsed().as_millis();
        
        match result {
            Ok((output, exit_code)) => Ok(ExecutionResult {
                success: exit_code == 0,
                output,
                error: None,
                exit_code: Some(exit_code),
                execution_time_ms: execution_time,
            }),
            Err(e) => Ok(ExecutionResult {
                success: false,
                output: String::new(),
                error: Some(e.to_string()),
                exit_code: Some(-1),
                execution_time_ms: execution_time,
            }),
        }
    }
    
    /// Kill process by PID
    pub async fn kill_process(pid: u32) -> Result<ExecutionResult> {
        let start_time = Instant::now();
        info!("Killing process PID: {}", pid);
        
        let result = if cfg!(target_os = "windows") {
            Self::kill_process_windows(pid).await
        } else {
            Self::kill_process_unix(pid).await
        };
        
        let execution_time = start_time.elapsed().as_millis();
        
        match result {
            Ok(output) => Ok(ExecutionResult {
                success: true,
                output,
                error: None,
                exit_code: Some(0),
                execution_time_ms: execution_time,
            }),
            Err(e) => Ok(ExecutionResult {
                success: false,
                output: String::new(),
                error: Some(e.to_string()),
                exit_code: Some(1),
                execution_time_ms: execution_time,
            }),
        }
    }
    
    /// List running processes
    pub async fn list_processes() -> Result<Vec<ProcessInfo>> {
        debug!("Listing system processes");
        
        let mut sys = sysinfo::System::new();
        sys.refresh_processes();
        
        let processes = sys.processes()
            .values()
            .map(|p| ProcessInfo {
                pid: p.pid().as_u32(),
                name: p.name().to_string(),
                cpu_percent: p.cpu_usage(),
                memory_mb: p.memory() as f64 / (1024.0 * 1024.0),
                status: format!("{:?}", p.status()),
                user: p.user_id().map(|u| u.to_string()),
            })
            .collect();
        
        Ok(processes)
    }
    
    // Platform-specific implementations
    
    async fn shutdown(delay_secs: u32) -> Result<String> {
        if cfg!(target_os = "linux") {
            let output = AsyncCommand::new("sudo")
                .args(["shutdown", "-h", &format!("+{}", delay_secs / 60)])
                .output()
                .await
                .context("Failed to execute shutdown command")?;
                
            if output.status.success() {
                Ok(format!("Shutdown scheduled in {} seconds", delay_secs))
            } else {
                Err(anyhow!("Shutdown failed: {}", String::from_utf8_lossy(&output.stderr)))
            }
        } else if cfg!(target_os = "windows") {
            let output = AsyncCommand::new("shutdown")
                .args(["/s", "/t", &delay_secs.to_string()])
                .output()
                .await
                .context("Failed to execute shutdown command")?;
                
            if output.status.success() {
                Ok(format!("Shutdown scheduled in {} seconds", delay_secs))
            } else {
                Err(anyhow!("Shutdown failed: {}", String::from_utf8_lossy(&output.stderr)))
            }
        } else {
            Err(anyhow!("Shutdown not supported on this platform"))
        }
    }
    
    async fn reboot(delay_secs: u32) -> Result<String> {
        if cfg!(target_os = "linux") {
            let output = AsyncCommand::new("sudo")
                .args(["reboot"])
                .output()
                .await
                .context("Failed to execute reboot command")?;
                
            if output.status.success() {
                Ok("Reboot initiated".to_string())
            } else {
                Err(anyhow!("Reboot failed: {}", String::from_utf8_lossy(&output.stderr)))
            }
        } else if cfg!(target_os = "windows") {
            let output = AsyncCommand::new("shutdown")
                .args(["/r", "/t", &delay_secs.to_string()])
                .output()
                .await
                .context("Failed to execute reboot command")?;
                
            if output.status.success() {
                Ok(format!("Reboot scheduled in {} seconds", delay_secs))
            } else {
                Err(anyhow!("Reboot failed: {}", String::from_utf8_lossy(&output.stderr)))
            }
        } else {
            Err(anyhow!("Reboot not supported on this platform"))
        }
    }
    
    async fn hibernate() -> Result<String> {
        if cfg!(target_os = "linux") {
            let output = AsyncCommand::new("sudo")
                .args(["systemctl", "hibernate"])
                .output()
                .await
                .context("Failed to execute hibernate command")?;
                
            if output.status.success() {
                Ok("Hibernate initiated".to_string())
            } else {
                Err(anyhow!("Hibernate failed: {}", String::from_utf8_lossy(&output.stderr)))
            }
        } else if cfg!(target_os = "windows") {
            let output = AsyncCommand::new("shutdown")
                .args(["/h"])
                .output()
                .await
                .context("Failed to execute hibernate command")?;
                
            if output.status.success() {
                Ok("Hibernate initiated".to_string())
            } else {
                Err(anyhow!("Hibernate failed: {}", String::from_utf8_lossy(&output.stderr)))
            }
        } else {
            Err(anyhow!("Hibernate not supported on this platform"))
        }
    }
    
    async fn execute_unix_command(command: &str, timeout_secs: u32) -> Result<(String, i32)> {
        let output = tokio::time::timeout(
            Duration::from_secs(timeout_secs as u64),
            AsyncCommand::new("bash")
                .arg("-c")
                .arg(command)
                .stdout(Stdio::piped())
                .stderr(Stdio::piped())
                .output()
        )
        .await
        .context("Command timed out")?
        .context("Failed to execute command")?;
        
        let stdout = String::from_utf8_lossy(&output.stdout);
        let stderr = String::from_utf8_lossy(&output.stderr);
        let combined_output = if stderr.is_empty() {
            stdout.to_string()
        } else {
            format!("{}\nSTDERR:\n{}", stdout, stderr)
        };
        
        let exit_code = output.status.code().unwrap_or(-1);
        
        Ok((combined_output, exit_code))
    }
    
    async fn execute_windows_command(command: &str, timeout_secs: u32) -> Result<(String, i32)> {
        let output = tokio::time::timeout(
            Duration::from_secs(timeout_secs as u64),
            AsyncCommand::new("cmd")
                .args(["/C", command])
                .stdout(Stdio::piped())
                .stderr(Stdio::piped())
                .output()
        )
        .await
        .context("Command timed out")?
        .context("Failed to execute command")?;
        
        let stdout = String::from_utf8_lossy(&output.stdout);
        let stderr = String::from_utf8_lossy(&output.stderr);
        let combined_output = if stderr.is_empty() {
            stdout.to_string()
        } else {
            format!("{}\nSTDERR:\n{}", stdout, stderr)
        };
        
        let exit_code = output.status.code().unwrap_or(-1);
        
        Ok((combined_output, exit_code))
    }
    
    async fn kill_process_unix(pid: u32) -> Result<String> {
        let output = AsyncCommand::new("kill")
            .arg(pid.to_string())
            .output()
            .await
            .context("Failed to execute kill command")?;
            
        if output.status.success() {
            Ok(format!("Process {} killed successfully", pid))
        } else {
            Err(anyhow!("Kill failed: {}", String::from_utf8_lossy(&output.stderr)))
        }
    }
    
    async fn kill_process_windows(pid: u32) -> Result<String> {
        let output = AsyncCommand::new("taskkill")
            .args(["/PID", &pid.to_string(), "/F"])
            .output()
            .await
            .context("Failed to execute taskkill command")?;
            
        if output.status.success() {
            Ok(format!("Process {} killed successfully", pid))
        } else {
            Err(anyhow!("Taskkill failed: {}", String::from_utf8_lossy(&output.stderr)))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[tokio::test]
    async fn test_shell_command_execution() {
        let result = if cfg!(target_os = "windows") {
            CommandExecutor::execute_shell_command("echo Hello World", 5).await.unwrap()
        } else {
            CommandExecutor::execute_shell_command("echo 'Hello World'", 5).await.unwrap()
        };
        
        assert!(result.success);
        assert!(result.output.contains("Hello World"));
        assert!(result.execution_time_ms < 5000);
    }
    
    #[tokio::test]
    async fn test_process_listing() {
        let processes = CommandExecutor::list_processes().await.unwrap();
        assert!(!processes.is_empty());
        assert!(processes.iter().any(|p| p.pid > 0));
    }
    
    #[tokio::test]
    async fn test_command_timeout() {
        let result = if cfg!(target_os = "windows") {
            CommandExecutor::execute_shell_command("ping -t 127.0.0.1", 2).await.unwrap()
        } else {
            CommandExecutor::execute_shell_command("sleep 10", 2).await.unwrap()
        };
        
        // Command should timeout and fail
        assert!(!result.success);
        assert!(result.error.is_some());
    }
}