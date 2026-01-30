/**
 * SYMBION KERNEL - Automations Persistence
 *
 * ROLE: JSON file storage for automation rules
 *
 * FEATURES:
 * - Thread-safe storage with RwLock
 * - CRUD operations
 * - Soft-delete with 7-day retention
 * - Debounced persistence (dirty flag pattern)
 */

use anyhow::{Context, Result};
use parking_lot::RwLock;
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use time::OffsetDateTime;
use uuid::Uuid;

use super::types::{Automation, AutomationRequest, ExecutionRecord};

/// Storage for automation rules
#[derive(Clone)]
pub struct AutomationStore {
    automations: Arc<RwLock<HashMap<String, Automation>>>,
    history: Arc<RwLock<Vec<ExecutionRecord>>>,
    storage_path: PathBuf,
    history_path: PathBuf,
    dirty: Arc<AtomicBool>,
}

impl AutomationStore {
    /// Create new store with file path
    pub fn new(data_dir: PathBuf) -> Result<Self> {
        let storage_path = data_dir.join("automations.json");
        let history_path = data_dir.join("automations_history.json");

        let store = Self {
            automations: Arc::new(RwLock::new(HashMap::new())),
            history: Arc::new(RwLock::new(Vec::new())),
            storage_path,
            history_path,
            dirty: Arc::new(AtomicBool::new(false)),
        };

        // Load existing data
        store.load_from_disk()?;

        Ok(store)
    }

    /// Load automations from disk
    fn load_from_disk(&self) -> Result<()> {
        if self.storage_path.exists() {
            let content = std::fs::read_to_string(&self.storage_path)
                .context("Failed to read automations.json")?;

            let automations: HashMap<String, Automation> =
                serde_json::from_str(&content).context("Failed to parse automations.json")?;

            *self.automations.write() = automations;
            eprintln!(
                "[automations] Loaded {} automations from disk",
                self.automations.read().len()
            );
        }

        if self.history_path.exists() {
            let content = std::fs::read_to_string(&self.history_path)
                .context("Failed to read automations_history.json")?;

            let history: Vec<ExecutionRecord> =
                serde_json::from_str(&content).unwrap_or_default();

            *self.history.write() = history;
        }

        Ok(())
    }

    /// Save automations to disk
    pub fn save_to_disk(&self) -> Result<()> {
        // Read automations without holding lock during IO
        let automations = { self.automations.read().clone() };

        let content = serde_json::to_string_pretty(&automations)
            .context("Failed to serialize automations")?;

        std::fs::write(&self.storage_path, content).context("Failed to write automations.json")?;

        self.dirty.store(false, Ordering::SeqCst);

        eprintln!("[automations] Saved {} automations to disk", automations.len());

        Ok(())
    }

    /// Save history to disk
    fn save_history(&self) -> Result<()> {
        let history = { self.history.read().clone() };

        let content =
            serde_json::to_string_pretty(&history).context("Failed to serialize history")?;

        std::fs::write(&self.history_path, content)
            .context("Failed to write automations_history.json")?;

        Ok(())
    }

    /// Check if store has unsaved changes
    pub fn is_dirty(&self) -> bool {
        self.dirty.load(Ordering::SeqCst)
    }

    /// Mark store as dirty (needs save)
    fn mark_dirty(&self) {
        self.dirty.store(true, Ordering::SeqCst);
    }

    // ===== CRUD Operations =====

    /// List all non-deleted automations
    pub fn list(&self) -> Vec<Automation> {
        self.automations
            .read()
            .values()
            .filter(|a| a.deleted_at.is_none())
            .cloned()
            .collect()
    }

    /// Get automation by ID
    pub fn get(&self, id: &str) -> Option<Automation> {
        self.automations
            .read()
            .get(id)
            .filter(|a| a.deleted_at.is_none())
            .cloned()
    }

