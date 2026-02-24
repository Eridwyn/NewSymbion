/**
 * SYMBION KERNEL - Automations Types
 *
 * ROLE: Data structures for automation rules
 *
 * STRUCTURES:
 * - Automation    : Complete rule definition
 * - Trigger       : Event that starts automation
 * - ConditionGroup: AND/OR logic for conditions
 * - Condition     : Single condition to evaluate
 * - ActionDefinition: Action to execute
 */

use serde::{Deserialize, Serialize};
use serde_json::Value;
use time::OffsetDateTime;

// Re-export ImpactLevel from decision module for convenience
pub use crate::decision::ImpactLevel;

/// Available automation categories
pub const DEFAULT_CATEGORIES: &[(&str, &str)] = &[
    ("systeme", "Système"),
    ("alertes", "Alertes"),
    ("modes", "Modes"),
    ("notifications", "Notifications"),
    ("custom", "Personnalisé"),
];

/// Complete automation rule
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Automation {
    pub id: String,
    pub name: String,
    #[serde(default)]
    pub description: Option<String>,
    /// Category for organizing automations (systeme, alertes, modes, notifications, custom)
    #[serde(default)]
    pub category: Option<String>,
    /// Goal mode - what context mode this automation aims to achieve/maintain
    /// Used for Intelligence feedback learning. If None, no learning occurs.
    #[serde(default)]
    pub goal_mode: Option<String>,
    #[serde(default = "default_true")]
    pub enabled: bool,
    /// Legacy single trigger (for backward compatibility)
    #[serde(skip_serializing_if = "Option::is_none")]
    #[serde(default)]
    pub trigger: Option<Trigger>,
    /// New trigger group with AND/OR logic
    #[serde(default)]
    pub triggers: Option<TriggerGroup>,
    #[serde(default)]
    pub conditions: Option<ConditionGroup>,
    pub actions: Vec<ActionDefinition>,
    #[serde(default = "default_cooldown")]
    pub cooldown_seconds: u32,

    // Intelligence flags
    /// Si true, bypass le Decision Engine (auto-approuvé)
    #[serde(default)]
    pub trusted: Option<bool>,
    /// Si true, ne pas exécuter si déjà dans le mode cible (évite spam)
    #[serde(default)]
    pub skip_if_same_mode: Option<bool>,
    /// Marqué true si créé automatiquement par le système d'intelligence
    #[serde(default)]
    pub auto_created: Option<bool>,

    // Execution tracking
    #[serde(default)]
    #[serde(with = "time::serde::iso8601::option")]
    pub last_executed_at: Option<OffsetDateTime>,
    #[serde(default)]
    pub execution_count: u64,

    // Metadata
    #[serde(default)]
    #[serde(with = "time::serde::iso8601::option")]
    pub created_at: Option<OffsetDateTime>,
    #[serde(default)]
    #[serde(with = "time::serde::iso8601::option")]
    pub updated_at: Option<OffsetDateTime>,
    #[serde(default)]
    #[serde(with = "time::serde::iso8601::option")]
    pub deleted_at: Option<OffsetDateTime>,
}

fn default_true() -> bool {
    true
}

fn default_cooldown() -> u32 {
    300 // 5 minutes default
}

/// Trigger types that can start an automation
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum Trigger {
    /// Triggered when context mode changes
    ModeChange {
        #[serde(default)]
        from_mode: Option<String>,
        #[serde(default)]
        to_mode: Option<String>,
    },

    /// Triggered when sensor alert level changes
    SensorAlert {
        #[serde(default)]
        room_id: Option<String>,
        #[serde(default)]
        alert_level: Option<AlertLevel>,
    },

    /// Triggered when agent status changes
    AgentStatus {
        #[serde(default)]
        agent_id: Option<String>,
        status: AgentStatusType,
    },

    /// Manual trigger only (via API)
    Manual,

    /// Triggered when plugin health changes
    PluginHealth {
        #[serde(default)]
        plugin_name: Option<String>,
        status: PluginHealthStatus,
    },

    /// Scheduled/polling trigger - fires at regular intervals
    Scheduled {
        /// Interval in seconds (minimum 60, default 300 = 5 minutes)
        interval_seconds: u32,
        /// Optional: only run between these hours (0-23), e.g., (9, 18) for 9am-6pm
        #[serde(default)]
        #[serde(skip_serializing_if = "Option::is_none")]
        active_hours: Option<(u8, u8)>,
    },

    /// Plugin-defined custom trigger
    Custom {
        plugin_name: String,
        trigger_type: String,
        #[serde(default)]
        config: Value,
    },
}

