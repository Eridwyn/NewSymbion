//! Notification Configuration Module
//!
//! Permet de configurer chaque type de notification :
//! - Activer/désactiver
//! - Modifier le message (titre, corps)
//! - Changer la priorité
//!
//! Les templates supportent des variables : {variable_name}

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::{Arc, RwLock};
use utoipa::ToSchema;

/// Fichier de configuration des notifications
const CONFIG_FILE: &str = "/var/lib/symbion/notification_configs.json";

/// Priorité de notification
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, ToSchema)]
#[serde(rename_all = "UPPERCASE")]
pub enum NotificationPriority {
    P0,
    P1,
    P2,
}

impl Default for NotificationPriority {
    fn default() -> Self {
        NotificationPriority::P2
    }
}

impl From<NotificationPriority> for crate::notifications::NotificationPriority {
    fn from(p: NotificationPriority) -> Self {
        match p {
            NotificationPriority::P0 => crate::notifications::NotificationPriority::P0,
            NotificationPriority::P1 => crate::notifications::NotificationPriority::P1,
            NotificationPriority::P2 => crate::notifications::NotificationPriority::P2,
        }
    }
}

/// Catégorie de notification
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, ToSchema)]
#[serde(rename_all = "snake_case")]
pub enum NotificationCategory {
    PluginHealth,
    Environment,
    Automation,
    Security,
    System,
}

/// Configuration d'un type de notification
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct NotificationTypeConfig {
    /// Identifiant unique du type
    pub type_id: String,
    /// Nom affiché dans l'UI
    pub display_name: String,
    /// Description du déclencheur
    pub description: String,
    /// Catégorie
    pub category: NotificationCategory,
    /// Activé ou non
    pub enabled: bool,
    /// Template du titre (supporte {variables})
    pub title_template: String,
    /// Template du corps (supporte {variables})
    pub body_template: String,
    /// Priorité
    pub priority: NotificationPriority,
    /// Variables disponibles pour ce type
    pub available_variables: Vec<VariableInfo>,
}

/// Information sur une variable disponible
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct VariableInfo {
    /// Nom de la variable (sans les {})
    pub name: String,
    /// Description de la variable
    pub description: String,
    /// Exemple de valeur
    pub example: String,
}

/// Manager de configuration des notifications
pub struct NotificationConfigManager {
    configs: Arc<RwLock<HashMap<String, NotificationTypeConfig>>>,
}

impl NotificationConfigManager {
    pub fn new() -> Self {
        let mut manager = Self {
            configs: Arc::new(RwLock::new(HashMap::new())),
        };
        manager.load_or_init_defaults();
        manager
    }

    /// Charge les configs depuis le fichier ou initialise les défauts
    fn load_or_init_defaults(&mut self) {
        // Essayer de charger depuis le fichier
        if let Ok(content) = std::fs::read_to_string(CONFIG_FILE) {
            if let Ok(configs) = serde_json::from_str::<Vec<NotificationTypeConfig>>(&content) {
                let mut map = self.configs.write().unwrap();
                for config in configs {
                    map.insert(config.type_id.clone(), config);
                }
                println!("[notification-config] Loaded {} configs from file", map.len());

                // Ajouter les nouveaux types qui n'existent pas encore
                drop(map);
                self.add_missing_defaults();
                return;
            }
        }

        // Initialiser avec les valeurs par défaut
        self.init_defaults();
        self.save_to_file();
    }

    /// Ajoute les types par défaut manquants
    fn add_missing_defaults(&self) {
        let defaults = Self::get_default_configs();
        let mut map = self.configs.write().unwrap();
        let mut added = 0;

        for config in defaults {
            if !map.contains_key(&config.type_id) {
                map.insert(config.type_id.clone(), config);
                added += 1;
            }
        }

        if added > 0 {
            println!("[notification-config] Added {} new notification types", added);
            drop(map);
            self.save_to_file();
        }
    }

