/**
 * SYMBION KERNEL - Pending Actions Registry
 *
 * ROLE: Store actions awaiting validation approval for later execution
 *
 * When an automation action requires validation, we store the full ActionDefinition
 * here so it can be executed when the validation is approved.
 *
 * Persistence: JSON file in data/ directory, atomic write (temp→rename).
 * Survives kernel restarts so pending validations are not lost.
 */

use super::types::ActionDefinition;
use parking_lot::RwLock;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;
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
    data_path: Option<PathBuf>,
    /// SQLite database (None = JSON-only fallback mode)
    db: std::sync::Mutex<Option<crate::database::SharedDatabase>>,
}

impl PendingActionRegistry {
    pub fn new(data_dir: Option<PathBuf>) -> Self {
        let data_path = data_dir.map(|d| d.join("pending_actions.json"));

        let registry = Self {
            actions: Arc::new(RwLock::new(HashMap::new())),
            data_path,
            db: std::sync::Mutex::new(None),
        };

        registry.load_from_disk();
        registry
    }

    /// Attach SQLite database and load/seed pending actions.
    /// Uses interior mutability so it works on Arc<PendingActionRegistry>.
    pub fn set_database(&self, db: crate::database::SharedDatabase) {
        // Load from DB if it has data
        match crate::database::pending_action_queries::count_pending_actions(&db) {
            Ok(count) if count > 0 => {
                match crate::database::pending_action_queries::list_pending_actions(&db) {
                    Ok(rows) => {
                        let mut actions = HashMap::new();
                        for row in rows {
                            let action: ActionDefinition = match serde_json::from_str(&row.action_json) {
                                Ok(a) => a,
                                Err(e) => {
                                    eprintln!("[pending_actions] Failed to parse action JSON from DB: {}", e);
                                    continue;
                                }
                            };
                            let created_at = time::OffsetDateTime::parse(
                                &row.created_at,
                                &time::format_description::well_known::Rfc3339,
                            ).unwrap_or_else(|_| OffsetDateTime::now_utc());

                            let pending = PendingAction {
                                validation_id: row.validation_id.clone(),
                                automation_id: row.automation_id,
                                automation_name: row.automation_name,
                                action,
                                action_index: row.action_index,
                                trust_score: row.trust_score.map(|s| s as f32).unwrap_or(0.0),
                                target_mode: row.target_mode,
                                created_at,
                            };
                            actions.insert(row.validation_id, pending);
                        }
                        let loaded = actions.len();
                        *self.actions.write() = actions;
                        eprintln!("[pending_actions] Loaded {} pending actions from SQLite", loaded);
                    }
                    Err(e) => eprintln!("[pending_actions] Failed to load from SQLite: {}", e),
                }
            }
            Ok(_) => {
                // DB empty, seed from current in-memory actions (loaded from JSON)
                let actions = self.actions.read();
                if !actions.is_empty() {
                    for (_, pa) in actions.iter() {
                        let action_json = serde_json::to_string(&pa.action).unwrap_or_else(|_| "{}".to_string());
                        let row = crate::database::pending_action_queries::PendingActionRow {
                            validation_id: pa.validation_id.clone(),
                            automation_id: pa.automation_id.clone(),
                            automation_name: pa.automation_name.clone(),
                            action_json,
                            action_index: pa.action_index,
                            trust_score: Some(pa.trust_score as f64),
                            target_mode: pa.target_mode.clone(),
                            created_at: pa.created_at.format(&time::format_description::well_known::Rfc3339).unwrap_or_default(),
                        };
                        if let Err(e) = crate::database::pending_action_queries::upsert_pending_action(&db, &row) {
                            eprintln!("[pending_actions] Failed to seed to SQLite: {}", e);
                        }
                    }
                    eprintln!("[pending_actions] Seeded {} pending actions to SQLite from JSON", actions.len());
                }
            }
            Err(e) => eprintln!("[pending_actions] Failed to count in SQLite: {}", e),
        }

        // Store the DB handle
        if let Ok(mut db_guard) = self.db.lock() {
            *db_guard = Some(db);
        }
    }

    /// Load pending actions from disk
    fn load_from_disk(&self) {
        let Some(path) = &self.data_path else { return };
        if !path.exists() {
            return;
        }

        match std::fs::read_to_string(path) {
            Ok(content) => {
                match serde_json::from_str::<HashMap<String, PendingAction>>(&content) {
                    Ok(actions) => {
                        let count = actions.len();
                        *self.actions.write() = actions;
                        eprintln!("[pending_actions] Loaded {} pending actions from disk", count);
                    }
                    Err(e) => {
                        eprintln!("[pending_actions] Failed to parse pending_actions.json: {}", e);
                    }
                }
            }
            Err(e) => {
                eprintln!("[pending_actions] Failed to read pending_actions.json: {}", e);
            }
        }
    }

