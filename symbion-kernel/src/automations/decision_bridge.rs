/**
 * SYMBION KERNEL - Automation to Decision Bridge
 *
 * ROLE: Convert automation actions to Decision Engine actions
 *
 * This module bridges the Automations system with the Decision Engine,
 * ensuring all automation actions are evaluated for trust and guards
 * before execution.
 */

use crate::automations::types::{ActionDefinition, Automation};
use crate::decision::{Action, ImpactLevel};
use uuid::Uuid;

/// Convert an automation action to a Decision Engine action
pub fn action_to_decision(
    action: &ActionDefinition,
    automation: &Automation,
) -> Action {
    let agent_id = action.agent_id()
        .map(|s| s.to_string())
        .unwrap_or_else(|| "system".to_string());

    // For AgentCommand, use command-specific impact level if not explicitly set
    let impact_level = match action {
        ActionDefinition::AgentCommand { command_type, impact_level, .. } => {
            // If impact is the default (High), check if command warrants different level
            if *impact_level == ImpactLevel::High {
                ActionDefinition::impact_for_command(command_type)
            } else {
                *impact_level
            }
        }
        _ => action.impact_level(),
    };

    Action {
        action_type: format!("automation.{}", action.type_name()),
        agent_id,
        impact_level,
        trace_id: format!("auto_{}_{}", automation.id, Uuid::new_v4()),
        expires_at: None, // Automations don't expire mid-execution
        dry_run: false,
        expected_mode: None, // We'll set this from context
        expected_ssid: None, // We'll set this from context
    }
}

/// Convert an automation action for dry-run evaluation
pub fn action_to_decision_dry_run(
    action: &ActionDefinition,
    automation: &Automation,
) -> Action {
    let mut decision_action = action_to_decision(action, automation);
    decision_action.dry_run = true;
    decision_action
}

/// Get a descriptive name for logging
pub fn action_description(action: &ActionDefinition) -> String {
    match action {
        ActionDefinition::SendNotification { title, priority, .. } => {
            format!("send_notification(title='{}', priority={})", title, priority)
        }
        ActionDefinition::ForceMode { mode, duration_minutes, .. } => {
            if let Some(mins) = duration_minutes {
                format!("force_mode(mode='{}', duration={}min)", mode, mins)
            } else {
                format!("force_mode(mode='{}')", mode)
            }
        }
        ActionDefinition::AgentCommand { agent_id, command_type, .. } => {
            format!("agent_command(agent='{}', cmd='{}')", agent_id, command_type)
        }
        ActionDefinition::Delay { seconds } => {
            format!("delay({}s)", seconds)
        }
        ActionDefinition::Custom { plugin_name, action_type, .. } => {
            format!("custom(plugin='{}', type='{}')", plugin_name, action_type)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::automations::types::Trigger;

    fn create_test_automation() -> Automation {
        Automation {
            id: "test-auto-1".to_string(),
            name: "Test Automation".to_string(),
            description: None,
            category: Some("custom".to_string()),
            goal_mode: None,
            enabled: true,
            trigger: Some(Trigger::Manual),
            triggers: None,
            conditions: None,
            actions: vec![],
            cooldown_seconds: 300,
            trusted: None,
            skip_if_same_mode: None,
            auto_created: None,
            last_executed_at: None,
            execution_count: 0,
            created_at: None,
            updated_at: None,
            deleted_at: None,
        }
    }

    #[test]
    fn test_notification_to_decision() {
        let automation = create_test_automation();
        let action = ActionDefinition::SendNotification {
            priority: "P1".to_string(),
            title: "Test".to_string(),
            body: "Body".to_string(),
            impact_level: ImpactLevel::Low,
        };

        let decision = action_to_decision(&action, &automation);

        assert_eq!(decision.action_type, "automation.send_notification");
        assert_eq!(decision.agent_id, "system");
        assert_eq!(decision.impact_level, ImpactLevel::Low);
        assert!(decision.trace_id.starts_with("auto_test-auto-1_"));
        assert!(!decision.dry_run);
    }

    #[test]
    fn test_agent_command_to_decision() {
        let automation = create_test_automation();
        let action = ActionDefinition::AgentCommand {
            agent_id: "pc-bureau".to_string(),
            command_type: "shutdown".to_string(),
            parameters: None,
            impact_level: ImpactLevel::High, // Default
        };

        let decision = action_to_decision(&action, &automation);

        assert_eq!(decision.action_type, "automation.agent_command");
        assert_eq!(decision.agent_id, "pc-bureau");
        // Should be upgraded to VeryHigh because shutdown is dangerous
        assert_eq!(decision.impact_level, ImpactLevel::VeryHigh);
    }

    #[test]
    fn test_agent_command_notify_medium() {
        let automation = create_test_automation();
        let action = ActionDefinition::AgentCommand {
            agent_id: "pc-salon".to_string(),
            command_type: "notify".to_string(),
            parameters: None,
            impact_level: ImpactLevel::High, // Default, should be downgraded
        };

        let decision = action_to_decision(&action, &automation);

        // notify command should be Medium, not High
        assert_eq!(decision.impact_level, ImpactLevel::Medium);
    }

    #[test]
    fn test_dry_run_conversion() {
        let automation = create_test_automation();
        let action = ActionDefinition::ForceMode {
            mode: "pro".to_string(),
            duration_minutes: Some(30),
            reason: "Test".to_string(),
            impact_level: ImpactLevel::Medium,
            use_override: None,
        };

        let decision = action_to_decision_dry_run(&action, &automation);

        assert!(decision.dry_run);
        assert_eq!(decision.action_type, "automation.force_mode");
    }

    #[test]
    fn test_action_description() {
        let action1 = ActionDefinition::SendNotification {
            priority: "P1".to_string(),
            title: "Alert".to_string(),
            body: "Test body".to_string(),
            impact_level: ImpactLevel::Low,
        };
        assert!(action_description(&action1).contains("Alert"));
        assert!(action_description(&action1).contains("P1"));

        let action2 = ActionDefinition::AgentCommand {
            agent_id: "pc-bureau".to_string(),
            command_type: "lock".to_string(),
            parameters: None,
            impact_level: ImpactLevel::High,
        };
        assert!(action_description(&action2).contains("pc-bureau"));
        assert!(action_description(&action2).contains("lock"));
    }
}
