/**
 * Bootstrap: Intelligence & Inference Subsystem
 *
 * Initializes: FeatureRegistry, InferenceEngine, SessionManager,
 * BootstrapScheduler, ScheduleRegistry
 */

use crate::database::SharedDatabase;
use crate::intelligence::{SharedFeatureRegistry, SharedInferenceEngine, SharedSessionManager};
use crate::schedule::SharedScheduleRegistry;
use std::sync::Arc;

pub struct IntelligenceSubsystem {
    pub feature_registry: SharedFeatureRegistry,
    pub inference_engine: SharedInferenceEngine,
    pub session_manager: SharedSessionManager,
    pub schedule_registry: SharedScheduleRegistry,
}

pub fn init_intelligence(db: &Option<SharedDatabase>) -> IntelligenceSubsystem {
    // Feature Registry for data-driven intelligence (v2)
    let feature_registry = Arc::new(crate::intelligence::FeatureRegistry::new());
    eprintln!("[kernel] initialized Feature Registry");

    // Inference Engine for case-based mode prediction (v2)
    let samples_path = std::path::PathBuf::from("./data/intelligence_samples.json");
    let inference_engine = crate::intelligence::InferenceEngine::with_persistence(
        crate::intelligence::InferenceConfig::default(),
        samples_path,
    );
    let inference_engine = if let Some(ref db) = db {
        inference_engine.with_database(db.clone())
    } else {
        inference_engine
    };
    let inference_engine = Arc::new(inference_engine);
    eprintln!("[kernel] initialized Inference Engine v2 (with persistence)");

    // Session Manager for hysteresis-based mode transitions (v2)
    let session_manager = Arc::new(crate::intelligence::SessionManager::default());
    eprintln!("[kernel] initialized Session Manager v2");

    // Bootstrap scheduler for cold start (seeds inference engine if needed)
    let bootstrap_scheduler = crate::intelligence::BootstrapScheduler::default();
    let initial_vector = crate::intelligence::VectorBuilder::new(&feature_registry).build();
    bootstrap_scheduler.seed_inference_engine(&inference_engine, &initial_vector);

    // Schedule Registry
    let schedule_registry = crate::schedule::ScheduleRegistry::new(std::path::PathBuf::from("./data"));
    let schedule_registry = if let Some(ref db) = db {
        schedule_registry.with_database(db.clone())
    } else {
        schedule_registry
    };
    let schedule_registry = Arc::new(schedule_registry);
    eprintln!("[kernel] initialized Schedule Registry ({} rules)", schedule_registry.count_rules());

    IntelligenceSubsystem {
        feature_registry,
        inference_engine,
        session_manager,
        schedule_registry,
    }
}
