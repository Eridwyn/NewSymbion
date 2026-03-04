//! Agent Plugin System
//!
//! Extensible plugin architecture for adding custom functionality to agents.
//! Plugins implement the `AgentPlugin` trait and are managed by `AgentPluginRegistry`.

pub mod trait_def;
pub mod registry;
pub mod activity_tracker;

pub use trait_def::AgentPlugin;
pub use registry::AgentPluginRegistry;
pub use activity_tracker::ActivityTracker;
