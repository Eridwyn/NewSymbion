// Configuration Manager - Hot-reload + Validation + Fallback
// Spec: PR3 P0 v3.1 REFINED - CORRECTION 1

use crate::decision::DecisionConfig;
use anyhow::{Context, Result, bail};
use parking_lot::RwLock;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::SystemTime;

/// Gestionnaire de configuration avec hot-reload
pub struct ConfigManager {
    config: Arc<RwLock<DecisionConfig>>,
    file_path: PathBuf,
    last_modified: Arc<RwLock<Option<SystemTime>>>,
}

impl ConfigManager {
    /// Créer un nouveau gestionnaire de configuration
    pub fn new<P: AsRef<Path>>(file_path: P) -> Result<Self> {
        let file_path = file_path.as_ref().to_path_buf();

        // Charger config initiale (avec fallback si nécessaire)
        let config = Self::load_config(&file_path)?;

        // Obtenir timestamp dernière modification
        let last_modified = fs::metadata(&file_path)
            .ok()
            .and_then(|m| m.modified().ok());

        Ok(Self {
            config: Arc::new(RwLock::new(config)),
            file_path,
            last_modified: Arc::new(RwLock::new(last_modified)),
        })
    }

    /// Charger configuration depuis fichier (avec fallback)
    fn load_config<P: AsRef<Path>>(file_path: P) -> Result<DecisionConfig> {
        let path = file_path.as_ref();

        // Fichier n'existe pas → Utiliser default
        if !path.exists() {
            eprintln!("[config] File not found: {:?}, using default config", path);
            return Ok(DecisionConfig::default());
        }

        // Lire fichier
        let contents = fs::read_to_string(path)
            .context("Failed to read config file")?;

        // Parser YAML
        let config: DecisionConfig = match serde_yaml::from_str(&contents) {
            Ok(c) => c,
            Err(e) => {
                eprintln!("[config] Parse error: {}, using default config", e);
                return Ok(DecisionConfig::default());
            }
        };

        // Valider configuration
        if let Err(e) = Self::validate_config(&config) {
            eprintln!("[config] Validation failed: {}, using default config", e);
            return Ok(DecisionConfig::default());
        }

        println!("[config] Loaded config version {} from {:?}", config.version, path);
        Ok(config)
    }

    /// Valider configuration
    fn validate_config(config: &DecisionConfig) -> Result<()> {
        // Valider weights (entre 0.0 et 1.0)
        let weights = &config.trust_weights;
        if !(0.0..=1.0).contains(&weights.context_match) {
            bail!("context_match weight out of range: {}", weights.context_match);
        }
        if !(0.0..=1.0).contains(&weights.temporal_consistency) {
            bail!("temporal_consistency weight out of range: {}", weights.temporal_consistency);
        }
        if !(0.0..=1.0).contains(&weights.agent_health) {
            bail!("agent_health weight out of range: {}", weights.agent_health);
        }
        if !(0.0..=1.0).contains(&weights.recent_success_rate) {
            bail!("recent_success_rate weight out of range: {}", weights.recent_success_rate);
        }
        if !(0.0..=1.0).contains(&weights.user_approval_history) {
            bail!("user_approval_history weight out of range: {}", weights.user_approval_history);
        }

        // Valider somme des weights = 1.0 (±0.01 tolérance)
        let sum = weights.context_match
            + weights.temporal_consistency
            + weights.agent_health
            + weights.recent_success_rate
            + weights.user_approval_history;

        if (sum - 1.0).abs() > 0.01 {
            bail!("Sum of weights must be 1.0 (±0.01), got {}", sum);
        }

        // Valider thresholds strictement croissants
        let thresholds = &config.impact_thresholds;
        if !(thresholds.low < thresholds.medium) {
            bail!("Thresholds must be strictly increasing: low ({}) < medium ({})",
                  thresholds.low, thresholds.medium);
        }
        if !(thresholds.medium < thresholds.high) {
            bail!("Thresholds must be strictly increasing: medium ({}) < high ({})",
                  thresholds.medium, thresholds.high);
        }
        if !(thresholds.high < thresholds.very_high) {
            bail!("Thresholds must be strictly increasing: high ({}) < very_high ({})",
                  thresholds.high, thresholds.very_high);
        }

        // Valider thresholds (low/medium/high: 0-1, very_high: 0-2 pour permettre auto-approve impossible)
        if !(0.0..=1.0).contains(&thresholds.low) {
            bail!("low threshold out of range [0,1]: {}", thresholds.low);
        }
        // very_high > 1.0 is intentional: makes auto-approval impossible for critical actions
        if !(0.0..=2.0).contains(&thresholds.very_high) {
            bail!("very_high threshold out of range [0,2]: {}", thresholds.very_high);
        }

        Ok(())
    }

