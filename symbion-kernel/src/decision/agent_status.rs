// Agent Health Status Manager with Hysteresis - 7 States
// Spec: PR3 P0 v3.1 REFINED

use crate::decision::{AgentState, AgentHealthMapping, Clock};
use parking_lot::RwLock;
use std::collections::HashMap;
use std::sync::Arc;
use time::OffsetDateTime;

/// État de santé agent (7 états avec hysteresis)
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AgentHealthStatus {
    Online,              // Agent connecté, métriques excellentes (score >= online_min)
    Active,              // Agent actif avec bonnes métriques (score >= active_min)
    Idle,                // Agent inactif mais OK (score >= idle_min)
    Degraded,            // Métriques dégradées (score >= degraded_min)
    ConsecutiveDegraded, // Degraded répété (X fois consécutives)
    Stale,               // Données trop anciennes (last_seen > stale_max_age)
    Offline,             // Agent déconnecté ou non trouvé
}

impl AgentHealthStatus {
    /// Score de santé 0.0-1.0 pour trust calculation
    pub fn trust_score(&self) -> f32 {
        match self {
            Self::Online => 1.0,
            Self::Active => 0.9,
            Self::Idle => 0.7,
            Self::Degraded => 0.5,
            Self::ConsecutiveDegraded => 0.3,
            Self::Stale => 0.2,
            Self::Offline => 0.0,
        }
    }

    /// Si l'état permet auto-approve (Online, Active)
    pub fn allows_auto_approve(&self) -> bool {
        matches!(self, Self::Online | Self::Active)
    }
}

/// Historique état agent pour hysteresis
#[derive(Debug, Clone)]
struct AgentHealthHistory {
    current_status: AgentHealthStatus,
    consecutive_degraded: u32,
    last_transition: OffsetDateTime,
}

/// Gestionnaire états de santé agents avec hysteresis
pub struct AgentHealthManager {
    histories: Arc<RwLock<HashMap<String, AgentHealthHistory>>>,
    clock: Arc<dyn Clock>,
    mapping: AgentHealthMapping,
}

impl AgentHealthManager {
    /// Créer nouveau gestionnaire
    pub fn new(clock: Arc<dyn Clock>, mapping: AgentHealthMapping) -> Self {
        Self {
            histories: Arc::new(RwLock::new(HashMap::new())),
            clock,
            mapping,
        }
    }

    /// Calculer état de santé agent
    pub fn evaluate_agent_health(
        &self,
        agent_id: &str,
        agent_state: Option<&AgentState>,
    ) -> AgentHealthStatus {
        let now = self.clock.now_utc();

        // Agent introuvable → Offline
        let agent = match agent_state {
            Some(a) => a,
            None => {
                self.update_history(agent_id, AgentHealthStatus::Offline, now);
                return AgentHealthStatus::Offline;
            }
        };

        // Données trop anciennes → Stale
        let age_seconds = (now - agent.last_seen).whole_seconds();
        if age_seconds > self.mapping.stale_max_age_secs as i64 {
            self.update_history(agent_id, AgentHealthStatus::Stale, now);
            return AgentHealthStatus::Stale;
        }

        // Calculer score santé basé sur métriques
        let health_score = self.calculate_health_score(&agent.metrics);

        // Déterminer état brut selon score
        let raw_status = self.score_to_status(health_score);

        // Appliquer hysteresis (consecutive degraded)
        let final_status = self.apply_hysteresis(agent_id, raw_status, now);

        self.update_history(agent_id, final_status, now);
        final_status
    }

    /// Calculer score santé 0.0-1.0 basé sur métriques agent
    fn calculate_health_score(&self, metrics: &crate::decision::AgentMetrics) -> f32 {
        // Inversion : high CPU/RAM = low health
        let cpu_health = 1.0 - metrics.cpu_usage;
        let ram_health = 1.0 - metrics.memory_usage_percent;

        // Moyenne pondérée CPU (60%) + RAM (40%)
        (cpu_health * 0.6 + ram_health * 0.4).clamp(0.0, 1.0)
    }

    /// Convertir score en état brut (avant hysteresis)
    fn score_to_status(&self, score: f32) -> AgentHealthStatus {
        if score >= self.mapping.online_min_score {
            AgentHealthStatus::Online
        } else if score >= self.mapping.active_min_score {
            AgentHealthStatus::Active
        } else if score >= self.mapping.idle_min_score {
            AgentHealthStatus::Idle
        } else if score >= self.mapping.degraded_min_score {
            AgentHealthStatus::Degraded
        } else {
            // Score très bas → ConsecutiveDegraded directement
            AgentHealthStatus::ConsecutiveDegraded
        }
    }

