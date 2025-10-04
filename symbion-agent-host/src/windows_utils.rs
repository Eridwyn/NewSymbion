//! Windows-specific utilities for silent command execution
//! Prevents CMD/PowerShell windows from flashing on screen

#[cfg(target_os = "windows")]
use std::os::windows::process::CommandExt;

/// Windows creation flags to hide console windows
#[cfg(target_os = "windows")]
const CREATE_NO_WINDOW: u32 = 0x08000000;

/// Execute a Windows command silently (no window flash)
#[cfg(target_os = "windows")]
#[allow(dead_code)]
pub fn silent_command(program: &str) -> std::process::Command {
    let mut cmd = std::process::Command::new(program);
    cmd.creation_flags(CREATE_NO_WINDOW);
    cmd
}

/// Execute a Windows command silently (async version)
#[cfg(target_os = "windows")]
pub fn silent_tokio_command(program: &str) -> tokio::process::Command {
    let mut cmd = tokio::process::Command::new(program);
    cmd.creation_flags(CREATE_NO_WINDOW);
    cmd
}

/// Placeholder for non-Windows platforms
#[cfg(not(target_os = "windows"))]
#[allow(dead_code)]
pub fn silent_command(program: &str) -> std::process::Command {
    std::process::Command::new(program)
}

/// Placeholder for non-Windows platforms (async)
#[cfg(not(target_os = "windows"))]
pub fn silent_tokio_command(program: &str) -> tokio::process::Command {
    tokio::process::Command::new(program)
}
