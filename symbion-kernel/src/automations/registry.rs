/**
 * SYMBION KERNEL - Automations Schema Registry
 *
 * ROLE: Provide schema definitions for PWA rule builder
 *
 * ARCHITECTURE:
 * - Static schemas for core triggers, conditions, actions
 * - Dynamic values from kernel registries (agents, rooms, modes)
 * - Plugin registration for custom schemas (Phase 5+)
 */

use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

/// Complete automation schema for PWA
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct AutomationSchema {
    pub triggers: Vec<TriggerSchema>,
    pub trigger_group: TriggerGroupSchema,
    pub conditions: Vec<ConditionSchema>,
    pub actions: Vec<ActionSchema>,
    pub dynamic_values: DynamicValues,
}

/// Schema for trigger groups (AND/OR logic)
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct TriggerGroupSchema {
    pub supports_groups: bool,
    pub default_operator: String,
    pub max_depth: u8,
    pub operators: Vec<ValueOption>,
}

/// Dynamic values from kernel registries
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct DynamicValues {
    pub modes: Vec<ValueOption>,
    pub agents: Vec<ValueOption>,
    pub rooms: Vec<ValueOption>,
    pub sensors: Vec<SensorOption>,
    pub features: Vec<ValueOption>,
    pub categories: Vec<ValueOption>,
    pub alert_levels: Vec<ValueOption>,
    pub priorities: Vec<ValueOption>,
    pub command_types: Vec<ValueOption>,
    pub sensor_metrics: Vec<ValueOption>,
    pub comparison_operators: Vec<ValueOption>,
    pub weekdays: Vec<ValueOption>,
    pub days_of_month: Vec<ValueOption>,
    pub months: Vec<ValueOption>,
    pub plugins: Vec<ValueOption>,
    pub plugin_health_statuses: Vec<ValueOption>,
    pub ssl_domains: Vec<ValueOption>,
    pub ssl_statuses: Vec<ValueOption>,
}

/// Sensor option with room context
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct SensorOption {
    pub value: String,
    pub label: String,
    pub room_id: String,
    pub sensor_type: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub status: Option<String>,
}

/// Value option for dropdowns
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct ValueOption {
    pub value: String,
    pub label: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
}

/// Schema for a trigger type
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct TriggerSchema {
    #[serde(rename = "type")]
    pub trigger_type: String,
    pub label: String,
    pub description: String,
    pub icon: String,
    pub fields: Vec<FieldSchema>,
}

/// Schema for a condition type
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct ConditionSchema {
    #[serde(rename = "type")]
    pub condition_type: String,
    pub label: String,
    pub description: String,
    pub fields: Vec<FieldSchema>,
}

/// Schema for an action type
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct ActionSchema {
    #[serde(rename = "type")]
    pub action_type: String,
    pub label: String,
    pub description: String,
    pub icon: String,
    pub fields: Vec<FieldSchema>,
}

/// Field definition for forms
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct FieldSchema {
    pub name: String,
    pub label: String,
    pub field_type: FieldType,
    pub required: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[schema(value_type = Option<Object>)]
    pub default_value: Option<serde_json::Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub placeholder: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub options_key: Option<String>, // Key into dynamic_values
    #[serde(skip_serializing_if = "Option::is_none")]
    pub min: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max: Option<f64>,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "snake_case")]
pub enum FieldType {
    Text,
    Number,
    Select,
    MultiSelect,
    Boolean,
    TextArea,
}

/// Schema registry - builds complete schema from static + dynamic data
pub struct SchemaRegistry;

/// Sensor info passed from kernel
pub struct SensorInfo {
    pub sensor_id: String,
    pub sensor_type: String,
    pub room_id: String,
    pub status: String,
}

impl SchemaRegistry {
    /// Get complete schema with dynamic values
    pub fn get_schema(
        agents: &[(String, String)],  // (id, name/hostname)
        rooms: &[String],
        sensors: &[SensorInfo],
        modes: &[(String, String, String)],  // (slug, label, description)
    ) -> AutomationSchema {
        Self::get_schema_full(agents, rooms, sensors, modes, &[], &[], &[])
    }

    /// Get schema avec domaines SSL (kept pour back-compat, sans features dynamiques).
    pub fn get_schema_with_ssl(
        agents: &[(String, String)],
        rooms: &[String],
        sensors: &[SensorInfo],
        modes: &[(String, String, String)],
        ssl_domains: &[(String, String)],
    ) -> AutomationSchema {
        Self::get_schema_full(agents, rooms, sensors, modes, ssl_domains, &[], &[])
    }

