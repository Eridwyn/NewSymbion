//! Cross-platform GUI implementation with system tray and embedded webview
//!
//! Provides a native application with:
//! - System tray icon with context menu
//! - Embedded WebView for local dashboard (toggle with left-click)
//! - No terminal window (windowless background service)

#![cfg(feature = "gui")]

use tray_icon::{
    TrayIconBuilder, TrayIconEvent,
    menu::{Menu, MenuItem, PredefinedMenuItem, MenuEvent},
};
use tao::{
    event::{Event, WindowEvent},
    event_loop::{ControlFlow, EventLoopBuilder},
    window::WindowBuilder,
    dpi::LogicalSize,
};
use wry::WebViewBuilder;
use tracing::{info, error};
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

        // Create hidden window for WebView — compact, app-like size
        let window = WindowBuilder::new()
            .with_title(format!("Symbion Agent - {}", hostname))
            .with_inner_size(LogicalSize::new(420.0, 650.0))
            .with_resizable(true)
            .with_visible(false) // Start hidden
            .build(&event_loop)
            .expect("Failed to create window");

        // Create WebView with embedded dashboard HTML
        let dashboard_html = include_str!("../ui/simple-dashboard.html");
        let _webview = WebViewBuilder::new()
            .with_html(dashboard_html)
            .with_transparent(true)
            .build(&window)
            .expect("Failed to build WebView");

        info!("WebView created with embedded dashboard");

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

        // App state for window visibility toggle
        let state = Arc::new(Mutex::new(AppState {
            window_visible: false,
        }));

        // Handle menu events
        let menu_channel = MenuEvent::receiver();
        let tray_channel = TrayIconEvent::receiver();

        info!("Starting GUI event loop with embedded WebView");

        // Clone for closures
        let broker_host = self.broker_host.clone();
        let state_clone = state.clone();

        // Run event loop
        event_loop.run(move |event, _, control_flow| {
            *control_flow = ControlFlow::Wait;

            // Handle tray icon events
            if let Ok(tray_event) = tray_channel.try_recv() {
                match tray_event {
                    tray_icon::TrayIconEvent::Click { button, .. } => {
                        // Toggle window visibility on left click
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
                    _ => {
                        // Ignore all other events
                    }
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
                    // Toggle window
                    let mut state = state_clone.lock().unwrap();
                    state.window_visible = !state.window_visible;
                    window.set_visible(state.window_visible);
                    if state.window_visible {
                        window.set_focus();
                    }
                } else if menu_id == "open_pwa" {
                    // Open main PWA in browser
                    let _ = windows_utils::open_url(&format!("http://{}:3001", broker_host));
                } else if menu_id == "open_config" {
                    let _ = windows_utils::open_config();
                } else if menu_id == "check_updates" {
                    // Open dashboard on status tab to show update info
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
                    // Hide window instead of closing
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
            &format!("Dashboard Principal ({}:3001)", self.broker_host), true, None);
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

    /// Generate a programmatic Symbion "S" icon (32x32 RGBA)
    fn generate_icon() -> Result<tray_icon::Icon, Box<dyn std::error::Error>> {
        const SIZE: usize = 32;
        let mut rgba = vec![0u8; SIZE * SIZE * 4];

        // Symbion purple: #cba6f7 = (203, 166, 247)
        let fg: [u8; 3] = [203, 166, 247];
        // Darker background: #313244 = (49, 50, 68)
        let bg: [u8; 3] = [49, 50, 68];

        // Helper to set a pixel
        let set = |buf: &mut Vec<u8>, x: usize, y: usize, r: u8, g: u8, b: u8, a: u8| {
            if x < SIZE && y < SIZE {
                let idx = (y * SIZE + x) * 4;
                buf[idx] = r;
                buf[idx + 1] = g;
                buf[idx + 2] = b;
                buf[idx + 3] = a;
            }
        };

        // Draw rounded rectangle background
        for y in 0..SIZE {
            for x in 0..SIZE {
                // Corner radius of 6
                let in_rect = {
                    let r = 6i32;
                    let xi = x as i32;
                    let yi = y as i32;
                    let s = SIZE as i32;
                    if xi < r && yi < r {
                        (xi - r) * (xi - r) + (yi - r) * (yi - r) <= r * r
                    } else if xi >= s - r && yi < r {
                        (xi - (s - r - 1)) * (xi - (s - r - 1)) + (yi - r) * (yi - r) <= r * r
                    } else if xi < r && yi >= s - r {
                        (xi - r) * (xi - r) + (yi - (s - r - 1)) * (yi - (s - r - 1)) <= r * r
                    } else if xi >= s - r && yi >= s - r {
                        (xi - (s - r - 1)) * (xi - (s - r - 1)) + (yi - (s - r - 1)) * (yi - (s - r - 1)) <= r * r
                    } else {
                        true
                    }
                };

                if in_rect {
                    set(&mut rgba, x, y, bg[0], bg[1], bg[2], 255);
                }
            }
        }

        // Draw a stylized "S" letter using horizontal strokes
        // The S is drawn in the region ~(8..24, 5..27)
        let draw_hline = |buf: &mut Vec<u8>, y: usize, x1: usize, x2: usize| {
            for x in x1..=x2 {
                set(buf, x, y, fg[0], fg[1], fg[2], 255);
                // Anti-aliasing: also draw with reduced opacity on neighbors for thickness
                if y + 1 < SIZE { set(buf, x, y + 1, fg[0], fg[1], fg[2], 200); }
            }
        };

        let draw_vline = |buf: &mut Vec<u8>, x: usize, y1: usize, y2: usize| {
            for y in y1..=y2 {
                set(buf, x, y, fg[0], fg[1], fg[2], 255);
                if x + 1 < SIZE { set(buf, x + 1, y, fg[0], fg[1], fg[2], 200); }
            }
        };

        // Top horizontal bar of S
        draw_hline(&mut rgba, 7, 10, 22);
        draw_hline(&mut rgba, 8, 10, 22);

        // Left vertical bar (top half)
        draw_vline(&mut rgba, 10, 7, 14);
        draw_vline(&mut rgba, 11, 7, 14);

        // Middle horizontal bar
        draw_hline(&mut rgba, 14, 10, 22);
        draw_hline(&mut rgba, 15, 10, 22);

        // Right vertical bar (bottom half)
        draw_vline(&mut rgba, 21, 15, 23);
        draw_vline(&mut rgba, 22, 15, 23);

        // Bottom horizontal bar of S
        draw_hline(&mut rgba, 23, 10, 22);
        draw_hline(&mut rgba, 24, 10, 22);

        let icon = tray_icon::Icon::from_rgba(rgba, SIZE as u32, SIZE as u32)?;
        Ok(icon)
    }
}

impl Default for SymbionGui {
    fn default() -> Self {
        Self::new("127.0.0.1".to_string())
    }
}
