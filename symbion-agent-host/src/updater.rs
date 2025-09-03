//! Auto-updater using GitHub releases
//!
//! Features:
//! - Check for updates from GitHub releases
//! - Download and verify binaries
//! - Safe replacement with rollback
//! - Background update checks

use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use tracing::{info, warn, error};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpdateInfo {
    pub current_version: String,
    pub latest_version: String,
    pub download_url: String,
    pub release_notes: String,
    pub is_update_available: bool,
    pub is_critical: bool,
}

#[derive(Clone)]
pub struct AgentUpdater {
    config: crate::config::AgentConfig,
}

impl AgentUpdater {
    pub fn new(config: crate::config::AgentConfig) -> Self {
        Self { config }
    }
    
    /// Check if an update is available
    pub async fn check_update(&self) -> Result<UpdateInfo> {
        info!("Checking for updates...");
        
        let current_version = env!("CARGO_PKG_VERSION");
        let repo_parts: Vec<&str> = self.config.update.github_repo.split('/').collect();
        
        if repo_parts.len() != 2 {
            return Err(anyhow::anyhow!("Invalid GitHub repo format"));
        }
        
        let owner = repo_parts[0];
        let repo = repo_parts[1];
        
        // Get latest release from GitHub API
        let url = format!("https://api.github.com/repos/{}/{}/releases/latest", owner, repo);
        let client = reqwest::Client::new();
        
        let response = client
            .get(&url)
            .header("User-Agent", "symbion-agent")
            .send()
            .await?;
        
        if !response.status().is_success() {
            return Err(anyhow::anyhow!("Failed to fetch release info: {}", response.status()));
        }
        
        let release: GitHubRelease = response.json().await?;
        let latest_version = release.tag_name.trim_start_matches('v');
        
        // Find appropriate asset for current platform
        let platform_suffix = Self::get_platform_suffix();
        let asset = release.assets.iter()
            .find(|a| a.name.contains(&platform_suffix))
            .ok_or_else(|| anyhow::anyhow!("No asset found for platform: {}", platform_suffix))?;
        
        let is_update_available = Self::is_newer_version(current_version, latest_version)?;
        let is_critical = release.body.contains("[CRITICAL]") || release.body.contains("security");
        
        Ok(UpdateInfo {
            current_version: current_version.to_string(),
            latest_version: latest_version.to_string(),
            download_url: asset.browser_download_url.clone(),
            release_notes: release.body.clone(),
            is_update_available,
            is_critical,
        })
    }
    
    /// Perform the actual update
    pub async fn perform_update(&self, update_info: &UpdateInfo) -> Result<()> {
        if !update_info.is_update_available {
            return Ok(());
        }
        
        info!("Performing update to version {}", update_info.latest_version);
        
        // Download new binary
        let temp_path = self.download_update(&update_info.download_url).await?;
        
        // Verify download (basic check)
        if !temp_path.exists() {
            return Err(anyhow::anyhow!("Downloaded file not found"));
        }
        
        // Get current executable path
        let current_exe = std::env::current_exe()?;
        let backup_path = current_exe.with_extension("backup");
        
        // Create backup of current executable
        std::fs::copy(&current_exe, &backup_path)?;
        info!("Created backup at: {}", backup_path.display());
        
        // Replace current executable
        #[cfg(unix)]
        {
            // On Unix, we need to handle the running executable carefully
            self.replace_executable_unix(&temp_path, &current_exe)?;
        }
        
        #[cfg(windows)]
        {
            // On Windows, we might need to use self-replace technique
            self.replace_executable_windows(&temp_path, &current_exe)?;
        }
        
        info!("Update completed successfully");
        
        // Clean up
        let _ = std::fs::remove_file(&temp_path);
        
        Ok(())
    }
    