    /// Get complete schema avec features actives énumérées depuis le FeatureRegistry.
    /// `live_features` doit contenir les feature_id réellement présents dans le registry
    /// (le caller s'en occupe via `feature_registry.get_all()`).
    /// `live_plugins` contient les noms de plugins réellement enregistrés au plugin_proxy.
    pub fn get_schema_full(
        agents: &[(String, String)],
        rooms: &[String],
        sensors: &[SensorInfo],
        modes: &[(String, String, String)],
        ssl_domains: &[(String, String)],
        live_features: &[String],  // feature_ids réels du registry
        live_plugins: &[String],   // noms de plugins réels (ex: ["coffee", "notes", ...])
    ) -> AutomationSchema {
        // Common features hardcodées (résilience offline + UX : on garde ces options même si
        // un plugin est down momentanément). Si elles existent en live, on les préfère.
        let mut features_by_id: std::collections::BTreeMap<String, String> = [
            ("presence.phone", "📱 Présence téléphone"),
            ("presence.anyone_home", "🏠 Quelqu'un à la maison"),
            ("env.temperature", "🌡️ Température"),
            ("env.humidity", "💧 Humidité"),
            ("process.category.ide", "💻 IDE actif"),
            ("process.category.media", "🎬 Média actif"),
            ("agent.online", "🖥️ Agent en ligne"),
            ("appearance.theme", "🌗 Thème interface (dark/light)"),
        ]
        .iter()
        .map(|(k, v)| (k.to_string(), v.to_string()))
        .collect();

        // SSL features par domaine (back-compat)
        for (domain_id, label) in ssl_domains {
            features_by_id.insert(
                format!("ssl.{}.valid", domain_id),
                format!("🔒 {} - Certificat valide", label),
            );
            features_by_id.insert(
                format!("ssl.{}.days_remaining", domain_id),
                format!("📅 {} - Jours restants", label),
            );
            features_by_id.insert(
                format!("ssl.{}.status", domain_id),
                format!("🚦 {} - Status (ok/warning/critical)", label),
            );
            features_by_id.insert(
                format!("ssl.{}.online", domain_id),
                format!("🌐 {} - Domaine en ligne", label),
            );
        }

        // Enrichir avec les features dynamiques du registry runtime
        for fid in live_features {
            // Skip si déjà présent (les hardcodées ont des labels manuels plus explicites)
            if features_by_id.contains_key(fid) {
                continue;
            }
            let icon = Self::feature_icon(fid);
            features_by_id.insert(fid.clone(), format!("{} {}", icon, fid));
        }

        let default_features: Vec<(String, String)> = features_by_id.into_iter().collect();

        AutomationSchema {
            triggers: Self::get_triggers(),
            trigger_group: Self::get_trigger_group_schema(),
            conditions: Self::get_conditions(),
            actions: Self::get_actions(),
            dynamic_values: Self::get_dynamic_values(
                agents, rooms, sensors, modes, &default_features, ssl_domains, live_plugins,
            ),
        }
    }

    /// Static trigger group schema
    fn get_trigger_group_schema() -> TriggerGroupSchema {
        TriggerGroupSchema {
            supports_groups: true,
            default_operator: "or".to_string(),
            max_depth: 2, // Maximum 2 levels of nesting
            operators: vec![
                ValueOption {
                    value: "or".to_string(),
                    label: "OU - Au moins un trigger".to_string(),
                    description: Some("L'automation se déclenche si au moins un trigger correspond".to_string()),
                },
                ValueOption {
                    value: "and".to_string(),
                    label: "ET - Tous les triggers".to_string(),
                    description: Some("Tous les triggers doivent correspondre au même événement".to_string()),
                },
            ],
        }
    }