impl Trigger {
    /// Returns true if this is an event-based trigger (vs scheduled/polling)
    pub fn is_event_based(&self) -> bool {
        !matches!(self, Trigger::Scheduled { .. })
    }

    /// Returns a discriminant tag for fast event-type pre-filtering (D7 optimization)
    pub fn event_type_tag(&self) -> &'static str {
        match self {
            Trigger::ModeChange { .. } => "ModeChange",
            Trigger::SensorAlert { .. } => "SensorAlert",
            Trigger::AgentStatus { .. } => "AgentStatus",
            Trigger::Manual => "Manual",
            Trigger::PluginHealth { .. } => "PluginHealth",
            Trigger::Scheduled { .. } => "Scheduled",
            Trigger::Custom { .. } => "Custom",
        }
    }
}

/// Alert levels for sensor triggers
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AlertLevel {
    Normal,
    Moderate,
    High,
    Critical,
}

/// Agent status types for triggers
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AgentStatusType {
    Online,
    Offline,
    Any,
}

/// Plugin health status for triggers
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PluginHealthStatus {
    Healthy,
    Unhealthy,
    RecoveryAttempt,
    RecoveryFailed,
    RecoverySuccess,
    Any,
}

/// Logical operator for condition/trigger groups
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum LogicalOperator {
    And,
    Or,
}

/// Group of triggers with AND/OR logic
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TriggerGroup {
    pub operator: LogicalOperator,
    pub triggers: Vec<TriggerItem>,
}

/// Trigger item - can be single trigger or nested group
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum TriggerItem {
    Group(Box<TriggerGroup>),
    Single(Trigger),
}

impl TriggerGroup {
    /// Collect all event type tags from this group (for D7 pre-filtering)
    pub fn event_type_tags(&self) -> Vec<&'static str> {
        let mut tags = Vec::new();
        for item in &self.triggers {
            match item {
                TriggerItem::Single(t) => tags.push(t.event_type_tag()),
                TriggerItem::Group(g) => tags.extend(g.event_type_tags()),
            }
        }
        tags
    }
}

impl Default for TriggerGroup {
    fn default() -> Self {
        TriggerGroup {
            operator: LogicalOperator::Or, // OR is more intuitive default for triggers
            triggers: vec![],
        }
    }
}

/// Group of conditions with AND/OR logic
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConditionGroup {
    pub operator: LogicalOperator,
    pub conditions: Vec<Condition>,
}

/// Comparison operators for conditions
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ComparisonOperator {
    Equals,
    NotEquals,
    GreaterThan,
    LessThan,
    GreaterOrEqual,
    LessOrEqual,
    Contains,
}

impl std::fmt::Display for ComparisonOperator {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Equals => write!(f, "=="),
            Self::NotEquals => write!(f, "!="),
            Self::GreaterThan => write!(f, ">"),
            Self::LessThan => write!(f, "<"),
            Self::GreaterOrEqual => write!(f, ">="),
            Self::LessOrEqual => write!(f, "<="),
            Self::Contains => write!(f, "contains"),
        }
    }
}

/// Sensor metric types for conditions
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SensorMetric {
    Temperature,
    Humidity,
    Battery,
    SignalStrength,
}

/// Individual condition to evaluate
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum Condition {
    /// Check current context mode
    CurrentMode {
        mode: String,
        #[serde(default = "default_equals")]
        operator: ComparisonOperator,
    },

    /// Check time of day
    TimeRange {
        start_hour: u8,
        end_hour: u8,
    },

    /// Check day of week (0=Sunday, 6=Saturday)
    DayOfWeek {
        days: Vec<u8>,
    },

    /// Check day of month (1-31, 31 = last day of any month)
    DayOfMonth {
        days: Vec<u8>,
    },

    /// Check month of year (1-12)
    Month {
        months: Vec<u8>,
    },

    /// Check sensor value
    SensorValue {
        room_id: String,
        metric: SensorMetric,
        operator: ComparisonOperator,
        value: f32,
    },

    /// Check if agent is online
    AgentOnline {
        agent_id: String,
    },

    /// Check feature value from FeatureRegistry (Intelligence v2)
    /// Supports presence, env, process, agent features
    Feature {
        feature_id: String,
        operator: ComparisonOperator,
        /// Value to compare against (bool, float, string)
        value: serde_json::Value,
    },

    /// Nested condition group (for complex logic)
    Group(Box<ConditionGroup>),

    /// Plugin-defined custom condition
    Custom {
        plugin_name: String,
        condition_type: String,
        #[serde(default)]
        config: Value,
    },
}

