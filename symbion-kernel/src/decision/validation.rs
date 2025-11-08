// Validation Manager - MFA Flow for Decisions
// Spec: PR3 P0 v3.1 REFINED - CORRECTION 4

use crate::decision::{Action, DecisionContext, DecisionResult, Clock};
use anyhow::{Context, Result, bail};
use parking_lot::RwLock;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use time::OffsetDateTime;
use uuid::Uuid;

/// Statut d'une validation
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum ValidationStatus {
    Pending,
    Approved,
    Denied,
    Expired,
}

/// Requête de validation en attente
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ValidationRequest {
    pub validation_id: String,
    pub decision_id: String,
    pub action: Action,
    pub context: DecisionContext,
    pub status: ValidationStatus,
    #[serde(with = "time::serde::rfc3339")]
    pub created_at: OffsetDateTime,
    #[serde(with = "time::serde::rfc3339")]
    pub expires_at: OffsetDateTime,
    #[serde(with = "time::serde::rfc3339::option")]
    pub resolved_at: Option<OffsetDateTime>,
    pub resolved_by: Option<String>, // username
}

/// Gestionnaire de validations MFA
pub struct ValidationManager {
    validations: Arc<RwLock<HashMap<String, ValidationRequest>>>,
    clock: Arc<dyn Clock>,
    default_ttl_seconds: i64,
}

impl ValidationManager {
    /// Créer un nouveau gestionnaire
    pub fn new(clock: Arc<dyn Clock>, default_ttl_seconds: i64) -> Self {
        Self {
            validations: Arc::new(RwLock::new(HashMap::new())),
            clock,
            default_ttl_seconds,
        }
    }

    /// Créer une nouvelle validation en attente
    pub fn create_validation(
        &self,
        decision_result: &DecisionResult,
        action: &Action,
        context: &DecisionContext,
    ) -> Result<ValidationRequest> {
        let validation_id = Uuid::new_v4().to_string();
        let now = self.clock.now_utc();
        let expires_at = now + time::Duration::seconds(self.default_ttl_seconds);

        let request = ValidationRequest {
            validation_id: validation_id.clone(),
            decision_id: decision_result.decision_id.clone(),
            action: action.clone(),
            context: context.clone(),
            status: ValidationStatus::Pending,
            created_at: now,
            expires_at,
            resolved_at: None,
            resolved_by: None,
        };

        // Stocker validation
        self.validations.write().insert(validation_id.clone(), request.clone());

        println!(
            "[validation] Created validation {} for decision {} (expires in {}s)",
            validation_id, decision_result.decision_id, self.default_ttl_seconds
        );

        Ok(request)
    }

    /// Résoudre une validation (approve/deny)
    pub fn resolve_validation(
        &self,
        validation_id: &str,
        approved: bool,
        username: &str,
    ) -> Result<ValidationRequest> {
        let mut validations = self.validations.write();

        let request = validations
            .get_mut(validation_id)
            .with_context(|| format!("Validation {} not found", validation_id))?;

        // Vérifier pas déjà résolu
        if request.status != ValidationStatus::Pending {
            bail!(
                "Validation {} already resolved with status: {:?}",
                validation_id,
                request.status
            );
        }

        let now = self.clock.now_utc();

        // Vérifier pas expiré
        if now > request.expires_at {
            request.status = ValidationStatus::Expired;
            return Err(anyhow::anyhow!(
                "Validation {} expired at {}",
                validation_id,
                request.expires_at
            ));
        }

        // Résoudre
        request.status = if approved {
            ValidationStatus::Approved
        } else {
            ValidationStatus::Denied
        };
        request.resolved_at = Some(now);
        request.resolved_by = Some(username.to_string());

        println!(
            "[validation] Resolved validation {} as {:?} by {}",
            validation_id, request.status, username
        );

        Ok(request.clone())
    }

    /// Obtenir une validation par ID
    pub fn get_validation(&self, validation_id: &str) -> Option<ValidationRequest> {
        self.validations.read().get(validation_id).cloned()
    }

