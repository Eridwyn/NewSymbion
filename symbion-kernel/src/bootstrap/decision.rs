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

/// Les guards du moteur de production.
pub fn guards_actifs(
    clock: Arc<dyn crate::decision::Clock>,
) -> Vec<Box<dyn crate::decision::Guard>> {
    vec![
        Box::new(crate::decision::TimeWindowGuard::new(clock)),
        Box::new(crate::decision::AgentHealthGuard),
        Box::new(crate::decision::PresenceGuard),
        Box::new(crate::decision::ContextModeGuard),
    ]
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
    // LES GUARDS N'ÉTAIENT ENREGISTRÉS QUE DANS LES TESTS. En production le
    // moteur se réduisait à « confiance ≥ seuil » : ni expiration, ni agent en
    // maintenance, ni nuit, ni présence. `guards_actifs` est la liste unique —
    // le test `bootstrap_enregistre_les_guards` la vérifie.
    let mut guards_evaluator = crate::decision::GuardsEvaluator::new(
        crate::decision::ShortCircuitStrategy::OnBlock,
    );
    for guard in guards_actifs(decision_clock.clone()) {
        guards_evaluator.add_guard(guard);
    }

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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::decision::{
        Action, DecisionConfig, DecisionContext, DecisionEngine, DecisionOutcome, GuardsEvaluator,
        ImpactLevel, ShortCircuitStrategy, SystemClock, TrustCalculator,
    };
    use std::collections::HashMap;

    /// Le moteur TEL QU'IL EST MONTÉ en production. Les tests de `engine.rs`
    /// montent leurs propres guards : c'est ce qui a caché, des mois durant,
    /// qu'il n'y en avait aucun ici.
    fn moteur_de_production() -> DecisionEngine {
        let clock = Arc::new(SystemClock);
        let mut guards = GuardsEvaluator::new(ShortCircuitStrategy::OnBlock);
        for g in guards_actifs(clock.clone()) {
            guards.add_guard(g);
        }
        let config = DecisionConfig::default();
        DecisionEngine::new(guards, TrustCalculator::new(config.clone(), clock), config)
    }

    fn robot(anyone_home: Option<bool>) -> (Action, DecisionContext) {
        (
            Action {
                action_type: "automation.plugin_command".into(),
                agent_id: "system".into(),
                impact_level: ImpactLevel::Medium,
                trace_id: "t".into(),
                expires_at: None,
                dry_run: false,
                expected_mode: None,
                expected_ssid: None,
                target: Some("plugin:robot/actions/clean_all".into()),
            },
            DecisionContext {
                mode: "maison".into(),
                ssid: "x".into(),
                agents: HashMap::new(),
                anyone_home,
            },
        )
    }

    #[test]
    fn bootstrap_enregistre_les_guards() {
        let noms: Vec<String> = guards_actifs(Arc::new(SystemClock))
            .iter()
            .map(|g| g.name().to_string())
            .collect();
        for attendu in ["TimeWindowGuard", "AgentHealthGuard", "PresenceGuard", "ContextModeGuard"] {
            assert!(noms.iter().any(|n| n == attendu), "{} absent : {:?}", attendu, noms);
        }
    }

    #[test]
    fn le_robot_ne_part_pas_chez_quelquun_sans_accord() {
        let (action, ctx) = robot(Some(true));
        match moteur_de_production().decide(&action, &ctx).outcome {
            DecisionOutcome::RequireValidation { explanation_codes, .. } => {
                assert!(explanation_codes.iter().any(|c| c == "GUARD.PRESENCE.SOMEONE_HOME"))
            }
            autre => panic!("le robot doit attendre un accord, pas {:?}", autre),
        }
    }

    #[test]
    fn une_action_system_nest_pas_bloquee_par_la_sante_des_agents() {
        let (mut action, ctx) = robot(None);
        action.target = Some("plugin:lights/actions/all_off".into());
        action.impact_level = ImpactLevel::Low;
        let outcome = moteur_de_production().decide(&action, &ctx).outcome;
        if let DecisionOutcome::RequireValidation { explanation_codes, .. } = &outcome {
            assert!(!explanation_codes.iter().any(|c| c.starts_with("GUARD.")),
                    "aucun guard ne doit retenir une action system Low : {:?}", explanation_codes);
        }
        assert!(!matches!(outcome, DecisionOutcome::Blocked { .. }));
    }

    #[test]
    fn trusted_dispense_de_la_confiance_pas_des_guards() {
        let moteur = moteur_de_production();
        let (action, chez_quelquun) = robot(Some(true));
        assert!(matches!(
            moteur.decide_trusted(&action, &chez_quelquun).outcome,
            DecisionOutcome::RequireValidation { .. }
        ));
        let (action, maison_vide) = robot(Some(false));
        match moteur.decide_trusted(&action, &maison_vide).outcome {
            DecisionOutcome::Approved { trust_score, auto } => {
                assert_eq!(trust_score, 1.0);
                assert!(auto);
            }
            autre => panic!("maison vide : trusted doit passer, pas {:?}", autre),
        }
    }

}

