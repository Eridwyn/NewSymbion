//! Interactive CLI setup wizard for first-time configuration
//! 
//! This module provides a user-friendly command-line interface for configuring
//! the Symbion agent on first launch, including MQTT broker settings,
//! privilege elevation, and auto-update preferences.

use anyhow::{Result, Context};
use std::io::{self, Write};
use crate::config::{AgentConfig, MqttConfig, ElevationConfig, UpdateConfig, UpdateChannel, AgentInfo};

pub struct SetupWizard;

impl SetupWizard {
    /// Run the interactive setup wizard
    pub async fn run() -> Result<()> {
        println!();
        println!("🤖 ======================================");
        println!("   SYMBION AGENT CONFIGURATION WIZARD");
        println!("🤖 ======================================");
        println!();
        
        // Display system information
        Self::display_system_info().await?;
        
        // Step 1: MQTT Configuration
        let mqtt_config = Self::configure_mqtt().await?;
        
        // Step 2: Elevation Configuration
        let elevation_config = Self::configure_elevation().await?;
        
        // Step 3: Update Configuration
        let update_config = Self::configure_updates().await?;
        
        // Step 4: Agent Information
        let agent_config = Self::configure_agent().await?;
        
        // Create and save final configuration
        let config = AgentConfig {
            mqtt: mqtt_config,
            elevation: elevation_config,
            update: update_config,
            agent: agent_config,
        };
        
        // Display summary and confirm
        Self::display_summary(&config).await?;
        
        if Self::confirm_save()? {
            config.save().await
                .context("Failed to save configuration")?;
                
            println!();
            println!("✅ Configuration saved successfully!");
            println!("🚀 The Symbion agent is now ready to start.");
            println!();
        } else {
            println!("❌ Configuration cancelled.");
            return Ok(());
        }
        
        Ok(())
    }
    
    async fn display_system_info() -> Result<()> {
        use crate::discovery::SystemInfo;
        
        println!("📋 SYSTEM INFORMATION");
        println!("────────────────────────────────────────");
        
        let system_info = SystemInfo::discover().await
            .context("Failed to discover system information")?;
            
        println!("🖥️  Hostname: {}", system_info.hostname);
        println!("🔧 OS: {} ({})", system_info.os, system_info.architecture);
        println!("🌐 Agent ID: {}", system_info.agent_id);
        println!("📍 Primary MAC: {}", system_info.network.primary_mac);
        println!();
        
        Ok(())
    }
    
    async fn configure_mqtt() -> Result<MqttConfig> {
        println!("📡 MQTT CONFIGURATION");
        println!("────────────────────────────────────────");
        println!("Configure connection to the Symbion kernel MQTT broker.");
        println!();
        
        let broker_host = Self::prompt_with_default(
            "MQTT Broker Host", 
            "127.0.0.1"
        )?;
        
        let broker_port: u16 = Self::prompt_with_default_parse(
            "MQTT Broker Port",
            "1883"
        )?;
        
        let client_id = Self::prompt_optional("Client ID (leave empty for auto-generation)")?;
        
        // Test connection
        println!("🔍 Testing MQTT connection...");
        match Self::test_mqtt_connection(&broker_host, broker_port).await {
            Ok(true) => println!("✅ Connection successful!"),
            Ok(false) => println!("⚠️  Connection failed, but configuration will be saved."),
            Err(e) => println!("⚠️  Connection test error: {} - Configuration will be saved anyway.", e),
        }
        
        println!();
        
        Ok(MqttConfig {
            broker_host,
            broker_port,
            client_id,
            keep_alive_secs: 60,
        })
    }
    
