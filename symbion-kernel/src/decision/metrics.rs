// Decision Engine Metrics - Prometheus Export
// Spec: PR3 P0 v3.1 REFINED - Observability

use crate::decision::{
    AuditStats, DecisionOutcome, OverrideStats, ValidationStats, AgentHealthStats,
};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;

/// Compteurs métriques Decision Engine
pub struct DecisionMetrics {
    // Compteurs totaux
    decisions_total: Arc<AtomicU64>,
    decisions_approved: Arc<AtomicU64>,
    decisions_blocked: Arc<AtomicU64>,
    decisions_require_validation: Arc<AtomicU64>,
    decisions_dry_run: Arc<AtomicU64>,

    // Compteurs guards
    guards_passed: Arc<AtomicU64>,
    guards_blocked: Arc<AtomicU64>,
    guards_warnings: Arc<AtomicU64>,

    // Compteurs overrides
    overrides_created: Arc<AtomicU64>,
    overrides_revoked: Arc<AtomicU64>,

    // Compteurs validations
    validations_created: Arc<AtomicU64>,
    validations_approved: Arc<AtomicU64>,
    validations_denied: Arc<AtomicU64>,
    validations_expired: Arc<AtomicU64>,
}

impl DecisionMetrics {
    /// Créer nouveau compteur de métriques
    pub fn new() -> Self {
        Self {
            decisions_total: Arc::new(AtomicU64::new(0)),
            decisions_approved: Arc::new(AtomicU64::new(0)),
            decisions_blocked: Arc::new(AtomicU64::new(0)),
            decisions_require_validation: Arc::new(AtomicU64::new(0)),
            decisions_dry_run: Arc::new(AtomicU64::new(0)),
            guards_passed: Arc::new(AtomicU64::new(0)),
            guards_blocked: Arc::new(AtomicU64::new(0)),
            guards_warnings: Arc::new(AtomicU64::new(0)),
            overrides_created: Arc::new(AtomicU64::new(0)),
            overrides_revoked: Arc::new(AtomicU64::new(0)),
            validations_created: Arc::new(AtomicU64::new(0)),
            validations_approved: Arc::new(AtomicU64::new(0)),
            validations_denied: Arc::new(AtomicU64::new(0)),
            validations_expired: Arc::new(AtomicU64::new(0)),
        }
    }

    /// Incrémenter compteur decision selon outcome
    pub fn record_decision(&self, outcome: &DecisionOutcome) {
        self.decisions_total.fetch_add(1, Ordering::Relaxed);

        match outcome {
            DecisionOutcome::Approved { .. } => {
                self.decisions_approved.fetch_add(1, Ordering::Relaxed);
            }
            DecisionOutcome::Blocked { .. } => {
                self.decisions_blocked.fetch_add(1, Ordering::Relaxed);
            }
            DecisionOutcome::RequireValidation { .. } => {
                self.decisions_require_validation
                    .fetch_add(1, Ordering::Relaxed);
            }
            DecisionOutcome::DryRun { .. } => {
                self.decisions_dry_run.fetch_add(1, Ordering::Relaxed);
            }
        }
    }

    /// Incrémenter guards passed
    pub fn record_guards_passed(&self) {
        self.guards_passed.fetch_add(1, Ordering::Relaxed);
    }

    /// Incrémenter guards blocked
    pub fn record_guards_blocked(&self) {
        self.guards_blocked.fetch_add(1, Ordering::Relaxed);
    }

    /// Incrémenter guards warnings
    pub fn record_guards_warning(&self) {
        self.guards_warnings.fetch_add(1, Ordering::Relaxed);
    }

    /// Incrémenter override created
    pub fn record_override_created(&self) {
        self.overrides_created.fetch_add(1, Ordering::Relaxed);
    }

    /// Incrémenter override revoked
    pub fn record_override_revoked(&self) {
        self.overrides_revoked.fetch_add(1, Ordering::Relaxed);
    }

    /// Incrémenter validation created
    pub fn record_validation_created(&self) {
        self.validations_created.fetch_add(1, Ordering::Relaxed);
    }

    /// Incrémenter validation approved
    pub fn record_validation_approved(&self) {
        self.validations_approved.fetch_add(1, Ordering::Relaxed);
    }

    /// Incrémenter validation denied
    pub fn record_validation_denied(&self) {
        self.validations_denied.fetch_add(1, Ordering::Relaxed);
    }

    /// Incrémenter validation expired
    pub fn record_validation_expired(&self) {
        self.validations_expired.fetch_add(1, Ordering::Relaxed);
    }

    /// Getters pour accès lecture seule (PR4 P1 - JSON metrics API)
    pub fn get_decisions_total(&self) -> u64 {
        self.decisions_total.load(Ordering::Relaxed)
    }

