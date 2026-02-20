// Guards System pour Decision Engine
// Spec: PR3 P0 v3.1 REFINED - CORRECTION 2 (priority rule explicit)

use crate::decision::{
    Action, DecisionContext, GuardBlock, GuardRequire, GuardWarning, GuardsEvaluation,
    Clock, ImpactLevel,
};
use std::sync::Arc;
// use time::OffsetDateTime; // Unused - time handling via Clock trait

/// Resultat d'evaluation d'un guard
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum GuardResult {
    Pass,
    Warn {
        reason: String,
        explanation_code: String,
        human_reason: String,
    },
    Block {
        reason: String,
        explanation_code: String,
        human_reason: String,
    },
    RequireValidation {
        reason: String,
        explanation_code: String,
        human_reason: String,
    },
}

impl GuardResult {
    /// Priorite du resultat (pour resolution conflits meme priorite)
    /// CORRECTION 2: Block > RequireValidation > Warn > Pass
    pub fn severity(&self) -> u8 {
        match self {
            GuardResult::Block { .. } => 3,
            GuardResult::RequireValidation { .. } => 2,
            GuardResult::Warn { .. } => 1,
            GuardResult::Pass => 0,
        }
    }

    /// Combine deux resultats selon regle priorite
    pub fn combine(self, other: GuardResult) -> GuardResult {
        if self.severity() >= other.severity() {
            self
        } else {
            other
        }
    }
}

/// Strategie de court-circuit
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ShortCircuitStrategy {
    Never,               // Evalue tous les guards
    OnBlock,             // Court-circuit si Block
    OnBlockOrRequire,    // Court-circuit si Block ou RequireValidation
}

/// Trait guard abstrait
pub trait Guard: Send + Sync {
    /// Priorite du guard (plus haut = execute en premier)
    fn priority(&self) -> u32;

    /// Evalue l'action
    fn evaluate(&self, action: &Action, context: &DecisionContext) -> GuardResult;

    /// Nom du guard (pour logs)
    fn name(&self) -> &str;
}

/// Guard fenetre temporelle
pub struct TimeWindowGuard {
    clock: Arc<dyn Clock>,
}

impl TimeWindowGuard {
    pub fn new(clock: Arc<dyn Clock>) -> Self {
        Self { clock }
    }
}

impl Guard for TimeWindowGuard {
    fn priority(&self) -> u32 {
        100 // Haute priorite (check rapide)
    }

    fn name(&self) -> &str {
        "TimeWindowGuard"
    }

    fn evaluate(&self, action: &Action, _context: &DecisionContext) -> GuardResult {
        let now = self.clock.now_utc();

        // Check TTL expiration
        if let Some(expires_at) = action.expires_at {
            if now >= expires_at {
                return GuardResult::Block {
                    reason: format!("Action expired at {}", expires_at),
                    explanation_code: "GUARD.TIME.EXPIRED".to_string(),
                    human_reason: "Cette action a expir\u{e9} et ne peut plus \u{ea}tre ex\u{e9}cut\u{e9}e".to_string(),
                };
            }
        }

        // Actions VeryHigh la nuit (23h-7h) → RequireValidation
        if action.impact_level == ImpactLevel::VeryHigh {
            let hour = now.hour();
            if hour >= 23 || hour < 7 {
                return GuardResult::RequireValidation {
                    reason: format!("VeryHigh action at night ({}h)", hour),
                    explanation_code: "GUARD.TIME.NIGHT_HIGH_IMPACT".to_string(),
                    human_reason: "Action critique en dehors des heures de bureau - validation recommand\u{e9}e".to_string(),
                };
            }
        }

        GuardResult::Pass
    }
}

/// Guard sante agent
pub struct AgentHealthGuard;

impl Guard for AgentHealthGuard {
    fn priority(&self) -> u32 {
        90 // Priorite elevee
    }

    fn name(&self) -> &str {
        "AgentHealthGuard"
    }

    fn evaluate(&self, action: &Action, context: &DecisionContext) -> GuardResult {
        let agent_state = match context.agents.get(&action.agent_id) {
            Some(state) => state,
            None => {
                return GuardResult::RequireValidation {
                    reason: format!("Agent {} not found in registry", action.agent_id),
                    explanation_code: "GUARD.AGENT.NOT_FOUND".to_string(),
                    human_reason: "Agent inconnu du syst\u{e8}me - validation requise".to_string(),
                };
            }
        };

        // Agent en maintenance → Block
        if agent_state.maintenance_mode {
            return GuardResult::Block {
                reason: format!("Agent {} in maintenance mode", action.agent_id),
                explanation_code: "GUARD.AGENT.MAINTENANCE".to_string(),
                human_reason: "Agent en maintenance, actions bloqu\u{e9}es".to_string(),
            };
        }

        // Metriques degradees → Warn
        if agent_state.metrics.cpu_usage > 0.9 || agent_state.metrics.memory_usage_percent > 0.9 {
            return GuardResult::Warn {
                reason: format!(
                    "Agent {} metrics degraded (CPU: {:.1}%, RAM: {:.1}%)",
                    action.agent_id,
                    agent_state.metrics.cpu_usage * 100.0,
                    agent_state.metrics.memory_usage_percent * 100.0
                ),
                explanation_code: "GUARD.AGENT.DEGRADED".to_string(),
                human_reason: "Agent sous charge, performances potentiellement impact\u{e9}es".to_string(),
            };
        }

        GuardResult::Pass
    }
}

