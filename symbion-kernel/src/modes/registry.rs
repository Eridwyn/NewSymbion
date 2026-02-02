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
}

impl ModeRegistry {
    /// Crée un nouveau registre avec persistance
    pub fn new(data_dir: PathBuf) -> Self {
        let persistence_path = data_dir.join("modes.json");
        let registry = Self {
            modes: RwLock::new(HashMap::new()),
            persistence_path,
        };

        // Charger les modes système par défaut
        registry.init_system_modes();

        // Charger les modes personnalisés depuis le fichier
        registry.load_from_disk();

        registry
    }

    /// Initialise les modes système (écrase les existants)
    fn init_system_modes(&self) {
        let mut modes = self.modes.write();
        for mode in default_system_modes() {
            modes.insert(mode.id.clone(), mode);
        }
    }

    /// Charge les modes personnalisés depuis le disque
    fn load_from_disk(&self) {
        if !self.persistence_path.exists() {
            eprintln!("[modes] No custom modes file found at {:?}", self.persistence_path);
            return;
        }

        match std::fs::read_to_string(&self.persistence_path) {
            Ok(content) => {
                match serde_json::from_str::<Vec<DynamicMode>>(&content) {
                    Ok(custom_modes) => {
                        let mut modes = self.modes.write();
                        let count = custom_modes.len();
                        for mode in custom_modes {
                            // Ne pas écraser les modes système
                            if !mode.is_system {
                                modes.insert(mode.id.clone(), mode);
                            }
                        }
                        eprintln!("[modes] Loaded {} custom modes from disk", count);
                    }
                    Err(e) => {
                        eprintln!("[modes] Failed to parse modes file: {}", e);
                    }
                }
            }
            Err(e) => {
                eprintln!("[modes] Failed to read modes file: {}", e);
            }
        }
    }

    /// Sauvegarde les modes personnalisés sur le disque
    fn save_to_disk(&self) -> Result<(), String> {
        let modes = self.modes.read();
        let custom_modes: Vec<&DynamicMode> = modes.values()
            .filter(|m| !m.is_system)
            .collect();

        // Créer le répertoire parent si nécessaire
        if let Some(parent) = self.persistence_path.parent() {
            std::fs::create_dir_all(parent)
                .map_err(|e| format!("Failed to create directory: {}", e))?;
        }

        let json = serde_json::to_string_pretty(&custom_modes)
            .map_err(|e| format!("Failed to serialize modes: {}", e))?;

        std::fs::write(&self.persistence_path, json)
            .map_err(|e| format!("Failed to write modes file: {}", e))?;

        eprintln!("[modes] Saved {} custom modes to disk", custom_modes.len());
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
