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
 */

mod types;
mod persistence;
mod events;
mod listener;
mod engine;
mod registry;
mod executors;
mod decision_bridge;
mod pending_actions;

pub use types::*;
pub use persistence::AutomationStore;
pub use events::{AutomationEvent, EventDispatcher};
pub use listener::spawn_automation_listener;
pub use engine::{AutomationEngine, ExecutionContext};
pub use registry::{SchemaRegistry, AutomationSchema, SensorInfo, SensorOption};
pub use executors::{
    ActionExecutor, ActionError, ExecutorContext,
    SendNotificationExecutor, ForceModeExecutor, AgentCommandExecutor,
    DelayExecutor, CustomActionExecutor, ActionExecutorRegistry,
};
pub use decision_bridge::{action_to_decision, action_to_decision_dry_run, action_description};
pub use pending_actions::{PendingAction, PendingActionRegistry, SharedPendingActionRegistry};
