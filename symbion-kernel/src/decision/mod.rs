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
pub mod metrics;
pub mod environment;  // F1: Environment monitoring rules
pub mod trust_tracker;  // Phase 7: Evolving trust statistics

// Re-exports principaux (public API - intentionally exposed for future use)
#[allow(unused_imports)]
pub use types::*;
pub use clock::{Clock, SystemClock};
#[allow(unused_imports)]
pub use clock::MockClock; // Test-only API
#[allow(unused_imports)]
pub use guards::{Guard, GuardResult, GuardsEvaluator, ShortCircuitStrategy};
pub use trust::TrustCalculator;
pub use engine::DecisionEngine;
#[allow(unused_imports)]
pub use persistence::{PersistenceManager, FsyncMode, PersistenceStats}; // Future persistence layer
#[allow(unused_imports)]
pub use idempotence::IdempotenceManager; // Future command deduplication
#[allow(unused_imports)]
pub use config::ConfigManager; // Future dynamic config
#[allow(unused_imports)]
pub use validation::{ValidationManager, ValidationRequest, ValidationStatus, ValidationStats};
pub use r#override::{OverrideManager, MasterOverride, OverrideType, OverrideStats};
pub use audit::{AuditManager, AuditStats};
#[allow(unused_imports)]
pub use agent_status::{AgentHealthManager, AgentHealthStatus, AgentHealthStats};
pub use metrics::DecisionMetrics;
pub use environment::{EnvironmentRules, Intention};  // F1: Environment alerts
pub use trust_tracker::{TrustTracker, SharedTrustTracker, TrustStats, ActionTrustStats, AgentTrustStats};
