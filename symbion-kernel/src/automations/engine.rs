/**
 * SYMBION KERNEL - Automations Engine
 *
 * ROLE: Evaluate conditions and execute actions with Decision Engine integration
 *
 * ARCHITECTURE:
 * - ConditionEvaluator: Evaluates AND/OR condition groups
 * - ActionExecutor: Executes actions sequentially
 * - DecisionEngine Integration: All actions pass through trust evaluation
 * - Requires access to kernel components for context
 */

use crate::agents::SharedAgentRegistry;
use crate::context::{ContextEngine, Mode};
use crate::context_intelligence::{SharedContextIntelligence, DecisionSignal};
use crate::intelligence::{SharedFeatureRegistry, FeatureValue};
use crate::decision::{DecisionEngine, DecisionContext, DecisionOutcome, SharedTrustTracker, ValidationManager};
use crate::modes::SharedModeRegistry;
use crate::notifications::SharedNotificationManager;
use crate::sensors::SensorRegistry;

use super::decision_bridge::{action_to_decision, action_description};
use super::pending_actions::SharedPendingActionRegistry;
use super::types::*;
use super::AutomationEvent;
use std::collections::HashMap;
use std::sync::Arc;
use std::time::Instant;
use time::OffsetDateTime;
// timezone handled via crate::intelligence::local_now()

/// Context available during condition evaluation and action execution
pub struct ExecutionContext {
    pub context_engine: Arc<ContextEngine>,
    pub agents: SharedAgentRegistry,
    pub sensors: Arc<SensorRegistry>,
    pub notifications_manager: SharedNotificationManager,
    pub event: AutomationEvent,
    /// Decision Engine for trust evaluation (Phase 7)
    pub decision_engine: Option<Arc<DecisionEngine>>,
    /// Trust Tracker for evolving statistics (Phase 7)
    pub trust_tracker: Option<SharedTrustTracker>,
    /// Validation Manager for pending approvals (Phase 7)
    pub validation_manager: Option<Arc<ValidationManager>>,
    /// Pending Action Registry for post-approval execution
    pub pending_action_registry: Option<SharedPendingActionRegistry>,
    /// Context Intelligence for feedback loop (Decision → Intelligence)
    pub context_intelligence: Option<SharedContextIntelligence>,
    /// Mode Registry for validating dynamic modes (Invariant 2)
    pub mode_registry: Option<SharedModeRegistry>,
    /// Feature Registry for Intelligence v2 condition evaluation
    pub feature_registry: Option<SharedFeatureRegistry>,
}

/// Automation engine - evaluates conditions and executes actions
pub struct AutomationEngine;

impl AutomationEngine {
    /// Evaluate all conditions for an automation
    pub fn evaluate_conditions(
        conditions: &Option<ConditionGroup>,
        ctx: &ExecutionContext,
    ) -> (bool, Vec<ConditionEvaluation>) {
        match conditions {
            None => {
                // No conditions = always pass
                (true, vec![])
            }
            Some(group) => Self::evaluate_group(group, ctx),
        }
    }

    /// Evaluate a condition group (AND/OR)
    fn evaluate_group(
        group: &ConditionGroup,
        ctx: &ExecutionContext,
    ) -> (bool, Vec<ConditionEvaluation>) {
        let mut evaluations = Vec::new();

        match group.operator {
            LogicalOperator::And => {
                // ALL conditions must pass
                let mut all_passed = true;
                for condition in &group.conditions {
                    let (passed, details) = Self::evaluate_condition(condition, ctx);
                    evaluations.push(ConditionEvaluation {
                        condition_type: Self::condition_type_name(condition),
                        passed,
                        details,
                    });
                    if !passed {
                        all_passed = false;
                        // Continue evaluating for complete report
                    }
                }
                (all_passed, evaluations)
            }
            LogicalOperator::Or => {
                // ANY condition must pass
                let mut any_passed = false;
                for condition in &group.conditions {
                    let (passed, details) = Self::evaluate_condition(condition, ctx);
                    evaluations.push(ConditionEvaluation {
                        condition_type: Self::condition_type_name(condition),
                        passed,
                        details,
                    });
                    if passed {
                        any_passed = true;
                    }
                }
                (any_passed, evaluations)
            }
        }
    }

