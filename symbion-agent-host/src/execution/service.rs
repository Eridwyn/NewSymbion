//! Service management — cross-platform start/stop/restart/status

use anyhow::{Result, Context, anyhow};
use serde::Serialize;
use std::process::Stdio;
use std::time::Instant;
use tracing::info;

use super::{create_async_command, ExecutionResult};

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

    async fn run_service_command(service_name: &str, action: &str) -> Result<ExecutionResult> {
        let start_time = Instant::now();

        let output = if cfg!(target_os = "linux") {
            create_async_command("sudo")
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
                "restart" => {
                    return Self::windows_restart(service_name, start_time).await;
                }
                _ => return Err(anyhow!("Unknown action: {}", action)),
            };
            create_async_command("sc")
                .args([sc_action, service_name])
                .stdout(Stdio::piped())
                .stderr(Stdio::piped())
                .output()
                .await
                .context(format!("Failed to {} service {}", action, service_name))?
        } else if cfg!(target_os = "macos") {
            create_async_command("sudo")
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
        let output = create_async_command("cmd")
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
        let output = create_async_command("systemctl")
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
        let output = create_async_command("sc")
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
        let output = create_async_command("launchctl")
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
