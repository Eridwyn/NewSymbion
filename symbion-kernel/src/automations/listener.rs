/**
 * SYMBION KERNEL - Automations Event Listener
 *
 * ROLE: Listen to automation events and trigger matching automations
 *
 * ARCHITECTURE:
 * - Subscribes to EventDispatcher broadcast channel
 * - Matches events against automation triggers
 * - Evaluates conditions via AutomationEngine
 * - Executes actions via AutomationEngine (with Decision Engine integration)
 * - Records execution history with trust scores
 */

use crate::agents::SharedAgentRegistry;
use crate::automations::{
    AutomationEvent, AutomationStore, Trigger, AlertLevel, AgentStatusType, PluginHealthStatus,
    AutomationEngine, ExecutionContext, ExecutionRecord, SharedPendingActionRegistry,
    TriggerGroup, TriggerItem, LogicalOperator,
};
use crate::context::ContextEngine;
use crate::decision::{DecisionEngine, SharedTrustTracker, ValidationManager};
use crate::notifications::SharedNotificationManager;
use crate::sensors::SensorRegistry;

use std::sync::Arc;
use tokio::sync::broadcast;
use time::OffsetDateTime;

/// Listener task that processes automation events
pub struct AutomationListener {
    store: Arc<AutomationStore>,
    context_engine: Arc<ContextEngine>,
    agents: SharedAgentRegistry,
    sensors: Arc<SensorRegistry>,
    notifications_manager: SharedNotificationManager,
    /// Decision Engine for trust evaluation (Phase 7)
    decision_engine: Option<Arc<DecisionEngine>>,
    /// Trust Tracker for evolving statistics (Phase 7)
    trust_tracker: Option<SharedTrustTracker>,
    /// Validation Manager for pending approvals (Phase 7)
    validation_manager: Option<Arc<ValidationManager>>,
    /// Pending Action Registry for post-approval execution
    pending_action_registry: Option<SharedPendingActionRegistry>,
}

impl AutomationListener {
    pub fn new(
        store: Arc<AutomationStore>,
        context_engine: Arc<ContextEngine>,
        agents: SharedAgentRegistry,
        sensors: Arc<SensorRegistry>,
        notifications_manager: SharedNotificationManager,
        decision_engine: Option<Arc<DecisionEngine>>,
        trust_tracker: Option<SharedTrustTracker>,
        validation_manager: Option<Arc<ValidationManager>>,
        pending_action_registry: Option<SharedPendingActionRegistry>,
    ) -> Self {
        Self {
            store,
            context_engine,
            agents,
            sensors,
            notifications_manager,
            decision_engine,
            trust_tracker,
            validation_manager,
            pending_action_registry,
        }
    }

    /// Spawn the listener task
    pub fn spawn(self, mut receiver: broadcast::Receiver<AutomationEvent>) {
        tokio::spawn(async move {
            eprintln!("[automations] Event listener started");

            loop {
                match receiver.recv().await {
                    Ok(event) => {
                        self.handle_event(event).await;
                    }
                    Err(broadcast::error::RecvError::Lagged(n)) => {
                        eprintln!("[automations] Listener lagged, skipped {} events", n);
                    }
                    Err(broadcast::error::RecvError::Closed) => {
                        eprintln!("[automations] Event channel closed, listener stopping");
                        break;
                    }
                }
            }
        });
    }