    /// Appliquer hysteresis pour éviter oscillations
    fn apply_hysteresis(
        &self,
        agent_id: &str,
        raw_status: AgentHealthStatus,
        now: OffsetDateTime,
    ) -> AgentHealthStatus {
        let mut histories = self.histories.write();

        let history = histories
            .entry(agent_id.to_string())
            .or_insert_with(|| AgentHealthHistory {
                current_status: raw_status,
                consecutive_degraded: 0,
                last_transition: now,
            });

        // Si degraded → incrémenter compteur
        if raw_status == AgentHealthStatus::Degraded {
            history.consecutive_degraded += 1;

            // Si seuil atteint → ConsecutiveDegraded
            if history.consecutive_degraded >= self.mapping.degraded_consecutive_threshold {
                return AgentHealthStatus::ConsecutiveDegraded;
            }

            return AgentHealthStatus::Degraded;
        }

        // Si retour à un état sain → reset compteur
        if matches!(
            raw_status,
            AgentHealthStatus::Online | AgentHealthStatus::Active | AgentHealthStatus::Idle
        ) {
            history.consecutive_degraded = 0;
        }

        raw_status
    }

    /// Mettre à jour historique agent
    fn update_history(
        &self,
        agent_id: &str,
        status: AgentHealthStatus,
        now: OffsetDateTime,
    ) {
        let mut histories = self.histories.write();

        histories
            .entry(agent_id.to_string())
            .and_modify(|h| {
                if h.current_status != status {
                    println!(
                        "[agent_status] {} transitioned from {:?} to {:?}",
                        agent_id, h.current_status, status
                    );
                    h.last_transition = now;
                }
                h.current_status = status;
            })
            .or_insert_with(|| AgentHealthHistory {
                current_status: status,
                consecutive_degraded: 0,
                last_transition: now,
            });
    }

    /// Obtenir état actuel agent
    pub fn get_current_status(&self, agent_id: &str) -> Option<AgentHealthStatus> {
        self.histories
            .read()
            .get(agent_id)
            .map(|h| h.current_status)
    }

    /// Obtenir nombre d'agents par état
    pub fn stats(&self) -> AgentHealthStats {
        let histories = self.histories.read();

        let mut stats = AgentHealthStats {
            total_agents: histories.len(),
            online: 0,
            active: 0,
            idle: 0,
            degraded: 0,
            consecutive_degraded: 0,
            stale: 0,
            offline: 0,
        };

        for history in histories.values() {
            match history.current_status {
                AgentHealthStatus::Online => stats.online += 1,
                AgentHealthStatus::Active => stats.active += 1,
                AgentHealthStatus::Idle => stats.idle += 1,
                AgentHealthStatus::Degraded => stats.degraded += 1,
                AgentHealthStatus::ConsecutiveDegraded => stats.consecutive_degraded += 1,
                AgentHealthStatus::Stale => stats.stale += 1,
                AgentHealthStatus::Offline => stats.offline += 1,
            }
        }

        stats
    }

    /// Nettoyer historique agents offline depuis longtemps
    pub fn cleanup_offline(&self, max_age_secs: u64) -> usize {
        let now = self.clock.now_utc();
        let threshold = now - time::Duration::seconds(max_age_secs as i64);

        let mut histories = self.histories.write();

        let before = histories.len();

        histories.retain(|_, h| {
            // Garder si online ou transition récente
            h.current_status != AgentHealthStatus::Offline || h.last_transition > threshold
        });

        let removed = before - histories.len();

        if removed > 0 {
            println!("[agent_status] Cleaned up {} offline agents", removed);
        }

        removed
    }
}

/// Statistiques états santé agents
#[derive(Debug, Clone)]
pub struct AgentHealthStats {
    pub total_agents: usize,
    pub online: usize,
    pub active: usize,
    pub idle: usize,
    pub degraded: usize,
    pub consecutive_degraded: usize,
    pub stale: usize,
    pub offline: usize,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::decision::{AgentMetrics, AgentState, MockClock, SystemClock};
    use time::macros::datetime;

    fn create_test_agent(cpu: f32, ram: f32, last_seen: OffsetDateTime) -> AgentState {
        AgentState {
            id: "test-agent".into(),
            last_seen,
            metrics: AgentMetrics {
                cpu_usage: cpu,
                memory_usage_percent: ram,
            },
            maintenance_mode: false,
            last_reconnect: None,
        }
    }