    async fn configure_elevation() -> Result<ElevationConfig> {
        println!("🔐 SYSTEM PRIVILEGES");
        println!("────────────────────────────────────────");
        println!("Configure privilege elevation for system operations (shutdown, kill processes, etc.)");
        println!();
        
        let store_credentials = Self::prompt_yes_no(
            "Store system credentials securely in OS keyring?", 
            false
        )?;
        
        let auto_elevate = Self::prompt_yes_no(
            "Enable automatic privilege elevation for system commands?", 
            false
        )?;
        
        let cached_password = if store_credentials {
            println!("⚠️  Note: Password will be encrypted and stored in OS keyring (Keychain/Credential Manager)");
            Self::prompt_password("System password for privilege elevation")?
        } else {
            None
        };
        
        println!();
        
        Ok(ElevationConfig {
            store_credentials,
            auto_elevate,
            cached_password,
        })
    }
    
    async fn configure_updates() -> Result<UpdateConfig> {
        println!("🔄 AUTO-UPDATES");
        println!("────────────────────────────────────────");
        println!("Configure automatic updates from GitHub releases.");
        println!();
        
        let auto_update = Self::prompt_yes_no(
            "Enable automatic updates?", 
            true
        )?;
        
        let channel = if auto_update {
            let channel_str = Self::prompt_with_options(
                "Update channel",
                &[("stable", "Stable releases only"), ("beta", "Beta releases"), ("dev", "Development builds")],
                "stable"
            )?;
            
            match channel_str.as_str() {
                "stable" => UpdateChannel::Stable,
                "beta" => UpdateChannel::Beta,
                "dev" => UpdateChannel::Dev,
                _ => UpdateChannel::Stable,
            }
        } else {
            UpdateChannel::Stable
        };
        
        let check_interval_hours = if auto_update {
            Self::prompt_with_default_parse(
                "Check interval (hours)",
                "24"
            )?
        } else {
            24
        };
        
        let github_repo = Self::prompt_with_default(
            "GitHub repository (owner/repository)",
            "eridwyn/NewSymbion"
        )?;
        
        println!();
        
        Ok(UpdateConfig {
            auto_update,
            channel,
            check_interval_hours,
            github_repo,
        })
    }
    
    async fn configure_agent() -> Result<AgentInfo> {
        println!("🤖 AGENT INFORMATION");
        println!("────────────────────────────────────────");
        
        let agent_id = Self::prompt_optional("Custom Agent ID (leave empty for MAC-based generation)")?;
        let hostname = Self::prompt_optional("Custom Hostname (leave empty for system hostname)")?;
        
        println!();
        
        Ok(AgentInfo {
            agent_id: agent_id.unwrap_or_else(|| "auto".to_string()),
            hostname: hostname.unwrap_or_else(|| "auto".to_string()),
            version: "1.0.0".to_string(),
        })
    }
    
    async fn display_summary(config: &AgentConfig) -> Result<()> {
        println!("📋 CONFIGURATION SUMMARY");
        println!("────────────────────────────────────────");
        
        println!("📡 MQTT:");
        println!("   Broker: {}:{}", config.mqtt.broker_host, config.mqtt.broker_port);
        println!("   Client ID: {}", config.mqtt.client_id.as_deref().unwrap_or("Auto-generated"));
        
        println!();
        println!("🔐 Privileges:");
        println!("   Store credentials: {}", if config.elevation.store_credentials { "✅ Yes" } else { "❌ No" });
        println!("   Auto-elevate: {}", if config.elevation.auto_elevate { "✅ Yes" } else { "❌ No" });
        
        println!();
        println!("🔄 Updates:");
        println!("   Auto-update: {}", if config.update.auto_update { "✅ Enabled" } else { "❌ Disabled" });
        println!("   Channel: {:?}", config.update.channel);
        println!("   Check interval: {}h", config.update.check_interval_hours);
        println!("   Repository: {}", config.update.github_repo);
        
        println!();
        println!("🤖 Agent:");
        println!("   Agent ID: {}", if config.agent.agent_id == "auto" { "Auto-generated" } else { &config.agent.agent_id });
        println!("   Hostname: {}", if config.agent.hostname == "auto" { "System hostname" } else { &config.agent.hostname });
        
        println!();
        
        Ok(())
    }
    