    pub fn get_decisions_approved(&self) -> u64 {
        self.decisions_approved.load(Ordering::Relaxed)
    }

    pub fn get_decisions_blocked(&self) -> u64 {
        self.decisions_blocked.load(Ordering::Relaxed)
    }

    /// Exporter métriques au format Prometheus
    pub fn export_prometheus(
        &self,
        audit_stats: &AuditStats,
        validation_stats: &ValidationStats,
        override_stats: &OverrideStats,
        agent_health_stats: &AgentHealthStats,
    ) -> String {
        let mut output = String::new();

        // Header
        output.push_str("# HELP symbion_decision_total Total number of decisions made\n");
        output.push_str("# TYPE symbion_decision_total counter\n");
        output.push_str(&format!(
            "symbion_decision_total {}\n",
            self.decisions_total.load(Ordering::Relaxed)
        ));

        // Decisions par type
        output.push_str("# HELP symbion_decision_approved_total Number of approved decisions\n");
        output.push_str("# TYPE symbion_decision_approved_total counter\n");
        output.push_str(&format!(
            "symbion_decision_approved_total {}\n",
            self.decisions_approved.load(Ordering::Relaxed)
        ));

        output.push_str("# HELP symbion_decision_blocked_total Number of blocked decisions\n");
        output.push_str("# TYPE symbion_decision_blocked_total counter\n");
        output.push_str(&format!(
            "symbion_decision_blocked_total {}\n",
            self.decisions_blocked.load(Ordering::Relaxed)
        ));

        output.push_str(
            "# HELP symbion_decision_require_validation_total Number of decisions requiring validation\n",
        );
        output.push_str("# TYPE symbion_decision_require_validation_total counter\n");
        output.push_str(&format!(
            "symbion_decision_require_validation_total {}\n",
            self.decisions_require_validation.load(Ordering::Relaxed)
        ));

        output.push_str("# HELP symbion_decision_dry_run_total Number of dry-run decisions\n");
        output.push_str("# TYPE symbion_decision_dry_run_total counter\n");
        output.push_str(&format!(
            "symbion_decision_dry_run_total {}\n",
            self.decisions_dry_run.load(Ordering::Relaxed)
        ));

        // Guards
        output.push_str("# HELP symbion_guards_passed_total Number of guards passed\n");
        output.push_str("# TYPE symbion_guards_passed_total counter\n");
        output.push_str(&format!(
            "symbion_guards_passed_total {}\n",
            self.guards_passed.load(Ordering::Relaxed)
        ));

        output.push_str("# HELP symbion_guards_blocked_total Number of guards blocked\n");
        output.push_str("# TYPE symbion_guards_blocked_total counter\n");
        output.push_str(&format!(
            "symbion_guards_blocked_total {}\n",
            self.guards_blocked.load(Ordering::Relaxed)
        ));

        output.push_str("# HELP symbion_guards_warnings_total Number of guards warnings\n");
        output.push_str("# TYPE symbion_guards_warnings_total counter\n");
        output.push_str(&format!(
            "symbion_guards_warnings_total {}\n",
            self.guards_warnings.load(Ordering::Relaxed)
        ));

        // Overrides
        output.push_str("# HELP symbion_overrides_created_total Number of overrides created\n");
        output.push_str("# TYPE symbion_overrides_created_total counter\n");
        output.push_str(&format!(
            "symbion_overrides_created_total {}\n",
            self.overrides_created.load(Ordering::Relaxed)
        ));

        output.push_str("# HELP symbion_overrides_active Current number of active overrides\n");
        output.push_str("# TYPE symbion_overrides_active gauge\n");
        output.push_str(&format!("symbion_overrides_active {}\n", override_stats.active));

        // Validations
        output.push_str("# HELP symbion_validations_created_total Number of validations created\n");
        output.push_str("# TYPE symbion_validations_created_total counter\n");
        output.push_str(&format!(
            "symbion_validations_created_total {}\n",
            self.validations_created.load(Ordering::Relaxed)
        ));

        output.push_str("# HELP symbion_validations_pending Current number of pending validations\n");
        output.push_str("# TYPE symbion_validations_pending gauge\n");
        output.push_str(&format!(
            "symbion_validations_pending {}\n",
            validation_stats.pending
        ));

        output.push_str("# HELP symbion_validations_approved_total Number of validations approved\n");
        output.push_str("# TYPE symbion_validations_approved_total counter\n");
        output.push_str(&format!(
            "symbion_validations_approved_total {}\n",
            self.validations_approved.load(Ordering::Relaxed)
        ));

        output.push_str("# HELP symbion_validations_denied_total Number of validations denied\n");
        output.push_str("# TYPE symbion_validations_denied_total counter\n");
        output.push_str(&format!(
            "symbion_validations_denied_total {}\n",
            self.validations_denied.load(Ordering::Relaxed)
        ));

        // Audit
        output.push_str("# HELP symbion_audit_records Current number of audit records\n");
        output.push_str("# TYPE symbion_audit_records gauge\n");
        output.push_str(&format!(
            "symbion_audit_records {}\n",
            audit_stats.total_records
        ));

        output.push_str("# HELP symbion_audit_usage_percent Audit queue usage percentage\n");
        output.push_str("# TYPE symbion_audit_usage_percent gauge\n");
        output.push_str(&format!(
            "symbion_audit_usage_percent {:.2}\n",
            audit_stats.usage_percent
        ));

        // Agent Health
        output.push_str("# HELP symbion_agents_total Total number of agents tracked\n");
        output.push_str("# TYPE symbion_agents_total gauge\n");
        output.push_str(&format!(
            "symbion_agents_total {}\n",
            agent_health_stats.total_agents
        ));

        output.push_str(
            "# HELP symbion_agents_online Number of agents in Online state\n",
        );
        output.push_str("# TYPE symbion_agents_online gauge\n");
        output.push_str(&format!(
            "symbion_agents_online {}\n",
            agent_health_stats.online
        ));

        output.push_str(
            "# HELP symbion_agents_active Number of agents in Active state\n",
        );
        output.push_str("# TYPE symbion_agents_active gauge\n");
        output.push_str(&format!(
            "symbion_agents_active {}\n",
            agent_health_stats.active
        ));

        output.push_str(
            "# HELP symbion_agents_degraded Number of agents in Degraded state\n",
        );
        output.push_str("# TYPE symbion_agents_degraded gauge\n");
        output.push_str(&format!(
            "symbion_agents_degraded {}\n",
            agent_health_stats.degraded
        ));

        output.push_str(
            "# HELP symbion_agents_offline Number of agents in Offline state\n",
        );
        output.push_str("# TYPE symbion_agents_offline gauge\n");
        output.push_str(&format!(
            "symbion_agents_offline {}\n",
            agent_health_stats.offline
        ));

        output
    }