    /// Evaluate a single condition
    fn evaluate_condition(condition: &Condition, ctx: &ExecutionContext) -> (bool, String) {
        match condition {
            Condition::CurrentMode { mode, operator } => {
                let current = ctx.context_engine.current_mode_str();
                let matches = match operator {
                    ComparisonOperator::Equals => current.eq_ignore_ascii_case(mode),
                    ComparisonOperator::NotEquals => !current.eq_ignore_ascii_case(mode),
                    _ => false,
                };

                // Debug logging for condition evaluation
                eprintln!(
                    "[automations] DEBUG: CurrentMode condition - current='{}', target='{}', operator={:?}, matches={}",
                    current, mode, operator, matches
                );

                let op_str = match operator {
                    ComparisonOperator::Equals => "==",
                    ComparisonOperator::NotEquals => "!=",
                    _ => "??",
                };
                (
                    matches,
                    format!("current mode '{}' {} '{}' → {}", current, op_str, mode, matches),
                )
            }

            Condition::TimeRange { start_hour, end_hour } => {
                let now_local = crate::intelligence::local_now();
                let hour = now_local.hour();

                let in_range = if start_hour <= end_hour {
                    // Normal range (e.g., 8-22)
                    hour >= *start_hour && hour < *end_hour
                } else {
                    // Overnight range (e.g., 22-6)
                    hour >= *start_hour || hour < *end_hour
                };

                (
                    in_range,
                    format!("hour {} in range [{}-{}): {}", hour, start_hour, end_hour, in_range),
                )
            }

            Condition::DayOfWeek { days } => {
                let now_local = crate::intelligence::local_now();
                let weekday = now_local.weekday().number_days_from_sunday(); // 0=Sun, 6=Sat
                let matches = days.contains(&weekday);
                (
                    matches,
                    format!("day {} in {:?}: {}", weekday, days, matches),
                )
            }

            Condition::DayOfMonth { days } => {
                let now_local = crate::intelligence::local_now();
                let current_day = now_local.day();
                // Get last day of current month using Month::length
                let last_day = now_local.month().length(now_local.year());
                // Check if current day matches any in list
                // Special case: 31 = last day of month (whatever that is)
                let matches = days.iter().any(|&d| {
                    if d == 31 {
                        // 31 means "last day of month" regardless of actual day count
                        current_day == last_day
                    } else {
                        current_day == d
                    }
                });
                (
                    matches,
                    format!("day of month {} (last={}) in {:?}: {}", current_day, last_day, days, matches),
                )
            }

            Condition::Month { months } => {
                let now_local = crate::intelligence::local_now();
                let current_month = now_local.month() as u8; // 1-12
                let matches = months.contains(&current_month);
                (
                    matches,
                    format!("month {} in {:?}: {}", current_month, months, matches),
                )
            }

            Condition::SensorValue { room_id, metric, operator, value } => {
                // Get sensor data for room
                if let Some(env) = ctx.sensors.get_environment_by_room(room_id) {
                    // Access current reading (EnvReading struct)
                    let sensor_value = match metric {
                        SensorMetric::Temperature => env.current.temperature_c,
                        SensorMetric::Humidity => env.current.humidity_pct,
                        // Battery and signal not available in EnvReading
                        SensorMetric::Battery | SensorMetric::SignalStrength => None,
                    };

                    match sensor_value {
                        Some(sv) => {
                            let matches = match operator {
                                ComparisonOperator::Equals => (sv - value).abs() < 0.01,
                                ComparisonOperator::NotEquals => (sv - value).abs() >= 0.01,
                                ComparisonOperator::GreaterThan => sv > *value,
                                ComparisonOperator::LessThan => sv < *value,
                                ComparisonOperator::GreaterOrEqual => sv >= *value,
                                ComparisonOperator::LessOrEqual => sv <= *value,
                                ComparisonOperator::Contains => false, // Not applicable for numbers
                            };
                            (
                                matches,
                                format!("{} {:?} {} {:?} {}: {}", room_id, metric, sv, operator, value, matches),
                            )
                        }
                        None => (false, format!("{} {:?} not available", room_id, metric)),
                    }
                } else {
                    (false, format!("room '{}' has no sensor data", room_id))
                }
            }

            Condition::AgentOnline { agent_id } => {
                // Check agent registry - this is sync so we need to handle it carefully
                // For now, we'll use a simple approach
                let is_online = ctx.agents.is_agent_online(agent_id);
                (
                    is_online,
                    format!("agent '{}' online: {}", agent_id, is_online),
                )
            }

            Condition::Feature { feature_id, operator, value } => {
                // Check feature value from Intelligence v2 FeatureRegistry
                if let Some(ref registry) = ctx.feature_registry {
                    if let Some(sample) = registry.get(feature_id) {
                        // Compare feature value against expected value
                        let (matches, details) = Self::evaluate_feature_value(
                            &sample.value,
                            operator,
                            value,
                            feature_id,
                        );
                        (matches, details)
                    } else {
                        (false, format!("feature '{}' not found in registry", feature_id))
                    }
                } else {
                    (false, "feature_registry not available".to_string())
                }
            }

            Condition::Group(nested_group) => {
                let (passed, _nested_evals) = Self::evaluate_group(nested_group, ctx);
                (passed, format!("nested group: {}", passed))
            }

            Condition::Custom { plugin_name, condition_type, .. } => {
                // Custom conditions will be implemented with plugin support
                (false, format!("custom condition {}/{} not implemented", plugin_name, condition_type))
            }
        }
    }