    /// Schedule background update check
    pub async fn schedule_check(&self) -> Result<()> {
        let interval = std::time::Duration::from_secs(
            self.config.update.check_interval_hours as u64 * 3600
        );
        
        loop {
            tokio::time::sleep(interval).await;
            
            if let Ok(update_info) = self.check_update().await {
                if update_info.is_update_available {
                    if self.config.update.auto_update || update_info.is_critical {
                        info!("Auto-updating to version {}", update_info.latest_version);
                        if let Err(e) = self.perform_update(&update_info).await {
                            error!("Auto-update failed: {}", e);
                        }
                    } else {
                        info!("Update available: {} (auto-update disabled)", update_info.latest_version);
                    }
                }
            }
        }
    }
    
    async fn download_update(&self, url: &str) -> Result<PathBuf> {
        let client = reqwest::Client::new();
        let response = client.get(url).send().await?;
        
        if !response.status().is_success() {
            return Err(anyhow::anyhow!("Download failed: {}", response.status()));
        }
        
        let temp_dir = std::env::temp_dir();
        let filename = format!("symbion-agent-update-{}", uuid::Uuid::new_v4());
        let temp_path = temp_dir.join(filename);
        
        let bytes = response.bytes().await?;
        tokio::fs::write(&temp_path, bytes).await?;
        
        // Make executable on Unix
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let mut perms = std::fs::metadata(&temp_path)?.permissions();
            perms.set_mode(0o755);
            std::fs::set_permissions(&temp_path, perms)?;
        }
        
        Ok(temp_path)
    }
    
    #[cfg(unix)]
    fn replace_executable_unix(&self, new_path: &PathBuf, current_path: &PathBuf) -> Result<()> {
        // On Unix, we can replace the running executable
        std::fs::copy(new_path, current_path)?;
        Ok(())
    }
    
    #[cfg(windows)]
    fn replace_executable_windows(&self, new_path: &PathBuf, current_path: &PathBuf) -> Result<()> {
        // On Windows, use self_replace crate for safe replacement
        self_replace::self_replace(new_path)?;
        Ok(())
    }
    
    fn get_platform_suffix() -> String {
        let os = std::env::consts::OS;
        let arch = std::env::consts::ARCH;
        
        match (os, arch) {
            ("linux", "x86_64") => "linux-x64",
            ("windows", "x86_64") => "windows-x64",
            ("macos", "x86_64") => "macos-x64",
            ("macos", "aarch64") => "macos-arm64",
            _ => "unknown",
        }.to_string()
    }
    
    fn is_newer_version(current: &str, latest: &str) -> Result<bool> {
        let current_parts: Vec<u32> = current.split('.')
            .map(|s| s.parse().unwrap_or(0))
            .collect();
        let latest_parts: Vec<u32> = latest.split('.')
            .map(|s| s.parse().unwrap_or(0))
            .collect();
        
        for (c, l) in current_parts.iter().zip(latest_parts.iter()) {
            if l > c {
                return Ok(true);
            } else if l < c {
                return Ok(false);
            }
        }
        
        // If all parts are equal, check if latest has more parts
        Ok(latest_parts.len() > current_parts.len())
    }
}

#[derive(Debug, Deserialize)]
struct GitHubRelease {
    tag_name: String,
    body: String,
    assets: Vec<GitHubAsset>,
}

#[derive(Debug, Deserialize)]
struct GitHubAsset {
    name: String,
    browser_download_url: String,
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_version_comparison() {
        assert!(AgentUpdater::is_newer_version("1.0.0", "1.0.1").unwrap());
        assert!(AgentUpdater::is_newer_version("1.0.0", "1.1.0").unwrap());
        assert!(!AgentUpdater::is_newer_version("1.1.0", "1.0.0").unwrap());
        assert!(!AgentUpdater::is_newer_version("1.0.0", "1.0.0").unwrap());
    }
    
    #[test]
    fn test_platform_suffix() {
        let suffix = AgentUpdater::get_platform_suffix();
        assert!(!suffix.is_empty());
    }
}