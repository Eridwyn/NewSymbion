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
        // En heure LOCALE : en UTC la fenêtre glissait à 0h/1h-8h/9h selon la
        // saison, et laissait passer 23h-minuit.
        if action.impact_level == ImpactLevel::VeryHigh {
            let hour = crate::intelligence::to_local(now).hour();
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
        // « system » n'est pas un agent : c'est le kernel lui-même (plugins,
        // notifications, modes). Il n'est jamais dans le registre — le traiter
        // comme un agent inconnu exigerait une validation pour TOUTE action.
        if action.agent_id == "system" {
            return GuardResult::Pass;
        }

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

/// Commandes d'agent qui coupent le PC sous les mains de quelqu'un.
/// Les noms sont ceux que l'agent EXÉCUTE (symbion-agent-host, PowerHandler),
/// plus les alias que la table d'impact connaît — un garde-fou qui ne
/// reconnaît pas `hibernate` laisse passer `hibernate`.
const COMMANDES_PERTURBANTES_AGENT: &[&str] =
    &["shutdown", "reboot", "restart", "hibernate", "sleep"];

/// Plugins dont les actions dérangent physiquement quelqu'un dans la maison.
/// Seules les actions déclarées au moins Medium comptent : « arrêter et
/// renvoyer à la base » (Low) doit toujours pouvoir passer.
const PLUGINS_PERTURBANTS: &[&str] = &["robot"];

/// Guard présence : on ne lance pas l'aspirateur ni n'éteint un PC chez
/// quelqu'un sans lui demander.
///
/// Le score de confiance ne regarde pas la présence : il mesure qu'une action
/// du même type a souvent RÉUSSI, pas qu'elle tombe au bon moment. Ce guard
/// est donc la seule barrière entre « une règle se déclenche » et « le robot
/// démarre pendant qu'on est là ».
///
/// Présence inconnue (feature absente ou expirée, Freebox muette) = on
/// demande. Ne pas savoir n'est pas « personne ».
pub struct PresenceGuard;

impl PresenceGuard {
    fn perturbe(action: &Action) -> bool {
        let Some(cible) = action.target.as_deref() else {
            return false;
        };
        if let Some(commande) = cible.strip_prefix("agent:") {
            return COMMANDES_PERTURBANTES_AGENT.contains(&commande);
        }
        if let Some(chemin) = cible.strip_prefix("plugin:") {
            let plugin = chemin.split('/').next().unwrap_or("");
            return PLUGINS_PERTURBANTS.contains(&plugin)
                && action.impact_level != ImpactLevel::Low;
        }
        false
    }
}

impl Guard for PresenceGuard {
    fn priority(&self) -> u32 {
        85
    }

    fn name(&self) -> &str {
        "PresenceGuard"
    }

    fn evaluate(&self, action: &Action, context: &DecisionContext) -> GuardResult {
        if !Self::perturbe(action) {
            return GuardResult::Pass;
        }
        let cible = action.target.as_deref().unwrap_or("");
        match context.anyone_home {
            Some(false) => GuardResult::Pass,
            Some(true) => GuardResult::RequireValidation {
                reason: format!("Someone is home, disruptive action {}", cible),
                explanation_code: "GUARD.PRESENCE.SOMEONE_HOME".to_string(),
                human_reason: format!(
                    "Quelqu'un est \u{e0} la maison \u{2014} {} demande ton accord", cible
                ),
            },
            None => GuardResult::RequireValidation {
                reason: format!("Presence unknown, disruptive action {}", cible),
                explanation_code: "GUARD.PRESENCE.UNKNOWN".to_string(),
                human_reason: format!(
                    "Pr\u{e9}sence inconnue (Freebox muette ?) \u{2014} {} demande ton accord", cible
                ),
            },
        }
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
            target: None,
        };

        let context = DecisionContext {
            mode: "veille".into(),
            ssid: "test".into(),
            agents: HashMap::new(),
            anyone_home: None,
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
            target: None,
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
            anyone_home: None,
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
            target: None,
        };

        let context = DecisionContext {
            mode: "maison".into(), // Mismatch
            ssid: "test".into(),
            agents: HashMap::new(),
            anyone_home: None,
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
            target: None,
        };

        let context = DecisionContext {
            mode: "veille".into(),
            ssid: "test".into(),
            agents: HashMap::new(),
            anyone_home: None,
        };

        let eval = evaluator.evaluate(&action, &context);
        assert!(!eval.passed);
        assert_eq!(eval.blocks.len(), 1); // TimeWindowGuard blocked
        // AgentHealthGuard pas execute grace au short-circuit
    }

    fn action_vers(target: &str, impact: ImpactLevel, agent_id: &str) -> Action {
        Action {
            action_type: "automation.test".into(),
            agent_id: agent_id.into(),
            impact_level: impact,
            trace_id: "trace".into(),
            expires_at: None,
            dry_run: false,
            expected_mode: None,
            expected_ssid: None,
            target: Some(target.into()),
        }
    }

    fn contexte(anyone_home: Option<bool>) -> DecisionContext {
        DecisionContext {
            mode: "maison".into(),
            ssid: "test".into(),
            agents: HashMap::new(),
            anyone_home,
        }
    }

    #[test]
    fn presence_robot_demande_si_quelquun_est_la() {
        let a = action_vers("plugin:robot/actions/clean_all", ImpactLevel::Medium, "system");
        assert_eq!(PresenceGuard.evaluate(&a, &contexte(Some(true))).severity(), 2);
        assert_eq!(PresenceGuard.evaluate(&a, &contexte(Some(false))).severity(), 0);
    }

    #[test]
    fn presence_inconnue_nest_pas_personne() {
        let a = action_vers("plugin:robot/actions/clean_rooms", ImpactLevel::Medium, "system");
        match PresenceGuard.evaluate(&a, &contexte(None)) {
            GuardResult::RequireValidation { explanation_code, .. } => {
                assert_eq!(explanation_code, "GUARD.PRESENCE.UNKNOWN")
            }
            autre => panic!("présence inconnue doit demander, pas {:?}", autre),
        }
    }

    #[test]
    fn presence_laisse_passer_le_retour_a_la_base() {
        // Low : renvoyer le robot doit TOUJOURS pouvoir passer.
        let a = action_vers("plugin:robot/actions/stop_and_dock", ImpactLevel::Low, "system");
        assert_eq!(PresenceGuard.evaluate(&a, &contexte(Some(true))).severity(), 0);
    }

    #[test]
    fn presence_couvre_les_vrais_noms_de_commandes_agent() {
        // Le niveau d'impact est déclaratif : un shutdown annoncé Low doit
        // quand même être retenu. Et hibernate/reboot sont ce que l'agent
        // exécute réellement.
        for cmd in ["shutdown", "reboot", "hibernate", "restart", "sleep"] {
            let a = action_vers(&format!("agent:{}", cmd), ImpactLevel::Low, "pc-bureau");
            assert_eq!(PresenceGuard.evaluate(&a, &contexte(Some(true))).severity(), 2, "{}", cmd);
        }
        let notify = action_vers("agent:notify", ImpactLevel::Medium, "pc-bureau");
        assert_eq!(PresenceGuard.evaluate(&notify, &contexte(Some(true))).severity(), 0);
    }

    #[test]
    fn presence_ignore_les_autres_plugins() {
        let lampes = action_vers("plugin:lights/actions/all_off", ImpactLevel::Medium, "system");
        assert_eq!(PresenceGuard.evaluate(&lampes, &contexte(Some(true))).severity(), 0);
        let telegram = action_vers("plugin:telegram/broadcast-summary", ImpactLevel::Medium, "system");
        assert_eq!(PresenceGuard.evaluate(&telegram, &contexte(None)).severity(), 0);
    }

    #[test]
    fn agent_health_ne_traite_pas_system_comme_inconnu() {
        let a = action_vers("plugin:lights/actions/all_off", ImpactLevel::Medium, "system");
        assert_eq!(AgentHealthGuard.evaluate(&a, &contexte(None)).severity(), 0);
        let b = action_vers("agent:notify", ImpactLevel::Medium, "pc-fantome");
        assert_eq!(AgentHealthGuard.evaluate(&b, &contexte(None)).severity(), 2);
    }

    #[test]
    fn la_nuit_se_compte_en_heure_locale() {
        // 21h30 UTC en été = 23h30 à Paris : c'est la nuit.
        let soir = TimeWindowGuard::new(Arc::new(crate::decision::MockClock::new(
            datetime!(2026-07-15 21:30 UTC))));
        let a = action_vers("agent:shutdown", ImpactLevel::VeryHigh, "pc-bureau");
        assert_eq!(soir.evaluate(&a, &contexte(None)).severity(), 2);
        // 5h30 UTC en été = 7h30 à Paris : c'est le jour.
        let matin = TimeWindowGuard::new(Arc::new(crate::decision::MockClock::new(
            datetime!(2026-07-15 05:30 UTC))));
        assert_eq!(matin.evaluate(&a, &contexte(None)).severity(), 0);
    }
}