    /// Lister toutes les validations en attente
    pub fn list_pending(&self) -> Vec<ValidationRequest> {
        let now = self.clock.now_utc();
        self.validations
            .read()
            .values()
            .filter(|v| v.status == ValidationStatus::Pending && v.expires_at > now)
            .cloned()
            .collect()
    }

    /// Lister toutes les validations expirées (non résolues mais TTL dépassé)
    pub fn list_expired(&self) -> Vec<ValidationRequest> {
        let now = self.clock.now_utc();
        self.validations
            .read()
            .values()
            .filter(|v| {
                (v.status == ValidationStatus::Pending && v.expires_at <= now)
                    || v.status == ValidationStatus::Expired
            })
            .cloned()
            .collect()
    }

    /// Lister validations par utilisateur (resolved_by)
    pub fn list_by_user(&self, username: &str) -> Vec<ValidationRequest> {
        self.validations
            .read()
            .values()
            .filter(|v| v.resolved_by.as_deref() == Some(username))
            .cloned()
            .collect()
    }

    /// Supprimer une validation par ID (pour nettoyage manuel)
    /// Retourne true si la validation a été supprimée, false si non trouvée
    pub fn delete_validation(&self, validation_id: &str) -> bool {
        let removed = self.validations.write().remove(validation_id).is_some();
        if removed {
            println!("[validation] Deleted validation {}", validation_id);
        }
        removed
    }

    /// Supprimer toutes les validations expirées (nettoyage manuel)
    /// Retourne le nombre de validations supprimées
    pub fn delete_all_expired(&self) -> usize {
        let now = self.clock.now_utc();
        let mut validations = self.validations.write();

        let expired_ids: Vec<String> = validations
            .iter()
            .filter(|(_, v)| {
                (v.status == ValidationStatus::Pending && v.expires_at <= now)
                    || v.status == ValidationStatus::Expired
            })
            .map(|(id, _)| id.clone())
            .collect();

        let count = expired_ids.len();

        for id in expired_ids {
            validations.remove(&id);
        }

        if count > 0 {
            println!("[validation] Deleted {} expired validations", count);
        }

        count
    }

    /// Nettoyer validations expirées (automatique via cron)
    /// Retourne le nombre de validations supprimées
    pub fn cleanup_expired(&self) -> usize {
        let now = self.clock.now_utc();
        let mut validations = self.validations.write();

        let expired_ids: Vec<String> = validations
            .iter()
            .filter(|(_, v)| v.status == ValidationStatus::Pending && v.expires_at <= now)
            .map(|(id, _)| id.clone())
            .collect();

        // Marquer comme expirés avant suppression
        for id in &expired_ids {
            if let Some(v) = validations.get_mut(id) {
                v.status = ValidationStatus::Expired;
            }
        }

        let count = expired_ids.len();

        // Supprimer après délai (pour permettre requêtes en cours)
        let cleanup_threshold = now - time::Duration::minutes(5);
        validations.retain(|_, v| {
            v.status != ValidationStatus::Expired || v.expires_at > cleanup_threshold
        });

        if count > 0 {
            println!("[validation] Cleaned up {} expired validations", count);
        }

        count
    }

    /// Obtenir statistiques
    pub fn stats(&self) -> ValidationStats {
        let validations = self.validations.read();

        let mut stats = ValidationStats {
            total: validations.len(),
            pending: 0,
            approved: 0,
            denied: 0,
            expired: 0,
        };

        for v in validations.values() {
            match v.status {
                ValidationStatus::Pending => stats.pending += 1,
                ValidationStatus::Approved => stats.approved += 1,
                ValidationStatus::Denied => stats.denied += 1,
                ValidationStatus::Expired => stats.expired += 1,
            }
        }

        stats
    }

    /// Obtenir nombre de validations actives
    pub fn active_count(&self) -> usize {
        self.validations.read().len()
    }
}