    /// Initialise les configurations par défaut
    fn init_defaults(&self) {
        let defaults = Self::get_default_configs();
        let mut map = self.configs.write().unwrap();

        for config in defaults {
            map.insert(config.type_id.clone(), config);
        }

        println!("[notification-config] Initialized {} default configs", map.len());
    }

    /// Retourne les configurations par défaut
    fn get_default_configs() -> Vec<NotificationTypeConfig> {
        vec![
            // === PLUGIN HEALTH ===
            NotificationTypeConfig {
                type_id: "plugin_health_recovery_attempt".to_string(),
                display_name: "Plugin - Tentative recovery".to_string(),
                description: "Quand un plugin ne répond plus et qu'une tentative de redémarrage est lancée".to_string(),
                category: NotificationCategory::PluginHealth,
                enabled: true,
                title_template: "🔄 Plugin {plugin_name} - Recovery".to_string(),
                body_template: "Le plugin {plugin_name} est injoignable ({failures} échecs). Tentative de redémarrage automatique...".to_string(),
                priority: NotificationPriority::P1,
                available_variables: vec![
                    VariableInfo { name: "plugin_name".to_string(), description: "Nom du plugin".to_string(), example: "notes".to_string() },
                    VariableInfo { name: "failures".to_string(), description: "Nombre d'échecs consécutifs".to_string(), example: "3".to_string() },
                ],
            },
            NotificationTypeConfig {
                type_id: "plugin_health_recovery_failed".to_string(),
                display_name: "Plugin - Recovery échouée".to_string(),
                description: "Quand le redémarrage automatique d'un plugin a échoué".to_string(),
                category: NotificationCategory::PluginHealth,
                enabled: true,
                title_template: "❌ Plugin {plugin_name} - Échec recovery".to_string(),
                body_template: "Le redémarrage automatique du plugin {plugin_name} a échoué: {error}".to_string(),
                priority: NotificationPriority::P0,
                available_variables: vec![
                    VariableInfo { name: "plugin_name".to_string(), description: "Nom du plugin".to_string(), example: "notes".to_string() },
                    VariableInfo { name: "error".to_string(), description: "Message d'erreur".to_string(), example: "systemctl restart failed".to_string() },
                ],
            },
            NotificationTypeConfig {
                type_id: "plugin_health_recovery_success".to_string(),
                display_name: "Plugin - Recovery réussie".to_string(),
                description: "Quand un plugin a été redémarré avec succès".to_string(),
                category: NotificationCategory::PluginHealth,
                enabled: true,
                title_template: "✅ Plugin {plugin_name} - Redémarré".to_string(),
                body_template: "Le plugin {plugin_name} a été redémarré automatiquement avec succès.".to_string(),
                priority: NotificationPriority::P2,
                available_variables: vec![
                    VariableInfo { name: "plugin_name".to_string(), description: "Nom du plugin".to_string(), example: "notes".to_string() },
                ],
            },

            // === ENVIRONMENT ALERTS ===
            NotificationTypeConfig {
                type_id: "environment_alert_danger".to_string(),
                display_name: "Environnement - DANGER".to_string(),
                description: "Condensation certaine détectée (HR > 75% ou ΔT ≤ 0°C)".to_string(),
                category: NotificationCategory::Environment,
                enabled: true,
                title_template: "🚨 DANGER - {room_id}".to_string(),
                body_template: "Condensation certaine! {diagnostics}\nAction: {suggestion}".to_string(),
                priority: NotificationPriority::P0,
                available_variables: vec![
                    VariableInfo { name: "room_id".to_string(), description: "Identifiant de la pièce".to_string(), example: "chambre".to_string() },
                    VariableInfo { name: "diagnostics".to_string(), description: "Données capteurs".to_string(), example: "HR: 78% | T: 18°C | Rosée: 14°C | ΔT: -1°C".to_string() },
                    VariableInfo { name: "suggestion".to_string(), description: "Action recommandée".to_string(), example: "Aérer immédiatement".to_string() },
                ],
            },
            NotificationTypeConfig {
                type_id: "environment_alert_critical".to_string(),
                display_name: "Environnement - Critique".to_string(),
                description: "Condensation très probable (HR > 70% ou ΔT < 2°C)".to_string(),
                category: NotificationCategory::Environment,
                enabled: true,
                title_template: "⚠️ CRITIQUE - {room_id}".to_string(),
                body_template: "Condensation très probable! {diagnostics}\nAction: {suggestion}".to_string(),
                priority: NotificationPriority::P0,
                available_variables: vec![
                    VariableInfo { name: "room_id".to_string(), description: "Identifiant de la pièce".to_string(), example: "salon".to_string() },
                    VariableInfo { name: "diagnostics".to_string(), description: "Données capteurs".to_string(), example: "HR: 72% | T: 20°C".to_string() },
                    VariableInfo { name: "suggestion".to_string(), description: "Action recommandée".to_string(), example: "Ventiler la pièce".to_string() },
                ],
            },
            NotificationTypeConfig {
                type_id: "environment_alert_strong".to_string(),
                display_name: "Environnement - Risque fort".to_string(),
                description: "Risque de condensation (HR > 65% ou ΔT < 3°C pendant 1h)".to_string(),
                category: NotificationCategory::Environment,
                enabled: true,
                title_template: "🟠 Risque condensation - {room_id}".to_string(),
                body_template: "Risque de condensation détecté. {diagnostics}\nAction: {suggestion}".to_string(),
                priority: NotificationPriority::P1,
                available_variables: vec![
                    VariableInfo { name: "room_id".to_string(), description: "Identifiant de la pièce".to_string(), example: "bureau".to_string() },
                    VariableInfo { name: "diagnostics".to_string(), description: "Données capteurs".to_string(), example: "HR: 67% | T: 21°C".to_string() },
                    VariableInfo { name: "suggestion".to_string(), description: "Action recommandée".to_string(), example: "Surveiller l'humidité".to_string() },
                ],
            },
            NotificationTypeConfig {
                type_id: "environment_alert_moderate".to_string(),
                display_name: "Environnement - Humidité excessive".to_string(),
                description: "Humidité excessive prolongée (HR > 60% pendant 3h)".to_string(),
                category: NotificationCategory::Environment,
                enabled: true,
                title_template: "🟡 Humidité excessive - {room_id}".to_string(),
                body_template: "Humidité excessive prolongée. {diagnostics}\nAction: {suggestion}".to_string(),
                priority: NotificationPriority::P1,
                available_variables: vec![
                    VariableInfo { name: "room_id".to_string(), description: "Identifiant de la pièce".to_string(), example: "cave".to_string() },
                    VariableInfo { name: "diagnostics".to_string(), description: "Données capteurs".to_string(), example: "HR: 63%".to_string() },
                    VariableInfo { name: "suggestion".to_string(), description: "Action recommandée".to_string(), example: "Activer le déshumidificateur".to_string() },
                ],
            },
            NotificationTypeConfig {
                type_id: "environment_alert_weak".to_string(),
                display_name: "Environnement - Humidité haute".to_string(),
                description: "Humidité en tendance haute (HR > 55% pendant 6h)".to_string(),
                category: NotificationCategory::Environment,
                enabled: false, // Désactivé par défaut (trop fréquent)
                title_template: "💧 Humidité haute - {room_id}".to_string(),
                body_template: "Humidité en tendance haute. {diagnostics}\nAction: {suggestion}".to_string(),
                priority: NotificationPriority::P2,
                available_variables: vec![
                    VariableInfo { name: "room_id".to_string(), description: "Identifiant de la pièce".to_string(), example: "sdb".to_string() },
                    VariableInfo { name: "diagnostics".to_string(), description: "Données capteurs".to_string(), example: "HR: 58%".to_string() },
                    VariableInfo { name: "suggestion".to_string(), description: "Action recommandée".to_string(), example: "Aérer après douche".to_string() },
                ],
            },
            NotificationTypeConfig {
                type_id: "environment_alert_recovery".to_string(),
                display_name: "Environnement - Retour normal".to_string(),
                description: "Les conditions environnementales sont revenues à la normale".to_string(),
                category: NotificationCategory::Environment,
                enabled: true,
                title_template: "✅ Retour normal - {room_id}".to_string(),
                body_template: "Les conditions sont revenues à la normale (était: {previous_level})".to_string(),
                priority: NotificationPriority::P2,
                available_variables: vec![
                    VariableInfo { name: "room_id".to_string(), description: "Identifiant de la pièce".to_string(), example: "chambre".to_string() },
                    VariableInfo { name: "previous_level".to_string(), description: "Niveau d'alerte précédent".to_string(), example: "Critical".to_string() },
                ],
            },

            // === AUTOMATION ===
            NotificationTypeConfig {
                type_id: "automation_requires_validation".to_string(),
                display_name: "Automation - Validation requise".to_string(),
                description: "Une automation nécessite une validation humaine avant exécution".to_string(),
                category: NotificationCategory::Automation,
                enabled: true,
                title_template: "⚠️ Validation requise: {automation_name}".to_string(),
                body_template: "Action: {action_description}\nRaison: {blocked_reasons}\nID: {validation_id}".to_string(),
                priority: NotificationPriority::P1,
                available_variables: vec![
                    VariableInfo { name: "automation_name".to_string(), description: "Nom de l'automation".to_string(), example: "Extinction PC soir".to_string() },
                    VariableInfo { name: "action_description".to_string(), description: "Description de l'action".to_string(), example: "Éteindre PC-Bureau".to_string() },
                    VariableInfo { name: "blocked_reasons".to_string(), description: "Raisons du blocage".to_string(), example: "impact élevé, hors heures".to_string() },
                    VariableInfo { name: "validation_id".to_string(), description: "ID de validation".to_string(), example: "abc-123".to_string() },
                ],
            },

            // === SECURITY ===
            NotificationTypeConfig {
                type_id: "auth_security_failed_login".to_string(),
                display_name: "Sécurité - Échec connexion".to_string(),
                description: "Une tentative de connexion a échoué".to_string(),
                category: NotificationCategory::Security,
                enabled: true,
                title_template: "🔒 Alerte sécurité - Échec connexion".to_string(),
                body_template: "Tentative échouée. IP: {ip}, User: {username}, Raison: {reason}".to_string(),
                priority: NotificationPriority::P1,
                available_variables: vec![
                    VariableInfo { name: "ip".to_string(), description: "Adresse IP".to_string(), example: "192.168.1.100".to_string() },
                    VariableInfo { name: "username".to_string(), description: "Nom d'utilisateur".to_string(), example: "admin".to_string() },
                    VariableInfo { name: "reason".to_string(), description: "Raison de l'échec".to_string(), example: "mot de passe incorrect".to_string() },
                ],
            },
        ]
    }

