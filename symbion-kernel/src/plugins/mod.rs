//! Symbion Plugin System
//!
//! This module implements the plugin architecture for Symbion.
//!
//! # Architecture
//!
//! - **Kernel** = Brain, all decisions
//! - **Plugin** = Pure executor, no intelligence
//! - **MQTT** = Nervous system
//!
//! See `docs/plugins/CONTRACT.md` for the full specification.

pub mod contract;
pub mod registry;

pub use contract::*;
pub use registry::{
    DispatchError, PluginRegistry, PluginState, RegisteredPlugin, RegistryError,
    RegistryStats, SharedPluginRegistry, new_shared_registry,
};