    /// Save pending actions (SQLite primary + JSON fallback, atomic)
    fn save_to_disk(&self) {
        // Try SQLite first: full replace all pending actions
        if let Ok(db_guard) = self.db.lock() {
            if let Some(ref db) = *db_guard {
                let actions = self.actions.read();
                // Delete all then re-insert (simple approach for small dataset)
                let _ = crate::database::pending_action_queries::delete_all_pending_actions(db);
                for (_, pa) in actions.iter() {
                    let action_json = serde_json::to_string(&pa.action).unwrap_or_else(|_| "{}".to_string());
                    let row = crate::database::pending_action_queries::PendingActionRow {
                        validation_id: pa.validation_id.clone(),
                        automation_id: pa.automation_id.clone(),
                        automation_name: pa.automation_name.clone(),
                        action_json,
                        action_index: pa.action_index,
                        trust_score: Some(pa.trust_score as f64),
                        target_mode: pa.target_mode.clone(),
                        created_at: pa.created_at.format(&time::format_description::well_known::Rfc3339).unwrap_or_default(),
                    };
                    if let Err(e) = crate::database::pending_action_queries::upsert_pending_action(db, &row) {
                        eprintln!("[pending_actions] Failed to save to SQLite: {}", e);
                    }
                }
            }
        }

        // JSON fallback (always write for backward compatibility during transition)
        let Some(path) = &self.data_path else { return };

        let json = {
            let actions = self.actions.read();
            match serde_json::to_string_pretty(&*actions) {
                Ok(json) => json,
                Err(e) => {
                    eprintln!("[pending_actions] Failed to serialize pending actions: {}", e);
                    return;
                }
            }
        };

        let path = path.clone();
        std::thread::spawn(move || {
            let tmp_path = path.with_extension("json.tmp");
            if let Err(e) = std::fs::write(&tmp_path, &json) {
                eprintln!("[pending_actions] Failed to write temp file: {}", e);
                return;
            }
            if let Err(e) = std::fs::rename(&tmp_path, &path) {
                eprintln!("[pending_actions] Failed to rename temp file: {}", e);
            }
        });
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
        self.save_to_disk();
    }

    /// Get and remove a pending action by validation ID
    pub fn take(&self, validation_id: &str) -> Option<PendingAction> {
        let pending = self.actions.write().remove(validation_id);
        if let Some(ref p) = pending {
            println!(
                "[pending_actions] Retrieved pending action for validation {} (automation: {})",
                validation_id, p.automation_name
            );
            self.save_to_disk();
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
            self.save_to_disk();
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
            drop(actions); // Release lock before I/O
            self.save_to_disk();
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
        Self::new(None)
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
        let registry = PendingActionRegistry::new(None);
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
        let registry = PendingActionRegistry::new(None);
        assert!(registry.take("nonexistent").is_none());
    }

    #[test]
    fn test_remove() {
        let registry = PendingActionRegistry::new(None);
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

    #[test]
    fn test_persistence_roundtrip() {
        let tmp_dir = std::env::temp_dir().join("symbion_test_pending");
        let _ = std::fs::create_dir_all(&tmp_dir);

        // Create and populate
        {
            let registry = PendingActionRegistry::new(Some(tmp_dir.clone()));
            let action = create_test_action();
            registry.register(
                "val-persist".to_string(),
                "auto-persist".to_string(),
                "Persist Test".to_string(),
                action,
                0,
                0.75,
                Some("pro".to_string()),
            );
            assert_eq!(registry.count(), 1);
            // Wait for background write to complete
            std::thread::sleep(std::time::Duration::from_millis(100));
        }

        // Reload from disk
        {
            let registry = PendingActionRegistry::new(Some(tmp_dir.clone()));
            assert_eq!(registry.count(), 1);
            let p = registry.get("val-persist").unwrap();
            assert_eq!(p.automation_name, "Persist Test");
            assert!((p.trust_score - 0.75).abs() < 0.001);
            assert_eq!(p.target_mode, Some("pro".to_string()));
        }

        // Cleanup
        let _ = std::fs::remove_dir_all(&tmp_dir);
    }
}
