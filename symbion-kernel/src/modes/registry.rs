// Modes Registry - Gestion et Persistance
// Decision Engine v2

use crate::modes::types::{DynamicMode, ModeTheme, CreateModeRequest, UpdateModeRequest, default_system_modes};
use parking_lot::RwLock;
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;

/// Registre des modes (système + personnalisés)
pub struct ModeRegistry {
    modes: RwLock<HashMap<String, DynamicMode>>,
    persistence_path: PathBuf,
    db: Option<crate::database::SharedDatabase>,
}

impl ModeRegistry {
    /// Crée un nouveau registre avec persistance
    pub fn new(data_dir: PathBuf) -> Self {
        let persistence_path = data_dir.join("modes.json");
        let registry = Self {
            modes: RwLock::new(HashMap::new()),
            persistence_path,
            db: None,
        };

        // Charger depuis le disque, ou bootstrap avec les défauts
        if !registry.load_from_disk() {
            registry.init_system_modes();
            let _ = registry.save_to_disk(); // Persister immédiatement
        }

        registry
    }

    /// Attache une base SQLite et charge les modes depuis la DB si elle en contient.
    /// DB-primary / JSON-fallback : si la DB a des modes, ils remplacent ceux en mémoire.
    pub fn with_database(&mut self, db: crate::database::SharedDatabase) {
        self.db = Some(db.clone());

        // Si la DB contient déjà des modes, les charger (DB = source de vérité)
        match crate::database::config_queries::count_modes(&db) {
            Ok(count) if count > 0 => {
                match crate::database::config_queries::list_modes(&db) {
                    Ok(rows) => {
                        let mut modes = self.modes.write();
                        modes.clear();
                        for row in rows {
                            let theme: ModeTheme = serde_json::from_str(&row.theme_json)
                                .unwrap_or_else(|_| ModeTheme {
                                    primary: "#6b7280".to_string(),
                                    background: "#f8fafc".to_string(),
                                    accent: "#374151".to_string(),
                                });
                            let created_at = time::OffsetDateTime::parse(
                                &row.created_at,
                                &time::format_description::well_known::Rfc3339,
                            )
                            .unwrap_or_else(|_| time::OffsetDateTime::now_utc());

                            let mode = DynamicMode {
                                id: row.id.clone(),
                                name: row.name,
                                slug: row.slug,
                                icon: row.icon,
                                theme,
                                is_system: row.is_system,
                                created_at,
                                display_order: row.display_order as u32,
                            };
                            modes.insert(row.id, mode);
                        }
                        eprintln!("[modes] Loaded {} modes from SQLite (DB-primary)", modes.len());
                    }
                    Err(e) => {
                        eprintln!("[modes] WARN: Failed to list modes from DB, keeping JSON data: {}", e);
                    }
                }
            }
            Ok(_) => {
                // DB is empty — seed it from current in-memory modes (loaded from JSON)
                eprintln!("[modes] DB empty, seeding from in-memory modes");
                let _ = self.persist_to_db();
            }
            Err(e) => {
                eprintln!("[modes] WARN: Failed to count modes in DB: {}", e);
            }
        }
    }

    /// Persiste tous les modes en mémoire vers la DB.
    fn persist_to_db(&self) -> Result<(), String> {
        if let Some(ref db) = self.db {
            let modes = self.modes.read();
            for mode in modes.values() {
                let theme_json = serde_json::to_string(&mode.theme)
                    .unwrap_or_else(|_| "{}".to_string());
                let created_at = mode.created_at
                    .format(&time::format_description::well_known::Rfc3339)
                    .unwrap_or_default();
                let row = crate::database::config_queries::ModeRow {
                    id: mode.id.clone(),
                    name: mode.name.clone(),
                    slug: mode.slug.clone(),
                    icon: mode.icon.clone(),
                    theme_json,
                    is_system: mode.is_system,
                    created_at,
                    display_order: mode.display_order as i32,
                };
                if let Err(e) = crate::database::config_queries::upsert_mode(db, &row) {
                    eprintln!("[modes] WARN: DB upsert failed for {}: {}", mode.id, e);
                    return Err(format!("DB upsert failed: {}", e));
                }
            }
        }
        Ok(())
    }

    /// Initialise les modes système par défaut (bootstrap uniquement)
    fn init_system_modes(&self) {
        let mut modes = self.modes.write();
        for mode in default_system_modes() {
            modes.insert(mode.id.clone(), mode);
        }
        eprintln!("[modes] Bootstrapped {} system modes", modes.len());
    }

    /// Charge TOUS les modes depuis le disque (système + custom)
    /// Retourne true si le fichier existait avec des modes valides
    fn load_from_disk(&self) -> bool {
        if !self.persistence_path.exists() {
            eprintln!("[modes] No modes file found, will bootstrap defaults");
            return false;
        }

        match std::fs::read_to_string(&self.persistence_path) {
            Ok(content) => {
                match serde_json::from_str::<Vec<DynamicMode>>(&content) {
                    Ok(all_modes) => {
                        // Si le fichier est vide, on bootstrap
                        if all_modes.is_empty() {
                            eprintln!("[modes] Modes file is empty, will bootstrap defaults");
                            return false;
                        }
                        let mut modes = self.modes.write();
                        let count = all_modes.len();
                        for mode in all_modes {
                            modes.insert(mode.id.clone(), mode);
                        }
                        eprintln!("[modes] Loaded {} modes from disk", count);
                        true
                    }
                    Err(e) => {
                        eprintln!("[modes] Failed to parse modes file: {}", e);
                        false
                    }
                }
            }
            Err(e) => {
                eprintln!("[modes] Failed to read modes file: {}", e);
                false
            }
        }
    }