    /// Sauvegarde les configs dans le fichier
    fn save_to_file(&self) {
        let configs = self.configs.read().unwrap();
        let list: Vec<&NotificationTypeConfig> = configs.values().collect();

        if let Some(parent) = std::path::Path::new(CONFIG_FILE).parent() {
            let _ = std::fs::create_dir_all(parent);
        }

        match serde_json::to_string_pretty(&list) {
            Ok(json) => {
                if let Err(e) = std::fs::write(CONFIG_FILE, json) {
                    eprintln!("[notification-config] Failed to save: {}", e);
                } else {
                    println!("[notification-config] Saved {} configs", list.len());
                }
            }
            Err(e) => eprintln!("[notification-config] Serialize error: {}", e),
        }
    }

    // === API publique ===

    /// Liste toutes les configurations
    pub fn list_all(&self) -> Vec<NotificationTypeConfig> {
        let configs = self.configs.read().unwrap();
        let mut list: Vec<NotificationTypeConfig> = configs.values().cloned().collect();
        list.sort_by(|a, b| a.type_id.cmp(&b.type_id));
        list
    }

    /// Récupère une configuration par son type_id
    pub fn get(&self, type_id: &str) -> Option<NotificationTypeConfig> {
        self.configs.read().unwrap().get(type_id).cloned()
    }