    fn default_mapping() -> AgentHealthMapping {
        AgentHealthMapping {
            online_min_score: 0.9,
            active_min_score: 0.85,
            idle_min_score: 0.7,
            degraded_min_score: 0.5,
            degraded_consecutive_threshold: 3,
            stale_max_age_secs: 300, // 5 minutes
        }
    }

    #[test]
    fn test_health_status_trust_scores() {
        assert_eq!(AgentHealthStatus::Online.trust_score(), 1.0);
        assert_eq!(AgentHealthStatus::Active.trust_score(), 0.9);
        assert_eq!(AgentHealthStatus::Idle.trust_score(), 0.7);
        assert_eq!(AgentHealthStatus::Degraded.trust_score(), 0.5);
        assert_eq!(AgentHealthStatus::ConsecutiveDegraded.trust_score(), 0.3);
        assert_eq!(AgentHealthStatus::Stale.trust_score(), 0.2);
        assert_eq!(AgentHealthStatus::Offline.trust_score(), 0.0);
    }

    #[test]
    fn test_allows_auto_approve() {
        assert!(AgentHealthStatus::Online.allows_auto_approve());
        assert!(AgentHealthStatus::Active.allows_auto_approve());
        assert!(!AgentHealthStatus::Idle.allows_auto_approve());
        assert!(!AgentHealthStatus::Degraded.allows_auto_approve());
    }

    #[test]
    fn test_calculate_health_score() {
        let clock = Arc::new(SystemClock);
        let manager = AgentHealthManager::new(clock, default_mapping());

        // Low usage = high health
        let metrics_good = AgentMetrics {
            cpu_usage: 0.1,
            memory_usage_percent: 0.2,
        };
        let score_good = manager.calculate_health_score(&metrics_good);
        assert!(score_good >= 0.8); // 0.9*0.6 + 0.8*0.4 = 0.86

        // High usage = low health
        let metrics_bad = AgentMetrics {
            cpu_usage: 0.9,
            memory_usage_percent: 0.85,
        };
        let score_bad = manager.calculate_health_score(&metrics_bad);
        assert!(score_bad <= 0.2); // 0.1*0.6 + 0.15*0.4 = 0.12
    }

    #[test]
    fn test_score_to_status() {
        let clock = Arc::new(SystemClock);
        let manager = AgentHealthManager::new(clock, default_mapping());

        assert_eq!(
            manager.score_to_status(0.95),
            AgentHealthStatus::Online
        );
        assert_eq!(
            manager.score_to_status(0.87),
            AgentHealthStatus::Active
        );
        assert_eq!(manager.score_to_status(0.75), AgentHealthStatus::Idle);
        assert_eq!(
            manager.score_to_status(0.55),
            AgentHealthStatus::Degraded
        );
        assert_eq!(
            manager.score_to_status(0.2),
            AgentHealthStatus::ConsecutiveDegraded
        );
    }

    #[test]
    fn test_agent_offline() {
        let clock = Arc::new(SystemClock);
        let manager = AgentHealthManager::new(clock, default_mapping());

        // Agent introuvable
        let status = manager.evaluate_agent_health("missing-agent", None);
        assert_eq!(status, AgentHealthStatus::Offline);
    }

    #[test]
    fn test_agent_stale() {
        let now = SystemClock.now_utc();
        let old_time = now - time::Duration::minutes(10); // 10 minutes ago

        let clock = Arc::new(SystemClock);
        let manager = AgentHealthManager::new(clock, default_mapping());

        let agent = create_test_agent(0.1, 0.1, old_time);

        let status = manager.evaluate_agent_health("test-agent", Some(&agent));
        assert_eq!(status, AgentHealthStatus::Stale);
    }

    #[test]
    fn test_agent_online() {
        let now = SystemClock.now_utc();

        let clock = Arc::new(SystemClock);
        let manager = AgentHealthManager::new(clock, default_mapping());

        // CPU 5%, RAM 5% → score = (0.95*0.6 + 0.95*0.4) = 0.95 → Online
        let agent = create_test_agent(0.05, 0.05, now);

        let status = manager.evaluate_agent_health("test-agent", Some(&agent));
        assert_eq!(status, AgentHealthStatus::Online);
    }

    #[test]
    fn test_agent_degraded() {
        let now = SystemClock.now_utc();

        let clock = Arc::new(SystemClock);
        let manager = AgentHealthManager::new(clock, default_mapping());

        // CPU 50%, RAM 45% → score ~0.52 → Degraded
        let agent = create_test_agent(0.5, 0.45, now);

        let status = manager.evaluate_agent_health("test-agent", Some(&agent));
        assert_eq!(status, AgentHealthStatus::Degraded);
    }