    /// Sauvegarde TOUS les modes sur le disque (système + custom)
    /// DB-primary / JSON-fallback: écrit en DB d'abord, puis JSON comme backup.
    fn save_to_disk(&self) -> Result<(), String> {
        // --- DB-primary: try writing to SQLite first ---
        if self.db.is_some() {
            match self.persist_to_db() {
                Ok(()) => {
                    eprintln!("[modes] Persisted modes to SQLite (DB-primary)");
                }
                Err(e) => {
                    eprintln!("[modes] WARN: DB write failed, falling through to JSON-only: {}", e);
                }
            }
        }

        // --- JSON fallback (always written as backup) ---
        let modes = self.modes.read();
        let all_modes: Vec<&DynamicMode> = modes.values().collect();

        // Créer le répertoire parent si nécessaire
        if let Some(parent) = self.persistence_path.parent() {
            std::fs::create_dir_all(parent)
                .map_err(|e| format!("Failed to create directory: {}", e))?;
        }

        let json = serde_json::to_string_pretty(&all_modes)
            .map_err(|e| format!("Failed to serialize modes: {}", e))?;

        std::fs::write(&self.persistence_path, json)
            .map_err(|e| format!("Failed to write modes file: {}", e))?;

        eprintln!("[modes] Saved {} modes to disk (JSON backup)", all_modes.len());
        Ok(())
    }

    /// Liste tous les modes triés par display_order
    pub fn list_all(&self) -> Vec<DynamicMode> {
        let modes = self.modes.read();
        let mut list: Vec<DynamicMode> = modes.values().cloned().collect();
        list.sort_by_key(|m| m.display_order);
        list
    }

    /// Récupère un mode par ID
    pub fn get(&self, id: &str) -> Option<DynamicMode> {
        self.modes.read().get(id).cloned()
    }

    /// Récupère un mode par slug
    pub fn get_by_slug(&self, slug: &str) -> Option<DynamicMode> {
        let slug_lower = slug.to_lowercase();
        self.modes.read().values()
            .find(|m| m.slug.to_lowercase() == slug_lower)
            .cloned()
    }

    /// Crée un nouveau mode personnalisé
    pub fn create(&self, request: CreateModeRequest) -> Result<DynamicMode, String> {
        // Vérifier que le nom n'existe pas déjà
        let slug = request.name.to_lowercase()
            .chars()
            .map(|c| if c.is_alphanumeric() { c } else { '-' })
            .collect::<String>()
            .trim_matches('-')
            .to_string();

        {
            let modes = self.modes.read();
            if modes.values().any(|m| m.slug == slug) {
                return Err(format!("Un mode avec le slug '{}' existe déjà", slug));
            }
        }

        let mode = DynamicMode::new(request.name, request.icon, request.theme);

        {
            let mut modes = self.modes.write();
            modes.insert(mode.id.clone(), mode.clone());
        }

        self.save_to_disk()?;
        eprintln!("[modes] Created custom mode: {} ({})", mode.name, mode.id);

        Ok(mode)
    }

    /// Met à jour un mode existant
    pub fn update(&self, id: &str, request: UpdateModeRequest) -> Result<DynamicMode, String> {
        let mut modes = self.modes.write();

        let mode = modes.get_mut(id)
            .ok_or_else(|| format!("Mode '{}' non trouvé", id))?;

        // Interdire la modification du slug des modes système
        if mode.is_system && request.name.is_some() {
            // On peut modifier le nom d'affichage mais pas le slug
        }

        if let Some(name) = request.name {
            mode.name = name;
            // Ne pas modifier le slug pour éviter de casser les références
        }
        if let Some(icon) = request.icon {
            mode.icon = icon;
        }
        if let Some(theme) = request.theme {
            mode.theme = theme;
        }
        if let Some(order) = request.display_order {
            mode.display_order = order;
        }

        let updated = mode.clone();
        drop(modes);

        self.save_to_disk()?;
        eprintln!("[modes] Updated mode: {} ({})", updated.name, updated.id);

        Ok(updated)
    }

    /// Supprime un mode personnalisé
    pub fn delete(&self, id: &str) -> Result<(), String> {
        let mut modes = self.modes.write();

        let mode = modes.get(id)
            .ok_or_else(|| format!("Mode '{}' non trouvé", id))?;

        if mode.is_system {
            return Err("Impossible de supprimer un mode système".to_string());
        }

        let name = mode.name.clone();
        modes.remove(id);
        drop(modes);

        self.save_to_disk()?;
        eprintln!("[modes] Deleted mode: {} ({})", name, id);

        Ok(())
    }

    /// Récupère le mode par défaut (Veille)
    pub fn get_default(&self) -> DynamicMode {
        self.get("mode-veille")
            .unwrap_or_else(|| default_system_modes().last()
                .expect("[P0-4] default_system_modes() is hardcoded non-empty Vec")
                .clone())
    }

    /// Compte le nombre de modes
    pub fn count(&self) -> usize {
        self.modes.read().len()
    }

    /// Vérifie si un mode existe
    pub fn exists(&self, id: &str) -> bool {
        self.modes.read().contains_key(id)
    }
}

/// Type partagé pour le registre
pub type SharedModeRegistry = Arc<ModeRegistry>;

/// Crée un registre partagé
pub fn create_shared_registry(data_dir: PathBuf) -> SharedModeRegistry {
    Arc::new(ModeRegistry::new(data_dir))
}
