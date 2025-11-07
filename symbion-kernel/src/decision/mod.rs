// Decision Engine Module
// Spec: PR3 P0 v3.1 REFINED

pub mod types;
pub mod clock;
pub mod guards;
pub mod trust;
pub mod engine;

// Re-exports principaux
pub use types::*;
pub use clock::{Clock, SystemClock, MockClock};
pub use guards::{Guard, GuardResult, GuardsEvaluator, ShortCircuitStrategy};
pub use trust::TrustCalculator;
pub use engine::DecisionEngine;