    /// Map un nom de plugin vers une icône emoji pour le menu PWA.
    fn plugin_icon(plugin: &str) -> &'static str {
        match plugin {
            "coffee" => "☕",
            "library" => "📚",
            "notes" => "📝",
            "ssl" => "🔒",
            "sensors" => "🌡️",
            "freebox" => "📡",
            "telegram" => "📱",
            "common" => "🔧",
            _ => "🔌",
        }
    }

    /// Map un préfixe de feature_id vers une icône emoji pour le menu PWA.
    fn feature_icon(feature_id: &str) -> &'static str {
        let id = feature_id;
        if id.starts_with("coffee.") { "☕" }
        else if id.starts_with("library.") { "📚" }
        else if id.starts_with("ssl.") { "🔒" }
        else if id.starts_with("agent.") { "🖥️" }
        else if id.starts_with("agents.") { "🖧" }
        else if id.starts_with("freebox.") { "📡" }
        else if id.starts_with("presence.") { "👤" }
        else if id.starts_with("env.") { "🌡️" }
        else if id.starts_with("sensor.") { "🌡️" }
        else if id.starts_with("process.") { "💻" }
        else if id.starts_with("classifier.") { "🤖" }
        else if id.starts_with("appearance.") { "🎨" }
        else if id.starts_with("notes.") { "📝" }
        else if id.starts_with("telegram.") { "📱" }
        else if id.starts_with("schedule.") { "⏰" }
        else if id.starts_with("kernel.") { "⚙️" }
        else { "🔧" }
    }

    /// Static trigger schemas
    fn get_triggers() -> Vec<TriggerSchema> {
        vec![
            TriggerSchema {
                trigger_type: "mode_change".to_string(),
                label: "Changement de mode".to_string(),
                description: "Déclenché quand le mode contextuel change".to_string(),
                icon: "🔄".to_string(),
                fields: vec![
                    FieldSchema {
                        name: "from_mode".to_string(),
                        label: "Mode précédent".to_string(),
                        field_type: FieldType::Select,
                        required: false,
                        default_value: None,
                        placeholder: Some("N'importe lequel".to_string()),
                        options_key: Some("modes".to_string()),
                        min: None,
                        max: None,
                    },
                    FieldSchema {
                        name: "to_mode".to_string(),
                        label: "Nouveau mode".to_string(),
                        field_type: FieldType::Select,
                        required: false,
                        default_value: None,
                        placeholder: Some("N'importe lequel".to_string()),
                        options_key: Some("modes".to_string()),
                        min: None,
                        max: None,
                    },
                ],
            },
            TriggerSchema {
                trigger_type: "sensor_alert".to_string(),
                label: "Alerte capteur".to_string(),
                description: "Déclenché quand un capteur passe en alerte".to_string(),
                icon: "🌡️".to_string(),
                fields: vec![
                    FieldSchema {
                        name: "room_id".to_string(),
                        label: "Pièce".to_string(),
                        field_type: FieldType::Select,
                        required: false,
                        default_value: None,
                        placeholder: Some("Toutes les pièces".to_string()),
                        options_key: Some("rooms".to_string()),
                        min: None,
                        max: None,
                    },
                    FieldSchema {
                        name: "alert_level".to_string(),
                        label: "Niveau d'alerte".to_string(),
                        field_type: FieldType::Select,
                        required: false,
                        default_value: None,
                        placeholder: Some("Tous les niveaux".to_string()),
                        options_key: Some("alert_levels".to_string()),
                        min: None,
                        max: None,
                    },
                ],
            },
            TriggerSchema {
                trigger_type: "agent_status".to_string(),
                label: "Status agent".to_string(),
                description: "Déclenché quand un agent passe online/offline".to_string(),
                icon: "🖥️".to_string(),
                fields: vec![
                    FieldSchema {
                        name: "agent_id".to_string(),
                        label: "Agent".to_string(),
                        field_type: FieldType::Select,
                        required: false,
                        default_value: None,
                        placeholder: Some("Tous les agents".to_string()),
                        options_key: Some("agents".to_string()),
                        min: None,
                        max: None,
                    },
                    FieldSchema {
                        name: "status".to_string(),
                        label: "Status".to_string(),
                        field_type: FieldType::Select,
                        required: true,
                        default_value: Some(serde_json::json!("offline")),
                        placeholder: None,
                        options_key: None,
                        min: None,
                        max: None,
                    },
                ],
            },
            TriggerSchema {
                trigger_type: "manual".to_string(),
                label: "Manuel".to_string(),
                description: "Déclenché uniquement via API ou interface".to_string(),
                icon: "👆".to_string(),
                fields: vec![],
            },
            TriggerSchema {
                trigger_type: "plugin_health".to_string(),
                label: "Santé plugin".to_string(),
                description: "Déclenché quand l'état d'un plugin change".to_string(),
                icon: "🔌".to_string(),
                fields: vec![
                    FieldSchema {
                        name: "plugin_name".to_string(),
                        label: "Plugin".to_string(),
                        field_type: FieldType::Select,
                        required: false,
                        default_value: None,
                        placeholder: Some("Tous les plugins".to_string()),
                        options_key: Some("plugins".to_string()),
                        min: None,
                        max: None,
                    },
                    FieldSchema {
                        name: "status".to_string(),
                        label: "État".to_string(),
                        field_type: FieldType::Select,
                        required: true,
                        default_value: Some(serde_json::json!("unhealthy")),
                        placeholder: None,
                        options_key: Some("plugin_health_statuses".to_string()),
                        min: None,
                        max: None,
                    },
                ],
            },
            TriggerSchema {
                trigger_type: "scheduled".to_string(),
                label: "Planifié (Polling)".to_string(),
                description: "Se déclenche à intervalles réguliers".to_string(),
                icon: "⏰".to_string(),
                fields: vec![
                    FieldSchema {
                        name: "interval_seconds".to_string(),
                        label: "Intervalle (secondes)".to_string(),
                        field_type: FieldType::Number,
                        required: true,
                        default_value: Some(serde_json::json!(300)),
                        placeholder: Some("300 = 5 minutes".to_string()),
                        options_key: None,
                        min: Some(60.0),
                        max: Some(86400.0),
                    },
                    FieldSchema {
                        name: "active_hours_start".to_string(),
                        label: "Heure début (optionnel)".to_string(),
                        field_type: FieldType::Number,
                        required: false,
                        default_value: None,
                        placeholder: Some("0-23".to_string()),
                        options_key: None,
                        min: Some(0.0),
                        max: Some(23.0),
                    },
                    FieldSchema {
                        name: "active_hours_end".to_string(),
                        label: "Heure fin (optionnel)".to_string(),
                        field_type: FieldType::Number,
                        required: false,
                        default_value: None,
                        placeholder: Some("0-23".to_string()),
                        options_key: None,
                        min: Some(0.0),
                        max: Some(23.0),
                    },
                ],
            },
            TriggerSchema {
                trigger_type: "ssl_alert".to_string(),
                label: "Alerte SSL".to_string(),
                description: "Déclenché quand un certificat SSL change de status".to_string(),
                icon: "🔒".to_string(),
                fields: vec![
                    FieldSchema {
                        name: "domain_id".to_string(),
                        label: "Domaine".to_string(),
                        field_type: FieldType::Select,
                        required: false,
                        default_value: None,
                        placeholder: Some("Tous les domaines".to_string()),
                        options_key: Some("ssl_domains".to_string()),
                        min: None,
                        max: None,
                    },
                    FieldSchema {
                        name: "status".to_string(),
                        label: "Status".to_string(),
                        field_type: FieldType::Select,
                        required: true,
                        default_value: Some(serde_json::json!("critical")),
                        placeholder: None,
                        options_key: Some("ssl_statuses".to_string()),
                        min: None,
                        max: None,
                    },
                ],
            },
        ]
    }

    /// Static condition schemas
    fn get_conditions() -> Vec<ConditionSchema> {
        vec![
            ConditionSchema {
                condition_type: "current_mode".to_string(),
                label: "Mode actuel".to_string(),
                description: "Vérifie si le mode actuel correspond".to_string(),
                fields: vec![
                    FieldSchema {
                        name: "mode".to_string(),
                        label: "Mode".to_string(),
                        field_type: FieldType::Select,
                        required: true,
                        default_value: None,
                        placeholder: None,
                        options_key: Some("modes".to_string()),
                        min: None,
                        max: None,
                    },
                    FieldSchema {
                        name: "operator".to_string(),
                        label: "Opérateur".to_string(),
                        field_type: FieldType::Select,
                        required: true,
                        default_value: Some(serde_json::json!("equals")),
                        placeholder: None,
                        options_key: Some("comparison_operators".to_string()),
                        min: None,
                        max: None,
                    },
                ],
            },
            ConditionSchema {
                condition_type: "time_range".to_string(),
                label: "Plage horaire".to_string(),
                description: "Vérifie si l'heure actuelle est dans la plage".to_string(),
                fields: vec![
                    FieldSchema {
                        name: "start_hour".to_string(),
                        label: "Heure début".to_string(),
                        field_type: FieldType::Number,
                        required: true,
                        default_value: Some(serde_json::json!(8)),
                        placeholder: None,
                        options_key: None,
                        min: Some(0.0),
                        max: Some(23.0),
                    },
                    FieldSchema {
                        name: "end_hour".to_string(),
                        label: "Heure fin".to_string(),
                        field_type: FieldType::Number,
                        required: true,
                        default_value: Some(serde_json::json!(22)),
                        placeholder: None,
                        options_key: None,
                        min: Some(0.0),
                        max: Some(23.0),
                    },
                ],
            },
            ConditionSchema {
                condition_type: "day_of_week".to_string(),
                label: "Jour de la semaine".to_string(),
                description: "Vérifie si le jour actuel correspond".to_string(),
                fields: vec![
                    FieldSchema {
                        name: "days".to_string(),
                        label: "Jours".to_string(),
                        field_type: FieldType::MultiSelect,
                        required: true,
                        default_value: Some(serde_json::json!(["1", "2", "3", "4", "5"])),
                        placeholder: None,
                        options_key: Some("weekdays".to_string()),
                        min: None,
                        max: None,
                    },
                ],
            },
            ConditionSchema {
                condition_type: "day_of_month".to_string(),
                label: "Jour du mois".to_string(),
                description: "Vérifie le jour du mois (31 = dernier jour)".to_string(),
                fields: vec![
                    FieldSchema {
                        name: "days".to_string(),
                        label: "Jours".to_string(),
                        field_type: FieldType::MultiSelect,
                        required: true,
                        default_value: Some(serde_json::json!(["1"])),
                        placeholder: None,
                        options_key: Some("days_of_month".to_string()),
                        min: None,
                        max: None,
                    },
                ],
            },
            ConditionSchema {
                condition_type: "month".to_string(),
                label: "Mois de l'année".to_string(),
                description: "Vérifie si on est dans un mois spécifique".to_string(),
                fields: vec![
                    FieldSchema {
                        name: "months".to_string(),
                        label: "Mois".to_string(),
                        field_type: FieldType::MultiSelect,
                        required: true,
                        default_value: Some(serde_json::json!(["1", "2", "3", "4", "5", "6", "7", "8", "9", "10", "11", "12"])),
                        placeholder: None,
                        options_key: Some("months".to_string()),
                        min: None,
                        max: None,
                    },
                ],
            },
            ConditionSchema {
                condition_type: "sensor_value".to_string(),
                label: "Valeur capteur".to_string(),
                description: "Vérifie une valeur de capteur".to_string(),
                fields: vec![
                    FieldSchema {
                        name: "room_id".to_string(),
                        label: "Pièce".to_string(),
                        field_type: FieldType::Select,
                        required: true,
                        default_value: None,
                        placeholder: None,
                        options_key: Some("rooms".to_string()),
                        min: None,
                        max: None,
                    },
                    FieldSchema {
                        name: "metric".to_string(),
                        label: "Métrique".to_string(),
                        field_type: FieldType::Select,
                        required: true,
                        default_value: Some(serde_json::json!("humidity")),
                        placeholder: None,
                        options_key: Some("sensor_metrics".to_string()),
                        min: None,
                        max: None,
                    },
                    FieldSchema {
                        name: "operator".to_string(),
                        label: "Opérateur".to_string(),
                        field_type: FieldType::Select,
                        required: true,
                        default_value: Some(serde_json::json!("greater_than")),
                        placeholder: None,
                        options_key: Some("comparison_operators".to_string()),
                        min: None,
                        max: None,
                    },
                    FieldSchema {
                        name: "value".to_string(),
                        label: "Valeur".to_string(),
                        field_type: FieldType::Number,
                        required: true,
                        default_value: Some(serde_json::json!(70)),
                        placeholder: None,
                        options_key: None,
                        min: None,
                        max: None,
                    },
                ],
            },
            ConditionSchema {
                condition_type: "agent_online".to_string(),
                label: "Agent en ligne".to_string(),
                description: "Vérifie si un agent est en ligne".to_string(),
                fields: vec![
                    FieldSchema {
                        name: "agent_id".to_string(),
                        label: "Agent".to_string(),
                        field_type: FieldType::Select,
                        required: true,
                        default_value: None,
                        placeholder: None,
                        options_key: Some("agents".to_string()),
                        min: None,
                        max: None,
                    },
                ],
            },
            ConditionSchema {
                condition_type: "feature".to_string(),
                label: "Feature Intelligence".to_string(),
                description: "Vérifie une feature du FeatureRegistry (présence, env, process, etc.)".to_string(),
                fields: vec![
                    FieldSchema {
                        name: "feature_id".to_string(),
                        label: "Feature ID".to_string(),
                        field_type: FieldType::Select,
                        required: true,
                        default_value: None,
                        placeholder: Some("Ex: presence.phone".to_string()),
                        options_key: Some("features".to_string()),
                        min: None,
                        max: None,
                    },
                    FieldSchema {
                        name: "operator".to_string(),
                        label: "Opérateur".to_string(),
                        field_type: FieldType::Select,
                        required: true,
                        default_value: Some(serde_json::json!("equals")),
                        placeholder: None,
                        options_key: Some("comparison_operators".to_string()),
                        min: None,
                        max: None,
                    },
                    FieldSchema {
                        name: "value".to_string(),
                        label: "Valeur".to_string(),
                        field_type: FieldType::Text,
                        required: true,
                        default_value: Some(serde_json::json!("true")),
                        placeholder: Some("true, false, ou valeur numérique".to_string()),
                        options_key: None,
                        min: None,
                        max: None,
                    },
                ],
            },
        ]
    }

    /// Static action schemas
    fn get_actions() -> Vec<ActionSchema> {
        vec![
            ActionSchema {
                action_type: "send_notification".to_string(),
                label: "Envoyer notification".to_string(),
                description: "Envoie une notification push".to_string(),
                icon: "🔔".to_string(),
                fields: vec![
                    FieldSchema {
                        name: "priority".to_string(),
                        label: "Priorité".to_string(),
                        field_type: FieldType::Select,
                        required: true,
                        default_value: Some(serde_json::json!("P1")),
                        placeholder: None,
                        options_key: Some("priorities".to_string()),
                        min: None,
                        max: None,
                    },
                    FieldSchema {
                        name: "title".to_string(),
                        label: "Titre".to_string(),
                        field_type: FieldType::Text,
                        required: true,
                        default_value: None,
                        placeholder: Some("Titre de la notification".to_string()),
                        options_key: None,
                        min: None,
                        max: None,
                    },
                    FieldSchema {
                        name: "body".to_string(),
                        label: "Message".to_string(),
                        field_type: FieldType::TextArea,
                        required: true,
                        default_value: None,
                        placeholder: Some("Corps du message".to_string()),
                        options_key: None,
                        min: None,
                        max: None,
                    },
                ],
            },
            ActionSchema {
                action_type: "force_mode".to_string(),
                label: "Forcer mode".to_string(),
                description: "Force un changement de mode temporaire".to_string(),
                icon: "🎯".to_string(),
                fields: vec![
                    FieldSchema {
                        name: "mode".to_string(),
                        label: "Mode".to_string(),
                        field_type: FieldType::Select,
                        required: true,
                        default_value: None,
                        placeholder: None,
                        options_key: Some("modes".to_string()),
                        min: None,
                        max: None,
                    },
                    FieldSchema {
                        name: "duration_minutes".to_string(),
                        label: "Durée (minutes)".to_string(),
                        field_type: FieldType::Number,
                        required: false,
                        default_value: Some(serde_json::json!(60)),
                        placeholder: None,
                        options_key: None,
                        min: Some(1.0),
                        max: Some(1440.0),
                    },
                    FieldSchema {
                        name: "reason".to_string(),
                        label: "Raison".to_string(),
                        field_type: FieldType::Text,
                        required: true,
                        default_value: None,
                        placeholder: Some("Raison du changement".to_string()),
                        options_key: None,
                        min: None,
                        max: None,
                    },
                ],
            },
            ActionSchema {
                action_type: "agent_command".to_string(),
                label: "Commande agent".to_string(),
                description: "Envoie une commande à un agent".to_string(),
                icon: "📤".to_string(),
                fields: vec![
                    FieldSchema {
                        name: "agent_id".to_string(),
                        label: "Agent".to_string(),
                        field_type: FieldType::Select,
                        required: true,
                        default_value: None,
                        placeholder: None,
                        options_key: Some("agents".to_string()),
                        min: None,
                        max: None,
                    },
                    FieldSchema {
                        name: "command_type".to_string(),
                        label: "Type de commande".to_string(),
                        field_type: FieldType::Select,
                        required: true,
                        default_value: None,
                        placeholder: None,
                        options_key: Some("command_types".to_string()),
                        min: None,
                        max: None,
                    },
                ],
            },
            ActionSchema {
                action_type: "set_feature".to_string(),
                label: "Définir une feature".to_string(),
                description: "Définit une valeur dans le FeatureRegistry (ex: thème dark/light)".to_string(),
                icon: "🔧".to_string(),
                fields: vec![
                    FieldSchema {
                        name: "feature_id".to_string(),
                        label: "Feature ID".to_string(),
                        field_type: FieldType::Select,
                        required: true,
                        default_value: None,
                        placeholder: Some("Ex: appearance.theme".to_string()),
                        options_key: Some("features".to_string()),
                        min: None,
                        max: None,
                    },
                    FieldSchema {
                        name: "value".to_string(),
                        label: "Valeur".to_string(),
                        field_type: FieldType::Text,
                        required: true,
                        default_value: None,
                        placeholder: Some("dark, light, true, 42...".to_string()),
                        options_key: None,
                        min: None,
                        max: None,
                    },
                ],
            },
            ActionSchema {
                action_type: "delay".to_string(),
                label: "Délai".to_string(),
                description: "Attend avant la prochaine action".to_string(),
                icon: "⏳".to_string(),
                fields: vec![
                    FieldSchema {
                        name: "seconds".to_string(),
                        label: "Secondes".to_string(),
                        field_type: FieldType::Number,
                        required: true,
                        default_value: Some(serde_json::json!(5)),
                        placeholder: None,
                        options_key: None,
                        min: Some(1.0),
                        max: Some(3600.0),
                    },
                ],
            },
            ActionSchema {
                action_type: "plugin_command".to_string(),
                label: "Commande plugin".to_string(),
                description: "POST HTTP sur une route d'un plugin (ex: coffee/power, library/reindex)".to_string(),
                icon: "🔌".to_string(),
                fields: vec![
                    FieldSchema {
                        name: "plugin".to_string(),
                        label: "Plugin".to_string(),
                        field_type: FieldType::Select,
                        required: true,
                        default_value: None,
                        placeholder: None,
                        options_key: Some("plugins".to_string()),
                        min: None,
                        max: None,
                    },
                    FieldSchema {
                        name: "route".to_string(),
                        label: "Route".to_string(),
                        field_type: FieldType::Text,
                        required: true,
                        default_value: None,
                        placeholder: Some("Ex: power, brew, config (sans slash)".to_string()),
                        options_key: None,
                        min: None,
                        max: None,
                    },
                    FieldSchema {
                        name: "payload".to_string(),
                        label: "Payload JSON".to_string(),
                        field_type: FieldType::TextArea,
                        required: false,
                        default_value: Some(serde_json::json!({})),
                        placeholder: Some(r#"{"on": true}"#.to_string()),
                        options_key: None,
                        min: None,
                        max: None,
                    },
                ],
            },
        ]
    }

    /// Build dynamic values from kernel state
    fn get_dynamic_values(
        agents: &[(String, String)],
        rooms: &[String],
        sensors: &[SensorInfo],
        modes: &[(String, String, String)],  // (slug, label, description)
        features: &[(String, String)],  // (feature_id, label)
        ssl_domains: &[(String, String)],  // (id, label)
        live_plugins: &[String],         // noms réels via PluginRegistry
    ) -> DynamicValues {
        DynamicValues {
            modes: modes
                .iter()
                .map(|(slug, label, description)| ValueOption {
                    value: slug.clone(),
                    label: label.clone(),
                    description: Some(description.clone()),
                })
                .collect(),
            agents: agents
                .iter()
                .map(|(id, name)| ValueOption {
                    value: id.clone(),
                    label: format!("🖥️ {}", name),
                    description: Some(format!("Agent ID: {}", id)),
                })
                .collect(),
            rooms: rooms
                .iter()
                .map(|room| {
                    let icon = match room.as_str() {
                        "chambre" => "🛏️",
                        "salon" => "🛋️",
                        "cuisine" => "🍳",
                        "bureau" => "💼",
                        "sdb" | "salle_de_bain" => "🚿",
                        _ => "🏠",
                    };
                    ValueOption {
                        value: room.clone(),
                        label: format!("{} {}", icon, capitalize(room)),
                        description: None,
                    }
                })
                .collect(),
            sensors: sensors
                .iter()
                .filter(|s| s.status != "deleted")
                .map(|s| {
                    let status_icon = if s.status == "online" { "🟢" } else { "🔴" };
                    SensorOption {
                        value: s.sensor_id.clone(),
                        label: format!("{} {} ({})", status_icon, s.sensor_id, s.room_id),
                        room_id: s.room_id.clone(),
                        sensor_type: s.sensor_type.clone(),
                        status: Some(s.status.clone()),
                    }
                })
                .collect(),
            features: features
                .iter()
                .map(|(id, label)| ValueOption {
                    value: id.clone(),
                    label: label.clone(),
                    description: Some(format!("Feature ID: {}", id)),
                })
                .collect(),
            categories: vec![
                ValueOption { value: "systeme".to_string(), label: "🔧 Système".to_string(), description: Some("Automations système".to_string()) },
                ValueOption { value: "alertes".to_string(), label: "🚨 Alertes".to_string(), description: Some("Alertes et notifications".to_string()) },
                ValueOption { value: "modes".to_string(), label: "🎯 Modes".to_string(), description: Some("Changements de mode".to_string()) },
                ValueOption { value: "notifications".to_string(), label: "🔔 Notifications".to_string(), description: Some("Notifications personnalisées".to_string()) },
                ValueOption { value: "custom".to_string(), label: "✨ Personnalisé".to_string(), description: Some("Automations personnalisées".to_string()) },
            ],
            alert_levels: vec![
                ValueOption {
                    value: "normal".to_string(),
                    label: "🟢 Normal".to_string(),
                    description: Some("Pas d'alerte".to_string()),
                },
                ValueOption {
                    value: "moderate".to_string(),
                    label: "🟡 Modéré".to_string(),
                    description: Some("Alerte légère".to_string()),
                },
                ValueOption {
                    value: "high".to_string(),
                    label: "🟠 Élevé".to_string(),
                    description: Some("Alerte importante".to_string()),
                },
                ValueOption {
                    value: "critical".to_string(),
                    label: "🔴 Critique".to_string(),
                    description: Some("Alerte urgente".to_string()),
                },
            ],
            priorities: vec![
                ValueOption {
                    value: "P0".to_string(),
                    label: "🔴 P0 - Critique".to_string(),
                    description: Some("Notification immédiate".to_string()),
                },
                ValueOption {
                    value: "P1".to_string(),
                    label: "🟠 P1 - Important".to_string(),
                    description: Some("Notification standard".to_string()),
                },
                ValueOption {
                    value: "P2".to_string(),
                    label: "🟢 P2 - Normal".to_string(),
                    description: Some("Notification basse priorité".to_string()),
                },
            ],
            command_types: vec![
                ValueOption {
                    value: "shutdown".to_string(),
                    label: "🔌 Éteindre".to_string(),
                    description: Some("Éteindre l'appareil".to_string()),
                },
                ValueOption {
                    value: "restart".to_string(),
                    label: "🔄 Redémarrer".to_string(),
                    description: Some("Redémarrer l'appareil".to_string()),
                },
                ValueOption {
                    value: "sleep".to_string(),
                    label: "😴 Veille".to_string(),
                    description: Some("Mettre en veille".to_string()),
                },
                ValueOption {
                    value: "wake".to_string(),
                    label: "☀️ Réveiller".to_string(),
                    description: Some("Sortir de veille (WoL)".to_string()),
                },
                ValueOption {
                    value: "lock".to_string(),
                    label: "🔒 Verrouiller".to_string(),
                    description: Some("Verrouiller la session".to_string()),
                },
                ValueOption {
                    value: "notify".to_string(),
                    label: "💬 Notifier".to_string(),
                    description: Some("Afficher une notification".to_string()),
                },
            ],
            sensor_metrics: vec![
                ValueOption {
                    value: "temperature".to_string(),
                    label: "🌡️ Température".to_string(),
                    description: Some("En degrés Celsius".to_string()),
                },
                ValueOption {
                    value: "humidity".to_string(),
                    label: "💧 Humidité".to_string(),
                    description: Some("En pourcentage".to_string()),
                },
                ValueOption {
                    value: "battery".to_string(),
                    label: "🔋 Batterie".to_string(),
                    description: Some("Niveau batterie capteur".to_string()),
                },
            ],
            comparison_operators: vec![
                ValueOption {
                    value: "equals".to_string(),
                    label: "= Égal".to_string(),
                    description: None,
                },
                ValueOption {
                    value: "not_equals".to_string(),
                    label: "≠ Différent".to_string(),
                    description: None,
                },
                ValueOption {
                    value: "greater_than".to_string(),
                    label: "> Supérieur".to_string(),
                    description: None,
                },
                ValueOption {
                    value: "less_than".to_string(),
                    label: "< Inférieur".to_string(),
                    description: None,
                },
                ValueOption {
                    value: "greater_or_equal".to_string(),
                    label: "≥ Supérieur ou égal".to_string(),
                    description: None,
                },
                ValueOption {
                    value: "less_or_equal".to_string(),
                    label: "≤ Inférieur ou égal".to_string(),
                    description: None,
                },
            ],
            weekdays: vec![
                ValueOption { value: "1".to_string(), label: "Lundi".to_string(), description: None },
                ValueOption { value: "2".to_string(), label: "Mardi".to_string(), description: None },
                ValueOption { value: "3".to_string(), label: "Mercredi".to_string(), description: None },
                ValueOption { value: "4".to_string(), label: "Jeudi".to_string(), description: None },
                ValueOption { value: "5".to_string(), label: "Vendredi".to_string(), description: None },
                ValueOption { value: "6".to_string(), label: "Samedi".to_string(), description: None },
                ValueOption { value: "0".to_string(), label: "Dimanche".to_string(), description: None },
            ],
            days_of_month: (1..=31).map(|d| ValueOption {
                value: d.to_string(),
                label: if d == 31 { "31 (Dernier jour)".to_string() } else { d.to_string() },
                description: if d == 31 { Some("Dernier jour du mois quel qu'il soit".to_string()) } else { None },
            }).collect(),
            months: vec![
                ValueOption { value: "1".to_string(), label: "Janvier".to_string(), description: None },
                ValueOption { value: "2".to_string(), label: "Février".to_string(), description: None },
                ValueOption { value: "3".to_string(), label: "Mars".to_string(), description: None },
                ValueOption { value: "4".to_string(), label: "Avril".to_string(), description: None },
                ValueOption { value: "5".to_string(), label: "Mai".to_string(), description: None },
                ValueOption { value: "6".to_string(), label: "Juin".to_string(), description: None },
                ValueOption { value: "7".to_string(), label: "Juillet".to_string(), description: None },
                ValueOption { value: "8".to_string(), label: "Août".to_string(), description: None },
                ValueOption { value: "9".to_string(), label: "Septembre".to_string(), description: None },
                ValueOption { value: "10".to_string(), label: "Octobre".to_string(), description: None },
                ValueOption { value: "11".to_string(), label: "Novembre".to_string(), description: None },
                ValueOption { value: "12".to_string(), label: "Décembre".to_string(), description: None },
            ],
            plugins: {
                // Hardcoded fallback (résilience offline) + enrichi avec live runtime
                let mut by_name: std::collections::BTreeMap<String, ValueOption> = [
                    ("notes", "📝 Notes", "Plugin de notes"),
                    ("sensors", "🌡️ Sensors", "Plugin capteurs environnement"),
                    ("ssl", "🔒 SSL", "Plugin surveillance certificats SSL"),
                    ("freebox", "📡 Freebox", "Plugin présence Freebox"),
                ]
                .iter()
                .map(|(name, label, desc)| (
                    name.to_string(),
                    ValueOption {
                        value: name.to_string(),
                        label: label.to_string(),
                        description: Some(desc.to_string()),
                    }
                ))
                .collect();
                // Enrichir avec les plugins runtime (peut-être ajout de coffee, library, telegram...)
                for plugin_name in live_plugins {
                    by_name.entry(plugin_name.clone()).or_insert_with(|| ValueOption {
                        value: plugin_name.clone(),
                        label: format!("{} {}", Self::plugin_icon(plugin_name), plugin_name),
                        description: Some(format!("Plugin {}", plugin_name)),
                    });
                }
                by_name.into_values().collect()
            },
            plugin_health_statuses: vec![
                ValueOption { value: "healthy".to_string(), label: "🟢 Healthy".to_string(), description: Some("Plugin fonctionne".to_string()) },
                ValueOption { value: "unhealthy".to_string(), label: "🔴 Unhealthy".to_string(), description: Some("Plugin ne répond pas".to_string()) },
                ValueOption { value: "recovery_attempt".to_string(), label: "🔄 Recovery".to_string(), description: Some("Tentative de redémarrage".to_string()) },
                ValueOption { value: "recovery_failed".to_string(), label: "❌ Recovery Failed".to_string(), description: Some("Redémarrage échoué".to_string()) },
                ValueOption { value: "recovery_success".to_string(), label: "✅ Recovery Success".to_string(), description: Some("Redémarrage réussi".to_string()) },
                ValueOption { value: "any".to_string(), label: "* Tout changement".to_string(), description: Some("N'importe quel changement".to_string()) },
            ],
            ssl_domains: ssl_domains
                .iter()
                .map(|(id, label)| ValueOption {
                    value: id.clone(),
                    label: format!("🔒 {}", label),
                    description: Some(format!("Domaine: {}", id)),
                })
                .collect(),
            ssl_statuses: vec![
                ValueOption { value: "ok".to_string(), label: "🟢 OK".to_string(), description: Some("Certificat valide".to_string()) },
                ValueOption { value: "warning".to_string(), label: "🟡 Warning".to_string(), description: Some("Expiration proche".to_string()) },
                ValueOption { value: "critical".to_string(), label: "🔴 Critical".to_string(), description: Some("Expiration imminente".to_string()) },
                ValueOption { value: "expired".to_string(), label: "⛔ Expired".to_string(), description: Some("Certificat expiré".to_string()) },
                ValueOption { value: "error".to_string(), label: "❌ Error".to_string(), description: Some("Erreur de vérification".to_string()) },
                ValueOption { value: "any".to_string(), label: "* Tout changement".to_string(), description: Some("N'importe quel changement de status".to_string()) },
            ],
        }
    }
}