    /// Create new automation
    pub fn create(&self, request: AutomationRequest) -> Result<Automation> {
        let id = format!("auto_{}", Uuid::new_v4().to_string().split('-').next().unwrap());
        let now = OffsetDateTime::now_utc();

        let automation = Automation {
            id: id.clone(),
            name: request.name,
            description: request.description,
            enabled: request.enabled,
            trigger: request.trigger,
            conditions: request.conditions,
            actions: request.actions,
            cooldown_seconds: request.cooldown_seconds,
            last_executed_at: None,
            execution_count: 0,
            created_at: Some(now),
            updated_at: Some(now),
            deleted_at: None,
        };

        {
            let mut automations = self.automations.write();
            automations.insert(id, automation.clone());
        }

        self.mark_dirty();
        self.save_to_disk()?;

        eprintln!("[automations] Created automation: {}", automation.name);

        Ok(automation)
    }

    /// Update existing automation
    pub fn update(&self, id: &str, request: AutomationRequest) -> Result<Option<Automation>> {
        let now = OffsetDateTime::now_utc();

        let updated = {
            let mut automations = self.automations.write();
            if let Some(automation) = automations.get_mut(id) {
                if automation.deleted_at.is_some() {
                    return Ok(None); // Already deleted
                }

                automation.name = request.name;
                automation.description = request.description;
                automation.enabled = request.enabled;
                automation.trigger = request.trigger;
                automation.conditions = request.conditions;
                automation.actions = request.actions;
                automation.cooldown_seconds = request.cooldown_seconds;
                automation.updated_at = Some(now);

                Some(automation.clone())
            } else {
                None
            }
        };

        if updated.is_some() {
            self.mark_dirty();
            self.save_to_disk()?;
            eprintln!("[automations] Updated automation: {}", id);
        }

        Ok(updated)
    }

    /// Soft-delete automation (will be purged after 7 days)
    pub fn delete(&self, id: &str) -> Result<bool> {
        let now = OffsetDateTime::now_utc();

        let deleted = {
            let mut automations = self.automations.write();
            if let Some(automation) = automations.get_mut(id) {
                if automation.deleted_at.is_some() {
                    return Ok(false); // Already deleted
                }
                automation.deleted_at = Some(now);
                automation.updated_at = Some(now);
                true
            } else {
                false
            }
        };

        if deleted {
            self.mark_dirty();
            self.save_to_disk()?;
            eprintln!(
                "[automations] Soft-deleted automation {} (will be purged in 7 days)",
                id
            );
        }

        Ok(deleted)
    }

    /// Toggle automation enabled status
    pub fn toggle(&self, id: &str, enabled: bool) -> Result<Option<Automation>> {
        let now = OffsetDateTime::now_utc();

        let updated = {
            let mut automations = self.automations.write();
            if let Some(automation) = automations.get_mut(id) {
                if automation.deleted_at.is_some() {
                    return Ok(None);
                }
                automation.enabled = enabled;
                automation.updated_at = Some(now);
                Some(automation.clone())
            } else {
                None
            }
        };

        if updated.is_some() {
            self.mark_dirty();
            self.save_to_disk()?;
            eprintln!("[automations] Toggled automation {} to enabled={}", id, enabled);
        }

        Ok(updated)
    }

    /// Record automation execution
    pub fn record_execution(&self, id: &str) -> Result<()> {
        let now = OffsetDateTime::now_utc();

        {
            let mut automations = self.automations.write();
            if let Some(automation) = automations.get_mut(id) {
                automation.last_executed_at = Some(now);
                automation.execution_count += 1;
            }
        }

        self.mark_dirty();
        // Don't save immediately for execution updates (debounced)

        Ok(())
    }

    /// Add execution record to history
    pub fn add_history(&self, record: ExecutionRecord) -> Result<()> {
        {
            let mut history = self.history.write();
            // Keep last 1000 records
            if history.len() >= 1000 {
                history.remove(0);
            }
            history.push(record);
        }

        self.save_history()?;

        Ok(())
    }

    /// Get execution history (most recent first)
    pub fn get_history(&self, limit: usize) -> Vec<ExecutionRecord> {
        let history = self.history.read();
        history
            .iter()
            .rev()
            .take(limit)
            .cloned()
            .collect()
    }

