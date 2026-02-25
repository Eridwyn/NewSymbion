// Audit Trail Manager - Bounded Queue with Auto-Rotation
// Spec: PR3 P0 v3.1 REFINED - CORRECTION 3

use crate::decision::{DecisionRecord, Clock};
use parking_lot::RwLock;
use std::collections::VecDeque;
use std::sync::Arc;

/// Gestionnaire d'audit trail avec bounded queue
pub struct AuditManager {
    records: Arc<RwLock<VecDeque<DecisionRecord>>>,
    max_capacity: usize,
    clock: Arc<dyn Clock>,
}

impl AuditManager {
    /// Créer un nouveau gestionnaire avec capacité max
    pub fn new(clock: Arc<dyn Clock>, max_capacity: usize) -> Self {
        Self {
            records: Arc::new(RwLock::new(VecDeque::with_capacity(max_capacity))),
            max_capacity,
            clock,
        }
    }

    /// Ajouter un record (avec auto-rotation si plein)
    pub fn add_record(&self, record: DecisionRecord) {
        let mut records = self.records.write();

        // Si capacité max atteinte, supprimer le plus ancien (FIFO)
        if records.len() >= self.max_capacity {
            records.pop_front();
            println!("[audit] Queue full, rotated oldest record (FIFO)");
        }

        records.push_back(record);
    }

    /// Obtenir tous les records (copie)
    pub fn get_all(&self) -> Vec<DecisionRecord> {
        self.records.read().iter().cloned().collect()
    }

    /// Obtenir les N derniers records
    pub fn get_last(&self, n: usize) -> Vec<DecisionRecord> {
        let records = self.records.read();
        let start = records.len().saturating_sub(n);
        records.iter().skip(start).cloned().collect()
    }

    /// Obtenir records depuis un timestamp
    pub fn get_since(&self, since: time::OffsetDateTime) -> Vec<DecisionRecord> {
        self.records
            .read()
            .iter()
            .filter(|r| r.timestamp >= since)
            .cloned()
            .collect()
    }

    /// Obtenir records par agent_id
    pub fn get_by_agent(&self, agent_id: &str) -> Vec<DecisionRecord> {
        self.records
            .read()
            .iter()
            .filter(|r| r.agent_id == agent_id)
            .cloned()
            .collect()
    }

    /// Obtenir records par trace_id
    pub fn get_by_trace(&self, trace_id: &str) -> Vec<DecisionRecord> {
        self.records
            .read()
            .iter()
            .filter(|r| r.trace_id == trace_id)
            .cloned()
            .collect()
    }

    /// Obtenir nombre de records actuels
    pub fn count(&self) -> usize {
        self.records.read().len()
    }

    /// Obtenir capacité max
    pub fn capacity(&self) -> usize {
        self.max_capacity
    }

    /// Vider la queue (pour maintenance)
    pub fn clear(&self) {
        let mut records = self.records.write();
        let count = records.len();
        records.clear();
        println!("[audit] Cleared {} records", count);
    }

    /// Obtenir statistiques audit
    pub fn stats(&self) -> AuditStats {
        let records = self.records.read();

        let mut stats = AuditStats {
            total_records: records.len(),
            capacity: self.max_capacity,
            usage_percent: (records.len() as f32 / self.max_capacity as f32) * 100.0,
            approved: 0,
            blocked: 0,
            require_validation: 0,
            dry_run: 0,
        };

        for record in records.iter() {
            match record.outcome {
                crate::decision::DecisionOutcome::Approved { .. } => stats.approved += 1,
                crate::decision::DecisionOutcome::Blocked { .. } => stats.blocked += 1,
                crate::decision::DecisionOutcome::RequireValidation { .. } => {
                    stats.require_validation += 1
                }
                crate::decision::DecisionOutcome::DryRun { .. } => stats.dry_run += 1,
            }
        }

        stats
    }
}

/// Statistiques audit trail
#[derive(Debug, Clone, serde::Serialize, utoipa::ToSchema)]
pub struct AuditStats {
    pub total_records: usize,
    pub capacity: usize,
    pub usage_percent: f32,
    pub approved: usize,
    pub blocked: usize,
    pub require_validation: usize,
    pub dry_run: usize,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::decision::{
        DecisionOutcome, DecisionRecord, ImpactLevel, SystemClock,
    };
    use uuid::Uuid;

    fn create_test_record(agent_id: &str, trace_id: &str) -> DecisionRecord {
        DecisionRecord {
            decision_id: Uuid::new_v4().to_string(),
            trace_id: trace_id.to_string(),
            action_type: "test_action".to_string(),
            agent_id: agent_id.to_string(),
            impact_level: ImpactLevel::Low,
            outcome: DecisionOutcome::Approved {
                trust_score: 0.85,
                auto: true,
            },
            trust_score: None,
            timestamp: SystemClock.now_utc(),
            config_version: 1,
        }
    }

    #[test]
    fn test_audit_add_record() {
        let clock = Arc::new(SystemClock);
        let audit = AuditManager::new(clock, 100);

        let record = create_test_record("agent1", "trace1");
        audit.add_record(record.clone());

        assert_eq!(audit.count(), 1);

        let all = audit.get_all();
        assert_eq!(all.len(), 1);
        assert_eq!(all[0].decision_id, record.decision_id);
    }