    /// Sauvegarder configuration dans fichier
    pub fn save(&self, config: &DecisionConfig) -> Result<()> {
        // Valider avant sauvegarde
        Self::validate_config(config)
            .context("Config validation failed before save")?;

        // Créer répertoire parent si nécessaire
        if let Some(parent) = self.file_path.parent() {
            fs::create_dir_all(parent)
                .with_context(|| format!("Failed to create parent directory: {:?}", parent))?;
        }

        // Sérialiser en YAML
        let yaml = serde_yaml::to_string(config)
            .context("Failed to serialize config to YAML")?;

        // Écrire dans fichier
        fs::write(&self.file_path, yaml)
            .with_context(|| format!("Failed to write config to {:?}", self.file_path))?;

        // Mettre à jour config en mémoire
        *self.config.write() = config.clone();

        // Mettre à jour timestamp
        if let Ok(metadata) = fs::metadata(&self.file_path) {
            if let Ok(modified) = metadata.modified() {
                *self.last_modified.write() = Some(modified);
            }
        }

        println!("[config] Saved config version {} to {:?}", config.version, self.file_path);
        Ok(())
    }

    /// Recharger configuration si fichier modifié
    /// Retourne true si config a changé, false sinon
    pub fn reload(&self) -> Result<bool> {
        // Vérifier si fichier modifié
        let metadata = fs::metadata(&self.file_path)
            .context("Failed to read config file metadata")?;

        let current_modified = metadata.modified()
            .context("Failed to get file modification time")?;

        let last_modified = *self.last_modified.read();

        // Pas de changement
        if let Some(last) = last_modified {
            if current_modified <= last {
                return Ok(false);
            }
        }

        // Fichier modifié, recharger
        println!("[config] File modified, reloading...");

        let new_config = Self::load_config(&self.file_path)?;

        // Vérifier si config réellement différente
        let current_config = self.config.read().clone();
        if new_config.version == current_config.version
            && new_config.trust_weights == current_config.trust_weights
            && new_config.impact_thresholds == current_config.impact_thresholds {
            println!("[config] Config unchanged (same values)");
            *self.last_modified.write() = Some(current_modified);
            return Ok(false);
        }

        // Appliquer nouvelle config
        *self.config.write() = new_config;
        *self.last_modified.write() = Some(current_modified);

        println!("[config] Config reloaded successfully");
        Ok(true)
    }

    /// Obtenir configuration actuelle (clone thread-safe)
    pub fn get_config(&self) -> DecisionConfig {
        self.config.read().clone()
    }

    /// Obtenir référence Arc pour partage entre composants
    pub fn config_ref(&self) -> Arc<RwLock<DecisionConfig>> {
        Arc::clone(&self.config)
    }