/// Capitalize first letter
fn capitalize(s: &str) -> String {
    let mut c = s.chars();
    match c.next() {
        None => String::new(),
        Some(f) => f.to_uppercase().collect::<String>() + c.as_str(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_schema_generation() {
        let agents = vec![
            ("agent1".to_string(), "PC Bureau".to_string()),
            ("agent2".to_string(), "PC Salon".to_string()),
        ];
        let rooms = vec!["chambre".to_string(), "salon".to_string()];
        let sensors = vec![
            SensorInfo {
                sensor_id: "esp32-001".to_string(),
                sensor_type: "bme280".to_string(),
                room_id: "chambre".to_string(),
                status: "online".to_string(),
            },
        ];
        let modes = vec![
            ("pro".to_string(), "Pro".to_string(), "Mode Pro".to_string()),
            ("maison".to_string(), "Maison".to_string(), "Mode Maison".to_string()),
            ("veille".to_string(), "Veille".to_string(), "Mode Veille".to_string()),
        ];

        let schema = SchemaRegistry::get_schema(&agents, &rooms, &sensors, &modes);

        assert_eq!(schema.triggers.len(), 7);  // mode_change, sensor_alert, agent_status, manual, plugin_health, scheduled, ssl_alert
        assert_eq!(schema.conditions.len(), 8);  // current_mode, time_range, day_of_week, day_of_month, month, sensor_value, agent_online, feature
        assert_eq!(schema.actions.len(), 5);
        assert_eq!(schema.dynamic_values.modes.len(), 3);
        assert_eq!(schema.dynamic_values.agents.len(), 2);
        assert_eq!(schema.dynamic_values.rooms.len(), 2);
        assert_eq!(schema.dynamic_values.sensors.len(), 1);
        assert_eq!(schema.dynamic_values.command_types.len(), 6);
        assert_eq!(schema.dynamic_values.sensor_metrics.len(), 3);
        assert_eq!(schema.dynamic_values.weekdays.len(), 7);
        assert_eq!(schema.dynamic_values.days_of_month.len(), 31);
        assert_eq!(schema.dynamic_values.months.len(), 12);
    }
}
