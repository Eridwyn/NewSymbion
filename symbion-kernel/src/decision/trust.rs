// Trust Score Calculator
// Spec: PR3 P0 v3.1 REFINED

use crate::decision::{
    Action, DecisionConfig, DecisionContext, TrustCriteria, TrustScore, Clock,
};
use std::sync::Arc;

/// Calculateur de trust score
pub struct TrustCalculator {
    config: DecisionConfig,
    clock: Arc<dyn Clock>,
}

impl TrustCalculator {
    pub fn new(config: DecisionConfig, clock: Arc<dyn Clock>) -> Self {
        Self { config, clock }
    }

    /// Calculer le trust score complet
    pub fn calculate(
        &self,
        action: &Action,
        context: &DecisionContext,
    ) -> TrustScore {
        let breakdown = TrustCriteria {
            context_match: self.calculate_context_match(action, context),
            temporal_consistency: self.calculate_temporal_consistency(action),
            agent_health: self.calculate_agent_health(&action.agent_id, context),
            recent_success_rate: self.calculate_recent_success_rate(&action.agent_id),
            user_approval_history: self.calculate_user_approval_history(&action.action_type),
        };

        let score = self.weighted_score(&breakdown);

        TrustScore {
            score,
            breakdown,
            config_version: self.config.version,
        }
    }

    /// Calcule le score pondéré
    fn weighted_score(&self, criteria: &TrustCriteria) -> f32 {
        let weights = &self.config.trust_weights;

        (criteria.context_match * weights.context_match)
            + (criteria.temporal_consistency * weights.temporal_consistency)
            + (criteria.agent_health * weights.agent_health)
            + (criteria.recent_success_rate * weights.recent_success_rate)
            + (criteria.user_approval_history * weights.user_approval_history)
    }

    /// Critère 1: Correspondance contexte (mode + SSID)
    fn calculate_context_match(&self, action: &Action, context: &DecisionContext) -> f32 {
        let mut score: f32 = 1.0;

        // Vérifier expected_mode
        if let Some(ref expected_mode) = action.expected_mode {
            if expected_mode.to_lowercase() != context.mode.to_lowercase() {
                score -= 0.5; // Pénalité mode mismatch
            }
        }

        // Vérifier expected_ssid
        if let Some(ref expected_ssid) = action.expected_ssid {
            if expected_ssid != &context.ssid {
                score -= 0.5; // Pénalité SSID mismatch
            }
        }

        score.max(0.0)
    }

    /// Critère 2: Cohérence temporelle
    fn calculate_temporal_consistency(&self, action: &Action) -> f32 {
        let now = self.clock.now_utc();

        // Si action a un TTL
        if let Some(expires_at) = action.expires_at {
            let time_until_expiry = expires_at - now;
            let total_seconds = time_until_expiry.whole_seconds();

            if total_seconds <= 0 {
                return 0.0; // Expiré
            }

            // Score basé sur le temps restant
            // Plus de 1h restant = 1.0
            // Moins de 5 min restant = 0.5
            if total_seconds > 3600 {
                1.0
            } else if total_seconds > 300 {
                0.5 + (total_seconds as f32 / 7200.0)
            } else {
                0.5
            }
        } else {
            // Pas de TTL = cohérence maximale
            1.0
        }
    }

    /// Critère 3: Santé de l'agent
    fn calculate_agent_health(&self, agent_id: &str, context: &DecisionContext) -> f32 {
        let agent_state = match context.agents.get(agent_id) {
            Some(state) => state,
            None => return 0.0, // Agent inconnu
        };

        // Agent en maintenance = 0
        if agent_state.maintenance_mode {
            return 0.0;
        }

        let now = self.clock.now_utc();
        let time_since_seen = now - agent_state.last_seen;
        let seconds_since_seen = time_since_seen.whole_seconds();

        // Agent stale (> 5 min) = pénalité
        let freshness_score = if seconds_since_seen > self.config.agent_health_mapping.stale_max_age_secs as i64 {
            0.3
        } else if seconds_since_seen > 60 {
            0.7
        } else {
            1.0
        };

        // Métriques CPU/RAM
        let cpu_score = 1.0 - agent_state.metrics.cpu_usage;
        let ram_score = 1.0 - agent_state.metrics.memory_usage_percent;

        // Moyenne pondérée
        (freshness_score * 0.4 + cpu_score * 0.3 + ram_score * 0.3).max(0.0)
    }

    /// Critère 4: Taux de succès récent (placeholder)
    /// TODO: Nécessite historique de décisions
    fn calculate_recent_success_rate(&self, _agent_id: &str) -> f32 {
        // Pour l'instant, retourne score neutre
        // Sera implémenté avec persistence.rs
        0.7
    }

