//! Symbion Freebox Plugin Library
//!
//! Provides Freebox integration for Symbion:
//! - Presence detection via LAN device tracking
//! - Internet connection status monitoring
//! - Download manager integration
//! - Full device list for network overview

pub mod config;
pub mod freebox;
pub mod mqtt;

pub use config::Config;
pub use freebox::FreeboxClient;
pub use mqtt::MqttPublisher;
