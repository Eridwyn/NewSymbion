// Decision Engine - Orchestrateur central
// Spec: PR3 P0 v3.1 REFINED

use crate::decision::{
    Action, BlockedReasonCategory, DecisionConfig, DecisionContext, DecisionOutcome,
    DecisionResult, GuardsEvaluation, GuardsEvaluator, TrustCalculator, ImpactLevel, GuardWarning,
};
use std::sync::RwLock;
use uuid::Uuid;

/// Decision Engine principal
pub struct DecisionEngine {
    guards_evaluator: GuardsEvaluator,
    trust_calculator: TrustCalculator,
    config: RwLock<DecisionConfig>,
}

impl DecisionEngine {
    pub fn new(
        guards_evaluator: GuardsEvaluator,
        trust_calculator: TrustCalculator,
        config: DecisionConfig,
    ) -> Self {
        Self {
            guards_evaluator,
            trust_calculator,
            config: RwLock::new(config),
        }
    }

    /// Prendre une décision sur une action
    pub fn decide(&self, action: &Action, context: &DecisionContext) -> DecisionResult {
        let decision_id = Uuid::new_v4().to_string();

        // Resolve trace_id inline (avoids cloning the entire Action)
        let trace_id = if action.trace_id.is_empty() {
            Uuid::new_v4().to_string()
        } else {
            action.trace_id.clone()
        };

        // Mode dry-run: évaluation sans exécution
        if action.dry_run {
            return self.dry_run_evaluate(action, context, decision_id, trace_id);
        }

        // Étape 1: Évaluer les guards
        let guards_eval = self.guards_evaluator.evaluate(action, context);

        // Destructure to consume fields (avoids .clone() on warnings)
        let GuardsEvaluation { passed: _, blocks, requires_validation, warnings } = guards_eval;

        // Si guards bloquent
        if !blocks.is_empty() {
            let reasons: Vec<String> = blocks
                .iter()
                .map(|b| b.reason.clone())
                .collect();
            let explanation_codes: Vec<String> = blocks
                .iter()
                .map(|b| b.explanation_code.clone())
                .collect();
            // Compute categories for selective learning
            let categories: Vec<BlockedReasonCategory> = explanation_codes
                .iter()
                .map(|code| BlockedReasonCategory::from_explanation(code))
                .collect();

            return DecisionResult {
                decision_id,
                outcome: DecisionOutcome::Blocked {
                    reasons,
                    explanation_codes,
                    categories,
                },
                trace_id,
                warnings,
            };
        }

        // Si guards requièrent validation
        if !requires_validation.is_empty() {
            let reasons: Vec<String> = requires_validation
                .iter()
                .map(|r| r.reason.clone())
                .collect();
            let explanation_codes: Vec<String> = requires_validation
                .iter()
                .map(|r| r.explanation_code.clone())
                .collect();
            let human_reasons: Vec<String> = requires_validation
                .iter()
                .map(|r| r.human_reason.clone())
                .collect();

            // Calculate trust for the response even when guards require validation
            let trust_score = self.trust_calculator.calculate(action, context);
            let threshold = self.get_threshold(&action.impact_level);

            return DecisionResult {
                decision_id,
                outcome: DecisionOutcome::RequireValidation {
                    trust_score: trust_score.score,
                    threshold,
                    reasons,
                    explanation_codes,
                    human_reasons,
                },
                trace_id,
                warnings,
            };
        }

        // Étape 2: Guards passés, calculer trust score
        let trust_score = self.trust_calculator.calculate(action, context);

        // Étape 3: Comparer avec seuil selon impact level
        let threshold = self.get_threshold(&action.impact_level);

        if trust_score.score >= threshold {
            // Approuvé automatiquement
            DecisionResult {
                decision_id,
                outcome: DecisionOutcome::Approved {
                    trust_score: trust_score.score,
                    auto: true,
                },
                trace_id,
                warnings,
            }
        } else {
            // Trust score insuffisant → Validation requise
            DecisionResult {
                decision_id,
                outcome: DecisionOutcome::RequireValidation {
                    trust_score: trust_score.score,
                    threshold,
                    reasons: vec![format!(
                        "Trust score {:.2} below threshold {:.2}",
                        trust_score.score, threshold
                    )],
                    explanation_codes: vec!["TRUST.BELOW_THRESHOLD".to_string()],
                    human_reasons: vec![format!(
                        "Niveau de confiance insuffisant ({:.0}% requis: {:.0}%)",
                        trust_score.score * 100.0,
                        threshold * 100.0
                    )],
                },
                trace_id,
                warnings,
            }
        }
    }

