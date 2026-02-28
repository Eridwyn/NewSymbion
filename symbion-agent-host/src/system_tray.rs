//! Simple system tray for Symbion Agent
//!
//! Provides a minimal system tray icon that opens the local dashboard
//! when clicked. Lightweight implementation without heavy dependencies.

use tracing::info;

use crate::windows_utils;

pub struct SystemTray {
    agent_id: String,
    hostname: String,
}

impl SystemTray {
    pub fn new() -> Self {
        Self {
            agent_id: String::new(),
            hostname: String::new(),
        }
    }

    pub fn initialize(&mut self, agent_id: &str, hostname: &str) -> Result<(), Box<dyn std::error::Error>> {
        info!("Initializing lightweight system tray for agent {}", agent_id);

        self.agent_id = agent_id.to_string();
        self.hostname = hostname.to_string();

        // Create a simple desktop entry that opens the dashboard
        self.create_desktop_entry()?;

        // Show initialization notification
        self.show_notification("Symbion Agent Started", &format!("Agent {} is running - dashboard available at http://localhost:9899", hostname))?;

        info!("System tray alternative initialized successfully");
        Ok(())
    }

    fn create_desktop_entry(&self) -> Result<(), Box<dyn std::error::Error>> {
        #[cfg(target_os = "linux")]
        {
            use std::fs;

            // Create desktop entry in user applications directory
            let desktop_dir = dirs::data_local_dir()
                .unwrap_or_else(|| std::path::PathBuf::from(".local/share"))
                .join("applications");

            fs::create_dir_all(&desktop_dir)?;

            let desktop_file = desktop_dir.join("symbion-agent-dashboard.desktop");
            let content = format!(
                r#"[Desktop Entry]
Version=1.0
Type=Application
Name=Symbion Agent Dashboard - {}
Comment=Open local dashboard for Symbion Agent
Exec=xdg-open http://localhost:9899
Icon=applications-internet
Terminal=false
Categories=System;Network;
"#, self.hostname
            );

            fs::write(&desktop_file, content)?;
            info!("Created desktop entry: {:?}", desktop_file);
        }

        Ok(())
    }

    fn show_notification(&self, title: &str, message: &str) -> Result<(), Box<dyn std::error::Error>> {
        #[cfg(target_os = "linux")]
        {
            std::process::Command::new("notify-send")
                .args(["--app-name=Symbion", title, message])
                .spawn()?;
        }

        #[cfg(target_os = "windows")]
        {
            windows_utils::silent_command("powershell")
                .args([
                    "-Command",
                    &format!(
                        "Add-Type -AssemblyName System.Windows.Forms; [System.Windows.Forms.MessageBox]::Show('{}', '{}')",
                        message, title
                    )
                ])
                .spawn()?;
        }

        #[cfg(target_os = "macos")]
        {
            std::process::Command::new("osascript")
                .args([
                    "-e",
                    &format!("display notification \"{}\" with title \"{}\"", message, title)
                ])
                .spawn()?;
        }

        Ok(())
    }

    pub fn show_dashboard_notification(&self) -> Result<(), Box<dyn std::error::Error>> {
        self.show_notification(
            "Symbion Agent",
            &format!("Click here to open dashboard\nAgent: {}", self.hostname)
        )?;

        // Also open dashboard directly
        Self::open_dashboard()?;

        Ok(())
    }

    fn open_dashboard() -> Result<(), std::io::Error> {
        windows_utils::open_url("http://localhost:9899")?;
        info!("Opened dashboard at http://localhost:9899");
        Ok(())
    }
}

impl Default for SystemTray {
    fn default() -> Self {
        Self::new()
    }
}