    #[test]
    fn test_audit_bounded_queue_rotation() {
        let clock = Arc::new(SystemClock);
        let audit = AuditManager::new(clock, 5); // Capacité max = 5

        // Ajouter 7 records
        for i in 0..7 {
            let record = create_test_record("agent1", &format!("trace{}", i));
            audit.add_record(record);
        }

        // Devrait avoir seulement 5 (les 2 plus anciens supprimés)
        assert_eq!(audit.count(), 5);
        assert_eq!(audit.capacity(), 5);

        // Les records devraient être trace2 à trace6 (FIFO)
        let all = audit.get_all();
        assert!(all[0].trace_id.contains("2")); // Premier = trace2
        assert!(all[4].trace_id.contains("6")); // Dernier = trace6
    }

    #[test]
    fn test_audit_get_last() {
        let clock = Arc::new(SystemClock);
        let audit = AuditManager::new(clock, 100);

        // Ajouter 10 records
        for i in 0..10 {
            let record = create_test_record("agent1", &format!("trace{}", i));
            audit.add_record(record);
        }

        // Obtenir les 3 derniers
        let last_3 = audit.get_last(3);
        assert_eq!(last_3.len(), 3);
        assert!(last_3[0].trace_id.contains("7"));
        assert!(last_3[2].trace_id.contains("9"));
    }

    #[test]
    fn test_audit_get_by_agent() {
        let clock = Arc::new(SystemClock);
        let audit = AuditManager::new(clock, 100);

        // Ajouter records de différents agents
        audit.add_record(create_test_record("agent1", "trace1"));
        audit.add_record(create_test_record("agent2", "trace2"));
        audit.add_record(create_test_record("agent1", "trace3"));
        audit.add_record(create_test_record("agent3", "trace4"));

        // Filtrer par agent1
        let agent1_records = audit.get_by_agent("agent1");
        assert_eq!(agent1_records.len(), 2);

        let agent2_records = audit.get_by_agent("agent2");
        assert_eq!(agent2_records.len(), 1);
    }

    #[test]
    fn test_audit_get_by_trace() {
        let clock = Arc::new(SystemClock);
        let audit = AuditManager::new(clock, 100);

        audit.add_record(create_test_record("agent1", "trace-ABC"));
        audit.add_record(create_test_record("agent2", "trace-XYZ"));
        audit.add_record(create_test_record("agent3", "trace-ABC"));

        let abc_records = audit.get_by_trace("trace-ABC");
        assert_eq!(abc_records.len(), 2);

        let xyz_records = audit.get_by_trace("trace-XYZ");
        assert_eq!(xyz_records.len(), 1);
    }

    #[test]
    fn test_audit_get_since() {
        let clock = Arc::new(SystemClock);
        let audit = AuditManager::new(clock, 100);

        let now = SystemClock.now_utc();

        // Ajouter record récent
        let mut recent = create_test_record("agent1", "recent");
        recent.timestamp = now;
        audit.add_record(recent);

        // Ajouter record ancien
        let mut old = create_test_record("agent2", "old");
        old.timestamp = now - time::Duration::hours(2);
        audit.add_record(old);

        // Obtenir depuis 1h
        let since = now - time::Duration::hours(1);
        let recent_records = audit.get_since(since);
        assert_eq!(recent_records.len(), 1);
        assert_eq!(recent_records[0].trace_id, "recent");
    }

    #[test]
    fn test_audit_clear() {
        let clock = Arc::new(SystemClock);
        let audit = AuditManager::new(clock, 100);

        // Ajouter 5 records
        for i in 0..5 {
            audit.add_record(create_test_record("agent1", &format!("trace{}", i)));
        }

        assert_eq!(audit.count(), 5);

        // Clear
        audit.clear();
        assert_eq!(audit.count(), 0);
    }

    #[test]
    fn test_audit_stats() {
        let clock = Arc::new(SystemClock);
        let audit = AuditManager::new(clock, 10);

        // Ajouter 3 approved
        for _ in 0..3 {
            let mut record = create_test_record("agent1", "trace");
            record.outcome = DecisionOutcome::Approved {
                trust_score: 0.9,
                auto: true,
            };
            audit.add_record(record);
        }

        // Ajouter 1 blocked
        let mut blocked = create_test_record("agent1", "trace");
        blocked.outcome = DecisionOutcome::Blocked {
            reasons: vec!["test".into()],
            explanation_codes: vec!["TEST".into()],
            categories: vec![],
        };
        audit.add_record(blocked);

        let stats = audit.stats();
        assert_eq!(stats.total_records, 4);
        assert_eq!(stats.capacity, 10);
        assert_eq!(stats.usage_percent, 40.0);
        assert_eq!(stats.approved, 3);
        assert_eq!(stats.blocked, 1);
    }

    #[test]
    fn test_audit_usage_percent() {
        let clock = Arc::new(SystemClock);
        let audit = AuditManager::new(clock, 100);

        // Ajouter 50 records
        for i in 0..50 {
            audit.add_record(create_test_record("agent1", &format!("trace{}", i)));
        }

        let stats = audit.stats();
        assert_eq!(stats.usage_percent, 50.0);
    }
}
