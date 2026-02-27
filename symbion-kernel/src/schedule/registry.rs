// Schedule Registry - Gestion et Persistance du Planning
// Decision Engine v2

use crate::schedule::types::{
    CreateRuleRequest, CurrentScheduleInfo, Schedule, ScheduleRule, UpdateRuleRequest,
};
use parking_lot::RwLock;
use std::path::PathBuf;
use std::sync::Arc;
use time::{OffsetDateTime, Time};

/// Registre du planning horaire
pub struct ScheduleRegistry {
    schedule: RwLock<Schedule>,
    persistence_path: PathBuf,
    /// SQLite database (None = JSON-only fallback mode)
    db: Option<crate::database::SharedDatabase>,
}

impl ScheduleRegistry {
    /// Crée un nouveau registre avec persistance
    pub fn new(data_dir: PathBuf) -> Self {
        let persistence_path = data_dir.join("schedule.json");
        let registry = Self {
            schedule: RwLock::new(Schedule::default()),
            persistence_path,
            db: None,
        };

        // Charger depuis le disque
        registry.load_from_disk();

        registry
    }

    /// Attach a database for SQLite persistence.
    pub fn with_database(mut self, db: crate::database::SharedDatabase) -> Self {
        // Load from DB if it has data
        let count = crate::database::config_queries::count_schedule_rules(&db).unwrap_or(0);
        if count > 0 {
            let rows = crate::database::config_queries::list_schedule_rules(&db).unwrap_or_default();
            let default_mode = crate::database::config_queries::get_schedule_config(&db, "default_mode_id")
                .unwrap_or(None)
                .unwrap_or_else(|| "veille".to_string());

            let mut rules = Vec::new();
            for row in rows {
                let days: Vec<u8> = serde_json::from_str(&row.days_json).unwrap_or_default();

                rules.push(ScheduleRule {
                    id: row.id,
                    mode_id: row.mode_id,
                    days,
                    start_time: row.start_time,
                    end_time: row.end_time,
                    priority: row.priority as u8,
                    enabled: row.enabled,
                    name: row.name,
                    created_at: OffsetDateTime::parse(&row.created_at,
                        &time::format_description::well_known::Rfc3339).unwrap_or_else(|_| OffsetDateTime::now_utc()),
                });
            }

            let mut schedule = self.schedule.write();
            schedule.rules = rules;
            schedule.default_mode_id = default_mode;
            eprintln!("[schedule] Loaded {} rules from SQLite", count);
        } else {
            // Seed DB from in-memory data (loaded from JSON)
            self.persist_to_db(&db);
        }

        self.db = Some(db);
        self
    }

    /// Persist current state to SQLite
    fn persist_to_db(&self, db: &crate::database::SharedDatabase) {
        let schedule = self.schedule.read();
        for rule in &schedule.rules {
            let days_json = serde_json::to_string(&rule.days).unwrap_or_else(|_| "[]".to_string());
            let created_at = rule.created_at.format(&time::format_description::well_known::Rfc3339)
                .unwrap_or_default();

            let row = crate::database::config_queries::ScheduleRuleRow {
                id: rule.id.clone(),
                mode_id: rule.mode_id.clone(),
                days_json,
                start_time: rule.start_time.clone(),
                end_time: rule.end_time.clone(),
                priority: rule.priority as i32,
                enabled: rule.enabled,
                name: rule.name.clone(),
                created_at,
            };
            let _ = crate::database::config_queries::upsert_schedule_rule(db, &row);
        }
        let _ = crate::database::config_queries::set_schedule_config(db, "default_mode_id", &schedule.default_mode_id);
    }