    /// Execute actions sequentially with Decision Engine evaluation
    pub async fn execute_actions(
        automation: &Automation,
        ctx: &ExecutionContext,
    ) -> Vec<ActionResult> {
        let mut results = Vec::new();

        // Check if automation is trusted (bypasses Decision Engine)
        let is_trusted = automation.trusted.unwrap_or(false);

        // Build decision context if we have a decision engine AND automation is not trusted
        let decision_ctx = if is_trusted {
            None // Bypass Decision Engine for trusted automations
        } else if ctx.decision_engine.is_some() {
            Some(Self::build_decision_context(ctx).await)
        } else {
            None
        };

        if is_trusted {
            eprintln!("[automations] 🛡️  Automation '{}' is TRUSTED - bypassing Decision Engine", automation.name);
        }

        for action in &automation.actions {
            // Check if there's an active manual override - skip ForceMode if so
            // Manual overrides ALWAYS take priority over automated mode changes
            if let ActionDefinition::ForceMode { use_override, .. } = action {
                // Only check if this action uses natural mode (not override itself)
                if !use_override.unwrap_or(false) {
                    if let Some(state) = ctx.context_engine.get_state() {
                        if let Some(override_info) = state.manual_override {
                            if override_info.until > time::OffsetDateTime::now_utc() {
                                eprintln!(
                                    "[automations] 🔒 Skipping ForceMode '{}' - manual override active until {}",
                                    automation.name,
                                    override_info.until.format(&time::format_description::well_known::Rfc3339).unwrap_or_default()
                                );
                                results.push(ActionResult {
                                    action_type: Self::action_type_name(action),
                                    success: true,
                                    error: None,
                                    duration_ms: 0,
                                    decision_id: None,
                                    trust_score: Some(1.0),
                                    decision_outcome: Some("skipped_override_active".to_string()),
                                    blocked_reasons: Some(vec!["manual override active".to_string()]),
                                });
                                continue;
                            }
                        }
                    }
                }
            }

            // Check skip_if_same_mode for ForceMode actions
            if automation.skip_if_same_mode.unwrap_or(false) {
                if let ActionDefinition::ForceMode { mode, .. } = action {
                    let current_mode = ctx.context_engine.current_mode_str();
                    if current_mode.eq_ignore_ascii_case(mode) {
                        eprintln!(
                            "[automations] ⏭️  Skipping ForceMode '{}' - already in mode '{}' (skip_if_same_mode=true)",
                            automation.name, current_mode
                        );
                        results.push(ActionResult {
                            action_type: Self::action_type_name(action),
                            success: true,
                            error: None,
                            duration_ms: 0,
                            decision_id: None,
                            trust_score: Some(1.0),
                            decision_outcome: Some("skipped".to_string()),
                            blocked_reasons: None,
                        });
                        continue;
                    }
                }
            }
            let start = Instant::now();

            // If we have a DecisionEngine, evaluate trust before executing
            let (decision_id, trust_score, outcome, blocked_reasons) =
                if let (Some(engine), Some(ref dctx)) = (&ctx.decision_engine, &decision_ctx) {
                    Self::evaluate_with_decision(action, automation, engine, dctx)
                } else {
                    // No decision engine = always approved (backwards compatibility)
                    (None, None, Some("approved".to_string()), None)
                };

            // Check if action was blocked
            if let Some(ref outcome_str) = outcome {
                if outcome_str == "blocked" {
                    let duration_ms = start.elapsed().as_millis() as u64;
                    eprintln!(
                        "[automations] ❌ action {} BLOCKED by DecisionEngine for '{}': {:?}",
                        action_description(action),
                        automation.name,
                        blocked_reasons
                    );

                    // Record blocked action in trust tracker
                    if let Some(ref tracker) = ctx.trust_tracker {
                        tracker.record_blocked(action.type_name());
                    }

                    // Notify Intelligence: action blocked (context was invalid)
                    // Use automation's target mode (goal_mode → action → trigger → None)
                    // NO fallback to current_mode - if no explicit intent, no learning
                    if let Some(ref intelligence) = ctx.context_intelligence {
                        if let Some(signals) = intelligence.last_signals() {
                            let target_mode = automation.target_mode();
                            if target_mode.is_none() {
                                eprintln!(
                                    "[automations] No explicit intent for '{}', skipping Intelligence feedback",
                                    automation.name
                                );
                            }
                            // TODO: extract categories from DecisionOutcome::Blocked when available
                            intelligence.record_decision_outcome(
                                DecisionSignal::Blocked,
                                target_mode.as_deref(),
                                &signals,
                                None, // blocked_categories - would need to thread from evaluate_with_decision
                            );
                        }
                    }

                    results.push(ActionResult {
                        action_type: Self::action_type_name(action),
                        success: false,
                        error: Some("Blocked by Decision Engine".to_string()),
                        duration_ms,
                        decision_id,
                        trust_score,
                        decision_outcome: outcome,
                        blocked_reasons,
                    });
                    continue;
                }

                if outcome_str == "require_validation" {
                    let duration_ms = start.elapsed().as_millis() as u64;
                    let ts = trust_score.unwrap_or(0.0);
                    eprintln!(
                        "[automations] ⏳ action {} PENDING VALIDATION for '{}' (trust: {:.2})",
                        action_description(action),
                        automation.name,
                        ts
                    );

                    // Create validation request in ValidationManager
                    let mut validation_id: Option<String> = None;
                    if let (Some(ref vm), Some(ref dctx)) = (&ctx.validation_manager, &decision_ctx) {
                        let decision_action = action_to_decision(action, automation);
                        // Create a minimal DecisionResult for validation
                        let decision_result = crate::decision::DecisionResult {
                            decision_id: decision_id.clone().unwrap_or_else(|| uuid::Uuid::new_v4().to_string()),
                            outcome: DecisionOutcome::RequireValidation {
                                trust_score: ts,
                                threshold: 0.7, // Default high impact threshold
                                reasons: blocked_reasons.clone().unwrap_or_default(),
                                explanation_codes: vec!["TRUST.BELOW_THRESHOLD".to_string()],
                                human_reasons: vec![format!("Confiance {:.0}% insuffisante", ts * 100.0)],
                            },
                            trace_id: format!("auto_{}", automation.id),
                            warnings: vec![],
                        };

                        match vm.create_validation(&decision_result, &decision_action, dctx) {
                            Ok(vr) => {
                                validation_id = Some(vr.validation_id.clone());
                                eprintln!("[automations] Created validation request: {}", vr.validation_id);

                                // Register pending action for post-approval execution
                                if let Some(ref par) = ctx.pending_action_registry {
                                    let action_index = automation.actions.iter().position(|a| {
                                        std::mem::discriminant(a) == std::mem::discriminant(action)
                                    }).unwrap_or(0);

                                    par.register(
                                        vr.validation_id.clone(),
                                        automation.id.clone(),
                                        automation.name.clone(),
                                        action.clone(),
                                        action_index,
                                        ts, // trust_score from decision
                                        automation.target_mode(), // For Intelligence feedback
                                    );
                                }
                            }
                            Err(e) => {
                                eprintln!("[automations] Failed to create validation: {}", e);
                            }
                        }
                    }

                    // Send notification with validation ID
                    let notif_title = format!("🔐 Validation requise: {}", automation.name);
                    let notif_body = format!(
                        "L'action '{}' nécessite votre approbation.\n\nConfiance: {:.0}%\nRaison: {}\n\nValidation ID: {}",
                        action_description(action),
                        ts * 100.0,
                        blocked_reasons.as_ref().map(|r| r.join(", ")).unwrap_or_else(|| "Impact élevé".to_string()),
                        validation_id.as_ref().unwrap_or(&"N/A".to_string())
                    );

                    // Send validation request notification via notifications manager
                    let validation_notif = crate::notifications::Notification {
                        id: String::new(),
                        priority: crate::notifications::NotificationPriority::P1,
                        title: notif_title.clone(),
                        body: notif_body,
                        source: "automation-validation".to_string(),
                        timestamp: time::OffsetDateTime::now_utc(),
                        acknowledged: false,
                        acknowledged_at: None,
                        actions: vec![],
                        data: validation_id.as_ref().map(|id| serde_json::json!({"validation_id": id})),
                    };

                    if let Err(e) = ctx.notifications_manager.send(validation_notif).await {
                        eprintln!("[automations] Failed to send validation notification: {}", e);
                    }

                    results.push(ActionResult {
                        action_type: Self::action_type_name(action),
                        success: false,
                        error: Some(format!("Requires validation - ID: {}", validation_id.unwrap_or_else(|| "none".to_string()))),
                        duration_ms,
                        decision_id,
                        trust_score,
                        decision_outcome: outcome,
                        blocked_reasons,
                    });
                    continue;
                }
            }

            // Action approved - execute it
            if let Some(ts) = trust_score {
                eprintln!(
                    "[automations] ✅ {} APPROVED (trust: {:.2})",
                    action_description(action),
                    ts
                );

                // Notify Intelligence: action approved automatically (strong positive)
                // Use automation's target mode (goal_mode → action → trigger → None)
                // NO fallback to current_mode - if no explicit intent, no learning
                if let Some(ref intelligence) = ctx.context_intelligence {
                    if let Some(signals) = intelligence.last_signals() {
                        let target_mode = automation.target_mode();
                        if target_mode.is_none() {
                            eprintln!(
                                "[automations] No explicit intent for '{}', skipping Intelligence feedback",
                                automation.name
                            );
                        }
                        intelligence.record_decision_outcome(
                            DecisionSignal::ApprovedAuto,
                            target_mode.as_deref(),
                            &signals,
                            None, // blocked_categories not applicable for ApprovedAuto
                        );
                    }
                }
            }

            let (success, error) = Self::execute_action(action, ctx).await;
            let duration_ms = start.elapsed().as_millis() as u64;

            // Record result in trust tracker for evolving statistics
            if let Some(ref tracker) = ctx.trust_tracker {
                tracker.record_action(
                    action.type_name(),
                    action.agent_id(),
                    success,
                );
            }

            // Log failure before moving error into result
            if !success {
                eprintln!(
                    "[automations] action {} failed for '{}': {:?}",
                    Self::action_type_name(action),
                    automation.name,
                    error
                );
            }

            results.push(ActionResult {
                action_type: Self::action_type_name(action),
                success,
                error,
                duration_ms,
                decision_id,
                trust_score,
                decision_outcome: outcome,
                blocked_reasons: None,
            });
        }

        results
    }

