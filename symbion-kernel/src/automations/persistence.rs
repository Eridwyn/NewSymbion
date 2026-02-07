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

use super::types::{
    Automation, AutomationRequest, ExecutionRecord, Trigger, ActionDefinition,
    AlertLevel, PluginHealthStatus, TriggerGroup, TriggerItem, LogicalOperator
};
use crate::decision::ImpactLevel;

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
        // Validate that at least one trigger is provided
        if !request.has_triggers() {
            anyhow::bail!("At least one trigger is required");
        }

        // [P0-4] UUID v4 format is always xxxxxxxx-xxxx-..., first segment always exists
        let id = format!("auto_{}", Uuid::new_v4().to_string().split('-').next()
            .expect("[P0-4] UUID v4 always has at least one hyphen-separated segment"));
        let now = OffsetDateTime::now_utc();

        // Normalize triggers: convert old format to new TriggerGroup
        let triggers = request.get_trigger_group();

        let automation = Automation {
            id: id.clone(),
            name: request.name,
            description: request.description,
            category: request.category.clone().or(Some("custom".to_string())),
            goal_mode: request.goal_mode,
            enabled: request.enabled,
            trigger: None, // Don't use old format anymore
            triggers,
            conditions: request.conditions,
            actions: request.actions,
            cooldown_seconds: request.cooldown_seconds,
            // Intelligence flags
            trusted: request.trusted,
            skip_if_same_mode: request.skip_if_same_mode,
            auto_created: request.auto_created,
            // Execution tracking
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
        // Validate that at least one trigger is provided
        if !request.has_triggers() {
            anyhow::bail!("At least one trigger is required");
        }

        let now = OffsetDateTime::now_utc();

        // Normalize triggers
        let triggers = request.get_trigger_group();

        let updated = {
            let mut automations = self.automations.write();
            if let Some(automation) = automations.get_mut(id) {
                if automation.deleted_at.is_some() {
                    return Ok(None); // Already deleted
                }

                automation.name = request.name;
                automation.description = request.description;
                automation.category = request.category.clone();
                automation.goal_mode = request.goal_mode.clone();
                automation.enabled = request.enabled;
                automation.trigger = None; // Don't use old format anymore
                automation.triggers = triggers;
                automation.conditions = request.conditions;
                automation.actions = request.actions;
                automation.cooldown_seconds = request.cooldown_seconds;
                // Intelligence flags (preserve if not set in request)
                if request.trusted.is_some() {
                    automation.trusted = request.trusted;
                }
                if request.skip_if_same_mode.is_some() {
                    automation.skip_if_same_mode = request.skip_if_same_mode;
                }
                if request.auto_created.is_some() {
                    automation.auto_created = request.auto_created;
                }
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

    /// Ensure default system automations exist
    /// Called at startup to create system automations if they don't exist
    #[allow(dead_code)]
    pub fn ensure_system_defaults(&self) -> Result<usize> {
        let mut created_count = 0;

        // Check if system automations already exist (by checking for a known system automation)
        let existing = self.list();
        let has_system_automations = existing.iter().any(|a| a.name.starts_with("[Système]"));

        if has_system_automations {
            eprintln!("[automations] System automations already exist, skipping creation");
            return Ok(0);
        }

        // Create environment alert automations
        let env_automations = vec![
            // Normal - P2 (return to safe state notification)
            AutomationRequest {
                name: "[Système] Environnement - Retour Normal".to_string(),
                description: Some("Notification quand les conditions environnementales redeviennent normales".to_string()),
                category: Some("systeme".to_string()),
                goal_mode: None,
                enabled: true,
                trigger: Some(Trigger::SensorAlert {
                    room_id: None, // Any room
                    alert_level: Some(AlertLevel::Normal),
                }),
                triggers: None,
                conditions: None,
                actions: vec![ActionDefinition::SendNotification {
                    priority: "P2".to_string(),
                    title: "✅ Environnement Normal".to_string(),
                    body: "Les conditions environnementales sont revenues à la normale.".to_string(),
                    impact_level: ImpactLevel::Low,
                }],
                cooldown_seconds: 300, // 5 min cooldown
                trusted: None,
                skip_if_same_mode: None,
                auto_created: None,
            },
            // Danger - P0 (highest priority)
            AutomationRequest {
                name: "[Système] Alerte Environnement - Danger".to_string(),
                description: Some("Notification P0 quand risque moisissure critique".to_string()),
                category: Some("systeme".to_string()),
                goal_mode: None, // System automation - no learning intent
                enabled: true,
                trigger: Some(Trigger::SensorAlert {
                    room_id: None, // Any room
                    alert_level: Some(AlertLevel::Critical), // Maps to "danger" or "critical"
                }),
                triggers: None,
                conditions: None,
                actions: vec![ActionDefinition::SendNotification {
                    priority: "P0".to_string(),
                    title: "🚨 DANGER Environnement".to_string(),
                    body: "Risque moisissure critique détecté. Action immédiate requise!".to_string(),
                    impact_level: ImpactLevel::Low,
                }],
                cooldown_seconds: 300, // 5 min cooldown
                trusted: None,
                skip_if_same_mode: None,
                auto_created: None,
            },
            // Critical - P1
            AutomationRequest {
                name: "[Système] Alerte Environnement - Critique".to_string(),
                description: Some("Notification P1 quand alerte environnement élevée".to_string()),
                category: Some("systeme".to_string()),
                goal_mode: None,
                enabled: true,
                trigger: Some(Trigger::SensorAlert {
                    room_id: None,
                    alert_level: Some(AlertLevel::High), // Maps to "strong" or "high"
                }),
                triggers: None,
                conditions: None,
                actions: vec![ActionDefinition::SendNotification {
                    priority: "P1".to_string(),
                    title: "⚠️ Alerte Environnement".to_string(),
                    body: "Niveau d'alerte élevé détecté. Surveillez l'humidité.".to_string(),
                    impact_level: ImpactLevel::Low,
                }],
                cooldown_seconds: 600, // 10 min cooldown
                trusted: None,
                skip_if_same_mode: None,
                auto_created: None,
            },
            // Moderate - P2
            AutomationRequest {
                name: "[Système] Alerte Environnement - Modéré".to_string(),
                description: Some("Notification P2 quand alerte environnement modérée".to_string()),
                category: Some("systeme".to_string()),
                goal_mode: None,
                enabled: true,
                trigger: Some(Trigger::SensorAlert {
                    room_id: None,
                    alert_level: Some(AlertLevel::Moderate),
                }),
                triggers: None,
                conditions: None,
                actions: vec![ActionDefinition::SendNotification {
                    priority: "P2".to_string(),
                    title: "📊 Info Environnement".to_string(),
                    body: "Niveau d'humidité à surveiller.".to_string(),
                    impact_level: ImpactLevel::Low,
                }],
                cooldown_seconds: 1800, // 30 min cooldown
                trusted: None,
                skip_if_same_mode: None,
                auto_created: None,
            },
        ];

        // Create plugin health automations
        let plugin_automations = vec![
            // Plugin unhealthy - P1
            AutomationRequest {
                name: "[Système] Plugin Défaillant".to_string(),
                description: Some("Notification P1 quand un plugin devient défaillant".to_string()),
                category: Some("systeme".to_string()),
                goal_mode: None,
                enabled: true,
                trigger: Some(Trigger::PluginHealth {
                    plugin_name: None, // Any plugin
                    status: PluginHealthStatus::Unhealthy,
                }),
                triggers: None,
                conditions: None,
                actions: vec![ActionDefinition::SendNotification {
                    priority: "P1".to_string(),
                    title: "⚠️ Plugin Défaillant".to_string(),
                    body: "Un plugin a cessé de répondre. Recovery en cours...".to_string(),
                    impact_level: ImpactLevel::Low,
                }],
                cooldown_seconds: 300,
                trusted: None,
                skip_if_same_mode: None,
                auto_created: None,
            },
            // Plugin recovery failed - P0
            AutomationRequest {
                name: "[Système] Échec Récupération Plugin".to_string(),
                description: Some("Notification P0 quand la récupération d'un plugin échoue".to_string()),
                category: Some("systeme".to_string()),
                goal_mode: None,
                enabled: true,
                trigger: Some(Trigger::PluginHealth {
                    plugin_name: None,
                    status: PluginHealthStatus::RecoveryFailed,
                }),
                triggers: None,
                conditions: None,
                actions: vec![ActionDefinition::SendNotification {
                    priority: "P0".to_string(),
                    title: "🚨 Plugin Non Récupérable".to_string(),
                    body: "La récupération du plugin a échoué. Intervention manuelle requise.".to_string(),
                    impact_level: ImpactLevel::Low,
                }],
                cooldown_seconds: 600,
                trusted: None,
                skip_if_same_mode: None,
                auto_created: None,
            },
            // Plugin recovery success - P2
            AutomationRequest {
                name: "[Système] Plugin Récupéré".to_string(),
                description: Some("Notification P2 quand un plugin est récupéré".to_string()),
                category: Some("systeme".to_string()),
                goal_mode: None,
                enabled: true,
                trigger: Some(Trigger::PluginHealth {
                    plugin_name: None,
                    status: PluginHealthStatus::RecoverySuccess,
                }),
                triggers: None,
                conditions: None,
                actions: vec![ActionDefinition::SendNotification {
                    priority: "P2".to_string(),
                    title: "✅ Plugin Récupéré".to_string(),
                    body: "Le plugin a été restauré avec succès.".to_string(),
                    impact_level: ImpactLevel::Low,
                }],
                cooldown_seconds: 60,
                trusted: None,
                skip_if_same_mode: None,
                auto_created: None,
            },
        ];

        // Create all automations
        for request in env_automations.into_iter().chain(plugin_automations.into_iter()) {
            match self.create(request) {
                Ok(automation) => {
                    eprintln!("[automations] Created system automation: {}", automation.name);
                    created_count += 1;
                }
                Err(e) => {
                    eprintln!("[automations] Failed to create system automation: {}", e);
                }
            }
        }

        eprintln!("[automations] Created {} system automations", created_count);
        Ok(created_count)
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
            category: Some("custom".to_string()),
            enabled: true,
            trigger: Some(Trigger::Manual),
            triggers: None,
            conditions: None,
            actions: vec![ActionDefinition::Delay { seconds: 1 }],
            cooldown_seconds: 60,
        };

        let created = store.create(request).unwrap();
        assert_eq!(created.name, "Test Automation");
        assert!(created.id.starts_with("auto_"));

        let retrieved = store.get(&created.id).unwrap();
        assert_eq!(retrieved.name, created.name);
        // Verify triggers were normalized
        assert!(retrieved.triggers.is_some());
    }

    #[test]
    fn test_soft_delete() {
        let (store, _temp) = create_test_store();

        let request = AutomationRequest {
            name: "To Delete".to_string(),
            description: None,
            category: None,
            enabled: true,
            trigger: Some(Trigger::Manual),
            triggers: None,
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
            category: None,
            enabled: true,
            trigger: Some(Trigger::Manual),
            triggers: None,
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
            category: None,
            enabled: true,
            trigger: Some(Trigger::Manual),
            triggers: None,
            conditions: None,
            actions: vec![],
            cooldown_seconds: 60,
        };

        let request2 = AutomationRequest {
            name: "Disabled".to_string(),
            description: None,
            category: None,
            enabled: false,
            trigger: Some(Trigger::Manual),
            triggers: None,
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
