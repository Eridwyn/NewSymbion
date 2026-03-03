//! Cross-platform GUI implementation with system tray and embedded webview
//!
//! Provides a native application with:
//! - Borderless window with custom title bar and window controls
//! - System tray icon with context menu
//! - Embedded WebView for local dashboard (toggle with double-click)
//! - IPC bridge for drag, resize, minimize, maximize, close

#![cfg(feature = "gui")]

use tray_icon::{
    TrayIconBuilder, TrayIconEvent,
    menu::{Menu, MenuItem, PredefinedMenuItem, MenuEvent},
};
use tao::{
    event::{Event, WindowEvent},
    event_loop::{ControlFlow, EventLoopBuilder},
    window::{WindowBuilder, ResizeDirection},
    dpi::LogicalSize,
};
use wry::WebViewBuilder;
use tracing::{info, error, debug};
use std::sync::{Arc, Mutex};

use crate::windows_utils;

pub struct SymbionGui {
    broker_host: String,
}

struct AppState {
    window_visible: bool,
}

impl SymbionGui {
    pub fn new(broker_host: String) -> Self {
        Self { broker_host }
    }

    /// Initialize and run the GUI event loop
    /// This function never returns - it runs until the process exits
    pub fn run(self, agent_id: String, hostname: String) -> ! {
        info!("Initializing GUI for agent: {} ({})", agent_id, hostname);

        // Create event loop for GUI with window support
        let event_loop = EventLoopBuilder::new().build();

        // Create borderless window — custom title bar handled in HTML/CSS
        let window = Arc::new(
            WindowBuilder::new()
                .with_title(format!("Symbion Agent - {}", hostname))
                .with_inner_size(LogicalSize::new(420.0, 680.0))
                .with_resizable(true)
                .with_visible(false)
                .with_decorations(false)
                .with_transparent(true)
                .build(&event_loop)
                .expect("Failed to create window")
        );

        // Shared state — created before WebView so IPC handler can access it
        let state = Arc::new(Mutex::new(AppState {
            window_visible: false,
        }));

        // Clone references for IPC handler
        let window_for_ipc = window.clone();
        let state_for_ipc = state.clone();

        // Create WebView pointing to local API server (avoids NavigateToString
        // issues on Windows and gives a proper HTTP origin for fetch/localStorage)
        let _webview = WebViewBuilder::new()
            .with_url("http://127.0.0.1:9899/")
            .with_transparent(true)
            .with_ipc_handler(move |request| {
                let body = request.body();
                match body.as_str() {
                    "drag" => {
                        let _ = window_for_ipc.drag_window();
                    }
                    "minimize" => {
                        window_for_ipc.set_minimized(true);
                    }
                    "maximize" => {
                        let is_max = window_for_ipc.is_maximized();
                        window_for_ipc.set_maximized(!is_max);
                    }
                    "close" => {
                        window_for_ipc.set_visible(false);
                        if let Ok(mut s) = state_for_ipc.lock() {
                            s.window_visible = false;
                        }
                        info!("Dashboard window hidden via custom close button");
                    }
                    cmd if cmd.starts_with("resize:") => {
                        let dir = &cmd[7..];
                        let direction = match dir {
                            "North" => Some(ResizeDirection::North),
                            "South" => Some(ResizeDirection::South),
                            "East" => Some(ResizeDirection::East),
                            "West" => Some(ResizeDirection::West),
                            "NorthEast" => Some(ResizeDirection::NorthEast),
                            "NorthWest" => Some(ResizeDirection::NorthWest),
                            "SouthEast" => Some(ResizeDirection::SouthEast),
                            "SouthWest" => Some(ResizeDirection::SouthWest),
                            _ => None,
                        };
                        if let Some(d) = direction {
                            let _ = window_for_ipc.drag_resize_window(d);
                        }
                    }
                    _ => {
                        debug!("Unknown IPC message: {}", body);
                    }
                }
            })
            .build(&*window)
            .expect("Failed to build WebView");

        info!("WebView created with borderless embedded dashboard");

        // Create system tray icon
        let tray_menu = match self.create_tray_menu() {
            Ok(m) => m,
            Err(e) => {
                error!("Failed to create tray menu: {}", e);
                std::process::exit(1);
            }
        };

        let _tray_icon = match self.create_tray_icon(tray_menu, &hostname) {
            Ok(t) => t,
            Err(e) => {
                error!("Failed to create tray icon: {}", e);
                std::process::exit(1);
            }
        };

        info!("System tray created successfully");

        // Handle menu events
        let menu_channel = MenuEvent::receiver();
        let tray_channel = TrayIconEvent::receiver();

        info!("Starting GUI event loop with borderless WebView");

        // Clone for closures
        let _broker_host = self.broker_host.clone();
        let state_clone = state.clone();

        // Run event loop
        event_loop.run(move |event, _, control_flow| {
            *control_flow = ControlFlow::Wait;

            // Handle tray icon events
            if let Ok(tray_event) = tray_channel.try_recv() {
                match tray_event {
                    tray_icon::TrayIconEvent::DoubleClick { button, .. } => {
                        if button == tray_icon::MouseButton::Left {
                            let mut state = state_clone.lock().unwrap();
                            state.window_visible = !state.window_visible;
                            window.set_visible(state.window_visible);

                            if state.window_visible {
                                window.set_focus();
                                info!("Dashboard window shown");
                            } else {
                                info!("Dashboard window hidden");
                            }
                        }
                    }
                    _ => {}
                }
            }

            // Handle menu events
            if let Ok(menu_event) = menu_channel.try_recv() {
                let menu_id = menu_event.id.0.as_str();
                info!("Menu event: {}", menu_id);

                if menu_id == "quit" {
                    info!("Quit requested from tray menu");
                    *control_flow = ControlFlow::Exit;
                } else if menu_id == "toggle_dashboard" {
                    let mut state = state_clone.lock().unwrap();
                    state.window_visible = !state.window_visible;
                    window.set_visible(state.window_visible);
                    if state.window_visible {
                        window.set_focus();
                    }
                } else if menu_id == "open_pwa" {
                    let _ = windows_utils::open_url("https://symbion.markcha.fr");
                } else if menu_id == "open_config" {
                    let _ = windows_utils::open_config();
                } else if menu_id == "check_updates" {
                    let mut state = state_clone.lock().unwrap();
                    state.window_visible = true;
                    window.set_visible(true);
                    window.set_focus();
                    info!("Update check — showing dashboard");
                }
            }

            // Handle window events
            match event {
                Event::WindowEvent {
                    event: WindowEvent::CloseRequested,
                    ..
                } => {
                    window.set_visible(false);
                    let mut state = state_clone.lock().unwrap();
                    state.window_visible = false;
                    info!("Dashboard window hidden via close button");
                }
                _ => {}
            }
        });
    }