    /// Met à jour une configuration
    pub fn update(&self, type_id: &str, update: NotificationConfigUpdate) -> Result<NotificationTypeConfig, String> {
        let mut configs = self.configs.write().unwrap();

        if let Some(config) = configs.get_mut(type_id) {
            if let Some(enabled) = update.enabled {
                config.enabled = enabled;
            }
            if let Some(title) = update.title_template {
                config.title_template = title;
            }
            if let Some(body) = update.body_template {
                config.body_template = body;
            }
            if let Some(priority) = update.priority {
                config.priority = priority;
            }

            let updated = config.clone();
            drop(configs);
            self.save_to_file();

            println!("[notification-config] Updated: {}", type_id);
            Ok(updated)
        } else {
            Err(format!("Notification type '{}' not found", type_id))
        }
    }

    /// Vérifie si un type de notification est activé
    pub fn is_enabled(&self, type_id: &str) -> bool {
        self.configs
            .read()
            .unwrap()
            .get(type_id)
            .map(|c| c.enabled)
            .unwrap_or(true) // Par défaut activé si non trouvé
    }

    /// Récupère le template et la priorité pour un type
    pub fn get_template(&self, type_id: &str) -> Option<(String, String, NotificationPriority)> {
        self.configs
            .read()
            .unwrap()
            .get(type_id)
            .filter(|c| c.enabled)
            .map(|c| (c.title_template.clone(), c.body_template.clone(), c.priority.clone()))
    }

