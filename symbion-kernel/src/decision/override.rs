// Master Override Manager - Force Decision with MFA
// Spec: PR3 P0 v3.1 REFINED

use crate::decision::Clock;
use anyhow::{Context, Result, bail};
use parking_lot::RwLock;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use time::OffsetDateTime;
use uuid::Uuid;

/// Type d'override
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum OverrideType {
    ForceApprove,  // Forcer approbation
    ForceDeny,     // Forcer refus
}

/// Override actif
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MasterOverride {
    pub override_id: String,
    pub override_type: OverrideType,
    pub decision_id: String,
    pub reason: String,
    pub created_by: String,  // username
    pub created_at: OffsetDateTime,
    pub expires_at: OffsetDateTime,
    pub mfa_verified: bool,  // MFA obligatoire
}

/// Gestionnaire de Master Overrides
pub struct OverrideManager {
    overrides: Arc<RwLock<HashMap<String, MasterOverride>>>,
    clock: Arc<dyn Clock>,
    default_ttl_seconds: i64,
}

impl OverrideManager {
    /// Créer un nouveau gestionnaire
    pub fn new(clock: Arc<dyn Clock>, default_ttl_seconds: i64) -> Self {
        Self {
            overrides: Arc::new(RwLock::new(HashMap::new())),
            clock,
            default_ttl_seconds,
        }
    }

    /// Créer un override (MFA obligatoire)
    pub fn create_override(
        &self,
        decision_id: &str,
        override_type: OverrideType,
        reason: &str,
        username: &str,
        mfa_verified: bool,
    ) -> Result<MasterOverride> {
        // MFA OBLIGATOIRE pour Master Override
        if !mfa_verified {
            bail!("MFA verification required for Master Override");
        }

        // Vérifier raison non vide
        if reason.trim().is_empty() {
            bail!("Override reason cannot be empty");
        }

        let override_id = Uuid::new_v4().to_string();
        let now = self.clock.now_utc();
        let expires_at = now + time::Duration::seconds(self.default_ttl_seconds);

        let override_entry = MasterOverride {
            override_id: override_id.clone(),
            override_type: override_type.clone(),
            decision_id: decision_id.to_string(),
            reason: reason.to_string(),
            created_by: username.to_string(),
            created_at: now,
            expires_at,
            mfa_verified,
        };

        // Stocker override
        self.overrides.write().insert(override_id.clone(), override_entry.clone());

        println!(
            "[override] Created {:?} override {} for decision {} by {} (MFA verified, expires in {}s)",
            override_type, override_id, decision_id, username, self.default_ttl_seconds
        );

        Ok(override_entry)
    }

    /// Vérifier si un override existe pour une décision
    pub fn get_override_for_decision(&self, decision_id: &str) -> Option<MasterOverride> {
        let now = self.clock.now_utc();
        self.overrides
            .read()
            .values()
            .find(|o| o.decision_id == decision_id && o.expires_at > now)
            .cloned()
    }

    /// Obtenir un override par ID
    pub fn get_override(&self, override_id: &str) -> Option<MasterOverride> {
        self.overrides.read().get(override_id).cloned()
    }

    /// Révoquer un override manuellement (MFA requis)
    pub fn revoke_override(
        &self,
        override_id: &str,
        username: &str,
        mfa_verified: bool,
    ) -> Result<()> {
        // MFA obligatoire pour révocation
        if !mfa_verified {
            bail!("MFA verification required to revoke override");
        }

        let mut overrides = self.overrides.write();

        let override_entry = overrides
            .get(override_id)
            .with_context(|| format!("Override {} not found", override_id))?;

        // Vérifier pas déjà expiré
        let now = self.clock.now_utc();
        if now > override_entry.expires_at {
            bail!("Override {} already expired", override_id);
        }

        overrides.remove(override_id);

        println!(
            "[override] Revoked override {} by {} (MFA verified)",
            override_id, username
        );

        Ok(())
    }

    /// Lister tous les overrides actifs
    pub fn list_active(&self) -> Vec<MasterOverride> {
        let now = self.clock.now_utc();
        self.overrides
            .read()
            .values()
            .filter(|o| o.expires_at > now)
            .cloned()
            .collect()
    }

    /// Lister overrides par utilisateur
    pub fn list_by_user(&self, username: &str) -> Vec<MasterOverride> {
        self.overrides
            .read()
            .values()
            .filter(|o| o.created_by == username)
            .cloned()
            .collect()
    }

