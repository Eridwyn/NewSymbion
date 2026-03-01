//! Process control (kill by PID)

use anyhow::{Result, Context, anyhow};
use std::time::Instant;
use tracing::info;

use super::{create_async_command, ExecutionResult};

/// Kill process by PID
pub async fn kill_process(pid: u32) -> Result<ExecutionResult> {
    let start_time = Instant::now();
    info!("Killing process PID: {}", pid);

    let result = if cfg!(target_os = "windows") {
        kill_process_windows(pid).await
    } else {
        kill_process_unix(pid).await
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

async fn kill_process_unix(pid: u32) -> Result<String> {
    let output = create_async_command("kill")
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
    let output = create_async_command("taskkill")
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
