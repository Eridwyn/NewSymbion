//! Windows GUI implementation with system tray and embedded webview
//!
//! Provides a native Windows application with:
//! - System tray icon with context menu
//! - Embedded WebView2 for local dashboard
//! - No terminal window (windowless background service)

#![cfg(feature = "gui")]

use anyhow::Result;
use std::sync::Arc;
use tray_icon::{
    TrayIconBuilder, TrayIconEvent,
    menu::{Menu, MenuItem, PredefinedMenuItem, MenuEvent},
};
use tao::{
    event::{Event, WindowEvent},
    event_loop::{ControlFlow, EventLoopBuilder},
    window::WindowBuilder,
};
use wry::WebViewBuilder;
use std::sync::atomic::{AtomicBool, Ordering};
use tracing::{info, error};

pub struct SymbionGui {
    window_visible: Arc<AtomicBool>,
}

impl SymbionGui {
    pub fn new() -> Self {
        Self {
            window_visible: Arc::new(AtomicBool::new(false)),
        }
    }

    /// Initialize and run the GUI event loop
    pub fn run(&self, agent_id: String, hostname: String) -> Result<()> {
        info!("Initializing GUI for agent: {} ({})", agent_id, hostname);

        // Create event loop for GUI
        let event_loop = EventLoopBuilder::new().build();

        // Create hidden window for WebView container
        let window = WindowBuilder::new()
            .with_title("Symbion Agent Dashboard")
            .with_inner_size(tao::dpi::LogicalSize::new(1024, 768))
            .with_visible(false) // Start hidden
            .build(&event_loop)?;

        // Create WebView for local dashboard
        let _webview = WebViewBuilder::new()
            .with_url("http://localhost:9899")
            .build(&window)?;

        info!("WebView created successfully");

        // Create system tray icon
        let tray_menu = self.create_tray_menu()?;
        let _tray_icon = self.create_tray_icon(tray_menu, &hostname)?;

        info!("System tray created successfully");

        let window_visible = self.window_visible.clone();

        // Handle menu events
        let menu_channel = MenuEvent::receiver();
        let tray_channel = TrayIconEvent::receiver();

        info!("Starting GUI event loop");

        // Run event loop
        event_loop.run(move |event, _, control_flow| {
            *control_flow = ControlFlow::Wait;

            // Handle tray icon events
            if let Ok(tray_event) = tray_channel.try_recv() {
                match tray_event {
                    tray_icon::TrayIconEvent::Click { .. } => {
                        // Toggle window visibility on tray icon click
                        let visible = !window_visible.load(Ordering::Relaxed);
                        window.set_visible(visible);
                        window_visible.store(visible, Ordering::Relaxed);

                        if visible {
                            let _ = window.set_focus();
                        }
                        info!("Tray icon clicked - window visible: {}", visible);
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
                } else if menu_id == "show" {
                    window.set_visible(true);
                    let _ = window.set_focus();
                    window_visible.store(true, Ordering::Relaxed);
                } else if menu_id == "hide" {
                    window.set_visible(false);
                    window_visible.store(false, Ordering::Relaxed);
                } else if menu_id == "open_pwa" {
                    let _ = open_browser("http://localhost:3001");
                }
            }

            // Handle window events
            match event {
                Event::WindowEvent {
                    event: WindowEvent::CloseRequested,
                    ..
                } => {
                    // Don't quit, just hide window
                    info!("Window close requested - hiding instead");
                    window.set_visible(false);
                    window_visible.store(false, Ordering::Relaxed);
                }
                Event::WindowEvent {
                    event: WindowEvent::Destroyed,
                    ..
                } => {
                    info!("Window destroyed - exiting");
                    *control_flow = ControlFlow::Exit;
                }
                _ => {}
            }
        });
    }

    /// Create system tray menu
    fn create_tray_menu(&self) -> Result<Menu> {
        use tray_icon::menu::MenuId;

        let menu = Menu::new();

        let show_item = MenuItem::with_id(MenuId::new("show"), "Afficher Dashboard", true, None);
        menu.append(&show_item)?;

        let hide_item = MenuItem::with_id(MenuId::new("hide"), "Masquer Dashboard", true, None);
        menu.append(&hide_item)?;

        menu.append(&PredefinedMenuItem::separator())?;

        let pwa_item = MenuItem::with_id(MenuId::new("open_pwa"), "Ouvrir Dashboard Principal", true, None);
        menu.append(&pwa_item)?;

        menu.append(&PredefinedMenuItem::separator())?;

        let quit_item = MenuItem::with_id(MenuId::new("quit"), "Quitter", true, None);
        menu.append(&quit_item)?;

        Ok(menu)
    }

    /// Create system tray icon
    fn create_tray_icon(&self, menu: Menu, hostname: &str) -> Result<tray_icon::TrayIcon> {
        // Load icon from embedded bytes (you'll need to add icon.ico to resources)
        let icon = self.load_icon()?;

        let tray = TrayIconBuilder::new()
            .with_menu(Box::new(menu))
            .with_tooltip(format!("Symbion Agent - {}", hostname))
            .with_icon(icon)
            .build()?;

        Ok(tray)
    }

    /// Load application icon
    fn load_icon(&self) -> Result<tray_icon::Icon> {
        // For now, create a simple colored icon programmatically
        // TODO: Replace with actual icon file
        let rgba = vec![255u8; 32 * 32 * 4]; // 32x32 white icon
        let icon = tray_icon::Icon::from_rgba(rgba, 32, 32)?;
        Ok(icon)
    }
}

impl Default for SymbionGui {
    fn default() -> Self {
        Self::new()
    }
}

/// Open URL in default browser
fn open_browser(url: &str) -> Result<(), std::io::Error> {
    #[cfg(target_os = "windows")]
    std::process::Command::new("rundll32")
        .args(&["url.dll,FileProtocolHandler", url])
        .spawn()?;

    #[cfg(target_os = "linux")]
    std::process::Command::new("xdg-open").arg(url).spawn()?;

    #[cfg(target_os = "macos")]
    std::process::Command::new("open").arg(url).spawn()?;

    Ok(())
}