    /// Nettoyer overrides expirés
    /// Retourne le nombre d'overrides supprimés
    pub fn cleanup_expired(&self) -> usize {
        let now = self.clock.now_utc();
        let mut overrides = self.overrides.write();

        let expired_ids: Vec<String> = overrides
            .iter()
            .filter(|(_, o)| o.expires_at <= now)
            .map(|(id, _)| id.clone())
            .collect();

        let count = expired_ids.len();

        for id in expired_ids {
            overrides.remove(&id);
        }

        if count > 0 {
            println!("[override] Cleaned up {} expired overrides", count);
        }

        count
    }

    /// Obtenir statistiques
    pub fn stats(&self) -> OverrideStats {
        let overrides = self.overrides.read();
        let now = self.clock.now_utc();

        let mut stats = OverrideStats {
            total: overrides.len(),
            active: 0,
            expired: 0,
            force_approve: 0,
            force_deny: 0,
        };

        for o in overrides.values() {
            if o.expires_at > now {
                stats.active += 1;
            } else {
                stats.expired += 1;
            }

            match o.override_type {
                OverrideType::ForceApprove => stats.force_approve += 1,
                OverrideType::ForceDeny => stats.force_deny += 1,
            }
        }

        stats
    }

    /// Obtenir nombre d'overrides actifs
    pub fn active_count(&self) -> usize {
        let now = self.clock.now_utc();
        self.overrides
            .read()
            .values()
            .filter(|o| o.expires_at > now)
            .count()
    }
}