    /// Obtenir chemin du fichier
    pub fn file_path(&self) -> &Path {
        &self.file_path
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::decision::{ImpactThresholds, TrustWeights};
    use uuid::Uuid;

    fn create_test_config() -> DecisionConfig {
        DecisionConfig {
            version: 1,
            trust_weights: TrustWeights {
                context_match: 0.3,
                temporal_consistency: 0.2,
                agent_health: 0.2,
                recent_success_rate: 0.15,
                user_approval_history: 0.15,
            },
            impact_thresholds: ImpactThresholds {
                low: 0.3,
                medium: 0.5,
                high: 0.7,
                very_high: 0.9,
            },
            agent_health_mapping: Default::default(),
        }
    }

    #[test]
    fn test_config_manager_new_with_default() {
        let temp_file = format!("/tmp/test-config-{}.yaml", Uuid::new_v4());

        // Fichier n'existe pas → devrait utiliser default
        let manager = ConfigManager::new(&temp_file)
            .expect("Failed to create ConfigManager");

        let config = manager.get_config();
        assert_eq!(config.version, 1);

        // Cleanup
        std::fs::remove_file(&temp_file).ok();
    }

    #[test]
    fn test_config_save_and_load() {
        let temp_file = format!("/tmp/test-config-{}.yaml", Uuid::new_v4());
        let manager = ConfigManager::new(&temp_file)
            .expect("Failed to create ConfigManager");

        let config = create_test_config();
        manager.save(&config).expect("Failed to save config");

        // Créer nouveau manager pour charger
        let manager2 = ConfigManager::new(&temp_file)
            .expect("Failed to create ConfigManager");

        let loaded = manager2.get_config();
        assert_eq!(loaded.version, 1);
        assert_eq!(loaded.trust_weights.context_match, 0.3);

        // Cleanup
        std::fs::remove_file(&temp_file).ok();
    }

    #[test]
    fn test_config_validation_weights_sum() {
        let mut config = create_test_config();

        // Poids invalides (somme != 1.0)
        config.trust_weights.context_match = 0.5;
        config.trust_weights.temporal_consistency = 0.5;
        config.trust_weights.agent_health = 0.5;
        config.trust_weights.recent_success_rate = 0.0;
        config.trust_weights.user_approval_history = 0.0;

        let result = ConfigManager::validate_config(&config);
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("Sum of weights"));
    }

    #[test]
    fn test_config_validation_thresholds_order() {
        let mut config = create_test_config();

        // Thresholds non croissants
        config.impact_thresholds.low = 0.8;
        config.impact_thresholds.medium = 0.5;

        let result = ConfigManager::validate_config(&config);
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("strictly increasing"));
    }

    #[test]
    fn test_config_reload_no_change() {
        let temp_file = format!("/tmp/test-config-{}.yaml", Uuid::new_v4());
        let manager = ConfigManager::new(&temp_file)
            .expect("Failed to create ConfigManager");

        let config = create_test_config();
        manager.save(&config).expect("Failed to save config");

        // Reload sans modification
        let changed = manager.reload().expect("Failed to reload");
        assert!(!changed);

        // Cleanup
        std::fs::remove_file(&temp_file).ok();
    }

    #[test]
    fn test_config_reload_with_change() {
        let temp_file = format!("/tmp/test-config-{}.yaml", Uuid::new_v4());
        let manager = ConfigManager::new(&temp_file)
            .expect("Failed to create ConfigManager");

        let config = create_test_config();
        manager.save(&config).expect("Failed to save config");

        // Attendre un peu pour garantir timestamp différent
        std::thread::sleep(std::time::Duration::from_millis(10));

        // Modifier fichier manuellement
        let mut modified_config = config.clone();
        modified_config.version = 2;
        let yaml = serde_yaml::to_string(&modified_config).unwrap();
        fs::write(&temp_file, yaml).unwrap();

        // Reload devrait détecter changement
        let changed = manager.reload().expect("Failed to reload");
        assert!(changed);

        let loaded = manager.get_config();
        assert_eq!(loaded.version, 2);

        // Cleanup
        std::fs::remove_file(&temp_file).ok();
    }

    #[test]
    fn test_config_fallback_on_parse_error() {
        let temp_file = format!("/tmp/test-config-{}.yaml", Uuid::new_v4());

        // Écrire fichier YAML invalide
        fs::write(&temp_file, "invalid: yaml: content: [[[").unwrap();

        // Devrait fallback sur default
        let manager = ConfigManager::new(&temp_file)
            .expect("Failed to create ConfigManager");

        let config = manager.get_config();
        assert_eq!(config.version, 1); // Default

        // Cleanup
        std::fs::remove_file(&temp_file).ok();
    }

    #[test]
    fn test_config_fallback_on_validation_error() {
        let temp_file = format!("/tmp/test-config-{}.yaml", Uuid::new_v4());

        // Créer config invalide
        let mut config = create_test_config();
        config.trust_weights.context_match = 2.0; // Hors range

        let yaml = serde_yaml::to_string(&config).unwrap();
        fs::write(&temp_file, yaml).unwrap();

        // Devrait fallback sur default
        let manager = ConfigManager::new(&temp_file)
            .expect("Failed to create ConfigManager");

        let loaded = manager.get_config();
        assert_eq!(loaded.trust_weights.context_match, 0.25); // Default

        // Cleanup
        std::fs::remove_file(&temp_file).ok();
    }
}