fn default_equals() -> ComparisonOperator {
    ComparisonOperator::Equals
}

/// Actions that can be executed
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum ActionDefinition {
    /// Send notification via plugin
    SendNotification {
        #[serde(default = "default_priority")]
        priority: String,
        title: String,
        body: String,
        #[serde(default = "default_impact_low")]
        impact_level: ImpactLevel,
    },

    /// Force context mode change
    ForceMode {
        mode: String,
        #[serde(default)]
        duration_minutes: Option<i64>,
        #[serde(default = "default_automation_reason")]
        reason: String,
        #[serde(default = "default_impact_medium")]
        impact_level: ImpactLevel,
        /// Si false (ou None), utilise set_mode_natural() au lieu de set_override()
        /// Cela évite de créer un override temporaire et permet au système de continuer à évoluer
        #[serde(default)]
        use_override: Option<bool>,
    },

    /// Send command to agent
    AgentCommand {
        agent_id: String,
        command_type: String,
        #[serde(default)]
        parameters: Option<Value>,
        #[serde(default = "default_impact_from_command")]
        impact_level: ImpactLevel,
    },

    /// Wait before next action
    Delay {
        seconds: u32,
        // No impact_level for Delay - always allowed
    },

    /// Set a feature value in the FeatureRegistry
    SetFeature {
        feature_id: String,
        value: Value,
        #[serde(default = "default_feature_source")]
        source: String,
        #[serde(default)]
        ttl_seconds: u32,
        #[serde(default = "default_impact_low")]
        impact_level: ImpactLevel,
    },

    /// Plugin-defined custom action
    Custom {
        plugin_name: String,
        action_type: String,
        #[serde(default)]
        config: Value,
        #[serde(default = "default_impact_medium")]
        impact_level: ImpactLevel,
    },
}

fn default_impact_low() -> ImpactLevel {
    ImpactLevel::Low
}

fn default_impact_medium() -> ImpactLevel {
    ImpactLevel::Medium
}

fn default_impact_from_command() -> ImpactLevel {
    // Default to High for agent commands - can be overridden per command
    ImpactLevel::High
}

fn default_priority() -> String {
    "P2".to_string()
}

fn default_feature_source() -> String {
    "automation".to_string()
}

fn default_automation_reason() -> String {
    "automation".to_string()
}

/// Execution history entry
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExecutionRecord {
    pub automation_id: String,
    pub automation_name: String,
    #[serde(with = "time::serde::iso8601")]
    pub executed_at: OffsetDateTime,
    pub trigger_event: String,
    pub conditions_met: bool,
    pub actions_executed: Vec<ActionResult>,
    pub success: bool,
    #[serde(default)]
    pub error: Option<String>,
    // Decision Engine integration
    #[serde(default)]
    pub trust_score: Option<f32>,
    #[serde(default)]
    pub decision_outcome: Option<String>, // "approved", "blocked", "require_validation"
}

/// Result of a single action execution
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ActionResult {
    pub action_type: String,
    pub success: bool,
    #[serde(default)]
    pub error: Option<String>,
    pub duration_ms: u64,
    // Decision Engine integration
    #[serde(default)]
    pub decision_id: Option<String>,
    #[serde(default)]
    pub trust_score: Option<f32>,
    #[serde(default)]
    pub decision_outcome: Option<String>, // "approved", "blocked", "require_validation", "skipped"
    #[serde(default)]
    pub blocked_reasons: Option<Vec<String>>,
}

/// Request to create/update automation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AutomationRequest {
    pub name: String,
    #[serde(default)]
    pub description: Option<String>,
    /// Category for organizing automations (systeme, alertes, modes, notifications, custom)
    #[serde(default)]
    pub category: Option<String>,
    /// Goal mode - what context mode this automation aims to achieve/maintain
    /// Used for Intelligence feedback learning. If None, no learning occurs.
    #[serde(default)]
    pub goal_mode: Option<String>,
    #[serde(default = "default_true")]
    pub enabled: bool,
    /// Legacy single trigger (for backward compatibility)
    #[serde(default)]
    pub trigger: Option<Trigger>,
    /// New trigger group with AND/OR logic
    #[serde(default)]
    pub triggers: Option<TriggerGroup>,
    #[serde(default)]
    pub conditions: Option<ConditionGroup>,
    pub actions: Vec<ActionDefinition>,
    #[serde(default = "default_cooldown")]
    pub cooldown_seconds: u32,

    // Intelligence flags (optionnels, pour création via API)
    /// Si true, bypass le Decision Engine (auto-approuvé)
    #[serde(default)]
    pub trusted: Option<bool>,
    /// Si true, ne pas exécuter si déjà dans le mode cible
    #[serde(default)]
    pub skip_if_same_mode: Option<bool>,
    /// Marqué true si créé automatiquement par le système d'intelligence
    #[serde(default)]
    pub auto_created: Option<bool>,
}

