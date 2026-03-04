//! Concrete command handlers for Symbion Agent
//!
//! Each handler implements `CommandHandler` and is registered in the `CommandRegistry`.

mod power;
mod shell;
mod process;
mod metrics;
mod service;
mod notify;
pub mod schedule;
pub mod file_transfer;
pub mod plugin_cmd;
mod screenshot;

pub use power::PowerHandler;
pub use shell::ShellHandler;
pub use process::ProcessHandler;
pub use metrics::MetricsHandler;
pub use service::ServiceHandler;
pub use notify::NotifyHandler;
pub use schedule::ScheduleHandler;
pub use file_transfer::FileTransferHandler;
pub use plugin_cmd::PluginCommandHandler;
pub use screenshot::ScreenshotHandler;

use super::handler::CommandRegistry;

/// Build a fully-populated command registry with all standard handlers.
/// Note: ScheduleHandler requires an Arc<Scheduler>, so it's registered separately in agent.rs.
pub fn build_default_registry() -> CommandRegistry {
    let mut registry = CommandRegistry::new();
    registry.register(Box::new(PowerHandler));
    registry.register(Box::new(ShellHandler));
    registry.register(Box::new(ProcessHandler));
    registry.register(Box::new(MetricsHandler));
    registry.register(Box::new(ServiceHandler));
    registry.register(Box::new(NotifyHandler));
    registry
}