/// Guard contexte mode
pub struct ContextModeGuard;

impl Guard for ContextModeGuard {
    fn priority(&self) -> u32 {
        80
    }

    fn name(&self) -> &str {
        "ContextModeGuard"
    }

    fn evaluate(&self, action: &Action, context: &DecisionContext) -> GuardResult {
        // Check expected_mode
        if let Some(ref expected_mode) = action.expected_mode {
            if expected_mode.to_lowercase() != context.mode.to_lowercase() {
                return GuardResult::RequireValidation {
                    reason: format!(
                        "Mode mismatch: expected '{}', current '{}'",
                        expected_mode, context.mode
                    ),
                    explanation_code: "GUARD.CONTEXT.MODE_MISMATCH".to_string(),
                    human_reason: format!(
                        "Mode contextuel a chang\u{e9} depuis la cr\u{e9}ation de l'action (attendu: {}, actuel: {})",
                        expected_mode, context.mode
                    ),
                };
            }
        }

        // Check expected_ssid
        if let Some(ref expected_ssid) = action.expected_ssid {
            if expected_ssid != &context.ssid {
                return GuardResult::RequireValidation {
                    reason: format!(
                        "SSID mismatch: expected '{}', current '{}'",
                        expected_ssid, context.ssid
                    ),
                    explanation_code: "GUARD.CONTEXT.SSID_MISMATCH".to_string(),
                    human_reason: format!(
                        "R\u{e9}seau chang\u{e9} depuis la cr\u{e9}ation de l'action (attendu: {}, actuel: {})",
                        expected_ssid, context.ssid
                    ),
                };
            }
        }

        GuardResult::Pass
    }
}

/// Evaluateur de guards avec court-circuit
pub struct GuardsEvaluator {
    guards: Vec<Box<dyn Guard>>,
    short_circuit: ShortCircuitStrategy,
}

impl GuardsEvaluator {
    pub fn new(short_circuit: ShortCircuitStrategy) -> Self {
        Self {
            guards: Vec::new(),
            short_circuit,
        }
    }

    /// Ajouter un guard
    pub fn add_guard(&mut self, guard: Box<dyn Guard>) {
        self.guards.push(guard);
        // Trier par priorite decroissante
        self.guards.sort_by(|a, b| b.priority().cmp(&a.priority()));
    }