    /// Charge le planning depuis le disque
    fn load_from_disk(&self) {
        if !self.persistence_path.exists() {
            eprintln!(
                "[schedule] No schedule file found at {:?}, using defaults",
                self.persistence_path
            );
            return;
        }

        match std::fs::read_to_string(&self.persistence_path) {
            Ok(content) => match serde_json::from_str::<Schedule>(&content) {
                Ok(schedule) => {
                    let count = schedule.rules.len();
                    *self.schedule.write() = schedule;
                    eprintln!("[schedule] Loaded {} rules from disk", count);
                }
                Err(e) => {
                    eprintln!("[schedule] Failed to parse schedule file: {}", e);
                }
            },
            Err(e) => {
                eprintln!("[schedule] Failed to read schedule file: {}", e);
            }
        }
    }

    /// Sauvegarde le planning sur le disque (DB-primary, JSON-fallback)
    fn save_to_disk(&self) -> Result<(), String> {
        // Try SQLite first
        if let Some(ref db) = self.db {
            self.persist_to_db(db);
            // Always write JSON as backup
            let _ = self.save_to_disk_json();
            return Ok(());
        }

        self.save_to_disk_json()
    }

    /// JSON-only save (fallback)
    fn save_to_disk_json(&self) -> Result<(), String> {
        let schedule = self.schedule.read();

        // Créer le répertoire parent si nécessaire
        if let Some(parent) = self.persistence_path.parent() {
            std::fs::create_dir_all(parent)
                .map_err(|e| format!("Failed to create directory: {}", e))?;
        }

        let json = serde_json::to_string_pretty(&*schedule)
            .map_err(|e| format!("Failed to serialize schedule: {}", e))?;

        std::fs::write(&self.persistence_path, json)
            .map_err(|e| format!("Failed to write schedule file: {}", e))?;

        eprintln!(
            "[schedule] Saved {} rules to disk",
            schedule.rules.len()
        );
        Ok(())
    }

    /// Récupère le planning complet
    pub fn get_schedule(&self) -> Schedule {
        self.schedule.read().clone()
    }

    /// Liste toutes les règles triées par priorité
    pub fn list_rules(&self) -> Vec<ScheduleRule> {
        let schedule = self.schedule.read();
        let mut rules = schedule.rules.clone();
        rules.sort_by(|a, b| b.priority.cmp(&a.priority));
        rules
    }

    /// Récupère une règle par ID
    pub fn get_rule(&self, id: &str) -> Option<ScheduleRule> {
        self.schedule
            .read()
            .rules
            .iter()
            .find(|r| r.id == id)
            .cloned()
    }

    /// Crée une nouvelle règle
    pub fn create_rule(&self, request: CreateRuleRequest) -> Result<ScheduleRule, String> {
        // Validation basique
        if request.days.is_empty() {
            return Err("Au moins un jour doit être sélectionné".to_string());
        }

        if request.days.iter().any(|&d| d > 6) {
            return Err("Jour invalide (0=Lundi à 6=Dimanche)".to_string());
        }

        let rule = ScheduleRule::new(
            request.mode_id,
            request.days,
            request.start_time,
            request.end_time,
            request.priority,
            request.name,
        );

        {
            let mut schedule = self.schedule.write();
            schedule.rules.push(rule.clone());
        }

        self.save_to_disk()?;
        eprintln!(
            "[schedule] Created rule: {} ({})",
            rule.name.as_deref().unwrap_or("Sans nom"),
            rule.id
        );

        Ok(rule)
    }