    /// Handle a single event
    async fn handle_event(&self, event: AutomationEvent) {
        let event_type = event.event_type();
        eprintln!("[automations] Processing {} event", event_type);

        // Get all enabled automations
        let automations = self.store.get_enabled();

        let mut matched_count = 0;

        for automation in automations {
            // Skip if disabled
            if !automation.enabled {
                continue;
            }

            // Check if trigger group matches
            let trigger_group = automation.get_trigger_group();
            if !self.trigger_group_matches(&trigger_group, &event, &automation.id) {
                continue;
            }

            // Check cooldown
            if automation.is_in_cooldown() {
                if let Some(remaining) = automation.cooldown_remaining() {
                    eprintln!(
                        "[automations] '{}' matched but in cooldown ({} seconds remaining)",
                        automation.name, remaining
                    );
                }
                continue;
            }

            matched_count += 1;

            eprintln!(
                "[automations] ✅ TRIGGER MATCH: '{}' (id: {})",
                automation.name, automation.id
            );

            // Create execution context with Decision Engine, Trust Tracker, ValidationManager, and PendingActionRegistry
            let ctx = ExecutionContext {
                context_engine: self.context_engine.clone(),
                agents: self.agents.clone(),
                sensors: self.sensors.clone(),
                notifications_manager: self.notifications_manager.clone(),
                event: event.clone(),
                decision_engine: self.decision_engine.clone(),
                trust_tracker: self.trust_tracker.clone(),
                validation_manager: self.validation_manager.clone(),
                pending_action_registry: self.pending_action_registry.clone(),
            };

            // Evaluate conditions
            let (conditions_met, evaluations) =
                AutomationEngine::evaluate_conditions(&automation.conditions, &ctx);

            if !conditions_met {
                eprintln!(
                    "[automations]   ❌ Conditions NOT met for '{}': {:?}",
                    automation.name,
                    evaluations.iter()
                        .filter(|e| !e.passed)
                        .map(|e| &e.details)
                        .collect::<Vec<_>>()
                );

                // Record failed execution
                let record = ExecutionRecord {
                    automation_id: automation.id.clone(),
                    automation_name: automation.name.clone(),
                    executed_at: OffsetDateTime::now_utc(),
                    trigger_event: event_type.to_string(),
                    conditions_met: false,
                    actions_executed: vec![],
                    success: false,
                    error: Some("Conditions not met".to_string()),
                    trust_score: None,
                    decision_outcome: None,
                };
                let _ = self.store.add_history(record);
                continue;
            }

            eprintln!(
                "[automations]   ✓ Conditions met, executing {} action(s)",
                automation.actions.len()
            );

            // Execute actions
            let action_results = AutomationEngine::execute_actions(&automation, &ctx).await;

            // Check overall success
            let all_success = action_results.iter().all(|r| r.success);
            let error_msg = if all_success {
                None
            } else {
                Some(action_results.iter()
                    .filter(|r| !r.success)
                    .filter_map(|r| r.error.as_ref())
                    .cloned()
                    .collect::<Vec<_>>()
                    .join("; "))
            };

            // Extract trust info from action results (use first approved action's trust score)
            let (overall_trust_score, overall_decision_outcome) = action_results.iter()
                .find(|r| r.trust_score.is_some())
                .map(|r| (r.trust_score, r.decision_outcome.clone()))
                .unwrap_or((None, None));

            // Check if all actions are pending validation - if so, skip history record
            // The final record will be created when validation is approved/rejected
            let all_pending_validation = action_results.iter().all(|r| {
                r.decision_outcome.as_ref().map(|o| o == "require_validation").unwrap_or(false)
            });

            if all_pending_validation {
                eprintln!(
                    "[automations] ⏳ '{}' pending validation - skipping history until resolved",
                    automation.name
                );
                continue;
            }

            // Record execution with trust info
            let record = ExecutionRecord {
                automation_id: automation.id.clone(),
                automation_name: automation.name.clone(),
                executed_at: OffsetDateTime::now_utc(),
                trigger_event: event_type.to_string(),
                conditions_met: true,
                actions_executed: action_results,
                success: all_success,
                error: error_msg,
                trust_score: overall_trust_score,
                decision_outcome: overall_decision_outcome,
            };
            let _ = self.store.add_history(record);

            // Update execution tracking (for cooldown)
            if let Err(e) = self.store.record_execution(&automation.id) {
                eprintln!(
                    "[automations] Failed to record execution for '{}': {}",
                    automation.name, e
                );
            }

            eprintln!(
                "[automations]   {} '{}' execution {}",
                if all_success { "✅" } else { "⚠️" },
                automation.name,
                if all_success { "completed" } else { "completed with errors" }
            );
        }

        if matched_count > 0 {
            eprintln!(
                "[automations] {} automation(s) triggered by {} event",
                matched_count, event_type
            );
        }
    }

