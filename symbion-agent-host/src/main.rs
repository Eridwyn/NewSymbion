//! Symbion Agent Host - Multi-OS system agent for network control
//!
//! This agent provides remote system control capabilities to the Symbion kernel:
//! - Auto-discovery and registration via MQTT
//! - System metrics monitoring and reporting
//! - Remote command execution (shutdown, reboot, process control)
//! - Cross-platform support (Linux, Windows, Android)

// Hide console window on Windows when using GUI mode
#![cfg_attr(all(target_os = "windows", feature = "gui"), windows_subsystem = "windows")]

mod agent;
mod discovery;
mod capabilities;
mod messages;
mod metrics;
mod mqtt_client;
mod execution;
mod config;
mod file_transfer;
mod log_collector;
mod plugins;
mod scheduler;
mod updater;
mod wizard;
mod local_api;
mod system_tray;
mod transport;
mod watchdog;
mod windows_utils;

#[cfg(feature = "gui")]
mod gui;

use anyhow::{Result, Context};
use std::sync::Arc;
use tracing::{info, error, warn};

#[tokio::main]
async fn main() -> Result<()> {
    // Initialize logging
    tracing_subscriber::fmt().init();

    info!("Symbion Agent Host v{} starting...", env!("CARGO_PKG_VERSION"));

    // First-time setup wizard
    if config::AgentConfig::is_first_time_setup() {
        println!("First-time setup detected!");
        println!("Starting interactive configuration wizard...");

        if let Err(e) = wizard::SetupWizard::run().await {
            eprintln!("Setup wizard failed: {}", e);
            if let Ok(config_path) = config::AgentConfig::config_file_path() {
                eprintln!("Please create configuration manually at: {}", config_path.display());
            }
            return Err(anyhow::anyhow!("Configuration setup failed"));
        }

        if config::AgentConfig::is_first_time_setup() {
            return Err(anyhow::anyhow!("Configuration was not completed"));
        }

        println!("Configuration completed! Starting agent...");
    }

    // Load configuration
    let agent_config = config::AgentConfig::load().await
        .context("Failed to load agent configuration")?;

    // Save broker host for GUI before moving agent_config
    let broker_host_for_gui = agent_config.mqtt.broker_host.clone();

    info!("Configuration loaded: MQTT broker at {}:{}",
          agent_config.mqtt.broker_host, agent_config.mqtt.broker_port);

    // Check for updates if enabled
    if agent_config.update.auto_update {
        run_update_check(&agent_config).await;
    }

    // Start local API server
    let system_info = discovery::SystemInfo::discover().await
        .context("Failed to discover system info")?;

    let (reconnect_tx, reconnect_rx) = tokio::sync::mpsc::channel::<()>(1);
    let local_api = Arc::new(local_api::LocalApiServer::new(
        system_info.agent_id.clone(),
        system_info.hostname.clone(),
        reconnect_tx,
    ));

    let local_api_clone = local_api.clone();
    tokio::spawn(async move {
        if let Err(e) = local_api_clone.start().await {
            eprintln!("[local-api] Server failed: {}", e);
        }
    });

    // Discover system info for GUI
    let system_info_for_gui = discovery::SystemInfo::discover().await
        .context("Failed to discover system info for GUI")?;

    // Create agent
    let mut agent = agent::Agent::new_with_config(agent_config).await
        .context("Failed to create agent")?;

    agent.set_local_api(local_api);
    agent.set_reconnect_rx(reconnect_rx);

    // Run with GUI or terminal mode
    run_agent(agent, broker_host_for_gui, system_info_for_gui).await
}

/// Run update check and schedule background checker
async fn run_update_check(agent_config: &config::AgentConfig) {
    info!("Auto-update enabled, checking for updates...");
    let updater = updater::AgentUpdater::new(agent_config.clone());

    match updater.check_update().await {
        Ok(update_info) => {
            if update_info.is_update_available {
                info!("Update available: {} -> {}",
                      update_info.current_version, update_info.latest_version);
                if update_info.is_critical {
                    warn!("Critical update detected, performing automatic update...");
                    if let Err(e) = updater.perform_update(&update_info).await {
                        error!("Auto-update failed: {}", e);
                    }
                }
            } else {
                info!("Agent is up to date ({})", update_info.current_version);
            }
        }
        Err(e) => {
            warn!("Failed to check for updates: {}", e);
        }
    }

    // Background update checker
    let updater_clone = updater.clone();
    tokio::spawn(async move {
        if let Err(e) = updater_clone.schedule_check().await {
            error!("Background update checker failed: {}", e);
        }
    });
}

/// Run agent in GUI or terminal mode based on platform and features
async fn run_agent(
    mut agent: agent::Agent,
    broker_host_for_gui: String,
    system_info_for_gui: discovery::SystemInfo,
) -> Result<()> {
    #[cfg(feature = "gui")]
    {
        // Check if GUI is available on Linux
        #[cfg(target_os = "linux")]
        let gui_available = {
            std::env::var("DISPLAY").ok().filter(|s| !s.is_empty()).is_some()
            || std::env::var("WAYLAND_DISPLAY").ok().filter(|s| !s.is_empty()).is_some()
        };

        #[cfg(not(target_os = "linux"))]
        let gui_available = true;

        if gui_available {
            info!("Starting in GUI mode with system tray");

            #[cfg(target_os = "windows")]
            {
                std::thread::spawn(move || {
                    let runtime = tokio::runtime::Runtime::new().unwrap();
                    runtime.block_on(async move {
                        if let Err(e) = agent.run().await {
                            error!("Agent execution failed: {}", e);
                        }
                    });
                });

                let gui = gui::SymbionGui::new(broker_host_for_gui);
                gui.run(system_info_for_gui.agent_id, system_info_for_gui.hostname);
            }

            #[cfg(not(target_os = "windows"))]
            {
                let system_info_clone = system_info_for_gui.clone();
                let broker_host_clone = broker_host_for_gui.clone();

                std::thread::spawn(move || {
                    let gui = gui::SymbionGui::new(broker_host_clone);
                    gui.run(system_info_clone.agent_id, system_info_clone.hostname);
                });

                agent.run().await.context("Agent execution failed")?;
            }
        } else {
            info!("No graphical environment detected, starting in terminal mode");
            let _ = agent.init_system_tray();
            agent.run().await.context("Agent execution failed")?;
        }
    }

    #[cfg(not(feature = "gui"))]
    {
        let _ = (broker_host_for_gui, system_info_for_gui); // suppress unused warnings
        info!("Starting in terminal mode");
        let _ = agent.init_system_tray();
        agent.run().await.context("Agent execution failed")?;
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use crate::discovery::SystemInfo;

    #[tokio::test]
    async fn test_system_discovery() {
        let system_info = SystemInfo::discover().await.unwrap();
        assert!(!system_info.agent_id.is_empty());
        assert!(!system_info.hostname.is_empty());
        assert!(!system_info.network.interfaces.is_empty());
    }
}