/// Statistiques validations
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ValidationStats {
    pub total: usize,
    pub pending: usize,
    pub approved: usize,
    pub denied: usize,
    pub expired: usize,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::decision::{
        DecisionOutcome, ImpactLevel, MockClock, SystemClock,
    };
    use std::collections::HashMap;
    use time::macros::datetime;

    fn create_test_action() -> Action {
        Action {
            action_type: "test_action".into(),
            agent_id: "agent1".into(),
            impact_level: ImpactLevel::Medium,
            trace_id: "trace1".into(),
            expires_at: None,
            dry_run: false,
            expected_mode: Some("intime".into()),
            expected_ssid: Some("home-wifi".into()),
        }
    }

    fn create_test_context() -> DecisionContext {
        DecisionContext {
            mode: "intime".into(),
            ssid: "home-wifi".into(),
            agents: HashMap::new(),
        }
    }

    fn create_test_decision_result() -> DecisionResult {
        DecisionResult {
            decision_id: Uuid::new_v4().to_string(),
            outcome: DecisionOutcome::RequireValidation {
                reasons: vec!["Test reason".into()],
                explanation_codes: vec!["TEST.CODE".into()],
                human_reasons: vec!["Human reason".into()],
            },
            trace_id: "trace1".into(),
            warnings: vec![],
        }
    }

    #[test]
    fn test_validation_create() {
        let clock = Arc::new(SystemClock);
        let manager = ValidationManager::new(clock, 300);

        let decision = create_test_decision_result();
        let action = create_test_action();
        let context = create_test_context();

        let validation = manager
            .create_validation(&decision, &action, &context)
            .expect("Failed to create validation");

        assert_eq!(validation.decision_id, decision.decision_id);
        assert_eq!(validation.status, ValidationStatus::Pending);
        assert!(validation.resolved_at.is_none());
        assert!(validation.resolved_by.is_none());

        // Vérifier stockage
        let stored = manager.get_validation(&validation.validation_id);
        assert!(stored.is_some());
        assert_eq!(stored.unwrap().validation_id, validation.validation_id);
    }

    #[test]
    fn test_validation_approve() {
        let clock = Arc::new(SystemClock);
        let manager = ValidationManager::new(clock, 300);

        let decision = create_test_decision_result();
        let action = create_test_action();
        let context = create_test_context();

        let validation = manager
            .create_validation(&decision, &action, &context)
            .expect("Failed to create validation");

        // Approuver
        let resolved = manager
            .resolve_validation(&validation.validation_id, true, "testuser")
            .expect("Failed to resolve validation");

        assert_eq!(resolved.status, ValidationStatus::Approved);
        assert!(resolved.resolved_at.is_some());
        assert_eq!(resolved.resolved_by, Some("testuser".to_string()));
    }

    #[test]
    fn test_validation_deny() {
        let clock = Arc::new(SystemClock);
        let manager = ValidationManager::new(clock, 300);

        let decision = create_test_decision_result();
        let action = create_test_action();
        let context = create_test_context();

        let validation = manager
            .create_validation(&decision, &action, &context)
            .expect("Failed to create validation");

        // Refuser
        let resolved = manager
            .resolve_validation(&validation.validation_id, false, "testuser")
            .expect("Failed to resolve validation");

        assert_eq!(resolved.status, ValidationStatus::Denied);
        assert_eq!(resolved.resolved_by, Some("testuser".to_string()));
    }

    #[test]
    fn test_validation_already_resolved() {
        let clock = Arc::new(SystemClock);
        let manager = ValidationManager::new(clock, 300);

        let decision = create_test_decision_result();
        let action = create_test_action();
        let context = create_test_context();

        let validation = manager
            .create_validation(&decision, &action, &context)
            .expect("Failed to create validation");

        // Première résolution
        manager
            .resolve_validation(&validation.validation_id, true, "testuser")
            .expect("Failed to resolve validation");

        // Tentative de re-résolution devrait échouer
        let result = manager.resolve_validation(&validation.validation_id, false, "otheruser");
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("already resolved"));
    }

    #[test]
    fn test_validation_expired() {
        let initial = datetime!(2025-11-07 10:00 UTC);
        let clock = MockClock::new(initial);
        let clock_arc = Arc::new(clock);
        let manager = ValidationManager::new(clock_arc.clone(), 60); // 1 minute TTL

        let decision = create_test_decision_result();
        let action = create_test_action();
        let context = create_test_context();

        let validation = manager
            .create_validation(&decision, &action, &context)
            .expect("Failed to create validation");

        // Avancer temps de 2 minutes
        clock_arc.advance(time::Duration::minutes(2));

        // Tentative résolution devrait échouer (expiré)
        let result = manager.resolve_validation(&validation.validation_id, true, "testuser");
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("expired"));
    }

    #[test]
    fn test_validation_list_pending() {
        let clock = Arc::new(SystemClock);
        let manager = ValidationManager::new(clock, 300);

        let decision = create_test_decision_result();
        let action = create_test_action();
        let context = create_test_context();

        // Créer 3 validations
        let v1 = manager
            .create_validation(&decision, &action, &context)
            .expect("Failed");
        let v2 = manager
            .create_validation(&decision, &action, &context)
            .expect("Failed");
        let v3 = manager
            .create_validation(&decision, &action, &context)
            .expect("Failed");

        // Résoudre v2
        manager
            .resolve_validation(&v2.validation_id, true, "testuser")
            .expect("Failed");

        // Lister pending (devrait avoir v1 et v3)
        let pending = manager.list_pending();
        assert_eq!(pending.len(), 2);
        let ids: Vec<String> = pending.iter().map(|v| v.validation_id.clone()).collect();
        assert!(ids.contains(&v1.validation_id));
        assert!(ids.contains(&v3.validation_id));
        assert!(!ids.contains(&v2.validation_id));
    }

    #[test]
    fn test_validation_cleanup_expired() {
        let initial = datetime!(2025-11-07 10:00 UTC);
        let clock = MockClock::new(initial);
        let clock_arc = Arc::new(clock);
        let manager = ValidationManager::new(clock_arc.clone(), 60); // 1 minute TTL

        let decision = create_test_decision_result();
        let action = create_test_action();
        let context = create_test_context();

        // Créer 2 validations
        manager
            .create_validation(&decision, &action, &context)
            .expect("Failed");
        manager
            .create_validation(&decision, &action, &context)
            .expect("Failed");

        assert_eq!(manager.active_count(), 2);

        // Avancer temps de 2 minutes
        clock_arc.advance(time::Duration::minutes(2));

        // Cleanup devrait marquer comme expirés
        let cleaned = manager.cleanup_expired();
        assert_eq!(cleaned, 2);
    }

    #[test]
    fn test_validation_stats() {
        let clock = Arc::new(SystemClock);
        let manager = ValidationManager::new(clock, 300);

        let decision = create_test_decision_result();
        let action = create_test_action();
        let context = create_test_context();

        // Créer 4 validations
        let v1 = manager.create_validation(&decision, &action, &context).unwrap();
        let v2 = manager.create_validation(&decision, &action, &context).unwrap();
        let v3 = manager.create_validation(&decision, &action, &context).unwrap();
        manager.create_validation(&decision, &action, &context).unwrap();

        // Résoudre différemment
        manager.resolve_validation(&v1.validation_id, true, "user1").unwrap();
        manager.resolve_validation(&v2.validation_id, false, "user2").unwrap();
        manager.resolve_validation(&v3.validation_id, true, "user1").unwrap();

        let stats = manager.stats();
        assert_eq!(stats.total, 4);
        assert_eq!(stats.pending, 1);
        assert_eq!(stats.approved, 2);
        assert_eq!(stats.denied, 1);
        assert_eq!(stats.expired, 0);
    }

    #[test]
    fn test_validation_list_by_user() {
        let clock = Arc::new(SystemClock);
        let manager = ValidationManager::new(clock, 300);

        let decision = create_test_decision_result();
        let action = create_test_action();
        let context = create_test_context();

        let v1 = manager.create_validation(&decision, &action, &context).unwrap();
        let v2 = manager.create_validation(&decision, &action, &context).unwrap();
        let v3 = manager.create_validation(&decision, &action, &context).unwrap();

        manager.resolve_validation(&v1.validation_id, true, "alice").unwrap();
        manager.resolve_validation(&v2.validation_id, false, "bob").unwrap();
        manager.resolve_validation(&v3.validation_id, true, "alice").unwrap();

        let alice_validations = manager.list_by_user("alice");
        assert_eq!(alice_validations.len(), 2);

        let bob_validations = manager.list_by_user("bob");
        assert_eq!(bob_validations.len(), 1);
    }
}
