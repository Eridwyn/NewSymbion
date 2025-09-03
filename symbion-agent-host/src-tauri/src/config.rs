//! Configuration management with secure storage
//!
//! Handles:
//! - MQTT broker settings
//! - Elevation credentials (encrypted)
//! - Auto-update preferences  
//! - Cross-platform storage

use anyhow::Result;
use serde::{Deserialize, Serialize};
use keyring::Entry;
use std::path::PathBuf;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentConfig {
    pub mqtt: MqttConfig,
    pub elevation: ElevationConfig,  
    pub update: UpdateConfig,
    pub agent: AgentInfo,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MqttConfig {
    pub broker_host: String,
    pub broker_port: u16,
    pub client_id: Option<String>,
    pub keep_alive_secs: u16,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ElevationConfig {
    pub store_credentials: bool,
    pub auto_elevate: bool,
    #[serde(skip)] // Never serialize passwords
    pub cached_password: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpdateConfig {
    pub auto_update: bool,
    pub channel: UpdateChannel,
    pub check_interval_hours: u32,
    pub github_repo: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentInfo {
    pub agent_id: String,
    pub hostname: String,
    pub version: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum UpdateChannel {
    Stable,
    Beta, 
    Dev,
}

impl Default for AgentConfig {
    fn default() -> Self {
        Self {
            mqtt: MqttConfig {
                broker_host: "127.0.0.1".to_string(),
                broker_port: 1883,
                client_id: None,
                keep_alive_secs: 60,
            },
            elevation: ElevationConfig {
                store_credentials: false,
                auto_elevate: false,
                cached_password: None,
            },
            update: UpdateConfig {
                auto_update: true,
                channel: UpdateChannel::Stable,
                check_interval_hours: 24,
                github_repo: "anthropics/NewSymbion".to_string(), // À ajuster
            },
            agent: AgentInfo {
                agent_id: uuid::Uuid::new_v4().to_string(),
                hostname: hostname::get().unwrap_or_default().to_string_lossy().to_string(),
                version: env!("CARGO_PKG_VERSION").to_string(),
            },
        }
    }
}

impl AgentConfig {
    /// Load config from OS-specific location
    pub async fn load() -> Result<Self> {
        let config_path = Self::config_file_path()?;
        
        if config_path.exists() {
            let content = tokio::fs::read_to_string(&config_path).await?;
            let mut config: AgentConfig = toml::from_str(&content)?;
            
            // Load password from secure keyring if enabled
            if config.elevation.store_credentials {
                config.elevation.cached_password = Self::load_password().ok();
            }
            
            Ok(config)
        } else {
            // First time setup - return default config
            Ok(Self::default())
        }
    }
    
    /// Save config to OS-specific location
    pub async fn save(&self) -> Result<()> {
        let config_path = Self::config_file_path()?;
        
        // Create parent directory if needed
        if let Some(parent) = config_path.parent() {
            tokio::fs::create_dir_all(parent).await?;
        }
        
        // Save config file (without sensitive data)
        let content = toml::to_string_pretty(self)?;
        tokio::fs::write(&config_path, content).await?;
        
        // Save password to secure keyring if enabled
        if self.elevation.store_credentials {
            if let Some(password) = &self.elevation.cached_password {
                Self::save_password(password)?;
            }
        }
        
        Ok(())
    }
    
    /// Get OS-specific config file path
    pub fn config_file_path() -> Result<PathBuf> {
        let mut path = dirs::config_dir()
            .ok_or_else(|| anyhow::anyhow!("Could not find config directory"))?;
        
        path.push("symbion-agent");
        path.push("config.toml");
        Ok(path)
    }
    
    /// Load password from secure OS keyring
    fn load_password() -> Result<String> {
        let entry = Entry::new("symbion-agent", "elevation-password")?;
        entry.get_password().map_err(Into::into)
    }
    
    /// Save password to secure OS keyring  
    fn save_password(password: &str) -> Result<()> {
        let entry = Entry::new("symbion-agent", "elevation-password")?;
        entry.set_password(password).map_err(Into::into)
    }
    
    /// Delete password from keyring
    pub fn delete_password() -> Result<()> {
        let entry = Entry::new("symbion-agent", "elevation-password")?;
        entry.delete_credential().map_err(Into::into)
    }
    
    /// Check if this is first-time setup
    pub fn is_first_time_setup() -> bool {
        Self::config_file_path()
            .map(|p| !p.exists())
            .unwrap_or(true)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[tokio::test]
    async fn test_default_config() {
        let config = AgentConfig::default();
        assert_eq!(config.mqtt.broker_port, 1883);
        assert_eq!(config.update.channel, UpdateChannel::Stable);
    }
    
    #[test] 
    fn test_config_file_path() {
        let path = AgentConfig::config_file_path().unwrap();
        assert!(path.to_string_lossy().contains("symbion-agent"));
        assert!(path.to_string_lossy().contains("config.toml"));
    }
}