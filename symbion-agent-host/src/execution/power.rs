//! Power management commands (shutdown, reboot, hibernate)

use anyhow::{Result, Context, anyhow};
use std::time::Instant;
use tracing::info;

use super::{create_async_command, ExecutionResult};

/// Execute power management command
pub async fn execute_power_command(command_type: &str, delay_secs: Option<u32>) -> Result<ExecutionResult> {
    let start_time = Instant::now();
    info!("Executing power command: {} (delay: {:?}s)", command_type, delay_secs);

    let result = match command_type {
        "shutdown" => shutdown(delay_secs.unwrap_or(0)).await,
        "reboot" => reboot(delay_secs.unwrap_or(0)).await,
        "hibernate" => hibernate().await,
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

async fn shutdown(delay_secs: u32) -> Result<String> {
    if cfg!(target_os = "linux") {
        let output = create_async_command("sudo")
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
        let output = create_async_command("shutdown")
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
        let output = create_async_command("sudo")
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
        let output = create_async_command("shutdown")
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
        let output = create_async_command("sudo")
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
        let output = create_async_command("shutdown")
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