    #[test]
    fn test_hysteresis_consecutive_degraded() {
        let now = SystemClock.now_utc();

        let clock = Arc::new(SystemClock);
        let manager = AgentHealthManager::new(clock, default_mapping());

        // CPU 50%, RAM 45% → Degraded
        let agent = create_test_agent(0.5, 0.45, now);

        // Première évaluation → Degraded
        let status1 = manager.evaluate_agent_health("test-agent", Some(&agent));
        assert_eq!(status1, AgentHealthStatus::Degraded);

        // Deuxième évaluation → Degraded
        let status2 = manager.evaluate_agent_health("test-agent", Some(&agent));
        assert_eq!(status2, AgentHealthStatus::Degraded);

        // Troisième évaluation → ConsecutiveDegraded (seuil = 3)
        let status3 = manager.evaluate_agent_health("test-agent", Some(&agent));
        assert_eq!(status3, AgentHealthStatus::ConsecutiveDegraded);
    }

    #[test]
    fn test_hysteresis_reset() {
        let now = SystemClock.now_utc();

        let clock = Arc::new(SystemClock);
        let manager = AgentHealthManager::new(clock, default_mapping());

        // Degraded
        let agent_degraded = create_test_agent(0.5, 0.45, now);
        manager.evaluate_agent_health("test-agent", Some(&agent_degraded));
        manager.evaluate_agent_health("test-agent", Some(&agent_degraded));

        // Retour à Online → reset compteur
        let agent_online = create_test_agent(0.1, 0.1, now);
        let status = manager.evaluate_agent_health("test-agent", Some(&agent_online));
        assert_eq!(status, AgentHealthStatus::Online);

        // Re-degraded → devrait repartir de 0
        let status2 = manager.evaluate_agent_health("test-agent", Some(&agent_degraded));
        assert_eq!(status2, AgentHealthStatus::Degraded); // Pas ConsecutiveDegraded
    }

    #[test]
    fn test_get_current_status() {
        let now = SystemClock.now_utc();

        let clock = Arc::new(SystemClock);
        let manager = AgentHealthManager::new(clock, default_mapping());

        // Avant évaluation
        assert_eq!(manager.get_current_status("test-agent"), None);

        // Après évaluation
        let agent = create_test_agent(0.1, 0.1, now);
        manager.evaluate_agent_health("test-agent", Some(&agent));

        assert_eq!(
            manager.get_current_status("test-agent"),
            Some(AgentHealthStatus::Online)
        );
    }

    #[test]
    fn test_stats() {
        let now = SystemClock.now_utc();

        let clock = Arc::new(SystemClock);
        let manager = AgentHealthManager::new(clock, default_mapping());

        // Créer 3 agents différents
        let agent_online = create_test_agent(0.1, 0.1, now);
        let agent_degraded = create_test_agent(0.5, 0.45, now);

        manager.evaluate_agent_health("agent1", Some(&agent_online));
        manager.evaluate_agent_health("agent2", Some(&agent_degraded));
        manager.evaluate_agent_health("agent3", None); // Offline

        let stats = manager.stats();
        assert_eq!(stats.total_agents, 3);
        assert_eq!(stats.online, 1);
        assert_eq!(stats.degraded, 1);
        assert_eq!(stats.offline, 1);
    }

    #[test]
    fn test_cleanup_offline() {
        let initial = datetime!(2025-11-07 10:00 UTC);
        let clock = MockClock::new(initial);
        let clock_arc = Arc::new(clock);
        let manager = AgentHealthManager::new(clock_arc.clone(), default_mapping());

        // Agent offline
        manager.evaluate_agent_health("offline-agent", None);

        assert_eq!(manager.stats().total_agents, 1);

        // Avancer temps de 10 minutes
        clock_arc.advance(time::Duration::minutes(10));

        // Cleanup offline > 5 minutes
        let cleaned = manager.cleanup_offline(300); // 5 minutes
        assert_eq!(cleaned, 1);
        assert_eq!(manager.stats().total_agents, 0);
    }

    #[test]
    fn test_cleanup_keeps_recent_offline() {
        let clock = Arc::new(SystemClock);
        let manager = AgentHealthManager::new(clock, default_mapping());

        // Agent offline récent
        manager.evaluate_agent_health("offline-agent", None);

        // Cleanup immédiat → devrait garder (récent)
        let cleaned = manager.cleanup_offline(300);
        assert_eq!(cleaned, 0);
        assert_eq!(manager.stats().total_agents, 1);
    }
}
