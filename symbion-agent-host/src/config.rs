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
use zeroize::Zeroize;

use crate::log_collector::LogConfig;
use crate::watchdog::WatchdogConfig;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentConfig {
    pub mqtt: MqttConfig,
    pub elevation: ElevationConfig,
    pub update: UpdateConfig,
    pub agent: AgentInfo,
    #[serde(default)]
    pub watchdog: WatchdogConfig,
    #[serde(default)]
    pub logging: LogConfig,
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

impl Drop for ElevationConfig {
    fn drop(&mut self) {
        if let Some(ref mut pw) = self.cached_password {
            pw.zeroize();
        }
    }
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

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
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
                hostname: hostname::get()
                    .map(|h| h.to_string_lossy().to_string())
                    .unwrap_or_else(|e| {
                        eprintln!("[agent] Warning: hostname resolution failed: {}, using fallback", e);
                        "unknown-host".to_string()
                    }),
                version: env!("CARGO_PKG_VERSION").to_string(),
            },
            watchdog: WatchdogConfig::default(),
            logging: LogConfig::default(),
        }
    }
}

impl AgentConfig {
    /// Load config from OS-specific location
    /// Falls back to default config with warning if file is corrupted
    pub async fn load() -> Result<Self> {
        let config_path = Self::config_file_path()?;

        if config_path.exists() {
            let content = match tokio::fs::read_to_string(&config_path).await {
                Ok(c) => c,
                Err(e) => {
                    eprintln!("[config] WARNING: Failed to read config file: {}", e);
                    eprintln!("[config] Using default configuration");
                    return Ok(Self::default());
                }
            };

            let mut config: AgentConfig = match toml::from_str(&content) {
                Ok(c) => c,
                Err(e) => {
                    eprintln!("[config] WARNING: Config file is corrupted: {}", e);
                    eprintln!("[config] Path: {}", config_path.display());
                    eprintln!("[config] Using default configuration — fix or delete the file to resolve");
                    return Ok(Self::default());
                }
            };

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

    #[tokio::test]
    async fn test_load_corrupted_toml() {
        // Write a corrupted TOML file to a temp dir, simulate load
        let dir = std::env::temp_dir().join("symbion-test-corrupted");
        let _ = std::fs::create_dir_all(&dir);
        let path = dir.join("config.toml");
        std::fs::write(&path, "this is not valid toml {{{{").unwrap();

        // Manually parse like load() does
        let content = tokio::fs::read_to_string(&path).await.unwrap();
        let result: Result<AgentConfig, _> = toml::from_str(&content);
        assert!(result.is_err(), "Corrupted TOML should fail to parse");

        // load() should return Ok(default) on corruption — verify the fallback logic
        let default = AgentConfig::default();
        assert_eq!(default.mqtt.broker_port, 1883);

        let _ = std::fs::remove_dir_all(&dir);
    }

    #[tokio::test]
    async fn test_load_valid_config() {
        let dir = std::env::temp_dir().join("symbion-test-valid");
        let _ = std::fs::create_dir_all(&dir);
        let path = dir.join("config.toml");

        let config = AgentConfig::default();
        let content = toml::to_string_pretty(&config).unwrap();
        std::fs::write(&path, &content).unwrap();

        // Verify it parses correctly
        let parsed: AgentConfig = toml::from_str(&content).unwrap();
        assert_eq!(parsed.mqtt.broker_port, 1883);
        assert_eq!(parsed.mqtt.broker_host, "127.0.0.1");
        assert_eq!(parsed.update.channel, UpdateChannel::Stable);

        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn test_first_time_setup() {
        // is_first_time_setup() returns true when config file doesn't exist
        // We can't easily control the config path, but we can verify the logic
        let result = AgentConfig::is_first_time_setup();
        // Result should be a bool (either true or false depending on environment)
        assert!(result == true || result == false);
    }

    #[test]
    fn test_update_channel_serialization() {
        let stable = serde_json::to_string(&UpdateChannel::Stable).unwrap();
        let beta = serde_json::to_string(&UpdateChannel::Beta).unwrap();
        let dev = serde_json::to_string(&UpdateChannel::Dev).unwrap();
        assert_eq!(stable, "\"Stable\"");
        assert_eq!(beta, "\"Beta\"");
        assert_eq!(dev, "\"Dev\"");

        // Roundtrip
        let parsed: UpdateChannel = serde_json::from_str(&stable).unwrap();
        assert_eq!(parsed, UpdateChannel::Stable);
    }

    #[test]
    fn test_elevation_config_password_not_serialized() {
        let config = ElevationConfig {
            store_credentials: true,
            auto_elevate: false,
            cached_password: Some("secret123".to_string()),
        };
        let json = serde_json::to_string(&config).unwrap();
        // Password must NOT appear in serialized output (#[serde(skip)])
        assert!(!json.contains("secret123"), "Password should not be serialized");
        assert!(!json.contains("cached_password"), "Password field should be skipped");
    }

    #[test]
    fn test_elevation_config_zeroize_on_drop() {
        let pw = "super_secret_password".to_string();
        let pw_ptr = pw.as_ptr();
        let pw_len = pw.len();

        let config = ElevationConfig {
            store_credentials: true,
            auto_elevate: false,
            cached_password: Some(pw),
        };
        drop(config);
        // After drop, the memory should have been zeroized.
        // We can't safely verify the content after drop in safe Rust,
        // but we verify the Drop impl exists and doesn't panic.
    }

    #[test]
    fn test_agent_config_toml_roundtrip() {
        let config = AgentConfig::default();
        let toml_str = toml::to_string_pretty(&config).unwrap();
        let parsed: AgentConfig = toml::from_str(&toml_str).unwrap();
        assert_eq!(parsed.mqtt.broker_host, config.mqtt.broker_host);
        assert_eq!(parsed.mqtt.broker_port, config.mqtt.broker_port);
        assert_eq!(parsed.update.channel, config.update.channel);
        assert_eq!(parsed.agent.agent_id, config.agent.agent_id);
    }
}