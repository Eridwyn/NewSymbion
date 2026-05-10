/**
 * SYMBION KERNEL - Automations Module
 *
 * ROLE: Event-driven automation system with rule builder support
 *
 * ARCHITECTURE:
 * - types.rs          : Automation, Trigger, Condition, Action structs
 * - persistence.rs    : JSON storage with soft-delete
 * - events.rs         : Event dispatcher for triggers
 * - listener.rs       : Event listener + trigger matching
 * - engine.rs         : Condition evaluation + action execution (Phase 3)
 * - registry.rs       : Schema for PWA + plugin registration (Phase 5)
 * - decision_bridge.rs: Bridge to DecisionEngine for trust evaluation (Phase 7)
 * - scheduler.rs      : Periodic polling for scheduled triggers
 */

pub(crate) mod types;
mod persistence;
mod events;
mod listener;
mod engine;
mod registry;
pub mod executors;
mod decision_bridge;
mod pending_actions;
mod scheduler;

pub use types::*;
pub use persistence::AutomationStore;
pub use events::{AutomationEvent, EventDispatcher};
pub use listener::spawn_automation_listener;
pub use engine::{AutomationEngine, ExecutionContext};
pub use registry::{SchemaRegistry, AutomationSchema, SensorInfo};
pub use pending_actions::{PendingActionRegistry, SharedPendingActionRegistry};
pub use scheduler::AutomationScheduler;
