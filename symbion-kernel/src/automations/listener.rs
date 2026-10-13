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
use crate::context_intelligence::SharedContextIntelligence;
use crate::decision::{DecisionEngine, SharedTrustTracker, ValidationManager};
use crate::intelligence::SharedFeatureRegistry;
use crate::modes::SharedModeRegistry;
use crate::notifications::SharedNotificationManager;
use crate::sensors::SensorRegistry;

use std::collections::HashSet;
use std::sync::Arc;
use tokio::sync::{broadcast, Mutex};
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
    /// Context Intelligence for feedback loop (Decision → Intelligence)
    context_intelligence: Option<SharedContextIntelligence>,
    /// Mode Registry for validating dynamic modes (Invariant 2)
    mode_registry: Option<SharedModeRegistry>,
    /// Feature Registry for Intelligence v2 condition evaluation
    feature_registry: Option<SharedFeatureRegistry>,
    /// Plugin Registry for plugin_command actions
    plugin_registry: Option<crate::plugin_proxy::PluginRegistry>,
    /// Track automations currently executing (prevent re-trigger during long actions)
    executing: Arc<Mutex<HashSet<String>>>,
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
        context_intelligence: Option<SharedContextIntelligence>,
        mode_registry: Option<SharedModeRegistry>,
        feature_registry: Option<SharedFeatureRegistry>,
        plugin_registry: Option<crate::plugin_proxy::PluginRegistry>,
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
            context_intelligence,
            mode_registry,
            feature_registry,
            plugin_registry,
            executing: Arc::new(Mutex::new(HashSet::new())),
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
                        eprintln!("[automations] ⚠️ Listener lagged, skipped {} events — consider increasing channel capacity", n);
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
        let event_tag = event.trigger_type_tag();
        eprintln!("[automations] Processing {} event", event_type);

        // Get all enabled automations
        let automations = self.store.get_enabled();

        let mut matched_count = 0;

        for automation in automations {
            // D7: Fast pre-filter by event type tag (avoids expensive group matching)
            let trigger_group = automation.get_trigger_group();
            if !trigger_group.event_type_tags().contains(&event_tag) {
                continue;
            }
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

            // Atomic check-and-insert: prevents race condition where two threads
            // could pass the check before either inserts (previously separate check + insert)
            {
                let mut executing = self.executing.lock().await;
                if !executing.insert(automation.id.clone()) {
                    eprintln!("[automations] '{}' already executing, skipping", automation.name);
                    continue;
                }
            }

            matched_count += 1;

            eprintln!(
                "[automations] ✅ TRIGGER MATCH: '{}' (id: {})",
                automation.name, automation.id
            );

            // Create execution context with Decision Engine, Trust Tracker, ValidationManager, PendingActionRegistry, Intelligence, ModeRegistry, and FeatureRegistry
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
                context_intelligence: self.context_intelligence.clone(),
                mode_registry: self.mode_registry.clone(),
                feature_registry: self.feature_registry.clone(),
                plugin_registry: self.plugin_registry.clone(),
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
                self.executing.lock().await.remove(&automation.id);
                continue;
            }

            eprintln!(
                "[automations]   ✓ Conditions met, executing {} action(s)",
                automation.actions.len()
            );

            // Record execution IMMEDIATELY for cooldown enforcement (before actions)
            if let Err(e) = self.store.record_execution(&automation.id) {
                eprintln!(
                    "[automations] Failed to record execution for '{}': {}",
                    automation.name, e
                );
            }

            // Execute actions
            let action_results = AutomationEngine::execute_actions(&automation, &ctx).await;

            // Release execution lock
            self.executing.lock().await.remove(&automation.id);

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

            // Déclencheur poussé par un plugin : POST /v1/automations/events.
            (
                Trigger::Custom { plugin_name, trigger_type, config },
                AutomationEvent::Custom {
                    plugin_name: event_plugin,
                    trigger_type: event_type,
                    data,
                    ..
                },
            ) => {
                plugin_name.trim().eq_ignore_ascii_case(event_plugin.trim())
                    && trigger_type.trim().eq_ignore_ascii_case(event_type.trim())
                    && Self::custom_config_matches(config, data)
            }

            // No match for other combinations
            _ => false,
        }
    }

    /// Confronte le `config` d'un Trigger::Custom aux `data` de l'événement.
    ///
    /// Filtre par INCLUSION : chaque clé de `config` doit se retrouver dans
    /// `data`. Un `config` vide accepte tout événement du bon plugin/type.
    ///
    /// **Un tableau, des deux côtés, vaut un ENSEMBLE** et la comparaison est
    /// une intersection non vide :
    ///
    /// - `config` en tableau → « l'une de ces valeurs »
    ///   (`{"room": ["cuisine","salon"]}`) ;
    /// - `data` en tableau → « contient cette valeur ». C'est le cas réel : une
    ///   zone du robot couvre plusieurs pièces au sens des lampes (la cuisine
    ///   et le salon sont tous deux dans « Salle de séjour »), donc l'événement
    ///   porte `rooms: ["salon","cuisine"]` et une règle se vise sur une seule
    ///   d'entre elles ;
    /// - les deux en tableau → intersection.
    ///
    /// Les clés acceptent la notation pointée (`zone.id`) pour descendre dans
    /// les données imbriquées.
    fn custom_config_matches(config: &serde_json::Value, data: &serde_json::Value) -> bool {
        let Some(filtres) = config.as_object() else {
            return true; // pas d'objet de filtre = aucun filtre
        };
        filtres.iter().all(|(cle, attendu)| {
            match Self::valeur_au_chemin(data, cle) {
                None => false, // la donnée ne porte pas la clé demandée
                Some(trouve) => {
                    let attendus = Self::en_ensemble(attendu);
                    let trouves = Self::en_ensemble(trouve);
                    attendus
                        .iter()
                        .any(|a| trouves.iter().any(|t| Self::valeurs_egales(a, t)))
                }
            }
        })
    }

    /// Une valeur scalaire vaut un ensemble d'un élément ; un tableau vaut ses
    /// éléments. Un tableau VIDE ne vaut rien, et ne doit rien déclencher.
    fn en_ensemble(valeur: &serde_json::Value) -> Vec<&serde_json::Value> {
        match valeur.as_array() {
            Some(elements) => elements.iter().collect(),
            None => vec![valeur],
        }
    }

    /// Descend dans un JSON par chemin pointé (`zone.id`), ou clé simple.
    fn valeur_au_chemin<'a>(data: &'a serde_json::Value, chemin: &str) -> Option<&'a serde_json::Value> {
        if let Some(direct) = data.get(chemin) {
            return Some(direct); // une clé qui contient un point reste prioritaire
        }
        let mut courant = data;
        for segment in chemin.split('.') {
            courant = courant.get(segment)?;
        }
        Some(courant)
    }

    /// Égalité tolérante : 10000 == "10000", 1 == 1.0, "Cuisine" == "cuisine".
    /// Les règles sont écrites à la main dans la PWA ; un filtre ne doit pas
    /// échouer silencieusement parce que le plugin a poussé un nombre là où la
    /// règle dit une chaîne.
    fn valeurs_egales(a: &serde_json::Value, b: &serde_json::Value) -> bool {
        if a == b {
            return true;
        }
        match (a.as_f64(), b.as_f64()) {
            (Some(x), Some(y)) => return (x - y).abs() < f64::EPSILON,
            _ => {}
        }
        let texte = |v: &serde_json::Value| match v {
            serde_json::Value::String(s) => Some(s.clone()),
            serde_json::Value::Number(n) => Some(n.to_string()),
            serde_json::Value::Bool(b) => Some(b.to_string()),
            _ => None,
        };
        match (texte(a), texte(b)) {
            (Some(x), Some(y)) => x.trim().eq_ignore_ascii_case(y.trim()),
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
    context_intelligence: Option<SharedContextIntelligence>,
    mode_registry: Option<SharedModeRegistry>,
    feature_registry: Option<SharedFeatureRegistry>,
    plugin_registry: Option<crate::plugin_proxy::PluginRegistry>,
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
        context_intelligence,
        mode_registry,
        feature_registry,
        plugin_registry,
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
            from_mode: Some("veille".to_string()),
            to_mode: Some("pro".to_string()),
        };

        let event = AutomationEvent::ModeChange {
            from_mode: "veille".to_string(),
            to_mode: "pro".to_string(),
            reason: "test".to_string(),
            timestamp: OffsetDateTime::now_utc(),
        };

        assert!(tester.trigger_matches(&trigger, &event, "auto_123"));
    }

    #[test]
    fn test_mode_change_trigger_wildcard_to() {
        let tester = TriggerMatchTester;

        let trigger = Trigger::ModeChange {
            from_mode: Some("veille".to_string()),
            to_mode: None, // Any destination mode
        };

        let event = AutomationEvent::ModeChange {
            from_mode: "veille".to_string(),
            to_mode: "maison".to_string(),
            reason: "test".to_string(),
            timestamp: OffsetDateTime::now_utc(),
        };

        assert!(tester.trigger_matches(&trigger, &event, "auto_123"));
    }

    #[test]
    fn test_mode_change_trigger_no_match() {
        let tester = TriggerMatchTester;

        let trigger = Trigger::ModeChange {
            from_mode: Some("pro".to_string()),
            to_mode: Some("veille".to_string()),
        };

        let event = AutomationEvent::ModeChange {
            from_mode: "veille".to_string(),
            to_mode: "pro".to_string(),
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
            from_mode: Some("VEILLE".to_string()),
            to_mode: Some("PRO".to_string()),
        };

        let event = AutomationEvent::ModeChange {
            from_mode: "veille".to_string(),
            to_mode: "pro".to_string(),
            reason: "test".to_string(),
            timestamp: OffsetDateTime::now_utc(),
        };

        assert!(tester.trigger_matches(&trigger, &event, "auto_123"));
    }

    // ------------------------------------------------------------------
    // Trigger::Custom — le filtre par inclusion
    //
    // Ces tests portent sur la vraie fonction du listener (pas le mock
    // au-dessus) : custom_config_matches est une fonction associée, donc
    // appelable sans instancier le listener et ses douze dépendances.
    // ------------------------------------------------------------------

    use crate::automations::listener::AutomationListener;
    use serde_json::json;

    fn filtre(config: serde_json::Value, data: serde_json::Value) -> bool {
        AutomationListener::custom_config_matches(&config, &data)
    }

    #[test]
    fn config_vide_accepte_tout() {
        assert!(filtre(json!({}), json!({"zone": "cuisine"})));
        assert!(filtre(json!(null), json!({"zone": "cuisine"})));
    }

    #[test]
    fn config_filtre_par_inclusion() {
        let data = json!({"zone": "cuisine", "etat": "nettoyage"});
        assert!(filtre(json!({"zone": "cuisine"}), data.clone()));
        assert!(filtre(json!({"zone": "cuisine", "etat": "nettoyage"}), data.clone()));
        assert!(!filtre(json!({"zone": "salon"}), data.clone()));
        // une clé demandée mais absente des données ne doit PAS passer
        assert!(!filtre(json!({"piece": "cuisine"}), data));
    }

    #[test]
    fn config_en_tableau_vaut_une_de_ces_valeurs() {
        let data = json!({"zone": "salon"});
        assert!(filtre(json!({"zone": ["cuisine", "salon"]}), data.clone()));
        assert!(!filtre(json!({"zone": ["cuisine", "chambre"]}), data));
    }

    #[test]
    fn egalite_toleante_entre_nombre_et_chaine() {
        // le plugin pousse un identifiant de pièce Matter en nombre,
        // la règle écrite dans la PWA le porte en chaîne : ça doit matcher.
        assert!(filtre(json!({"zone_id": "10000"}), json!({"zone_id": 10000})));
        assert!(filtre(json!({"zone_id": 10000}), json!({"zone_id": "10000"})));
        assert!(filtre(json!({"zone": "Cuisine"}), json!({"zone": "cuisine"})));
        assert!(!filtre(json!({"zone_id": 10001}), json!({"zone_id": 10000})));
    }

    #[test]
    fn donnee_en_tableau_vaut_contient_cette_valeur() {
        // Le cas réel : « Salle de séjour » chez le robot couvre la cuisine ET
        // le salon côté lampes. L'événement porte les deux, une règle n'en
        // vise qu'une.
        let data = json!({"rooms": ["salon", "cuisine"], "zone": "Salle de séjour"});
        assert!(filtre(json!({"rooms": "salon"}), data.clone()));
        assert!(filtre(json!({"rooms": "cuisine"}), data.clone()));
        assert!(!filtre(json!({"rooms": "chambre"}), data.clone()));
        // tableau des deux côtés = intersection non vide
        assert!(filtre(json!({"rooms": ["chambre", "cuisine"]}), data.clone()));
        assert!(!filtre(json!({"rooms": ["chambre", "couloir"]}), data));
    }

    #[test]
    fn tableau_vide_ne_declenche_rien() {
        // Hors nettoyage, le robot ne couvre aucune pièce : une règle visant
        // une pièce ne doit pas partir.
        assert!(!filtre(json!({"rooms": "salon"}), json!({"rooms": []})));
        assert!(!filtre(json!({"rooms": []}), json!({"rooms": ["salon"]})));
    }

    #[test]
    fn chemin_pointe_descend_dans_les_donnees() {
        let data = json!({"zone": {"id": 10000, "nom": "Cuisine"}});
        assert!(filtre(json!({"zone.nom": "Cuisine"}), data.clone()));
        assert!(!filtre(json!({"zone.nom": "Salon"}), data.clone()));
        assert!(!filtre(json!({"zone.absent": "x"}), data));
    }

    #[test]
    fn un_evenement_custom_reveille_le_bon_declencheur() {
        // Bout en bout sur le tag de pré-filtrage : si les tags ne
        // correspondent pas, handle_event écarte l'automation avant même
        // d'appeler le matching, et la règle ne partirait jamais.
        let event = AutomationEvent::Custom {
            plugin_name: "robot".to_string(),
            trigger_type: "zone_changed".to_string(),
            data: json!({"zone": "cuisine"}),
            timestamp: OffsetDateTime::now_utc(),
        };
        let trigger = Trigger::Custom {
            plugin_name: "robot".to_string(),
            trigger_type: "zone_changed".to_string(),
            config: json!({"zone": "cuisine"}),
        };
        assert_eq!(event.trigger_type_tag(), trigger.event_type_tag());
        assert_eq!(event.event_type(), "custom");
    }

}
