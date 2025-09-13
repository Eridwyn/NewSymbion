#[cfg(feature = "gui")]
use tauri::{
    CustomMenuItem, Manager, SystemTray, SystemTrayEvent, 
    SystemTrayMenu, SystemTrayMenuItem, AppHandle, WindowEvent
};
use std::sync::Arc;
use crate::metrics::SystemMetrics;

#[cfg(feature = "gui")]
pub struct TrayManager {
    pub app_handle: Option<AppHandle>,
    pub metrics: Arc<SystemMetrics>,
}

#[cfg(feature = "gui")]
impl TrayManager {
    pub fn new(metrics: Arc<SystemMetrics>) -> Self {
        Self {
            app_handle: None,
            metrics,
        }
    }

    pub fn create_tray() -> SystemTray {
        let dashboard = CustomMenuItem::new("dashboard".to_string(), "📊 Dashboard");
        let logs = CustomMenuItem::new("logs".to_string(), "📝 View Logs");
        let reconnect = CustomMenuItem::new("reconnect".to_string(), "🔄 Reconnect");
        let main_pwa = CustomMenuItem::new("main_pwa".to_string(), "🌐 Open Main PWA");
        let quit = CustomMenuItem::new("quit".to_string(), "🚪 Quit");
        
        let tray_menu = SystemTrayMenu::new()
            .add_item(dashboard)
            .add_native_item(SystemTrayMenuItem::Separator)
            .add_item(main_pwa)
            .add_item(logs)
            .add_item(reconnect)
            .add_native_item(SystemTrayMenuItem::Separator)
            .add_item(quit);
        
        SystemTray::new().with_menu(tray_menu).with_tooltip("Symbion Agent")
    }

    pub fn handle_tray_event(&mut self, app: &AppHandle, event: SystemTrayEvent) {
        match event {
            SystemTrayEvent::LeftClick { .. } => {
                // Clic gauche : ouvre le dashboard
                self.show_dashboard_window(app);
            }
            SystemTrayEvent::MenuItemClick { id, .. } => {
                match id.as_str() {
                    "dashboard" => self.show_dashboard_window(app),
                    "logs" => self.open_logs(),
                    "reconnect" => self.reconnect(),
                    "main_pwa" => self.open_main_pwa(),
                    "quit" => {
                        println!("[tray] Quit requested via system tray");
                        std::process::exit(0);
                    }
                    _ => {}
                }
            }
            _ => {}
        }
    }

    fn show_dashboard_window(&mut self, app: &AppHandle) {
        if let Some(window) = app.get_window("main") {
            let _ = window.show();
            let _ = window.set_focus();
        }
    }

    fn open_logs(&self) {
        // Ouvrir les logs dans l'éditeur par défaut
        let log_path = "/var/log/symbion/agent.log";
        if std::path::Path::new(log_path).exists() {
            let _ = std::process::Command::new("xdg-open")
                .arg(log_path)
                .spawn();
        } else {
            println!("[tray] Log file not found at {}", log_path);
        }
    }

    fn reconnect(&self) {
        // TODO: Signal l'agent principal pour reconnecter MQTT
        println!("[tray] Reconnect requested - signaling agent...");
    }

    fn open_main_pwa(&self) {
        // Ouvrir le PWA principal
        let pwa_url = "http://localhost:3000";
        let _ = std::process::Command::new("xdg-open")
            .arg(pwa_url)
            .spawn();
    }

    pub fn update_tray_status(&self, app: &AppHandle, connected: bool) {
        // Mettre à jour l'icône selon le statut
        let status = if connected { "Connected" } else { "Disconnected" };
        let tooltip = format!("Symbion Agent - {}", status);
        
        if let Err(e) = app.tray_handle().set_tooltip(&tooltip) {
            eprintln!("[tray] Failed to update tooltip: {}", e);
        }
    }
}

#[cfg(feature = "gui")]
pub fn handle_window_event(_event: &WindowEvent) {
    // Gérer les événements de fenêtre si nécessaire
}

#[cfg(not(feature = "gui"))]
pub struct TrayManager;

#[cfg(not(feature = "gui"))]
impl TrayManager {
    pub fn new(_metrics: Arc<crate::metrics::SystemMetrics>) -> Self {
        Self
    }
}