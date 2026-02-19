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
    
    /// Kill process by PID and restart it by name
    pub async fn kill_and_restart(pid: u32, process_name: &str) -> Result<ExecutionResult> {
        let start_time = Instant::now();
        info!("Kill and restart: PID {} ({})", pid, process_name);

        // Kill the process
        let kill_result = Self::kill_process(pid).await?;
        if !kill_result.success {
            return Ok(kill_result);
        }

        // Wait for process to fully terminate
        tokio::time::sleep(Duration::from_secs(1)).await;

        // Attempt to restart using the process name
        let restart_result = if cfg!(target_os = "windows") {
            AsyncCommand::new("cmd")
                .args(["/C", "start", "", process_name])
                .stdout(Stdio::piped())
                .stderr(Stdio::piped())
                .output()
                .await
        } else {
            AsyncCommand::new("bash")
                .args(["-c", &format!("{} &", process_name)])
                .stdout(Stdio::piped())
                .stderr(Stdio::piped())
                .output()
                .await
        };

        let execution_time = start_time.elapsed().as_millis();

        match restart_result {
            Ok(output) => Ok(ExecutionResult {
                success: output.status.success(),
                output: format!("Killed PID {}, restarted '{}'", pid, process_name),
                error: if output.status.success() { None } else {
                    Some(String::from_utf8_lossy(&output.stderr).to_string())
                },
                exit_code: output.status.code(),
                execution_time_ms: execution_time,
            }),
            Err(e) => Ok(ExecutionResult {
                success: false,
                output: format!("Killed PID {} but failed to restart '{}'", pid, process_name),
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
        let child = AsyncCommand::new("bash")
            .arg("-c")
            .arg(command)
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .context("Failed to spawn command")?;

        // Save PID before wait_with_output() takes ownership
        let child_pid = child.id();

        match tokio::time::timeout(
            Duration::from_secs(timeout_secs as u64),
            child.wait_with_output()
        ).await {
            Ok(result) => {
                let output = result.context("Failed to execute command")?;
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
            Err(_) => {
                // Timeout — kill the orphaned child process by PID
                if let Some(pid) = child_pid {
                    let _ = std::process::Command::new("kill")
                        .args(["-9", &pid.to_string()])
                        .output();
                }
                Err(anyhow!("Command timed out after {}s (process killed)", timeout_secs))
            }
        }
    }
    
    async fn execute_windows_command(command: &str, timeout_secs: u32) -> Result<(String, i32)> {
        let child = AsyncCommand::new("cmd")
            .args(["/C", command])
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .context("Failed to spawn command")?;

        // Save PID before wait_with_output() takes ownership
        let child_pid = child.id();

        match tokio::time::timeout(
            Duration::from_secs(timeout_secs as u64),
            child.wait_with_output()
        ).await {
            Ok(result) => {
                let output = result.context("Failed to execute command")?;
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
            Err(_) => {
                // Timeout — kill the orphaned child process by PID
                if let Some(pid) = child_pid {
                    let _ = std::process::Command::new("taskkill")
                        .args(["/PID", &pid.to_string(), "/F"])
                        .output();
                }
                Err(anyhow!("Command timed out after {}s (process killed)", timeout_secs))
            }
        }
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

// ============================================================================
// Service Manager — Cross-platform service start/stop/restart/status
// ============================================================================

/// Service status result
#[derive(Debug, Serialize)]
pub struct ServiceStatus {
    pub name: String,
    pub running: bool,
    pub status_text: String,
}

/// Cross-platform service manager
pub struct ServiceManager;

impl ServiceManager {
    /// Get service status
    pub async fn status(service_name: &str) -> Result<ServiceStatus> {
        info!("Checking service status: {}", service_name);

        if cfg!(target_os = "linux") {
            Self::linux_status(service_name).await
        } else if cfg!(target_os = "windows") {
            Self::windows_status(service_name).await
        } else if cfg!(target_os = "macos") {
            Self::macos_status(service_name).await
        } else {
            Err(anyhow!("Service management not supported on this platform"))
        }
    }

    /// Start a service
    pub async fn start(service_name: &str) -> Result<ExecutionResult> {
        info!("Starting service: {}", service_name);
        Self::run_service_command(service_name, "start").await
    }

    /// Stop a service
    pub async fn stop(service_name: &str) -> Result<ExecutionResult> {
        info!("Stopping service: {}", service_name);
        Self::run_service_command(service_name, "stop").await
    }

    /// Restart a service (stop + start)
    pub async fn restart(service_name: &str) -> Result<ExecutionResult> {
        info!("Restarting service: {}", service_name);
        Self::run_service_command(service_name, "restart").await
    }

    /// Run a platform-specific service command
    async fn run_service_command(service_name: &str, action: &str) -> Result<ExecutionResult> {
        let start_time = Instant::now();

        let output = if cfg!(target_os = "linux") {
            AsyncCommand::new("sudo")
                .args(["systemctl", action, service_name])
                .stdout(Stdio::piped())
                .stderr(Stdio::piped())
                .output()
                .await
                .context(format!("Failed to {} service {}", action, service_name))?
        } else if cfg!(target_os = "windows") {
            let sc_action = match action {
                "start" => "start",
                "stop" => "stop",
                // Windows doesn't have native restart — use net stop/start in one command
                "restart" => {
                    return Self::windows_restart(service_name, start_time).await;
                }
                _ => return Err(anyhow!("Unknown action: {}", action)),
            };
            AsyncCommand::new("sc")
                .args([sc_action, service_name])
                .stdout(Stdio::piped())
                .stderr(Stdio::piped())
                .output()
                .await
                .context(format!("Failed to {} service {}", action, service_name))?
        } else if cfg!(target_os = "macos") {
            AsyncCommand::new("sudo")
                .args(["launchctl", action, service_name])
                .stdout(Stdio::piped())
                .stderr(Stdio::piped())
                .output()
                .await
                .context(format!("Failed to {} service {}", action, service_name))?
        } else {
            return Err(anyhow!("Service management not supported on this platform"));
        };

        let execution_time = start_time.elapsed().as_millis();
        let stdout = String::from_utf8_lossy(&output.stdout).to_string();
        let stderr = String::from_utf8_lossy(&output.stderr).to_string();

        Ok(ExecutionResult {
            success: output.status.success(),
            output: if stderr.is_empty() { stdout } else { format!("{}\n{}", stdout, stderr) },
            error: if output.status.success() { None } else { Some(stderr) },
            exit_code: output.status.code(),
            execution_time_ms: execution_time,
        })
    }

    async fn windows_restart(service_name: &str, start_time: Instant) -> Result<ExecutionResult> {
        let cmd = format!("net stop \"{}\" && net start \"{}\"", service_name, service_name);
        let output = AsyncCommand::new("cmd")
            .args(["/C", &cmd])
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .output()
            .await
            .context(format!("Failed to restart service {}", service_name))?;

        let execution_time = start_time.elapsed().as_millis();
        let stdout = String::from_utf8_lossy(&output.stdout).to_string();
        let stderr = String::from_utf8_lossy(&output.stderr).to_string();

        Ok(ExecutionResult {
            success: output.status.success(),
            output: if stderr.is_empty() { stdout } else { format!("{}\n{}", stdout, stderr) },
            error: if output.status.success() { None } else { Some(stderr) },
            exit_code: output.status.code(),
            execution_time_ms: execution_time,
        })
    }

    async fn linux_status(service_name: &str) -> Result<ServiceStatus> {
        let output = AsyncCommand::new("systemctl")
            .args(["is-active", service_name])
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .output()
            .await
            .context("Failed to check service status")?;

        let status_text = String::from_utf8_lossy(&output.stdout).trim().to_string();
        Ok(ServiceStatus {
            name: service_name.to_string(),
            running: status_text == "active",
            status_text,
        })
    }

    async fn windows_status(service_name: &str) -> Result<ServiceStatus> {
        let output = AsyncCommand::new("sc")
            .args(["query", service_name])
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .output()
            .await
            .context("Failed to check service status")?;

        let stdout = String::from_utf8_lossy(&output.stdout).to_string();
        let running = stdout.contains("RUNNING");
        let status_text = if running { "running" } else { "stopped" }.to_string();
        Ok(ServiceStatus {
            name: service_name.to_string(),
            running,
            status_text,
        })
    }

    async fn macos_status(service_name: &str) -> Result<ServiceStatus> {
        let output = AsyncCommand::new("launchctl")
            .args(["list", service_name])
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .output()
            .await
            .context("Failed to check service status")?;

        let running = output.status.success();
        Ok(ServiceStatus {
            name: service_name.to_string(),
            running,
            status_text: if running { "running".into() } else { "not found".into() },
        })
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