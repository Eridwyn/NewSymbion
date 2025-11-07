// Decision Engine Module
// Spec: PR3 P0 v3.1 REFINED

pub mod types;
pub mod clock;
pub mod guards;
pub mod trust;
pub mod engine;
pub mod persistence;
pub mod idempotence;
pub mod config;
pub mod validation;
pub mod r#override;
pub mod audit;
pub mod agent_status;

// Re-exports principaux
pub use types::*;
pub use clock::{Clock, SystemClock, MockClock};
pub use guards::{Guard, GuardResult, GuardsEvaluator, ShortCircuitStrategy};
pub use trust::TrustCalculator;
pub use engine::DecisionEngine;
pub use persistence::{PersistenceManager, FsyncMode, PersistenceStats};
pub use idempotence::IdempotenceManager;
pub use config::ConfigManager;
pub use validation::{ValidationManager, ValidationRequest, ValidationStatus, ValidationStats};
pub use r#override::{OverrideManager, MasterOverride, OverrideType, OverrideStats};
pub use audit::{AuditManager, AuditStats};
pub use agent_status::{AgentHealthManager, AgentHealthStatus, AgentHealthStats};