    /// Check if a trigger matches an event
    fn trigger_matches(&self, trigger: &Trigger, event: &AutomationEvent, automation_id: &str) -> bool {
        match (trigger, event) {
            // Mode change trigger
            (
                Trigger::ModeChange { from_mode, to_mode },
                AutomationEvent::ModeChange {
                    from_mode: event_from,
                    to_mode: event_to,
                    ..
                },
            ) => {
                let from_matches = from_mode
                    .as_ref()
                    .map(|m| m.eq_ignore_ascii_case(event_from))
                    .unwrap_or(true);

                let to_matches = to_mode
                    .as_ref()
                    .map(|m| m.eq_ignore_ascii_case(event_to))
                    .unwrap_or(true);

                from_matches && to_matches
            }

            // Sensor alert trigger
            (
                Trigger::SensorAlert { room_id, alert_level },
                AutomationEvent::SensorAlert {
                    room_id: event_room,
                    alert_level: event_level,
                    ..
                },
            ) => {
                let room_matches = room_id
                    .as_ref()
                    .map(|r| r.eq_ignore_ascii_case(event_room))
                    .unwrap_or(true);

                let level_matches = alert_level
                    .as_ref()
                    .map(|l| self.alert_level_matches(*l, event_level))
                    .unwrap_or(true);

                room_matches && level_matches
            }

            // Agent status trigger
            (
                Trigger::AgentStatus { agent_id, status },
                AutomationEvent::AgentStatus {
                    agent_id: event_agent,
                    status: event_status,
                    ..
                },
            ) => {
                let agent_matches = agent_id
                    .as_ref()
                    .map(|a| a.eq_ignore_ascii_case(event_agent))
                    .unwrap_or(true);

                let status_matches = match status {
                    AgentStatusType::Any => true,
                    AgentStatusType::Online => event_status.eq_ignore_ascii_case("online"),
                    AgentStatusType::Offline => event_status.eq_ignore_ascii_case("offline"),
                };

                agent_matches && status_matches
            }

            // Manual trigger only via API - must match specific automation
            (Trigger::Manual, AutomationEvent::Manual { automation_id: target_id, .. }) => {
                // Manual triggers are specific to an automation - check ID matches
                automation_id == target_id
            }

            // Plugin health trigger
            (
                Trigger::PluginHealth { plugin_name, status },
                AutomationEvent::PluginHealth {
                    plugin_name: event_plugin,
                    status: event_status,
                    ..
                },
            ) => {
                let plugin_matches = plugin_name
                    .as_ref()
                    .map(|p| p.eq_ignore_ascii_case(event_plugin))
                    .unwrap_or(true);

                let status_matches = self.plugin_health_status_matches(*status, event_status);

                plugin_matches && status_matches
            }

            // Scheduled trigger - matches only Scheduled events for this specific automation
            (
                Trigger::Scheduled { .. },
                AutomationEvent::Scheduled { automation_id: event_auto_id, .. }
            ) => {
                // Scheduled events are targeted to a specific automation
                automation_id == event_auto_id
            }

            // Custom plugin trigger (Phase 5+)
            (Trigger::Custom { plugin_name: _, trigger_type: _, .. }, _) => {
                // Custom triggers will be implemented with plugin support
                false
            }

            // No match for other combinations
            _ => false,
        }
    }

    /// Check if plugin health status matches
    fn plugin_health_status_matches(&self, trigger_status: PluginHealthStatus, event_status: &str) -> bool {
        let normalized_event = event_status.to_lowercase();
        match trigger_status {
            PluginHealthStatus::Any => true,
            PluginHealthStatus::Healthy => normalized_event == "healthy",
            PluginHealthStatus::Unhealthy => normalized_event == "unhealthy",
            PluginHealthStatus::RecoveryAttempt => normalized_event == "recovery_attempt",
            PluginHealthStatus::RecoveryFailed => normalized_event == "recovery_failed",
            PluginHealthStatus::RecoverySuccess => normalized_event == "recovery_success",
        }
    }

    /// Check if alert level matches (handles string comparison)
    fn alert_level_matches(&self, trigger_level: AlertLevel, event_level: &str) -> bool {
        // Also match danger/critical variants
        let normalized_event = event_level.to_lowercase();
        match trigger_level {
            AlertLevel::Normal => normalized_event == "safe" || normalized_event == "normal",
            AlertLevel::Moderate => normalized_event == "moderate" || normalized_event == "weak",
            AlertLevel::High => normalized_event == "strong" || normalized_event == "high",
            AlertLevel::Critical => normalized_event == "critical" || normalized_event == "danger",
        }
    }