/// Statistiques overrides
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OverrideStats {
    pub total: usize,
    pub active: usize,
    pub expired: usize,
    pub force_approve: usize,
    pub force_deny: usize,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::decision::{MockClock, SystemClock};
    use time::macros::datetime;

    #[test]
    fn test_override_create_with_mfa() {
        let clock = Arc::new(SystemClock);
        let manager = OverrideManager::new(clock, 300);

        let override_entry = manager
            .create_override(
                "decision-123",
                OverrideType::ForceApprove,
                "Emergency maintenance",
                "admin",
                true,  // MFA verified
            )
            .expect("Failed to create override");

        assert_eq!(override_entry.decision_id, "decision-123");
        assert_eq!(override_entry.override_type, OverrideType::ForceApprove);
        assert_eq!(override_entry.created_by, "admin");
        assert!(override_entry.mfa_verified);

        // Vérifier stockage
        let stored = manager.get_override(&override_entry.override_id);
        assert!(stored.is_some());
    }

    #[test]
    fn test_override_create_without_mfa_fails() {
        let clock = Arc::new(SystemClock);
        let manager = OverrideManager::new(clock, 300);

        // Sans MFA devrait échouer
        let result = manager.create_override(
            "decision-123",
            OverrideType::ForceApprove,
            "Emergency",
            "admin",
            false,  // MFA NOT verified
        );

        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("MFA verification required"));
    }

    #[test]
    fn test_override_empty_reason_fails() {
        let clock = Arc::new(SystemClock);
        let manager = OverrideManager::new(clock, 300);

        // Raison vide devrait échouer
        let result = manager.create_override(
            "decision-123",
            OverrideType::ForceApprove,
            "",  // Empty reason
            "admin",
            true,
        );

        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("reason cannot be empty"));
    }

    #[test]
    fn test_override_get_for_decision() {
        let clock = Arc::new(SystemClock);
        let manager = OverrideManager::new(clock, 300);

        manager
            .create_override(
                "decision-123",
                OverrideType::ForceApprove,
                "Emergency",
                "admin",
                true,
            )
            .expect("Failed");

        // Chercher par decision_id
        let found = manager.get_override_for_decision("decision-123");
        assert!(found.is_some());
        assert_eq!(found.unwrap().decision_id, "decision-123");

        // Decision inexistante
        let not_found = manager.get_override_for_decision("decision-999");
        assert!(not_found.is_none());
    }

    #[test]
    fn test_override_revoke_with_mfa() {
        let clock = Arc::new(SystemClock);
        let manager = OverrideManager::new(clock, 300);

        let override_entry = manager
            .create_override(
                "decision-123",
                OverrideType::ForceApprove,
                "Emergency",
                "admin",
                true,
            )
            .expect("Failed");

        // Révoquer avec MFA
        manager
            .revoke_override(&override_entry.override_id, "admin", true)
            .expect("Failed to revoke");

        // Override devrait être supprimé
        let found = manager.get_override(&override_entry.override_id);
        assert!(found.is_none());
    }

    #[test]
    fn test_override_revoke_without_mfa_fails() {
        let clock = Arc::new(SystemClock);
        let manager = OverrideManager::new(clock, 300);

        let override_entry = manager
            .create_override(
                "decision-123",
                OverrideType::ForceApprove,
                "Emergency",
                "admin",
                true,
            )
            .expect("Failed");

        // Révoquer sans MFA devrait échouer
        let result = manager.revoke_override(&override_entry.override_id, "admin", false);
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("MFA verification required"));
    }

    #[test]
    fn test_override_expired() {
        let initial = datetime!(2025-11-07 10:00 UTC);
        let clock = MockClock::new(initial);
        let clock_arc = Arc::new(clock);
        let manager = OverrideManager::new(clock_arc.clone(), 60); // 1 minute TTL

        manager
            .create_override(
                "decision-123",
                OverrideType::ForceApprove,
                "Emergency",
                "admin",
                true,
            )
            .expect("Failed");

        assert_eq!(manager.active_count(), 1);

        // Avancer temps de 2 minutes
        clock_arc.advance(time::Duration::minutes(2));

        // Override devrait être expiré
        let found = manager.get_override_for_decision("decision-123");
        assert!(found.is_none());
    }

    #[test]
    fn test_override_list_active() {
        let clock = Arc::new(SystemClock);
        let manager = OverrideManager::new(clock, 300);

        manager
            .create_override("decision-1", OverrideType::ForceApprove, "Reason 1", "admin", true)
            .expect("Failed");
        manager
            .create_override("decision-2", OverrideType::ForceDeny, "Reason 2", "admin", true)
            .expect("Failed");

        let active = manager.list_active();
        assert_eq!(active.len(), 2);
    }

    #[test]
    fn test_override_cleanup_expired() {
        let initial = datetime!(2025-11-07 10:00 UTC);
        let clock = MockClock::new(initial);
        let clock_arc = Arc::new(clock);
        let manager = OverrideManager::new(clock_arc.clone(), 60);

        manager
            .create_override("decision-1", OverrideType::ForceApprove, "Emergency", "admin", true)
            .expect("Failed");
        manager
            .create_override("decision-2", OverrideType::ForceDeny, "Emergency", "admin", true)
            .expect("Failed");

        assert_eq!(manager.active_count(), 2);

        // Avancer temps
        clock_arc.advance(time::Duration::minutes(2));

        // Cleanup
        let cleaned = manager.cleanup_expired();
        assert_eq!(cleaned, 2);
        assert_eq!(manager.active_count(), 0);
    }

    #[test]
    fn test_override_stats() {
        let clock = Arc::new(SystemClock);
        let manager = OverrideManager::new(clock, 300);

        manager
            .create_override("decision-1", OverrideType::ForceApprove, "Reason", "admin", true)
            .expect("Failed");
        manager
            .create_override("decision-2", OverrideType::ForceApprove, "Reason", "admin", true)
            .expect("Failed");
        manager
            .create_override("decision-3", OverrideType::ForceDeny, "Reason", "admin", true)
            .expect("Failed");

        let stats = manager.stats();
        assert_eq!(stats.total, 3);
        assert_eq!(stats.active, 3);
        assert_eq!(stats.force_approve, 2);
        assert_eq!(stats.force_deny, 1);
    }

    #[test]
    fn test_override_list_by_user() {
        let clock = Arc::new(SystemClock);
        let manager = OverrideManager::new(clock, 300);

        manager
            .create_override("decision-1", OverrideType::ForceApprove, "Reason", "alice", true)
            .expect("Failed");
        manager
            .create_override("decision-2", OverrideType::ForceDeny, "Reason", "bob", true)
            .expect("Failed");
        manager
            .create_override("decision-3", OverrideType::ForceApprove, "Reason", "alice", true)
            .expect("Failed");

        let alice_overrides = manager.list_by_user("alice");
        assert_eq!(alice_overrides.len(), 2);

        let bob_overrides = manager.list_by_user("bob");
        assert_eq!(bob_overrides.len(), 1);
    }
}