    /// Evaluate action with Decision Engine
    fn evaluate_with_decision(
        action: &ActionDefinition,
        automation: &Automation,
        engine: &DecisionEngine,
        ctx: &DecisionContext,
    ) -> (Option<String>, Option<f32>, Option<String>, Option<Vec<String>>) {
        // Convert automation action to decision action
        let decision_action = action_to_decision(action, automation);

        // Get decision (catch panics from Decision Engine)
        let decide_result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            engine.decide(&decision_action, ctx)
        }));

        let result = match decide_result {
            Ok(r) => r,
            Err(_) => {
                eprintln!("[automations] Decision Engine panicked for '{}' — blocking action as safety fallback", automation.name);
                return (None, None, Some("blocked".to_string()), Some(vec!["decision_engine_panic".to_string()]));
            }
        };

        // Extract outcome info
        match &result.outcome {
            DecisionOutcome::Approved { trust_score, .. } => {
                (
                    Some(result.decision_id),
                    Some(*trust_score),
                    Some("approved".to_string()),
                    None,
                )
            }
            DecisionOutcome::Blocked { reasons, .. } => {
                (
                    Some(result.decision_id),
                    None,
                    Some("blocked".to_string()),
                    Some(reasons.clone()),
                )
            }
            DecisionOutcome::RequireValidation { trust_score, reasons, .. } => {
                (
                    Some(result.decision_id),
                    Some(*trust_score),
                    Some("require_validation".to_string()),
                    Some(reasons.clone()),
                )
            }
            DecisionOutcome::DryRun { trust_score, .. } => {
                // Dry-run shouldn't happen in normal execution
                (
                    Some(result.decision_id),
                    Some(*trust_score),
                    Some("dry_run".to_string()),
                    None,
                )
            }
        }
    }

    /// Build DecisionContext from ExecutionContext (async to access agent registry)
    async fn build_decision_context(ctx: &ExecutionContext) -> DecisionContext {
        let current_mode = ctx.context_engine.current_mode_str();
        let current_ssid = ctx.feature_registry
            .as_ref()
            .and_then(|fr| fr.get_string("net.ssid"))
            .unwrap_or_else(|| "unknown".to_string());

        // Build agent states from agent registry
        let agents_map = ctx.agents.list_agents().await;
        let agents = agents_map.into_iter().map(|(id, agent)| {
            let (cpu_usage, memory_usage) = agent.status.system.as_ref()
                .map(|s| (s.cpu.percent / 100.0, s.memory.percent_used / 100.0))
                .unwrap_or((0.0, 0.0));

            (id.clone(), crate::decision::AgentState {
                id,
                last_seen: agent.last_seen,
                metrics: crate::decision::AgentMetrics {
                    cpu_usage,
                    memory_usage_percent: memory_usage,
                },
                maintenance_mode: agent.status.status == "maintenance",
                last_reconnect: None,
            })
        }).collect();

        DecisionContext {
            mode: current_mode,
            ssid: current_ssid,
            agents,
        }
    }

    /// Execute a single action (impact_level already evaluated by DecisionEngine)
    async fn execute_action(action: &ActionDefinition, ctx: &ExecutionContext) -> (bool, Option<String>) {
        match action {
            ActionDefinition::SendNotification { priority, title, body, .. } => {
                // Parse priority string to enum
                let prio = match priority.to_uppercase().as_str() {
                    "P0" | "CRITICAL" => crate::notifications::NotificationPriority::P0,
                    "P1" | "IMPORTANT" => crate::notifications::NotificationPriority::P1,
                    _ => crate::notifications::NotificationPriority::P2,
                };

                let notification = crate::notifications::Notification {
                    id: String::new(), // Will be assigned by manager
                    priority: prio,
                    title: title.clone(),
                    body: body.clone(),
                    source: "automation".to_string(),
                    timestamp: time::OffsetDateTime::now_utc(),
                    acknowledged: false,
                    acknowledged_at: None,
                    actions: vec![],
                    data: None,
                };

                match ctx.notifications_manager.send(notification).await {
                    Ok(()) => {
                        eprintln!("[automations] ✉️  notification sent: {}", title);
                        (true, None)
                    }
                    Err(e) => (false, Some(format!("notification failed: {}", e))),
                }
            }

            ActionDefinition::ForceMode { mode, duration_minutes, reason, use_override, .. } => {
                // INVARIANT 2: Validate mode explicitly - no implicit behavior
                // Phase 1: Check core system modes (backward compatibility)
                let mode_lower = mode.to_lowercase();
                let core_mode = match mode_lower.as_str() {
                    "cravate" | "work" | "professional" | "pro" => Some((Mode::Cravate, "pro".to_string())),
                    "intime" | "home" | "domestic" | "maison" => Some((Mode::Intime, "maison".to_string())),
                    "neutre" | "neutral" | "eco" | "veille" => Some((Mode::Neutre, "veille".to_string())),
                    "focus" => Some((Mode::Cravate, "focus".to_string())), // Focus maps to Cravate theme
                    _ => None,
                };

                // Phase 2: If not a core mode, check mode_registry for dynamic modes
                let (target_mode, mode_slug, theme_override) = if let Some((mode_enum, slug)) = core_mode {
                    (mode_enum, slug, None)
                } else if let Some(ref registry) = ctx.mode_registry {
                    // Check if mode exists in dynamic registry
                    if let Some(dynamic_mode) = registry.get_by_slug(&mode_lower) {
                        // Dynamic mode found - infer base Mode enum from system mode mappings
                        // Custom modes default to Neutre unless they match a known pattern
                        let base_mode = if dynamic_mode.slug.contains("pro") || dynamic_mode.slug.contains("work") {
                            Mode::Cravate
                        } else if dynamic_mode.slug.contains("maison") || dynamic_mode.slug.contains("home") {
                            Mode::Intime
                        } else {
                            Mode::Neutre // Safe default for custom modes
                        };
                        let theme = crate::context::Theme {
                            primary: dynamic_mode.theme.primary.clone(),
                            bg: dynamic_mode.theme.background.clone(),
                            accent: dynamic_mode.theme.accent.clone(),
                        };
                        (base_mode, dynamic_mode.slug.clone(), Some(theme))
                    } else {
                        // GUARD: Mode not found in registry - explicit error, no fallback
                        eprintln!(
                            "[automations] ❌ GUARD: Mode '{}' not found in mode_registry (known modes: {:?})",
                            mode,
                            registry.list_all().iter().map(|m| &m.slug).collect::<Vec<_>>()
                        );
                        return (false, Some(format!(
                            "mode '{}' inconnu - modes valides: pro, maison, veille, focus ou modes personnalisés du registre",
                            mode
                        )));
                    }
                } else {
                    // No registry available - only accept core modes, explicit error
                    eprintln!(
                        "[automations] ❌ GUARD: Mode '{}' inconnu et mode_registry non disponible",
                        mode
                    );
                    return (false, Some(format!(
                        "mode '{}' inconnu - modes valides: pro, maison, veille, focus",
                        mode
                    )));
                };

                // Determine if we should use override (temporary) or natural (permanent until next change)
                let should_use_override = use_override.unwrap_or(false);

                if should_use_override {
                    // GUARD: Override with dynamic modes requires explicit duration
                    if theme_override.is_some() && duration_minutes.is_none() {
                        return (false, Some(format!(
                            "mode dynamique '{}' en override requiert une durée explicite",
                            mode_slug
                        )));
                    }

                    // Use override (temporary, with expiration)
                    let duration = duration_minutes.unwrap_or(60);
                    match ctx.context_engine.set_override(target_mode, duration, reason.clone()) {
                        Some(_state) => {
                            eprintln!(
                                "[automations] 🎯 forced mode '{}' for {} minutes (override - reason: {})",
                                mode_slug, duration, reason
                            );
                            (true, None)
                        }
                        None => (false, Some("failed to set mode override".to_string())),
                    }
                } else {
                    // Use natural mode change (no expiration, system can continue to evolve)
                    let theme = theme_override.unwrap_or_else(|| target_mode.theme());
                    match ctx.context_engine.set_mode_natural(mode_slug.clone(), theme, reason.clone()) {
                        Some(_state) => {
                            eprintln!(
                                "[automations] 🌿 set natural mode '{}' (reason: {})",
                                mode_slug, reason
                            );
                            (true, None)
                        }
                        None => (false, Some("failed to set natural mode".to_string())),
                    }
                }
            }

            ActionDefinition::AgentCommand { agent_id, command_type, parameters, .. } => {
                match ctx.agents.send_command(agent_id, command_type, parameters.clone()).await {
                    Ok(_) => {
                        eprintln!("[automations] 📤 command '{}' sent to agent '{}'", command_type, agent_id);
                        (true, None)
                    }
                    Err(e) => (false, Some(format!("agent command failed: {}", e))),
                }
            }

            ActionDefinition::Delay { seconds } => {
                eprintln!("[automations] ⏳ waiting {} seconds", seconds);
                tokio::time::sleep(tokio::time::Duration::from_secs(*seconds as u64)).await;
                (true, None)
            }

            ActionDefinition::Custom { plugin_name, action_type, .. } => {
                // Custom actions will be implemented with plugin support
                eprintln!(
                    "[automations] custom action {}/{} not implemented",
                    plugin_name, action_type
                );
                (false, Some(format!("custom action {}/{} not implemented", plugin_name, action_type)))
            }
        }
    }

    /// Evaluate a feature value against an expected value
    fn evaluate_feature_value(
        actual: &FeatureValue,
        operator: &ComparisonOperator,
        expected: &serde_json::Value,
        feature_id: &str,
    ) -> (bool, String) {
        match actual {
            FeatureValue::Bool(actual_bool) => {
                // Compare booleans
                let expected_bool = expected.as_bool().unwrap_or(false);
                let matches = match operator {
                    ComparisonOperator::Equals => *actual_bool == expected_bool,
                    ComparisonOperator::NotEquals => *actual_bool != expected_bool,
                    _ => false, // Other operators don't make sense for booleans
                };
                let op_str = match operator {
                    ComparisonOperator::Equals => "==",
                    ComparisonOperator::NotEquals => "!=",
                    _ => "??",
                };
                (
                    matches,
                    format!("feature '{}' {} {} {} → {}", feature_id, actual_bool, op_str, expected_bool, matches),
                )
            }
            FeatureValue::Float(actual_float) => {
                // Compare floats
                let expected_float = expected.as_f64().unwrap_or(0.0);
                let matches = match operator {
                    ComparisonOperator::Equals => (*actual_float - expected_float).abs() < 0.01,
                    ComparisonOperator::NotEquals => (*actual_float - expected_float).abs() >= 0.01,
                    ComparisonOperator::GreaterThan => *actual_float > expected_float,
                    ComparisonOperator::LessThan => *actual_float < expected_float,
                    ComparisonOperator::GreaterOrEqual => *actual_float >= expected_float,
                    ComparisonOperator::LessOrEqual => *actual_float <= expected_float,
                    ComparisonOperator::Contains => false,
                };
                let op_str = match operator {
                    ComparisonOperator::Equals => "==",
                    ComparisonOperator::NotEquals => "!=",
                    ComparisonOperator::GreaterThan => ">",
                    ComparisonOperator::LessThan => "<",
                    ComparisonOperator::GreaterOrEqual => ">=",
                    ComparisonOperator::LessOrEqual => "<=",
                    ComparisonOperator::Contains => "contains",
                };
                (
                    matches,
                    format!("feature '{}' {} {} {} → {}", feature_id, actual_float, op_str, expected_float, matches),
                )
            }
            FeatureValue::Int(actual_int) => {
                // Compare integers
                let expected_int = expected.as_i64().unwrap_or(0);
                let matches = match operator {
                    ComparisonOperator::Equals => *actual_int == expected_int,
                    ComparisonOperator::NotEquals => *actual_int != expected_int,
                    ComparisonOperator::GreaterThan => *actual_int > expected_int,
                    ComparisonOperator::LessThan => *actual_int < expected_int,
                    ComparisonOperator::GreaterOrEqual => *actual_int >= expected_int,
                    ComparisonOperator::LessOrEqual => *actual_int <= expected_int,
                    ComparisonOperator::Contains => false,
                };
                let op_str = match operator {
                    ComparisonOperator::Equals => "==",
                    ComparisonOperator::NotEquals => "!=",
                    ComparisonOperator::GreaterThan => ">",
                    ComparisonOperator::LessThan => "<",
                    ComparisonOperator::GreaterOrEqual => ">=",
                    ComparisonOperator::LessOrEqual => "<=",
                    ComparisonOperator::Contains => "contains",
                };
                (
                    matches,
                    format!("feature '{}' {} {} {} → {}", feature_id, actual_int, op_str, expected_int, matches),
                )
            }
            FeatureValue::String(actual_str) => {
                // Compare strings
                let expected_str = expected.as_str().unwrap_or("");
                let matches = match operator {
                    ComparisonOperator::Equals => actual_str.eq_ignore_ascii_case(expected_str),
                    ComparisonOperator::NotEquals => !actual_str.eq_ignore_ascii_case(expected_str),
                    ComparisonOperator::Contains => actual_str.to_lowercase().contains(&expected_str.to_lowercase()),
                    _ => false,
                };
                let op_str = match operator {
                    ComparisonOperator::Equals => "==",
                    ComparisonOperator::NotEquals => "!=",
                    ComparisonOperator::Contains => "contains",
                    _ => "??",
                };
                (
                    matches,
                    format!("feature '{}' '{}' {} '{}' → {}", feature_id, actual_str, op_str, expected_str, matches),
                )
            }
            FeatureValue::StringList(actual_list) => {
                // For string lists, check if expected value is in the list
                let expected_str = expected.as_str().unwrap_or("");
                let matches = match operator {
                    ComparisonOperator::Contains => actual_list.iter().any(|s| s.eq_ignore_ascii_case(expected_str)),
                    ComparisonOperator::Equals => actual_list.iter().any(|s| s.eq_ignore_ascii_case(expected_str)),
                    ComparisonOperator::NotEquals => !actual_list.iter().any(|s| s.eq_ignore_ascii_case(expected_str)),
                    _ => false,
                };
                let op_str = match operator {
                    ComparisonOperator::Contains | ComparisonOperator::Equals => "contains",
                    ComparisonOperator::NotEquals => "!contains",
                    _ => "??",
                };
                (
                    matches,
                    format!("feature '{}' list {} '{}' → {}", feature_id, op_str, expected_str, matches),
                )
            }
        }
    }

    /// Get condition type name for reporting
    fn condition_type_name(condition: &Condition) -> String {
        match condition {
            Condition::CurrentMode { .. } => "current_mode".to_string(),
            Condition::TimeRange { .. } => "time_range".to_string(),
            Condition::DayOfWeek { .. } => "day_of_week".to_string(),
            Condition::DayOfMonth { .. } => "day_of_month".to_string(),
            Condition::Month { .. } => "month".to_string(),
            Condition::SensorValue { .. } => "sensor_value".to_string(),
            Condition::AgentOnline { .. } => "agent_online".to_string(),
            Condition::Feature { .. } => "feature".to_string(),
            Condition::Group(_) => "group".to_string(),
            Condition::Custom { plugin_name, condition_type, .. } => {
                format!("custom:{}/{}", plugin_name, condition_type)
            }
        }
    }

    /// Get action type name for reporting
    fn action_type_name(action: &ActionDefinition) -> String {
        match action {
            ActionDefinition::SendNotification { .. } => "send_notification".to_string(),
            ActionDefinition::ForceMode { .. } => "force_mode".to_string(),
            ActionDefinition::AgentCommand { .. } => "agent_command".to_string(),
            ActionDefinition::Delay { .. } => "delay".to_string(),
            ActionDefinition::Custom { plugin_name, action_type, .. } => {
                format!("custom:{}/{}", plugin_name, action_type)
            }
        }
    }

    /// Generate action preview strings (for dry-run/test)
    pub fn preview_actions(actions: &[ActionDefinition]) -> Vec<String> {
        actions
            .iter()
            .map(|action| match action {
                ActionDefinition::SendNotification { priority, title, .. } => {
                    format!("Send {} notification: {}", priority, title)
                }
                ActionDefinition::ForceMode { mode, duration_minutes, .. } => {
                    match duration_minutes {
                        Some(mins) => format!("Force mode '{}' for {} minutes", mode, mins),
                        None => format!("Force mode '{}' indefinitely", mode),
                    }
                }
                ActionDefinition::AgentCommand { agent_id, command_type, .. } => {
                    format!("Send '{}' command to agent '{}'", command_type, agent_id)
                }
                ActionDefinition::Delay { seconds } => {
                    format!("Wait {} seconds", seconds)
                }
                ActionDefinition::Custom { plugin_name, action_type, .. } => {
                    format!("Custom action: {}/{}", plugin_name, action_type)
                }
            })
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // Tests would require mocking the context components
    // For now, just test the helper functions

    #[test]
    fn test_condition_type_name() {
        let cond = Condition::CurrentMode {
            mode: "intime".to_string(),
            operator: ComparisonOperator::Equals,
        };
        assert_eq!(AutomationEngine::condition_type_name(&cond), "current_mode");
    }

    #[test]
    fn test_action_type_name() {
        let action = ActionDefinition::SendNotification {
            priority: "P1".to_string(),
            title: "Test".to_string(),
            body: "Body".to_string(),
            impact_level: ImpactLevel::Low,
        };
        assert_eq!(AutomationEngine::action_type_name(&action), "send_notification");
    }

    #[test]
    fn test_preview_actions() {
        let actions = vec![
            ActionDefinition::SendNotification {
                priority: "P1".to_string(),
                title: "Alert".to_string(),
                body: "Test".to_string(),
                impact_level: ImpactLevel::Low,
            },
            ActionDefinition::Delay { seconds: 5 },
        ];

        let previews = AutomationEngine::preview_actions(&actions);
        assert_eq!(previews.len(), 2);
        assert!(previews[0].contains("Alert"));
        assert!(previews[1].contains("5 seconds"));
    }
}