    /// Create system tray menu
    fn create_tray_menu(&self) -> Result<Menu, Box<dyn std::error::Error>> {
        use tray_icon::menu::MenuId;

        let menu = Menu::new();

        let toggle_item = MenuItem::with_id(MenuId::new("toggle_dashboard"), "Afficher/Masquer Dashboard", true, None);
        menu.append(&toggle_item)?;

        let pwa_item = MenuItem::with_id(MenuId::new("open_pwa"),
            "Dashboard Principal (symbion.markcha.fr)", true, None);
        menu.append(&pwa_item)?;

        menu.append(&PredefinedMenuItem::separator())?;

        let update_item = MenuItem::with_id(MenuId::new("check_updates"), "Vérifier les mises à jour", true, None);
        menu.append(&update_item)?;

        let config_item = MenuItem::with_id(MenuId::new("open_config"), "Configuration...", true, None);
        menu.append(&config_item)?;

        menu.append(&PredefinedMenuItem::separator())?;

        let quit_item = MenuItem::with_id(MenuId::new("quit"), "Quitter", true, None);
        menu.append(&quit_item)?;

        Ok(menu)
    }

    /// Create system tray icon
    fn create_tray_icon(&self, menu: Menu, hostname: &str) -> Result<tray_icon::TrayIcon, Box<dyn std::error::Error>> {
        let icon = Self::generate_icon()?;

        let tray = TrayIconBuilder::new()
            .with_menu(Box::new(menu))
            .with_tooltip(format!("Symbion Agent - {}", hostname))
            .with_icon(icon)
            .build()?;

        Ok(tray)
    }

    /// Load Symbion logo from embedded PNG asset and resize to 32x32 RGBA
    fn generate_icon() -> Result<tray_icon::Icon, Box<dyn std::error::Error>> {
        const ICON_SIZE: u32 = 32;
        let png_bytes = include_bytes!("../assets/tray-icon.png");

        let img = image::load_from_memory(png_bytes)?
            .resize_exact(ICON_SIZE, ICON_SIZE, image::imageops::FilterType::Lanczos3)
            .into_rgba8();

        let rgba = img.into_raw();
        let icon = tray_icon::Icon::from_rgba(rgba, ICON_SIZE, ICON_SIZE)?;
        Ok(icon)
    }
}

impl Default for SymbionGui {
    fn default() -> Self {
        Self::new("127.0.0.1".to_string())
    }
}