    /// Check if a trigger group matches an event (AND/OR logic)
    fn trigger_group_matches(&self, group: &TriggerGroup, event: &AutomationEvent, automation_id: &str) -> bool {
        if group.triggers.is_empty() {
            return false; // No triggers = no match
        }

        match group.operator {
            LogicalOperator::Or => {
                // ANY trigger matches = group matches
                group.triggers.iter().any(|item| {
                    self.trigger_item_matches(item, event, automation_id)
                })
            }
            LogicalOperator::And => {
                // ALL triggers must match the same event
                group.triggers.iter().all(|item| {
                    self.trigger_item_matches(item, event, automation_id)
                })
            }
        }
    }

    /// Check if a single trigger item matches (can be single trigger or nested group)
    fn trigger_item_matches(&self, item: &TriggerItem, event: &AutomationEvent, automation_id: &str) -> bool {
        match item {
            TriggerItem::Single(trigger) => self.trigger_matches(trigger, event, automation_id),
            TriggerItem::Group(nested_group) => self.trigger_group_matches(nested_group, event, automation_id),
        }
    }
}

/// Spawn the automation listener with given store and dispatcher
pub fn spawn_automation_listener(
    store: Arc<AutomationStore>,
    context_engine: Arc<ContextEngine>,
    agents: SharedAgentRegistry,
    sensors: Arc<SensorRegistry>,
    notifications_manager: SharedNotificationManager,
    receiver: broadcast::Receiver<AutomationEvent>,
    decision_engine: Option<Arc<DecisionEngine>>,
    trust_tracker: Option<SharedTrustTracker>,
    validation_manager: Option<Arc<ValidationManager>>,
    pending_action_registry: Option<SharedPendingActionRegistry>,
) {
    let listener = AutomationListener::new(
        store,
        context_engine,
        agents,
        sensors,
        notifications_manager,
        decision_engine,
        trust_tracker,
        validation_manager,
        pending_action_registry,
    );
    listener.spawn(receiver);
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::automations::{AlertLevel, AgentStatusType, Trigger};
    use time::OffsetDateTime;

    /// Helper to create a test listener for trigger matching tests
    /// We only need the trigger_matches method, so we can create a minimal listener
    struct TriggerMatchTester;

    impl TriggerMatchTester {
        fn trigger_matches(&self, trigger: &Trigger, event: &AutomationEvent, automation_id: &str) -> bool {
            match (trigger, event) {
                // Mode change trigger
                (
                    Trigger::ModeChange { from_mode, to_mode },
                    AutomationEvent::ModeChange {
                        from_mode: event_from,
                        to_mode: event_to,
                        ..
                    },
                ) => {
                    let from_matches = from_mode
                        .as_ref()
                        .map(|m| m.eq_ignore_ascii_case(event_from))
                        .unwrap_or(true);

                    let to_matches = to_mode
                        .as_ref()
                        .map(|m| m.eq_ignore_ascii_case(event_to))
                        .unwrap_or(true);

                    from_matches && to_matches
                }

                // Sensor alert trigger
                (
                    Trigger::SensorAlert { room_id, alert_level },
                    AutomationEvent::SensorAlert {
                        room_id: event_room,
                        alert_level: event_level,
                        ..
                    },
                ) => {
                    let room_matches = room_id
                        .as_ref()
                        .map(|r| r.eq_ignore_ascii_case(event_room))
                        .unwrap_or(true);

                    let level_matches = alert_level
                        .as_ref()
                        .map(|l| self.alert_level_matches(*l, event_level))
                        .unwrap_or(true);

                    room_matches && level_matches
                }

                // Agent status trigger
                (
                    Trigger::AgentStatus { agent_id, status },
                    AutomationEvent::AgentStatus {
                        agent_id: event_agent,
                        status: event_status,
                        ..
                    },
                ) => {
                    let agent_matches = agent_id
                        .as_ref()
                        .map(|a| a.eq_ignore_ascii_case(event_agent))
                        .unwrap_or(true);

                    let status_matches = match status {
                        AgentStatusType::Any => true,
                        AgentStatusType::Online => event_status.eq_ignore_ascii_case("online"),
                        AgentStatusType::Offline => event_status.eq_ignore_ascii_case("offline"),
                    };

                    agent_matches && status_matches
                }

                // Manual trigger
                (Trigger::Manual, AutomationEvent::Manual { automation_id: target_id, .. }) => {
                    automation_id == target_id
                }

                (Trigger::Custom { .. }, _) => false,
                _ => false,
            }
        }

        fn alert_level_matches(&self, trigger_level: AlertLevel, event_level: &str) -> bool {
            let normalized_event = event_level.to_lowercase();
            match trigger_level {
                AlertLevel::Normal => normalized_event == "safe" || normalized_event == "normal",
                AlertLevel::Moderate => normalized_event == "moderate" || normalized_event == "weak",
                AlertLevel::High => normalized_event == "strong" || normalized_event == "high",
                AlertLevel::Critical => normalized_event == "critical" || normalized_event == "danger",
            }
        }
    }

    #[test]
    fn test_mode_change_trigger_exact_match() {
        let tester = TriggerMatchTester;

        let trigger = Trigger::ModeChange {
            from_mode: Some("neutre".to_string()),
            to_mode: Some("cravate".to_string()),
        };

        let event = AutomationEvent::ModeChange {
            from_mode: "neutre".to_string(),
            to_mode: "cravate".to_string(),
            reason: "test".to_string(),
            timestamp: OffsetDateTime::now_utc(),
        };

        assert!(tester.trigger_matches(&trigger, &event, "auto_123"));
    }

    #[test]
    fn test_mode_change_trigger_wildcard_to() {
        let tester = TriggerMatchTester;

        let trigger = Trigger::ModeChange {
            from_mode: Some("neutre".to_string()),
            to_mode: None, // Any destination mode
        };

        let event = AutomationEvent::ModeChange {
            from_mode: "neutre".to_string(),
            to_mode: "intime".to_string(),
            reason: "test".to_string(),
            timestamp: OffsetDateTime::now_utc(),
        };

        assert!(tester.trigger_matches(&trigger, &event, "auto_123"));
    }

    #[test]
    fn test_mode_change_trigger_no_match() {
        let tester = TriggerMatchTester;

        let trigger = Trigger::ModeChange {
            from_mode: Some("cravate".to_string()),
            to_mode: Some("neutre".to_string()),
        };

        let event = AutomationEvent::ModeChange {
            from_mode: "neutre".to_string(),
            to_mode: "cravate".to_string(),
            reason: "test".to_string(),
            timestamp: OffsetDateTime::now_utc(),
        };

        assert!(!tester.trigger_matches(&trigger, &event, "auto_123"));
    }

    #[test]
    fn test_sensor_alert_trigger_exact_match() {
        let tester = TriggerMatchTester;

        let trigger = Trigger::SensorAlert {
            room_id: Some("chambre".to_string()),
            alert_level: Some(AlertLevel::High),
        };

        let event = AutomationEvent::SensorAlert {
            room_id: "chambre".to_string(),
            sensor_id: "esp32-001".to_string(),
            alert_level: "high".to_string(),
            previous_level: Some("normal".to_string()),
            temperature: Some(28.5),
            humidity: Some(85.0),
            timestamp: OffsetDateTime::now_utc(),
        };

        assert!(tester.trigger_matches(&trigger, &event, "auto_123"));
    }

    #[test]
    fn test_sensor_alert_trigger_any_room() {
        let tester = TriggerMatchTester;

        let trigger = Trigger::SensorAlert {
            room_id: None, // Any room
            alert_level: Some(AlertLevel::Critical),
        };

        let event = AutomationEvent::SensorAlert {
            room_id: "salon".to_string(),
            sensor_id: "esp32-002".to_string(),
            alert_level: "critical".to_string(),
            previous_level: Some("high".to_string()),
            temperature: Some(35.0),
            humidity: Some(90.0),
            timestamp: OffsetDateTime::now_utc(),
        };

        assert!(tester.trigger_matches(&trigger, &event, "auto_123"));
    }

    #[test]
    fn test_sensor_alert_level_aliases() {
        let tester = TriggerMatchTester;

        // Test "danger" alias for "critical"
        assert!(tester.alert_level_matches(AlertLevel::Critical, "danger"));
        assert!(tester.alert_level_matches(AlertLevel::Critical, "critical"));

        // Test "strong" alias for "high"
        assert!(tester.alert_level_matches(AlertLevel::High, "strong"));
        assert!(tester.alert_level_matches(AlertLevel::High, "high"));

        // Test "safe" alias for "normal"
        assert!(tester.alert_level_matches(AlertLevel::Normal, "safe"));
        assert!(tester.alert_level_matches(AlertLevel::Normal, "normal"));
    }

    #[test]
    fn test_agent_status_trigger_online() {
        let tester = TriggerMatchTester;

        let trigger = Trigger::AgentStatus {
            agent_id: Some("pc-salon".to_string()),
            status: AgentStatusType::Online,
        };

        let event = AutomationEvent::AgentStatus {
            agent_id: "pc-salon".to_string(),
            status: "online".to_string(),
            previous_status: Some("offline".to_string()),
            timestamp: OffsetDateTime::now_utc(),
        };

        assert!(tester.trigger_matches(&trigger, &event, "auto_123"));
    }

    #[test]
    fn test_agent_status_trigger_any_status() {
        let tester = TriggerMatchTester;

        let trigger = Trigger::AgentStatus {
            agent_id: Some("pc-bureau".to_string()),
            status: AgentStatusType::Any,
        };

        let event_offline = AutomationEvent::AgentStatus {
            agent_id: "pc-bureau".to_string(),
            status: "offline".to_string(),
            previous_status: Some("online".to_string()),
            timestamp: OffsetDateTime::now_utc(),
        };

        let event_online = AutomationEvent::AgentStatus {
            agent_id: "pc-bureau".to_string(),
            status: "online".to_string(),
            previous_status: Some("offline".to_string()),
            timestamp: OffsetDateTime::now_utc(),
        };

        assert!(tester.trigger_matches(&trigger, &event_offline, "auto_123"));
        assert!(tester.trigger_matches(&trigger, &event_online, "auto_123"));
    }

    #[test]
    fn test_agent_status_trigger_any_agent() {
        let tester = TriggerMatchTester;

        let trigger = Trigger::AgentStatus {
            agent_id: None, // Any agent
            status: AgentStatusType::Offline,
        };

        let event = AutomationEvent::AgentStatus {
            agent_id: "any-agent-001".to_string(),
            status: "offline".to_string(),
            previous_status: Some("online".to_string()),
            timestamp: OffsetDateTime::now_utc(),
        };

        assert!(tester.trigger_matches(&trigger, &event, "auto_123"));
    }

    #[test]
    fn test_manual_trigger_matches_correct_automation() {
        let tester = TriggerMatchTester;

        let trigger = Trigger::Manual;

        let event = AutomationEvent::Manual {
            automation_id: "auto_abc123".to_string(),
            triggered_by: Some("admin".to_string()),
            timestamp: OffsetDateTime::now_utc(),
        };

        // Should match when automation_id matches
        assert!(tester.trigger_matches(&trigger, &event, "auto_abc123"));

        // Should NOT match when automation_id is different
        assert!(!tester.trigger_matches(&trigger, &event, "auto_xyz789"));
    }

    #[test]
    fn test_trigger_type_mismatch() {
        let tester = TriggerMatchTester;

        let trigger = Trigger::ModeChange {
            from_mode: None,
            to_mode: None,
        };

        // Agent status event should not match mode change trigger
        let event = AutomationEvent::AgentStatus {
            agent_id: "pc-salon".to_string(),
            status: "online".to_string(),
            previous_status: None,
            timestamp: OffsetDateTime::now_utc(),
        };

        assert!(!tester.trigger_matches(&trigger, &event, "auto_123"));
    }

    #[test]
    fn test_case_insensitive_matching() {
        let tester = TriggerMatchTester;

        let trigger = Trigger::ModeChange {
            from_mode: Some("NEUTRE".to_string()),
            to_mode: Some("CRAVATE".to_string()),
        };

        let event = AutomationEvent::ModeChange {
            from_mode: "neutre".to_string(),
            to_mode: "cravate".to_string(),
            reason: "test".to_string(),
            timestamp: OffsetDateTime::now_utc(),
        };

        assert!(tester.trigger_matches(&trigger, &event, "auto_123"));
    }
}