    /// Evaluer tous les guards
    pub fn evaluate(&self, action: &Action, context: &DecisionContext) -> GuardsEvaluation {
        let mut blocks = Vec::new();
        let mut requires_validation = Vec::new();
        let mut warnings = Vec::new();

        for guard in &self.guards {
            let result = guard.evaluate(action, context);

            match result {
                GuardResult::Pass => {}
                GuardResult::Warn {
                    reason,
                    explanation_code,
                    human_reason,
                } => {
                    warnings.push(GuardWarning {
                        reason,
                        explanation_code,
                        human_reason,
                    });
                }
                GuardResult::Block {
                    reason,
                    explanation_code,
                    human_reason,
                } => {
                    blocks.push(GuardBlock {
                        reason,
                        explanation_code,
                        human_reason,
                    });

                    // Court-circuit si demande
                    if self.short_circuit == ShortCircuitStrategy::OnBlock
                        || self.short_circuit == ShortCircuitStrategy::OnBlockOrRequire
                    {
                        break;
                    }
                }
                GuardResult::RequireValidation {
                    reason,
                    explanation_code,
                    human_reason,
                } => {
                    requires_validation.push(GuardRequire {
                        reason,
                        explanation_code,
                        human_reason,
                    });

                    // Court-circuit si demande
                    if self.short_circuit == ShortCircuitStrategy::OnBlockOrRequire {
                        break;
                    }
                }
            }
        }

        GuardsEvaluation {
            passed: blocks.is_empty() && requires_validation.is_empty(),
            blocks,
            requires_validation,
            warnings,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::decision::{AgentMetrics, AgentState, SystemClock};
    use std::collections::HashMap;
    use time::macros::datetime;

    #[test]
    fn test_guard_result_severity() {
        assert_eq!(GuardResult::Pass.severity(), 0);
        assert_eq!(
            GuardResult::Warn {
                reason: "test".into(),
                explanation_code: "TEST".into(),
                human_reason: "test".into()
            }
            .severity(),
            1
        );
        assert_eq!(
            GuardResult::RequireValidation {
                reason: "test".into(),
                explanation_code: "TEST".into(),
                human_reason: "test".into()
            }
            .severity(),
            2
        );
        assert_eq!(
            GuardResult::Block {
                reason: "test".into(),
                explanation_code: "TEST".into(),
                human_reason: "test".into()
            }
            .severity(),
            3
        );
    }

    #[test]
    fn test_guard_result_combine() {
        let pass = GuardResult::Pass;
        let warn = GuardResult::Warn {
            reason: "warn".into(),
            explanation_code: "WARN".into(),
            human_reason: "warn".into(),
        };
        let block = GuardResult::Block {
            reason: "block".into(),
            explanation_code: "BLOCK".into(),
            human_reason: "block".into(),
        };

        // Block > Warn > Pass
        assert_eq!(block.clone().combine(pass.clone()).severity(), 3);
        assert_eq!(warn.clone().combine(pass.clone()).severity(), 1);
        assert_eq!(block.clone().combine(warn.clone()).severity(), 3);
    }

    #[test]
    fn test_time_window_guard_expired() {
        let clock = Arc::new(SystemClock);
        let guard = TimeWindowGuard::new(clock);

        let action = Action {
            action_type: "test".into(),
            agent_id: "agent1".into(),
            impact_level: ImpactLevel::Low,
            trace_id: "trace1".into(),
            expires_at: Some(datetime!(2020-01-01 00:00 UTC)), // Expired
            dry_run: false,
            expected_mode: None,
            expected_ssid: None,
        };

        let context = DecisionContext {
            mode: "veille".into(),
            ssid: "test".into(),
            agents: HashMap::new(),
        };

        let result = guard.evaluate(&action, &context);
        assert_eq!(result.severity(), 3); // Block
    }

    #[test]
    fn test_agent_health_guard_maintenance() {
        let guard = AgentHealthGuard;

        let action = Action {
            action_type: "shutdown".into(),
            agent_id: "agent1".into(),
            impact_level: ImpactLevel::Medium,
            trace_id: "trace1".into(),
            expires_at: None,
            dry_run: false,
            expected_mode: None,
            expected_ssid: None,
        };

        let mut agents = HashMap::new();
        agents.insert(
            "agent1".to_string(),
            AgentState {
                id: "agent1".into(),
                last_seen: datetime!(2025-11-07 10:00 UTC),
                metrics: AgentMetrics {
                    cpu_usage: 0.3,
                    memory_usage_percent: 0.5,
                },
                maintenance_mode: true,
                last_reconnect: None,
            },
        );

        let context = DecisionContext {
            mode: "veille".into(),
            ssid: "test".into(),
            agents,
        };

        let result = guard.evaluate(&action, &context);
        assert_eq!(result.severity(), 3); // Block
    }

    #[test]
    fn test_context_mode_guard_mismatch() {
        let guard = ContextModeGuard;

        let action = Action {
            action_type: "test".into(),
            agent_id: "agent1".into(),
            impact_level: ImpactLevel::Low,
            trace_id: "trace1".into(),
            expires_at: None,
            dry_run: false,
            expected_mode: Some("pro".into()),
            expected_ssid: None,
        };

        let context = DecisionContext {
            mode: "maison".into(), // Mismatch
            ssid: "test".into(),
            agents: HashMap::new(),
        };

        let result = guard.evaluate(&action, &context);
        assert_eq!(result.severity(), 2); // RequireValidation
    }

    #[test]
    fn test_guards_evaluator_short_circuit() {
        let clock = Arc::new(SystemClock);
        let mut evaluator = GuardsEvaluator::new(ShortCircuitStrategy::OnBlock);

        evaluator.add_guard(Box::new(TimeWindowGuard::new(clock)));
        evaluator.add_guard(Box::new(AgentHealthGuard));

        let action = Action {
            action_type: "test".into(),
            agent_id: "agent1".into(),
            impact_level: ImpactLevel::Low,
            trace_id: "trace1".into(),
            expires_at: Some(datetime!(2020-01-01 00:00 UTC)), // Expired → Block
            dry_run: false,
            expected_mode: None,
            expected_ssid: None,
        };

        let context = DecisionContext {
            mode: "veille".into(),
            ssid: "test".into(),
            agents: HashMap::new(),
        };

        let eval = evaluator.evaluate(&action, &context);
        assert!(!eval.passed);
        assert_eq!(eval.blocks.len(), 1); // TimeWindowGuard blocked
        // AgentHealthGuard pas execute grace au short-circuit
    }
}
