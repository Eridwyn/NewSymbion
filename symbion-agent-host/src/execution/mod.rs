//! Command execution module for Symbion agents
//!
//! Handles secure execution of system commands:
//! - Power management commands (shutdown, reboot, hibernate)
//! - Process control (kill by PID)
//! - Shell command execution with timeout
//! - Service management (start/stop/status)
//! - Cross-platform implementation
//! - Trait-based handler system for extensible command dispatch

pub mod handler;
pub mod handlers;
mod power;
mod shell;
mod process;
pub mod service;

use serde::Serialize;
use tokio::process::Command as AsyncCommand;

/// Create an async command, hiding console window on Windows
fn create_async_command(program: &str) -> AsyncCommand {
    #[cfg(target_os = "windows")]
    {
        crate::windows_utils::silent_tokio_command(program)
    }
    #[cfg(not(target_os = "windows"))]
    {
        AsyncCommand::new(program)
    }
}

/// Create a sync command, hiding console window on Windows
fn create_command(program: &str) -> std::process::Command {
    #[cfg(target_os = "windows")]
    {
        crate::windows_utils::silent_command(program)
    }
    #[cfg(not(target_os = "windows"))]
    {
        std::process::Command::new(program)
    }
}

/// Command execution result
#[derive(Debug, Serialize)]
pub struct ExecutionResult {
    pub success: bool,
    pub output: String,
    pub error: Option<String>,
    pub exit_code: Option<i32>,
    pub execution_time_ms: u128,
}

/// Cross-platform command executor — delegates to sub-modules
pub struct CommandExecutor;

impl CommandExecutor {
    /// Execute power management command
    pub async fn execute_power_command(command_type: &str, delay_secs: Option<u32>) -> anyhow::Result<ExecutionResult> {
        power::execute_power_command(command_type, delay_secs).await
    }

    /// Execute shell command with timeout
    pub async fn execute_shell_command(command: &str, timeout_secs: u32) -> anyhow::Result<ExecutionResult> {
        shell::execute_shell_command(command, timeout_secs).await
    }

    /// Kill process by PID
    pub async fn kill_process(pid: u32) -> anyhow::Result<ExecutionResult> {
        process::kill_process(pid).await
    }
}

// Re-export ServiceManager for backward compatibility
pub use service::ServiceManager;