    /// Get enabled automations (for engine)
    pub fn get_enabled(&self) -> Vec<Automation> {
        self.automations
            .read()
            .values()
            .filter(|a| a.enabled && a.deleted_at.is_none())
            .cloned()
            .collect()
    }

    /// Purge automations soft-deleted more than 7 days ago
    pub fn purge_deleted(&self) -> Result<usize> {
        let now = OffsetDateTime::now_utc();
        let seven_days_ago = now - time::Duration::days(7);

        let to_remove: Vec<String> = {
            self.automations
                .read()
                .iter()
                .filter(|(_, a)| {
                    if let Some(deleted_at) = a.deleted_at {
                        deleted_at < seven_days_ago
                    } else {
                        false
                    }
                })
                .map(|(id, _)| id.clone())
                .collect()
        };

        let count = to_remove.len();

        if count > 0 {
            {
                let mut automations = self.automations.write();
                for id in &to_remove {
                    automations.remove(id);
                    eprintln!("[automations] Purged automation: {}", id);
                }
            }

            self.mark_dirty();
            self.save_to_disk()?;
        }

        Ok(count)
    }

    /// Get count statistics
    pub fn stats(&self) -> (usize, usize) {
        let automations = self.automations.read();
        let total = automations.values().filter(|a| a.deleted_at.is_none()).count();
        let enabled = automations
            .values()
            .filter(|a| a.enabled && a.deleted_at.is_none())
            .count();
        (total, enabled)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::automations::types::{ActionDefinition, Trigger};
    use tempfile::TempDir;

    fn create_test_store() -> (AutomationStore, TempDir) {
        let temp_dir = TempDir::new().unwrap();
        let store = AutomationStore::new(temp_dir.path().to_path_buf()).unwrap();
        (store, temp_dir)
    }

    #[test]
    fn test_create_and_get() {
        let (store, _temp) = create_test_store();

        let request = AutomationRequest {
            name: "Test Automation".to_string(),
            description: Some("Test description".to_string()),
            enabled: true,
            trigger: Trigger::Manual,
            conditions: None,
            actions: vec![ActionDefinition::Delay { seconds: 1 }],
            cooldown_seconds: 60,
        };

        let created = store.create(request).unwrap();
        assert_eq!(created.name, "Test Automation");
        assert!(created.id.starts_with("auto_"));

        let retrieved = store.get(&created.id).unwrap();
        assert_eq!(retrieved.name, created.name);
    }

    #[test]
    fn test_soft_delete() {
        let (store, _temp) = create_test_store();

        let request = AutomationRequest {
            name: "To Delete".to_string(),
            description: None,
            enabled: true,
            trigger: Trigger::Manual,
            conditions: None,
            actions: vec![],
            cooldown_seconds: 60,
        };

        let created = store.create(request).unwrap();
        assert!(store.get(&created.id).is_some());

        store.delete(&created.id).unwrap();
        assert!(store.get(&created.id).is_none()); // Filtered out
    }

    #[test]
    fn test_toggle() {
        let (store, _temp) = create_test_store();

        let request = AutomationRequest {
            name: "Toggle Test".to_string(),
            description: None,
            enabled: true,
            trigger: Trigger::Manual,
            conditions: None,
            actions: vec![],
            cooldown_seconds: 60,
        };

        let created = store.create(request).unwrap();
        assert!(created.enabled);

        let toggled = store.toggle(&created.id, false).unwrap().unwrap();
        assert!(!toggled.enabled);
    }

    #[test]
    fn test_stats() {
        let (store, _temp) = create_test_store();

        let request1 = AutomationRequest {
            name: "Enabled".to_string(),
            description: None,
            enabled: true,
            trigger: Trigger::Manual,
            conditions: None,
            actions: vec![],
            cooldown_seconds: 60,
        };

        let request2 = AutomationRequest {
            name: "Disabled".to_string(),
            description: None,
            enabled: false,
            trigger: Trigger::Manual,
            conditions: None,
            actions: vec![],
            cooldown_seconds: 60,
        };

        store.create(request1).unwrap();
        store.create(request2).unwrap();

        let (total, enabled) = store.stats();
        assert_eq!(total, 2);
        assert_eq!(enabled, 1);
    }
}