    /// Reset tous les compteurs (pour tests)
    #[cfg(test)]
    pub fn reset(&self) {
        self.decisions_total.store(0, Ordering::Relaxed);
        self.decisions_approved.store(0, Ordering::Relaxed);
        self.decisions_blocked.store(0, Ordering::Relaxed);
        self.decisions_require_validation.store(0, Ordering::Relaxed);
        self.decisions_dry_run.store(0, Ordering::Relaxed);
        self.guards_passed.store(0, Ordering::Relaxed);
        self.guards_blocked.store(0, Ordering::Relaxed);
        self.guards_warnings.store(0, Ordering::Relaxed);
        self.overrides_created.store(0, Ordering::Relaxed);
        self.overrides_revoked.store(0, Ordering::Relaxed);
        self.validations_created.store(0, Ordering::Relaxed);
        self.validations_approved.store(0, Ordering::Relaxed);
        self.validations_denied.store(0, Ordering::Relaxed);
        self.validations_expired.store(0, Ordering::Relaxed);
    }
}

impl Default for DecisionMetrics {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_metrics_new() {
        let metrics = DecisionMetrics::new();
        assert_eq!(metrics.decisions_total.load(Ordering::Relaxed), 0);
        assert_eq!(metrics.decisions_approved.load(Ordering::Relaxed), 0);
    }

    #[test]
    fn test_record_decision_approved() {
        let metrics = DecisionMetrics::new();

        let outcome = DecisionOutcome::Approved {
            trust_score: 0.9,
            auto: true,
        };

        metrics.record_decision(&outcome);

        assert_eq!(metrics.decisions_total.load(Ordering::Relaxed), 1);
        assert_eq!(metrics.decisions_approved.load(Ordering::Relaxed), 1);
        assert_eq!(metrics.decisions_blocked.load(Ordering::Relaxed), 0);
    }

    #[test]
    fn test_record_decision_blocked() {
        let metrics = DecisionMetrics::new();

        let outcome = DecisionOutcome::Blocked {
            reasons: vec!["test".into()],
            explanation_codes: vec!["TEST".into()],
        };

        metrics.record_decision(&outcome);

        assert_eq!(metrics.decisions_total.load(Ordering::Relaxed), 1);
        assert_eq!(metrics.decisions_blocked.load(Ordering::Relaxed), 1);
        assert_eq!(metrics.decisions_approved.load(Ordering::Relaxed), 0);
    }