    /// Met à jour une règle existante
    pub fn update_rule(&self, id: &str, request: UpdateRuleRequest) -> Result<ScheduleRule, String> {
        let mut schedule = self.schedule.write();

        let rule = schedule
            .rules
            .iter_mut()
            .find(|r| r.id == id)
            .ok_or_else(|| format!("Règle '{}' non trouvée", id))?;

        if let Some(mode_id) = request.mode_id {
            rule.mode_id = mode_id;
        }
        if let Some(days) = request.days {
            if days.is_empty() {
                return Err("Au moins un jour doit être sélectionné".to_string());
            }
            rule.days = days;
        }
        if let Some(start_time) = request.start_time {
            rule.start_time = start_time;
        }
        if let Some(end_time) = request.end_time {
            rule.end_time = end_time;
        }
        if let Some(priority) = request.priority {
            rule.priority = priority;
        }
        if let Some(enabled) = request.enabled {
            rule.enabled = enabled;
        }
        if let Some(name) = request.name {
            rule.name = Some(name);
        }

        let updated = rule.clone();
        drop(schedule);

        self.save_to_disk()?;
        eprintln!(
            "[schedule] Updated rule: {} ({})",
            updated.name.as_deref().unwrap_or("Sans nom"),
            updated.id
        );

        Ok(updated)
    }

    /// Supprime une règle
    pub fn delete_rule(&self, id: &str) -> Result<(), String> {
        let mut schedule = self.schedule.write();

        let idx = schedule
            .rules
            .iter()
            .position(|r| r.id == id)
            .ok_or_else(|| format!("Règle '{}' non trouvée", id))?;

        let removed = schedule.rules.remove(idx);
        drop(schedule);

        // Delete from DB explicitly
        if let Some(ref db) = self.db {
            let _ = crate::database::config_queries::delete_schedule_rule(db, id);
        }

        self.save_to_disk()?;
        eprintln!(
            "[schedule] Deleted rule: {} ({})",
            removed.name.as_deref().unwrap_or("Sans nom"),
            removed.id
        );

        Ok(())
    }

    /// Change le mode par défaut
    pub fn set_default_mode(&self, mode_id: String) -> Result<(), String> {
        {
            let mut schedule = self.schedule.write();
            schedule.default_mode_id = mode_id.clone();
        }

        // Persist default_mode to DB
        if let Some(ref db) = self.db {
            let _ = crate::database::config_queries::set_schedule_config(db, "default_mode_id", &mode_id);
        }

        self.save_to_disk()?;
        eprintln!("[schedule] Default mode set to: {}", mode_id);

        Ok(())
    }

    /// Récupère le mode par défaut
    pub fn get_default_mode(&self) -> String {
        self.schedule.read().default_mode_id.clone()
    }

    /// Détermine le mode actif pour le moment actuel
    pub fn get_current_mode(&self) -> CurrentScheduleInfo {
        let now = OffsetDateTime::now_utc();
        let time = now.time();
        // time::Weekday: Monday=1, Sunday=7, on veut 0=Lundi, 6=Dimanche
        let weekday = (now.weekday().number_from_monday() - 1) as u8;

        self.get_mode_at(time, weekday)
    }

    /// Détermine le mode pour un moment donné
    pub fn get_mode_at(&self, time: Time, weekday: u8) -> CurrentScheduleInfo {
        let schedule = self.schedule.read();

        // Trouver la règle active avec la plus haute priorité
        let mut active_rules: Vec<&ScheduleRule> = schedule
            .rules
            .iter()
            .filter(|r| r.is_active_at(time, weekday))
            .collect();

        active_rules.sort_by(|a, b| b.priority.cmp(&a.priority));

        if let Some(rule) = active_rules.first() {
            CurrentScheduleInfo {
                active_rule: Some((*rule).clone()),
                current_mode_id: rule.mode_id.clone(),
                is_default: false,
            }
        } else {
            CurrentScheduleInfo {
                active_rule: None,
                current_mode_id: schedule.default_mode_id.clone(),
                is_default: true,
            }
        }
    }

    /// Compte le nombre de règles
    pub fn count_rules(&self) -> usize {
        self.schedule.read().rules.len()
    }
}

/// Type partagé pour le registre
pub type SharedScheduleRegistry = Arc<ScheduleRegistry>;

/// Crée un registre partagé
pub fn create_shared_registry(data_dir: PathBuf) -> SharedScheduleRegistry {
    Arc::new(ScheduleRegistry::new(data_dir))
}