    /// Évaluation en mode dry-run
    fn dry_run_evaluate(
        &self,
        action: &Action,
        context: &DecisionContext,
        decision_id: String,
        trace_id: String,
    ) -> DecisionResult {
        let guards_eval = self.guards_evaluator.evaluate(action, context);
        let trust_score = self.trust_calculator.calculate(action, context);
        let threshold = self.get_threshold(&action.impact_level);

        let GuardsEvaluation { passed, blocks, requires_validation: _, warnings } = guards_eval;
        let would_approve = passed && trust_score.score >= threshold;

        DecisionResult {
            decision_id,
            outcome: DecisionOutcome::DryRun {
                would_approve,
                trust_score: trust_score.score,
                threshold,
                guards_passed: passed,
                blocks,
            },
            trace_id,
            warnings,
        }
    }

    /// Obtenir le seuil selon impact level
    fn get_threshold(&self, impact_level: &ImpactLevel) -> f32 {
        let config = self.config.read().unwrap_or_else(|e| e.into_inner());
        match impact_level {
            ImpactLevel::Low => config.impact_thresholds.low,
            ImpactLevel::Medium => config.impact_thresholds.medium,
            ImpactLevel::High => config.impact_thresholds.high,
            ImpactLevel::VeryHigh => config.impact_thresholds.very_high,
        }
    }

    /// Mettre à jour la configuration
    pub fn update_config(&self, config: DecisionConfig) {
        let mut current = self.config.write().unwrap_or_else(|e| e.into_inner());
        *current = config;
        eprintln!("[decision-engine] Config updated: {:?}", current.impact_thresholds);
    }

