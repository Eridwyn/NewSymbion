// Module Modes Dynamiques
// Decision Engine v2

pub mod types;
pub mod registry;

pub use types::{DynamicMode, ModeTheme, CreateModeRequest, UpdateModeRequest, default_system_modes};
pub use registry::{ModeRegistry, SharedModeRegistry, create_shared_registry};
