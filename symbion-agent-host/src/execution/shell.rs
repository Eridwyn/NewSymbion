//! Shell command execution with timeout

use anyhow::{Result, Context, anyhow};
use std::process::Stdio;
use std::time::{Duration, Instant};
use tracing::debug;

use super::{create_async_command, create_command, ExecutionResult};

/// Execute shell command with timeout
pub async fn execute_shell_command(command: &str, timeout_secs: u32) -> Result<ExecutionResult> {
    let start_time = Instant::now();
    debug!("Executing shell command: {} (timeout: {}s)", command, timeout_secs);

    let result = if cfg!(target_os = "windows") {
        execute_windows_command(command, timeout_secs).await
    } else {
        execute_unix_command(command, timeout_secs).await
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

async fn execute_unix_command(command: &str, timeout_secs: u32) -> Result<(String, i32)> {
    let child = create_async_command("bash")
        .arg("-c")
        .arg(command)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .context("Failed to spawn command")?;

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
            if let Some(pid) = child_pid {
                let _ = create_command("kill")
                    .args(["-9", &pid.to_string()])
                    .output();
            }
            Err(anyhow!("Command timed out after {}s (process killed)", timeout_secs))
        }
    }
}

async fn execute_windows_command(command: &str, timeout_secs: u32) -> Result<(String, i32)> {
    let child = create_async_command("cmd")
        .args(["/C", command])
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .context("Failed to spawn command")?;

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
            if let Some(pid) = child_pid {
                let _ = create_command("taskkill")
                    .args(["/PID", &pid.to_string(), "/F"])
                    .output();
            }
            Err(anyhow!("Command timed out after {}s (process killed)", timeout_secs))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_shell_command_execution() {
        let result = if cfg!(target_os = "windows") {
            execute_shell_command("echo Hello World", 5).await.unwrap()
        } else {
            execute_shell_command("echo 'Hello World'", 5).await.unwrap()
        };

        assert!(result.success);
        assert!(result.output.contains("Hello World"));
        assert!(result.execution_time_ms < 5000);
    }

    #[tokio::test]
    async fn test_command_timeout() {
        let result = if cfg!(target_os = "windows") {
            execute_shell_command("ping -t 127.0.0.1", 2).await.unwrap()
        } else {
            execute_shell_command("sleep 10", 2).await.unwrap()
        };

        assert!(!result.success);
        assert!(result.error.is_some());
    }
}
