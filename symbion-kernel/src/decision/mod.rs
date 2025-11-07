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
