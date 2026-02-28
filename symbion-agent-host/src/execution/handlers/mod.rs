//! Concrete command handlers for Symbion Agent
//!
//! Each handler implements `CommandHandler` and is registered in the `CommandRegistry`.

mod power;
mod shell;
mod process;
mod metrics;
mod service;

pub use power::PowerHandler;
pub use shell::ShellHandler;
pub use process::ProcessHandler;
pub use metrics::MetricsHandler;
pub use service::ServiceHandler;

use super::handler::CommandRegistry;

/// Build a fully-populated command registry with all standard handlers.
pub fn build_default_registry() -> CommandRegistry {
    let mut registry = CommandRegistry::new();
    registry.register(Box::new(PowerHandler));
    registry.register(Box::new(ShellHandler));
    registry.register(Box::new(ProcessHandler));
    registry.register(Box::new(MetricsHandler));
    registry.register(Box::new(ServiceHandler));
    registry
}
