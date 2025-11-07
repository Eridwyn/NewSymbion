// Anti-Replay Idempotence Manager
// Spec: PR3 P0 v3.1 REFINED

use crate::decision::Clock;
use parking_lot::RwLock;
use std::collections::{BTreeMap, HashSet};
use std::sync::Arc;
use time::OffsetDateTime;

/// Gestionnaire d'idempotence anti-replay
pub struct IdempotenceManager {
    // BTreeMap indexé par expires_at pour cleanup efficace
    // expires_at -> Set<trace_id>
    traces: Arc<RwLock<BTreeMap<OffsetDateTime, HashSet<String>>>>,
    clock: Arc<dyn Clock>,
}

impl IdempotenceManager {
    /// Créer un nouveau gestionnaire
    pub fn new(clock: Arc<dyn Clock>) -> Self {
        Self {
            traces: Arc::new(RwLock::new(BTreeMap::new())),
            clock,
        }
    }

    /// Vérifier si un trace_id a déjà été vu (et l'enregistrer si non)
    /// Retourne true si c'est un replay (déjà vu), false sinon
    pub fn check_and_register(&self, trace_id: &str, expires_at: OffsetDateTime) -> bool {
        let mut traces = self.traces.write();

        // Cleanup automatique des expirés avant check
        self.cleanup_expired_internal(&mut traces);

        // Chercher dans tous les buckets si trace_id existe déjà
        for (_, trace_set) in traces.iter() {
            if trace_set.contains(trace_id) {
                return true; // Replay détecté
            }
        }

        // Pas de replay, enregistrer
        traces
            .entry(expires_at)
            .or_insert_with(HashSet::new)
            .insert(trace_id.to_string());

        false // Première fois vu
    }

    /// Nettoyer manuellement les trace_id expirés
    pub fn cleanup_expired(&self) {
        let mut traces = self.traces.write();
        self.cleanup_expired_internal(&mut traces);
    }

    /// Cleanup interne (lock déjà acquis)
    fn cleanup_expired_internal(&self, traces: &mut BTreeMap<OffsetDateTime, HashSet<String>>) {
        let now = self.clock.now_utc();

        // BTreeMap::split_off() retire tous les éléments >= now
        // On garde seulement les futurs
        let future_traces = traces.split_off(&now);

        // Remplacer par les traces futures uniquement
        *traces = future_traces;
    }

    /// Obtenir le nombre de trace_id actifs (non expirés)
    pub fn active_count(&self) -> usize {
        let traces = self.traces.read();
        traces.values().map(|set| set.len()).sum()
    }

    /// Obtenir le nombre de buckets temporels
    pub fn bucket_count(&self) -> usize {
        let traces = self.traces.read();
        traces.len()
    }