    fn confirm_save() -> Result<bool> {
        print!("💾 Save this configuration? [Y/n]: ");
        io::stdout().flush()?;
        
        let mut input = String::new();
        io::stdin().read_line(&mut input)?;
        let input = input.trim().to_lowercase();
        
        Ok(input.is_empty() || input == "y" || input == "yes")
    }
    
    // Helper functions for user input
    fn prompt_with_default(prompt: &str, default: &str) -> Result<String> {
        print!("❓ {} [{}]: ", prompt, default);
        io::stdout().flush()?;
        
        let mut input = String::new();
        io::stdin().read_line(&mut input)?;
        let input = input.trim();
        
        if input.is_empty() {
            Ok(default.to_string())
        } else {
            Ok(input.to_string())
        }
    }
    
    fn prompt_with_default_parse<T: std::str::FromStr>(prompt: &str, default: &str) -> Result<T>
    where
        T::Err: std::fmt::Display,
    {
        loop {
            let input = Self::prompt_with_default(prompt, default)?;
            match input.parse::<T>() {
                Ok(value) => return Ok(value),
                Err(e) => {
                    println!("❌ Invalid input: {}. Please try again.", e);
                    continue;
                }
            }
        }
    }
    
    fn prompt_optional(prompt: &str) -> Result<Option<String>> {
        print!("❓ {}: ", prompt);
        io::stdout().flush()?;
        
        let mut input = String::new();
        io::stdin().read_line(&mut input)?;
        let input = input.trim();
        
        if input.is_empty() {
            Ok(None)
        } else {
            Ok(Some(input.to_string()))
        }
    }
    
    fn prompt_yes_no(prompt: &str, default: bool) -> Result<bool> {
        let default_str = if default { "Y/n" } else { "y/N" };
        
        loop {
            print!("❓ {} [{}]: ", prompt, default_str);
            io::stdout().flush()?;
            
            let mut input = String::new();
            io::stdin().read_line(&mut input)?;
            let input = input.trim().to_lowercase();
            
            if input.is_empty() {
                return Ok(default);
            }
            
            match input.as_str() {
                "y" | "yes" => return Ok(true),
                "n" | "no" => return Ok(false),
                _ => {
                    println!("❌ Please enter 'y' or 'n'.");
                    continue;
                }
            }
        }
    }
    
    fn prompt_with_options(prompt: &str, options: &[(&str, &str)], default: &str) -> Result<String> {
        println!("❓ {}:", prompt);
        for (key, description) in options {
            let marker = if *key == default { "►" } else { " " };
            println!("  {} {} - {}", marker, key, description);
        }
        
        loop {
            print!("Choice [{}]: ", default);
            io::stdout().flush()?;
            
            let mut input = String::new();
            io::stdin().read_line(&mut input)?;
            let input = input.trim();
            
            if input.is_empty() {
                return Ok(default.to_string());
            }
            
            if options.iter().any(|(key, _)| *key == input) {
                return Ok(input.to_string());
            }
            
            println!("❌ Invalid choice. Please select from the available options.");
        }
    }
    
    fn prompt_password(prompt: &str) -> Result<Option<String>> {
        // Note: For security, we should use a proper password input library in production
        // For now, we'll use regular input with a warning
        println!("⚠️  WARNING: Password input will be visible on screen.");
        print!("🔐 {}: ", prompt);
        io::stdout().flush()?;
        
        let mut input = String::new();
        io::stdin().read_line(&mut input)?;
        let input = input.trim();
        
        if input.is_empty() {
            Ok(None)
        } else {
            Ok(Some(input.to_string()))
        }
    }
    
    async fn test_mqtt_connection(host: &str, port: u16) -> Result<bool> {
        use std::time::Duration;
        
        let address = format!("{}:{}", host, port);
        match std::net::TcpStream::connect_timeout(
            &address.parse()?,
            Duration::from_secs(5)
        ) {
            Ok(_) => Ok(true),
            Err(_) => Ok(false),
        }
    }
}