    /// Obtenir la configuration actuelle
    pub fn config(&self) -> DecisionConfig {
        self.config.read().unwrap_or_else(|e| e.into_inner()).clone()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::decision::{
        AgentMetrics, AgentState, ShortCircuitStrategy, SystemClock, Clock,
        guards::{TimeWindowGuard, AgentHealthGuard, ContextModeGuard},
    };
    use std::collections::HashMap;
    use std::sync::Arc;
    use time::macros::datetime;

    fn create_test_engine() -> DecisionEngine {
        let clock = Arc::new(SystemClock);

        let mut guards_evaluator = GuardsEvaluator::new(ShortCircuitStrategy::OnBlockOrRequire);
        guards_evaluator.add_guard(Box::new(TimeWindowGuard::new(clock.clone())));
        guards_evaluator.add_guard(Box::new(AgentHealthGuard));
        guards_evaluator.add_guard(Box::new(ContextModeGuard));

        let config = DecisionConfig::default();
        let trust_calculator = TrustCalculator::new(config.clone(), clock);

        DecisionEngine::new(guards_evaluator, trust_calculator, config)
    }

    #[test]
    fn test_decision_approved_low_impact() {
        let engine = create_test_engine();

        let action = Action {
            action_type: "test_action".into(),
            agent_id: "agent1".into(),
            impact_level: ImpactLevel::Low,
            trace_id: "trace1".into(),
            expires_at: None,
            dry_run: false,
            expected_mode: Some("maison".into()),
            expected_ssid: Some("home-wifi".into()),
        };

        let mut agents = HashMap::new();
        agents.insert(
            "agent1".to_string(),
            AgentState {
                id: "agent1".into(),
                last_seen: SystemClock.now_utc(),
                metrics: AgentMetrics {
                    cpu_usage: 0.2,
                    memory_usage_percent: 0.3,
                },
                maintenance_mode: false,
                last_reconnect: None,
            },
        );

        let context = DecisionContext {
            mode: "maison".into(),
            ssid: "home-wifi".into(),
            agents,
        };

        let result = engine.decide(&action, &context);

        match result.outcome {
            DecisionOutcome::Approved { trust_score, auto } => {
                assert!(trust_score > 0.3); // Seuil Low = 0.3
                assert!(auto);
            }
            _ => panic!("Expected Approved, got {:?}", result.outcome),
        }
    }

    #[test]
    fn test_decision_blocked_expired() {
        let engine = create_test_engine();

        let action = Action {
            action_type: "test_action".into(),
            agent_id: "agent1".into(),
            impact_level: ImpactLevel::Low,
            trace_id: "trace1".into(),
            expires_at: Some(datetime!(2020-01-01 00:00 UTC)), // Expired
            dry_run: false,
            expected_mode: None,
            expected_ssid: None,
        };

        let context = DecisionContext {
            mode: "maison".into(),
            ssid: "home-wifi".into(),
            agents: HashMap::new(),
        };

        let result = engine.decide(&action, &context);

        match result.outcome {
            DecisionOutcome::Blocked { reasons, .. } => {
                assert!(reasons[0].contains("expired"));
            }
            _ => panic!("Expected Blocked, got {:?}", result.outcome),
        }
    }

    #[test]
    fn test_decision_require_validation_mode_mismatch() {
        let engine = create_test_engine();

        let action = Action {
            action_type: "test_action".into(),
            agent_id: "agent1".into(),
            impact_level: ImpactLevel::Medium,
            trace_id: "trace1".into(),
            expires_at: None,
            dry_run: false,
            expected_mode: Some("pro".into()),
            expected_ssid: None,
        };

        // Ajouter l'agent pour éviter que AgentHealthGuard bloque avant ContextModeGuard
        let mut agents = HashMap::new();
        agents.insert(
            "agent1".to_string(),
            AgentState {
                id: "agent1".into(),
                last_seen: SystemClock.now_utc(),
                metrics: AgentMetrics {
                    cpu_usage: 0.2,
                    memory_usage_percent: 0.3,
                },
                maintenance_mode: false,
                last_reconnect: None,
            },
        );

        let context = DecisionContext {
            mode: "maison".into(), // Mismatch
            ssid: "home-wifi".into(),
            agents,
        };

        let result = engine.decide(&action, &context);

        match result.outcome {
            DecisionOutcome::RequireValidation { reasons, .. } => {
                assert!(reasons[0].contains("Mode mismatch"));
            }
            _ => panic!("Expected RequireValidation, got {:?}", result.outcome),
        }
    }

    #[test]
    fn test_decision_dry_run() {
        let engine = create_test_engine();

        let action = Action {
            action_type: "test_action".into(),
            agent_id: "agent1".into(),
            impact_level: ImpactLevel::Low,
            trace_id: "trace1".into(),
            expires_at: None,
            dry_run: true, // Dry-run
            expected_mode: Some("maison".into()),
            expected_ssid: Some("home-wifi".into()),
        };

        let mut agents = HashMap::new();
        agents.insert(
            "agent1".to_string(),
            AgentState {
                id: "agent1".into(),
                last_seen: SystemClock.now_utc(),
                metrics: AgentMetrics {
                    cpu_usage: 0.2,
                    memory_usage_percent: 0.3,
                },
                maintenance_mode: false,
                last_reconnect: None,
            },
        );

        let context = DecisionContext {
            mode: "maison".into(),
            ssid: "home-wifi".into(),
            agents,
        };

        let result = engine.decide(&action, &context);

        match result.outcome {
            DecisionOutcome::DryRun {
                would_approve,
                trust_score,
                threshold,
                guards_passed,
                ..
            } => {
                assert!(guards_passed);
                assert!(trust_score > threshold);
                assert!(would_approve);
            }
            _ => panic!("Expected DryRun, got {:?}", result.outcome),
        }
    }
}