impl AutomationRequest {
    /// Get the effective TriggerGroup from request (prefers `triggers`, falls back to `trigger`)
    pub fn get_trigger_group(&self) -> Option<TriggerGroup> {
        if let Some(ref tg) = self.triggers {
            Some(tg.clone())
        } else if let Some(ref t) = self.trigger {
            Some(TriggerGroup {
                operator: LogicalOperator::Or,
                triggers: vec![TriggerItem::Single(t.clone())],
            })
        } else {
            None
        }
    }

    /// Check if request has any triggers defined
    pub fn has_triggers(&self) -> bool {
        if let Some(ref tg) = self.triggers {
            !tg.triggers.is_empty()
        } else {
            self.trigger.is_some()
        }
    }
}

/// Response for automation list
#[derive(Debug, Serialize)]
pub struct AutomationsListResponse {
    pub automations: Vec<Automation>,
    pub count: usize,
    pub enabled_count: usize,
}

/// Response for toggle endpoint
#[derive(Debug, Serialize, Deserialize)]
pub struct ToggleRequest {
    pub enabled: bool,
}

/// Response for test endpoint (dry-run)
#[derive(Debug, Serialize)]
pub struct TestResult {
    pub automation_id: String,
    pub would_execute: bool,
    pub conditions_evaluation: Vec<ConditionEvaluation>,
    pub actions_preview: Vec<String>,
}

/// Single condition evaluation result
#[derive(Debug, Serialize)]
pub struct ConditionEvaluation {
    pub condition_type: String,
    pub passed: bool,
    pub details: String,
}

impl ActionDefinition {
    /// Get the action type name
    pub fn type_name(&self) -> &'static str {
        match self {
            ActionDefinition::SendNotification { .. } => "send_notification",
            ActionDefinition::ForceMode { .. } => "force_mode",
            ActionDefinition::AgentCommand { .. } => "agent_command",
            ActionDefinition::Delay { .. } => "delay",
            ActionDefinition::SetFeature { .. } => "set_feature",
            ActionDefinition::Custom { .. } => "custom",
        }
    }

    /// Get the impact level (Delay always returns Low)
    pub fn impact_level(&self) -> ImpactLevel {
        match self {
            ActionDefinition::SendNotification { impact_level, .. } => *impact_level,
            ActionDefinition::ForceMode { impact_level, .. } => *impact_level,
            ActionDefinition::AgentCommand { impact_level, .. } => *impact_level,
            ActionDefinition::Delay { .. } => ImpactLevel::Low, // Always allowed
            ActionDefinition::SetFeature { impact_level, .. } => *impact_level,
            ActionDefinition::Custom { impact_level, .. } => *impact_level,
        }
    }

    /// Get the agent_id if applicable
    pub fn agent_id(&self) -> Option<&str> {
        match self {
            ActionDefinition::AgentCommand { agent_id, .. } => Some(agent_id),
            _ => None,
        }
    }

    /// Get impact level based on command type for AgentCommand
    pub fn impact_for_command(command_type: &str) -> ImpactLevel {
        match command_type {
            "notify" | "focus" => ImpactLevel::Medium,
            "lock" | "sleep" => ImpactLevel::High,
            "restart" | "shutdown" => ImpactLevel::VeryHigh,
            _ => ImpactLevel::High, // Default to High for unknown commands
        }
    }
}

impl Automation {
    /// Check if automation is in cooldown period
    pub fn is_in_cooldown(&self) -> bool {
        if let Some(last_exec) = self.last_executed_at {
            let now = OffsetDateTime::now_utc();
            let elapsed = (now - last_exec).whole_seconds();
            elapsed < self.cooldown_seconds as i64
        } else {
            false
        }
    }

    /// Get seconds remaining in cooldown
    pub fn cooldown_remaining(&self) -> Option<i64> {
        if let Some(last_exec) = self.last_executed_at {
            let now = OffsetDateTime::now_utc();
            let elapsed = (now - last_exec).whole_seconds();
            let remaining = self.cooldown_seconds as i64 - elapsed;
            if remaining > 0 {
                Some(remaining)
            } else {
                None
            }
        } else {
            None
        }
    }