    #[test]
    fn test_record_decision_require_validation() {
        let metrics = DecisionMetrics::new();

        let outcome = DecisionOutcome::RequireValidation {
            trust_score: 0.5,
            threshold: 0.7,
            reasons: vec!["test".into()],
            explanation_codes: vec!["TEST".into()],
            human_reasons: vec!["Human reason".into()],
        };

        metrics.record_decision(&outcome);

        assert_eq!(metrics.decisions_total.load(Ordering::Relaxed), 1);
        assert_eq!(
            metrics
                .decisions_require_validation
                .load(Ordering::Relaxed),
            1
        );
    }

    #[test]
    fn test_record_guards() {
        let metrics = DecisionMetrics::new();

        metrics.record_guards_passed();
        metrics.record_guards_blocked();
        metrics.record_guards_warning();
        metrics.record_guards_warning();

        assert_eq!(metrics.guards_passed.load(Ordering::Relaxed), 1);
        assert_eq!(metrics.guards_blocked.load(Ordering::Relaxed), 1);
        assert_eq!(metrics.guards_warnings.load(Ordering::Relaxed), 2);
    }

    #[test]
    fn test_record_overrides() {
        let metrics = DecisionMetrics::new();

        metrics.record_override_created();
        metrics.record_override_created();
        metrics.record_override_revoked();

        assert_eq!(metrics.overrides_created.load(Ordering::Relaxed), 2);
        assert_eq!(metrics.overrides_revoked.load(Ordering::Relaxed), 1);
    }

    #[test]
    fn test_record_validations() {
        let metrics = DecisionMetrics::new();

        metrics.record_validation_created();
        metrics.record_validation_approved();
        metrics.record_validation_denied();
        metrics.record_validation_expired();

        assert_eq!(metrics.validations_created.load(Ordering::Relaxed), 1);
        assert_eq!(metrics.validations_approved.load(Ordering::Relaxed), 1);
        assert_eq!(metrics.validations_denied.load(Ordering::Relaxed), 1);
        assert_eq!(metrics.validations_expired.load(Ordering::Relaxed), 1);
    }

    #[test]
    fn test_export_prometheus_format() {
        let metrics = DecisionMetrics::new();

        // Record some metrics
        metrics.record_decision(&DecisionOutcome::Approved {
            trust_score: 0.9,
            auto: true,
        });
        metrics.record_guards_passed();

        let audit_stats = AuditStats {
            total_records: 10,
            capacity: 100,
            usage_percent: 10.0,
            approved: 5,
            blocked: 3,
            require_validation: 2,
            dry_run: 0,
        };

        let validation_stats = ValidationStats {
            total: 5,
            pending: 2,
            approved: 2,
            denied: 1,
            expired: 0,
        };

        let override_stats = OverrideStats {
            total: 3,
            active: 2,
            expired: 1,
            force_approve: 2,
            force_deny: 1,
        };

        let agent_health_stats = AgentHealthStats {
            total_agents: 4,
            online: 2,
            active: 1,
            idle: 0,
            degraded: 0,
            consecutive_degraded: 0,
            stale: 0,
            offline: 1,
        };

        let output = metrics.export_prometheus(
            &audit_stats,
            &validation_stats,
            &override_stats,
            &agent_health_stats,
        );

        // Vérifier format Prometheus
        assert!(output.contains("# HELP symbion_decision_total"));
        assert!(output.contains("# TYPE symbion_decision_total counter"));
        assert!(output.contains("symbion_decision_total 1"));
        assert!(output.contains("symbion_decision_approved_total 1"));
        assert!(output.contains("symbion_guards_passed_total 1"));
        assert!(output.contains("symbion_audit_records 10"));
        assert!(output.contains("symbion_validations_pending 2"));
        assert!(output.contains("symbion_overrides_active 2"));
        assert!(output.contains("symbion_agents_online 2"));
    }

    #[test]
    fn test_metrics_reset() {
        let metrics = DecisionMetrics::new();

        metrics.record_decision(&DecisionOutcome::Approved {
            trust_score: 0.9,
            auto: true,
        });
        assert_eq!(metrics.decisions_total.load(Ordering::Relaxed), 1);

        metrics.reset();
        assert_eq!(metrics.decisions_total.load(Ordering::Relaxed), 0);
    }

    #[test]
    fn test_multiple_decisions() {
        let metrics = DecisionMetrics::new();

        // 3 approved
        for _ in 0..3 {
            metrics.record_decision(&DecisionOutcome::Approved {
                trust_score: 0.9,
                auto: true,
            });
        }

        // 2 blocked
        for _ in 0..2 {
            metrics.record_decision(&DecisionOutcome::Blocked {
                reasons: vec!["test".into()],
                explanation_codes: vec!["TEST".into()],
            });
        }

        assert_eq!(metrics.decisions_total.load(Ordering::Relaxed), 5);
        assert_eq!(metrics.decisions_approved.load(Ordering::Relaxed), 3);
        assert_eq!(metrics.decisions_blocked.load(Ordering::Relaxed), 2);
    }
}
