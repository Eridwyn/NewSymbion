/**
 * Bootstrap: Decision Engine Subsystem
 *
 * Initializes: DecisionEngine, ValidationManager, OverrideManager,
 * AuditManager, AgentHealthManager, DecisionMetrics, TrustTracker
 */

use crate::automations::SharedPendingActionRegistry;
use crate::database::SharedDatabase;
use crate::decision::SharedTrustTracker;
use std::sync::Arc;

pub struct DecisionSubsystem {
    pub decision_engine: Arc<crate::decision::DecisionEngine>,
    pub decision_validation_manager: Arc<crate::decision::ValidationManager>,
    pub decision_override_manager: Arc<crate::decision::OverrideManager>,
    pub decision_audit_manager: Arc<crate::decision::AuditManager>,
    pub decision_agent_health_manager: Arc<crate::decision::AgentHealthManager>,
    pub decision_metrics: Arc<crate::decision::DecisionMetrics>,
    pub trust_tracker: SharedTrustTracker,
    pub pending_action_registry: SharedPendingActionRegistry,
}

pub fn init_decision(db: &Option<SharedDatabase>) -> DecisionSubsystem {
    let decision_clock = Arc::new(crate::decision::SystemClock);
    let decision_config = crate::decision::DecisionConfig::default();

    let decision_validation_manager = Arc::new(crate::decision::ValidationManager::new(
        decision_clock.clone(),
        1800, // TTL 30 minutes
    ));

    let decision_override_manager = Arc::new(crate::decision::OverrideManager::new(
        decision_clock.clone(),
        86400, // Default TTL 24h
    ));

    let decision_audit_manager = Arc::new(crate::decision::AuditManager::new(
        decision_clock.clone(),
        10000, // Max 10k records
    ));

    let agent_health_mapping = crate::decision::AgentHealthMapping {
        online_min_score: 0.8,
        active_min_score: 0.7,
        idle_min_score: 0.5,
        degraded_min_score: 0.3,
        degraded_consecutive_threshold: 3,
        stale_max_age_secs: 120,
    };

    let decision_agent_health_manager = Arc::new(crate::decision::AgentHealthManager::new(
        decision_clock.clone(),
        agent_health_mapping,
    ));

    let decision_metrics = Arc::new(crate::decision::DecisionMetrics::new());

    // Guards Evaluator
    let guards_evaluator = crate::decision::GuardsEvaluator::new(
        crate::decision::ShortCircuitStrategy::OnBlock,
    );

    // Trust Tracker (must be created BEFORE TrustCalculator)
    let trust_tracker = crate::decision::TrustTracker::new("./data");
    let trust_tracker = if let Some(ref db) = db {
        trust_tracker.with_database(db.clone())
    } else {
        trust_tracker
    };
    let trust_tracker = Arc::new(trust_tracker);
    println!("[kernel] initialized Trust Tracker (evolving statistics)");

    // Trust Calculator with Trust Tracker integration
    let trust_calculator = crate::decision::TrustCalculator::with_trust_tracker(
        decision_config.clone(),
        decision_clock.clone(),
        trust_tracker.clone(),
    );

    let decision_engine = Arc::new(crate::decision::DecisionEngine::new(
        guards_evaluator,
        trust_calculator,
        decision_config,
    ));
    println!("[kernel] initialized Decision Engine PR3 (with Trust Tracker)");

    // Periodic cleanup of expired validations and overrides (every 10 minutes)
    {
        let vm = decision_validation_manager.clone();
        let om = decision_override_manager.clone();
        tokio::spawn(async move {
            let mut timer = tokio::time::interval(std::time::Duration::from_secs(600));
            loop {
                timer.tick().await;
                let v = vm.cleanup_expired();
                let o = om.cleanup_expired();
                if v > 0 || o > 0 {
                    eprintln!("[decision] Cleanup: {} expired validations, {} expired overrides", v, o);
                }
            }
        });
    }

    // Pending Action Registry
    let pending_action_registry = Arc::new(
        crate::automations::PendingActionRegistry::new(Some(std::path::PathBuf::from("./data"))),
    );
    if let Some(ref db) = db {
        pending_action_registry.set_database(db.clone());
    }
    println!("[kernel] initialized Pending Action Registry");

    DecisionSubsystem {
        decision_engine,
        decision_validation_manager,
        decision_override_manager,
        decision_audit_manager,
        decision_agent_health_manager,
        decision_metrics,
        trust_tracker,
        pending_action_registry,
    }
}
