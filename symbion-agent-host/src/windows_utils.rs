//! Cross-platform utilities for silent command execution and URL opening
//! On Windows, prevents CMD/PowerShell windows from flashing on screen

#[cfg(target_os = "windows")]
use std::os::windows::process::CommandExt;

/// Windows creation flags to hide console windows
#[cfg(target_os = "windows")]
const CREATE_NO_WINDOW: u32 = 0x08000000;

/// Execute a command silently (no window flash on Windows)
#[cfg(target_os = "windows")]
pub fn silent_command(program: &str) -> std::process::Command {
    let mut cmd = std::process::Command::new(program);
    cmd.creation_flags(CREATE_NO_WINDOW);
    cmd
}

/// Execute a command silently (async version, no window flash on Windows)
#[cfg(target_os = "windows")]
pub fn silent_tokio_command(program: &str) -> tokio::process::Command {
    let mut cmd = tokio::process::Command::new(program);
    cmd.creation_flags(CREATE_NO_WINDOW);
    cmd
}

/// Placeholder for non-Windows platforms
#[cfg(not(target_os = "windows"))]
pub fn silent_command(program: &str) -> std::process::Command {
    std::process::Command::new(program)
}

/// Placeholder for non-Windows platforms (async)
#[cfg(not(target_os = "windows"))]
pub fn silent_tokio_command(program: &str) -> tokio::process::Command {
    tokio::process::Command::new(program)
}

/// Open a URL in the default browser (cross-platform, silent on Windows)
pub fn open_url(url: &str) -> Result<(), std::io::Error> {
    #[cfg(target_os = "windows")]
    {
        silent_command("rundll32")
            .args(["url.dll,FileProtocolHandler", url])
            .spawn()?;
    }

    #[cfg(target_os = "linux")]
    {
        std::process::Command::new("xdg-open")
            .arg(url)
            .spawn()?;
    }

    #[cfg(target_os = "macos")]
    {
        std::process::Command::new("open")
            .arg(url)
            .spawn()?;
    }

    Ok(())
}

/// Open the agent config file in the default editor (cross-platform)
pub fn open_config() -> Result<(), std::io::Error> {
    let config_path = match crate::config::AgentConfig::config_file_path() {
        Ok(path) => path,
        Err(_) => return Err(std::io::Error::new(
            std::io::ErrorKind::NotFound,
            "Could not find config file path",
        )),
    };

    #[cfg(target_os = "windows")]
    {
        silent_command("notepad")
            .arg(&config_path)
            .spawn()?;
    }

    #[cfg(target_os = "linux")]
    {
        std::process::Command::new("xdg-open")
            .arg(&config_path)
            .spawn()?;
    }

    #[cfg(target_os = "macos")]
    {
        std::process::Command::new("open")
            .arg(&config_path)
            .spawn()?;
    }

    Ok(())
}
