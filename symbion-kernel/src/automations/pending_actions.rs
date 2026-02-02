/**
 * SYMBION KERNEL - Pending Actions Registry
 *
 * ROLE: Store actions awaiting validation approval for later execution
 *
 * When an automation action requires validation, we store the full ActionDefinition
 * here so it can be executed when the validation is approved.
 */

use super::types::ActionDefinition;
use parking_lot::RwLock;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use time::OffsetDateTime;

/// Pending action stored while awaiting validation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PendingAction {
    pub validation_id: String,
    pub automation_id: String,
    pub automation_name: String,
    pub action: ActionDefinition,
    pub action_index: usize, // Index in automation's action list
    pub trust_score: f32, // Original trust score from decision
    pub target_mode: Option<String>, // Target mode for Intelligence feedback (from automation)
    #[serde(with = "time::serde::rfc3339")]
    pub created_at: OffsetDateTime,
}

/// Registry for pending actions awaiting validation
pub struct PendingActionRegistry {
    actions: Arc<RwLock<HashMap<String, PendingAction>>>,
}

impl PendingActionRegistry {
    pub fn new() -> Self {
        Self {
            actions: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// Register a pending action for a validation
    pub fn register(
        &self,
        validation_id: String,
        automation_id: String,
        automation_name: String,
        action: ActionDefinition,
        action_index: usize,
        trust_score: f32,
        target_mode: Option<String>,
    ) {
        let pending = PendingAction {
            validation_id: validation_id.clone(),
            automation_id,
            automation_name: automation_name.clone(),
            action,
            action_index,
            trust_score,
            target_mode,
            created_at: OffsetDateTime::now_utc(),
        };

        self.actions.write().insert(validation_id.clone(), pending);
        println!(
            "[pending_actions] Registered pending action for validation {} (automation: {})",
            validation_id, automation_name
        );
    }

    /// Get and remove a pending action by validation ID
    pub fn take(&self, validation_id: &str) -> Option<PendingAction> {
        let pending = self.actions.write().remove(validation_id);
        if let Some(ref p) = pending {
            println!(
                "[pending_actions] Retrieved pending action for validation {} (automation: {})",
                validation_id, p.automation_name
            );
        }
        pending
    }

    /// Get a pending action without removing it
    pub fn get(&self, validation_id: &str) -> Option<PendingAction> {
        self.actions.read().get(validation_id).cloned()
    }

    /// Remove a pending action (e.g., when validation is denied)
    pub fn remove(&self, validation_id: &str) -> bool {
        let removed = self.actions.write().remove(validation_id).is_some();
        if removed {
            println!("[pending_actions] Removed pending action for validation {}", validation_id);
        }
        removed
    }

    /// List all pending actions
    pub fn list(&self) -> Vec<PendingAction> {
        self.actions.read().values().cloned().collect()
    }

    /// Cleanup old pending actions (older than TTL)
    pub fn cleanup(&self, max_age_secs: i64) -> usize {
        let now = OffsetDateTime::now_utc();
        let mut actions = self.actions.write();

        let old_count = actions.len();
        actions.retain(|_, p| {
            let age = (now - p.created_at).whole_seconds();
            age < max_age_secs
        });

        let removed = old_count - actions.len();
        if removed > 0 {
            println!("[pending_actions] Cleaned up {} expired pending actions", removed);
        }
        removed
    }

    /// Get count of pending actions
    pub fn count(&self) -> usize {
        self.actions.read().len()
    }
}

impl Default for PendingActionRegistry {
    fn default() -> Self {
        Self::new()
    }
}

// Make it thread-safe for sharing across handlers
pub type SharedPendingActionRegistry = Arc<PendingActionRegistry>;

#[cfg(test)]
mod tests {
    use super::*;
    use crate::decision::ImpactLevel;

    fn create_test_action() -> ActionDefinition {
        ActionDefinition::AgentCommand {
            agent_id: "test-agent".to_string(),
            command_type: "wake".to_string(),
            parameters: None,
            impact_level: ImpactLevel::High,
        }
    }

    #[test]
    fn test_register_and_take() {
        let registry = PendingActionRegistry::new();
        let action = create_test_action();

        registry.register(
            "val-123".to_string(),
            "auto-1".to_string(),
            "Test Auto".to_string(),
            action,
            0,
            0.45, // trust_score
            Some("focus".to_string()), // target_mode
        );

        assert_eq!(registry.count(), 1);

        let pending = registry.take("val-123");
        assert!(pending.is_some());
        let p = pending.unwrap();
        assert_eq!(p.automation_name, "Test Auto");
        assert!((p.trust_score - 0.45).abs() < 0.001);
        assert_eq!(registry.count(), 0);
    }

    #[test]
    fn test_take_not_found() {
        let registry = PendingActionRegistry::new();
        assert!(registry.take("nonexistent").is_none());
    }

    #[test]
    fn test_remove() {
        let registry = PendingActionRegistry::new();
        let action = create_test_action();

        registry.register(
            "val-456".to_string(),
            "auto-2".to_string(),
            "Test Auto 2".to_string(),
            action,
            0,
            0.5, // trust_score
            None, // target_mode
        );

        assert!(registry.remove("val-456"));
        assert!(!registry.remove("val-456")); // Already removed
        assert_eq!(registry.count(), 0);
    }
}