    /// Interpole les variables dans un template
    pub fn interpolate(template: &str, variables: &HashMap<String, String>) -> String {
        let mut result = template.to_string();
        for (key, value) in variables {
            result = result.replace(&format!("{{{}}}", key), value);
        }
        result
    }

    /// Génère le titre et corps interpolés pour une notification
    /// Retourne None si le type est désactivé
    pub fn build_notification(
        &self,
        type_id: &str,
        variables: &HashMap<String, String>,
    ) -> Option<(String, String, NotificationPriority)> {
        let (title_template, body_template, priority) = self.get_template(type_id)?;

        let title = Self::interpolate(&title_template, variables);
        let body = Self::interpolate(&body_template, variables);

        Some((title, body, priority))
    }
}

/// Mise à jour partielle d'une configuration
#[derive(Debug, Clone, Deserialize, ToSchema)]
pub struct NotificationConfigUpdate {
    pub enabled: Option<bool>,
    pub title_template: Option<String>,
    pub body_template: Option<String>,
    pub priority: Option<NotificationPriority>,
}

impl Default for NotificationConfigManager {
    fn default() -> Self {
        Self::new()
    }
}

/// Type partagé pour le manager
pub type SharedNotificationConfigManager = Arc<NotificationConfigManager>;

/// Crée un manager partagé
pub fn create_shared_config_manager() -> SharedNotificationConfigManager {
    Arc::new(NotificationConfigManager::new())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_interpolate() {
        let template = "Hello {name}, you have {count} messages";
        let mut vars = HashMap::new();
        vars.insert("name".to_string(), "Alice".to_string());
        vars.insert("count".to_string(), "5".to_string());

        let result = NotificationConfigManager::interpolate(template, &vars);
        assert_eq!(result, "Hello Alice, you have 5 messages");
    }

    #[test]
    fn test_default_configs() {
        let configs = NotificationConfigManager::get_default_configs();
        assert!(configs.len() >= 10);

        // Vérifier qu'on a des configs de chaque catégorie
        assert!(configs.iter().any(|c| matches!(c.category, NotificationCategory::PluginHealth)));
        assert!(configs.iter().any(|c| matches!(c.category, NotificationCategory::Environment)));
        assert!(configs.iter().any(|c| matches!(c.category, NotificationCategory::Automation)));
        assert!(configs.iter().any(|c| matches!(c.category, NotificationCategory::Security)));
    }
}