    /// Get the effective TriggerGroup (prefers `triggers`, falls back to wrapping `trigger`)
    pub fn get_trigger_group(&self) -> TriggerGroup {
        if let Some(ref tg) = self.triggers {
            tg.clone()
        } else if let Some(ref t) = self.trigger {
            TriggerGroup {
                operator: LogicalOperator::Or,
                triggers: vec![TriggerItem::Single(t.clone())],
            }
        } else {
            TriggerGroup::default()
        }
    }

    /// Check if automation has any triggers defined
    pub fn has_triggers(&self) -> bool {
        if let Some(ref tg) = self.triggers {
            !tg.triggers.is_empty()
        } else {
            self.trigger.is_some()
        }
    }

    /// Extract the target mode from this automation (for Intelligence feedback)
    ///
    /// SIMPLIFIED (v1.1.9): Only goal_mode is used for learning.
    /// ForceMode actions are EXECUTED but don't influence Intelligence learning.
    /// This prevents side-effect actions from polluting the learning model.
    ///
    /// If None is returned, NO LEARNING should occur.
    pub fn target_mode(&self) -> Option<String> {
        // ONLY source: explicit goal_mode
        // ForceMode/ModeChange are still executed but don't drive learning
        self.goal_mode.clone()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_trigger_serialization() {
        let trigger = Trigger::ModeChange {
            from_mode: Some("maison".to_string()),
            to_mode: Some("pro".to_string()),
        };

        let json = serde_json::to_string(&trigger).unwrap();
        assert!(json.contains("mode_change"));
        assert!(json.contains("from_mode"));

        let parsed: Trigger = serde_json::from_str(&json).unwrap();
        if let Trigger::ModeChange { from_mode, to_mode } = parsed {
            assert_eq!(from_mode, Some("maison".to_string()));
            assert_eq!(to_mode, Some("pro".to_string()));
        } else {
            panic!("Wrong trigger type");
        }
    }

    #[test]
    fn test_condition_group() {
        let group = ConditionGroup {
            operator: LogicalOperator::And,
            conditions: vec![
                Condition::CurrentMode {
                    mode: "maison".to_string(),
                    operator: ComparisonOperator::Equals,
                },
                Condition::TimeRange {
                    start_hour: 8,
                    end_hour: 22,
                },
            ],
        };

        let json = serde_json::to_string(&group).unwrap();
        assert!(json.contains("and"));
        assert!(json.contains("current_mode"));
        assert!(json.contains("time_range"));
    }

    #[test]
    fn test_action_serialization() {
        let action = ActionDefinition::SendNotification {
            priority: "P1".to_string(),
            title: "Test".to_string(),
            body: "Message".to_string(),
            impact_level: ImpactLevel::Low,
        };

        let json = serde_json::to_string(&action).unwrap();
        assert!(json.contains("send_notification"));
        assert!(json.contains("priority"));
        assert!(json.contains("impact_level"));
    }

    #[test]
    fn test_action_impact_levels() {
        let notif = ActionDefinition::SendNotification {
            priority: "P1".to_string(),
            title: "Test".to_string(),
            body: "Body".to_string(),
            impact_level: ImpactLevel::Low,
        };
        assert_eq!(notif.impact_level(), ImpactLevel::Low);
        assert_eq!(notif.type_name(), "send_notification");

        let cmd = ActionDefinition::AgentCommand {
            agent_id: "agent1".to_string(),
            command_type: "shutdown".to_string(),
            parameters: None,
            impact_level: ImpactLevel::VeryHigh,
        };
        assert_eq!(cmd.impact_level(), ImpactLevel::VeryHigh);
        assert_eq!(cmd.agent_id(), Some("agent1"));

        let delay = ActionDefinition::Delay { seconds: 5 };
        assert_eq!(delay.impact_level(), ImpactLevel::Low); // Always low
    }

    #[test]
    fn test_cooldown_check() {
        let automation = Automation {
            id: "test".to_string(),
            name: "Test".to_string(),
            description: None,
            category: Some("custom".to_string()),
            goal_mode: None,
            enabled: true,
            trigger: Some(Trigger::Manual),
            triggers: None,
            conditions: None,
            actions: vec![],
            cooldown_seconds: 60,
            trusted: None,
            skip_if_same_mode: None,
            auto_created: None,
            last_executed_at: Some(OffsetDateTime::now_utc()),
            execution_count: 1,
            created_at: None,
            updated_at: None,
            deleted_at: None,
        };

        assert!(automation.is_in_cooldown());
        assert!(automation.cooldown_remaining().unwrap() > 0);
    }
}