    /// Vérifier si un trace_id est présent (sans l'enregistrer)
    pub fn contains(&self, trace_id: &str) -> bool {
        let traces = self.traces.read();
        traces.values().any(|set| set.contains(trace_id))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::decision::{MockClock, SystemClock};
    use time::macros::datetime;

    #[test]
    fn test_idempotence_first_time() {
        let clock = Arc::new(SystemClock);
        let manager = IdempotenceManager::new(clock.clone());

        let expires_at = clock.now_utc() + time::Duration::minutes(5);
        let is_replay = manager.check_and_register("trace-123", expires_at);

        assert!(!is_replay); // Première fois = pas replay
        assert_eq!(manager.active_count(), 1);
    }

    #[test]
    fn test_idempotence_replay_detected() {
        let clock = Arc::new(SystemClock);
        let manager = IdempotenceManager::new(clock.clone());

        let expires_at = clock.now_utc() + time::Duration::minutes(5);

        // Premier appel
        let is_replay_1 = manager.check_and_register("trace-123", expires_at);
        assert!(!is_replay_1);

        // Deuxième appel avec même trace_id = replay
        let is_replay_2 = manager.check_and_register("trace-123", expires_at);
        assert!(is_replay_2);

        // Count reste à 1 (pas de doublon)
        assert_eq!(manager.active_count(), 1);
    }

    #[test]
    fn test_idempotence_different_trace_ids() {
        let clock = Arc::new(SystemClock);
        let manager = IdempotenceManager::new(clock.clone());

        let expires_at = clock.now_utc() + time::Duration::minutes(5);

        // Deux trace_id différents
        let is_replay_1 = manager.check_and_register("trace-123", expires_at);
        let is_replay_2 = manager.check_and_register("trace-456", expires_at);

        assert!(!is_replay_1);
        assert!(!is_replay_2);
        assert_eq!(manager.active_count(), 2);
    }

    #[test]
    fn test_idempotence_cleanup_expired() {
        let initial = datetime!(2025-11-07 10:00 UTC);
        let clock = MockClock::new(initial);
        let clock_arc = Arc::new(clock);
        let manager = IdempotenceManager::new(clock_arc.clone());

        // Enregistrer un trace qui expire dans 1 minute
        let expires_at = initial + time::Duration::minutes(1);
        manager.check_and_register("trace-123", expires_at);
        assert_eq!(manager.active_count(), 1);

        // Avancer le temps de 2 minutes (après expiration)
        clock_arc.advance(time::Duration::minutes(2));

        // Cleanup manuel
        manager.cleanup_expired();

        // Trace_id devrait être supprimé
        assert_eq!(manager.active_count(), 0);
    }

    #[test]
    fn test_idempotence_automatic_cleanup_on_check() {
        let initial = datetime!(2025-11-07 10:00 UTC);
        let clock = MockClock::new(initial);
        let clock_arc = Arc::new(clock);
        let manager = IdempotenceManager::new(clock_arc.clone());

        // Enregistrer trace qui expire dans 1 min
        let expires_at_1 = initial + time::Duration::minutes(1);
        manager.check_and_register("trace-old", expires_at_1);
        assert_eq!(manager.active_count(), 1);

        // Avancer temps de 2 min
        clock_arc.advance(time::Duration::minutes(2));

        // Nouveau check avec nouveau trace (cleanup auto)
        let expires_at_2 = clock_arc.now_utc() + time::Duration::minutes(5);
        manager.check_and_register("trace-new", expires_at_2);

        // Seul le nouveau devrait rester
        assert_eq!(manager.active_count(), 1);
        assert!(manager.contains("trace-new"));
        assert!(!manager.contains("trace-old"));
    }

    #[test]
    fn test_idempotence_multiple_buckets() {
        let clock = Arc::new(SystemClock);
        let manager = IdempotenceManager::new(clock.clone());

        let now = clock.now_utc();

        // 3 traces avec expirations différentes
        manager.check_and_register("trace-1", now + time::Duration::minutes(1));
        manager.check_and_register("trace-2", now + time::Duration::minutes(2));
        manager.check_and_register("trace-3", now + time::Duration::minutes(3));

        assert_eq!(manager.active_count(), 3);
        assert_eq!(manager.bucket_count(), 3);
    }

    #[test]
    fn test_idempotence_same_bucket() {
        let clock = Arc::new(SystemClock);
        let manager = IdempotenceManager::new(clock.clone());

        let expires_at = clock.now_utc() + time::Duration::minutes(5);

        // 3 traces avec même expiration = même bucket
        manager.check_and_register("trace-1", expires_at);
        manager.check_and_register("trace-2", expires_at);
        manager.check_and_register("trace-3", expires_at);

        assert_eq!(manager.active_count(), 3);
        assert_eq!(manager.bucket_count(), 1); // Un seul bucket
    }

    #[test]
    fn test_idempotence_contains() {
        let clock = Arc::new(SystemClock);
        let manager = IdempotenceManager::new(clock.clone());

        let expires_at = clock.now_utc() + time::Duration::minutes(5);
        manager.check_and_register("trace-123", expires_at);

        assert!(manager.contains("trace-123"));
        assert!(!manager.contains("trace-999"));
    }
}