    /// Critère 5: Historique approbations utilisateur (placeholder)
    /// TODO: Nécessite historique de décisions
    fn calculate_user_approval_history(&self, _action_type: &str) -> f32 {
        // Pour l'instant, retourne score neutre
        // Sera implémenté avec persistence.rs
        0.7
    }

    /// Met à jour la configuration
    pub fn update_config(&mut self, config: DecisionConfig) {
        self.config = config;
    }

    /// Obtenir la configuration actuelle
    pub fn config(&self) -> &DecisionConfig {
        &self.config
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::decision::{AgentMetrics, AgentState, ImpactLevel, SystemClock, Clock};
    use std::collections::HashMap;
    use time::macros::datetime;

    #[test]
    fn test_context_match_perfect() {
        let config = DecisionConfig::default();
        let clock = Arc::new(SystemClock);
        let calculator = TrustCalculator::new(config, clock.clone());

        let action = Action {
            action_type: "test".into(),
            agent_id: "agent1".into(),
            impact_level: ImpactLevel::Low,
            trace_id: "trace1".into(),
            expires_at: None,
            dry_run: false,
            expected_mode: Some("intime".into()),
            expected_ssid: Some("home-wifi".into()),
        };

        let context = DecisionContext {
            mode: "intime".into(),
            ssid: "home-wifi".into(),
            agents: HashMap::new(),
        };

        let score = calculator.calculate_context_match(&action, &context);
        assert_eq!(score, 1.0);
    }

    #[test]
    fn test_context_match_mode_mismatch() {
        let config = DecisionConfig::default();
        let clock = Arc::new(SystemClock);
        let calculator = TrustCalculator::new(config, clock.clone());

        let action = Action {
            action_type: "test".into(),
            agent_id: "agent1".into(),
            impact_level: ImpactLevel::Low,
            trace_id: "trace1".into(),
            expires_at: None,
            dry_run: false,
            expected_mode: Some("cravate".into()),
            expected_ssid: None,
        };

        let context = DecisionContext {
            mode: "intime".into(),
            ssid: "home-wifi".into(),
            agents: HashMap::new(),
        };

        let score = calculator.calculate_context_match(&action, &context);
        assert_eq!(score, 0.5); // Pénalité mode mismatch
    }

    #[test]
    fn test_agent_health_perfect() {
        let config = DecisionConfig::default();
        let clock = Arc::new(SystemClock);
        let calculator = TrustCalculator::new(config, clock.clone());

        let mut agents = HashMap::new();
        agents.insert(
            "agent1".to_string(),
            AgentState {
                id: "agent1".into(),
                last_seen: clock.now_utc(),
                metrics: AgentMetrics {
                    cpu_usage: 0.1,
                    memory_usage_percent: 0.2,
                },
                maintenance_mode: false,
                last_reconnect: None,
            },
        );

        let context = DecisionContext {
            mode: "intime".into(),
            ssid: "home-wifi".into(),
            agents,
        };

        let score = calculator.calculate_agent_health("agent1", &context);
        assert!(score > 0.8); // Bon état
    }

    #[test]
    fn test_agent_health_maintenance() {
        let config = DecisionConfig::default();
        let clock = Arc::new(SystemClock);
        let calculator = TrustCalculator::new(config, clock);

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
                maintenance_mode: true, // Maintenance
                last_reconnect: None,
            },
        );

        let context = DecisionContext {
            mode: "intime".into(),
            ssid: "home-wifi".into(),
            agents,
        };

        let score = calculator.calculate_agent_health("agent1", &context);
        assert_eq!(score, 0.0); // Maintenance = 0
    }

    #[test]
    fn test_full_trust_score_calculation() {
        let config = DecisionConfig::default();
        let clock = Arc::new(SystemClock);
        let calculator = TrustCalculator::new(config, clock.clone());

        let action = Action {
            action_type: "shutdown".into(),
            agent_id: "agent1".into(),
            impact_level: ImpactLevel::Medium,
            trace_id: "trace1".into(),
            expires_at: None,
            dry_run: false,
            expected_mode: Some("intime".into()),
            expected_ssid: Some("home-wifi".into()),
        };

        let mut agents = HashMap::new();
        agents.insert(
            "agent1".to_string(),
            AgentState {
                id: "agent1".into(),
                last_seen: clock.now_utc(),
                metrics: AgentMetrics {
                    cpu_usage: 0.2,
                    memory_usage_percent: 0.3,
                },
                maintenance_mode: false,
                last_reconnect: None,
            },
        );

        let context = DecisionContext {
            mode: "intime".into(),
            ssid: "home-wifi".into(),
            agents,
        };

        let trust_score = calculator.calculate(&action, &context);

        assert!(trust_score.score > 0.7); // Bon trust score global
        assert_eq!(trust_score.config_version, 1);
        assert_eq!(trust_score.breakdown.context_match, 1.0);
    }
}
