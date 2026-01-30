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

/// Complete automation schema for PWA
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AutomationSchema {
    pub triggers: Vec<TriggerSchema>,
    pub conditions: Vec<ConditionSchema>,
    pub actions: Vec<ActionSchema>,
    pub dynamic_values: DynamicValues,
}

/// Dynamic values from kernel registries
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DynamicValues {
    pub modes: Vec<ValueOption>,
    pub agents: Vec<ValueOption>,
    pub rooms: Vec<ValueOption>,
    pub sensors: Vec<SensorOption>,
    pub alert_levels: Vec<ValueOption>,
    pub priorities: Vec<ValueOption>,
    pub command_types: Vec<ValueOption>,
    pub sensor_metrics: Vec<ValueOption>,
    pub comparison_operators: Vec<ValueOption>,
    pub weekdays: Vec<ValueOption>,
}

/// Sensor option with room context
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SensorOption {
    pub value: String,
    pub label: String,
    pub room_id: String,
    pub sensor_type: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub status: Option<String>,
}

/// Value option for dropdowns
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ValueOption {
    pub value: String,
    pub label: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
}

/// Schema for a trigger type
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TriggerSchema {
    #[serde(rename = "type")]
    pub trigger_type: String,
    pub label: String,
    pub description: String,
    pub icon: String,
    pub fields: Vec<FieldSchema>,
}

/// Schema for a condition type
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConditionSchema {
    #[serde(rename = "type")]
    pub condition_type: String,
    pub label: String,
    pub description: String,
    pub fields: Vec<FieldSchema>,
}

/// Schema for an action type
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ActionSchema {
    #[serde(rename = "type")]
    pub action_type: String,
    pub label: String,
    pub description: String,
    pub icon: String,
    pub fields: Vec<FieldSchema>,
}

/// Field definition for forms
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FieldSchema {
    pub name: String,
    pub label: String,
    pub field_type: FieldType,
    pub required: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
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

#[derive(Debug, Clone, Serialize, Deserialize)]
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
    ) -> AutomationSchema {
        AutomationSchema {
            triggers: Self::get_triggers(),
            conditions: Self::get_conditions(),
            actions: Self::get_actions(),
            dynamic_values: Self::get_dynamic_values(agents, rooms, sensors),
        }
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
        ]
    }

    /// Build dynamic values from kernel state
    fn get_dynamic_values(
        agents: &[(String, String)],
        rooms: &[String],
        sensors: &[SensorInfo],
    ) -> DynamicValues {
        DynamicValues {
            modes: vec![
                ValueOption {
                    value: "cravate".to_string(),
                    label: "👔 Cravate".to_string(),
                    description: Some("Mode professionnel".to_string()),
                },
                ValueOption {
                    value: "intime".to_string(),
                    label: "🏡 Intime".to_string(),
                    description: Some("Mode domestique".to_string()),
                },
                ValueOption {
                    value: "neutre".to_string(),
                    label: "🌱 Neutre".to_string(),
                    description: Some("Mode surveillance".to_string()),
                },
            ],
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

        let schema = SchemaRegistry::get_schema(&agents, &rooms, &sensors);

        assert_eq!(schema.triggers.len(), 4);
        assert_eq!(schema.conditions.len(), 5);
        assert_eq!(schema.actions.len(), 4);
        assert_eq!(schema.dynamic_values.modes.len(), 3);
        assert_eq!(schema.dynamic_values.agents.len(), 2);
        assert_eq!(schema.dynamic_values.rooms.len(), 2);
        assert_eq!(schema.dynamic_values.sensors.len(), 1);
        assert_eq!(schema.dynamic_values.command_types.len(), 6);
        assert_eq!(schema.dynamic_values.sensor_metrics.len(), 3);
        assert_eq!(schema.dynamic_values.weekdays.len(), 7);
    }
}
