//! Symbion SSL Plugin Library v2
//!
//! Provides SSL certificate monitoring capabilities:
//! - Certificate expiry checking
//! - Domain health monitoring
//! - Dynamic domain management via API
//! - Per-domain alert thresholds
//! - Fingerprint tracking for change detection

pub mod config;
pub mod mqtt;
pub mod ssl;
pub mod state;

pub use config::Config;
pub use mqtt::MqttPublisher;
pub use ssl::{CertificateStatus, SslChecker};
pub use state::{DomainState, DynamicDomain};
