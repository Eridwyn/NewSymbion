/**
 * SYMBION KERNEL - Action Executors
 *
 * ROLE: Execute automation actions with proper error handling
 *
 * ARCHITECTURE:
 * - ActionExecutor trait: Async execution interface
 * - SendNotificationExecutor: Sends notifications via plugin
 * - ForceModeExecutor: Changes context mode via ContextEngine
 * - AgentCommandExecutor: Sends commands to agents via MQTT
 * - DelayExecutor: Simple async delay
 * - CustomActionExecutor: Plugin-defined actions (Phase 5+)
 * - ActionExecutorRegistry: Central registry for all executors
 *
 * USAGE:
 * ```rust
 * let registry = ActionExecutorRegistry::new(ctx);
 * let result = registry.execute(&action).await;
 * ```
 */

use crate::agents::SharedAgentRegistry;
use crate::context::{ContextEngine, Mode};
use crate::notifications::{SharedNotificationManager, Notification, NotificationPriority};
use crate::plugin_proxy::PluginRegistry;
use crate::sensors::SensorRegistry;

use super::types::{ActionDefinition, ActionResult};
use super::AutomationEvent;

use async_trait::async_trait;
use serde_json::Value;
use std::sync::Arc;
use std::time::Instant;

/// Error type for action execution
#[derive(Debug, Clone)]
pub struct ActionError {
    pub message: String,
    pub recoverable: bool,
}

impl ActionError {
    pub fn new(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
            recoverable: false,
        }
    }

    pub fn recoverable(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
            recoverable: true,
        }
    }
}

impl std::fmt::Display for ActionError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.message)
    }
}

impl std::error::Error for ActionError {}

/// Execution context passed to all executors
/// Contains references to kernel components needed for action execution
pub struct ExecutorContext {
    pub context_engine: Arc<ContextEngine>,
    pub agents: SharedAgentRegistry,
    pub sensors: Arc<SensorRegistry>,
    pub notifications_manager: SharedNotificationManager,
    pub plugin_registry: Option<PluginRegistry>,
    pub event: AutomationEvent,
}

impl ExecutorContext {
    pub fn new(
        context_engine: Arc<ContextEngine>,
        agents: SharedAgentRegistry,
        sensors: Arc<SensorRegistry>,
        notifications_manager: SharedNotificationManager,
        event: AutomationEvent,
    ) -> Self {
        Self {
            context_engine,
            agents,
            sensors,
            notifications_manager,
            plugin_registry: None,
            event,
        }
    }

    /// Set plugin registry for custom actions
    pub fn with_plugin_registry(mut self, registry: PluginRegistry) -> Self {
        self.plugin_registry = Some(registry);
        self
    }
}

/// Trait for action executors
/// Each action type has its own executor implementing this trait
#[async_trait]
pub trait ActionExecutor: Send + Sync {
    /// Execute the action and return success/failure with optional error message
    async fn execute(&self, ctx: &ExecutorContext) -> Result<(), ActionError>;

    /// Get the action type name for logging/reporting
    fn action_type(&self) -> &'static str;

    /// Check if this executor can handle the given action definition
    fn can_handle(&self, action: &ActionDefinition) -> bool;
}

// =============================================================================
// SendNotificationExecutor
// =============================================================================

/// Executor for SendNotification actions
/// Sends notifications via the notification plugin (if available)
pub struct SendNotificationExecutor {
    pub priority: String,
    pub title: String,
    pub body: String,
}

impl SendNotificationExecutor {
    pub fn new(priority: String, title: String, body: String) -> Self {
        Self { priority, title, body }
    }

    pub fn from_action(action: &ActionDefinition) -> Option<Self> {
        if let ActionDefinition::SendNotification { priority, title, body, .. } = action {
            Some(Self::new(priority.clone(), title.clone(), body.clone()))
        } else {
            None
        }
    }
}

#[async_trait]
impl ActionExecutor for SendNotificationExecutor {
    async fn execute(&self, ctx: &ExecutorContext) -> Result<(), ActionError> {
        // Parse priority string to enum
        let priority = match self.priority.to_uppercase().as_str() {
            "P0" | "CRITICAL" => NotificationPriority::P0,
            "P1" | "IMPORTANT" => NotificationPriority::P1,
            _ => NotificationPriority::P2,
        };

        let notification = Notification {
            id: String::new(), // Will be assigned by manager
            priority,
            title: self.title.clone(),
            body: self.body.clone(),
            source: "automation".to_string(),
            timestamp: time::OffsetDateTime::now_utc(),
            acknowledged: false,
            acknowledged_at: None,
            actions: vec![],
            data: None,
        };

        match ctx.notifications_manager.send(notification).await {
            Ok(()) => {
                eprintln!("[executors] notification sent: {} ({})", self.title, self.priority);
                Ok(())
            }
            Err(e) => Err(ActionError::recoverable(format!("notification failed: {}", e))),
        }
    }

    fn action_type(&self) -> &'static str {
        "send_notification"
    }

    fn can_handle(&self, action: &ActionDefinition) -> bool {
        matches!(action, ActionDefinition::SendNotification { .. })
    }
}

// =============================================================================
// ForceModeExecutor
// =============================================================================

/// Executor for ForceMode actions
/// Changes the context mode via ContextEngine with optional duration
pub struct ForceModeExecutor {
    pub mode: String,
    pub duration_minutes: Option<i64>,
    pub reason: String,
}

impl ForceModeExecutor {
    pub fn new(mode: String, duration_minutes: Option<i64>, reason: String) -> Self {
        Self { mode, duration_minutes, reason }
    }

    pub fn from_action(action: &ActionDefinition) -> Option<Self> {
        if let ActionDefinition::ForceMode { mode, duration_minutes, reason, .. } = action {
            Some(Self::new(mode.clone(), *duration_minutes, reason.clone()))
        } else {
            None
        }
    }

    /// Parse mode string to Mode enum
    fn parse_mode(&self) -> Result<Mode, ActionError> {
        match self.mode.to_lowercase().as_str() {
            "cravate" | "work" | "professional" | "pro" | "focus" => Ok(Mode::Pro),
            "intime" | "home" | "domestic" | "maison" => Ok(Mode::Maison),
            "neutre" | "neutral" | "eco" | "veille" | "sleep" => Ok(Mode::Veille),
            _ => Err(ActionError::new(format!("unknown mode: {}", self.mode))),
        }
    }
}

#[async_trait]
impl ActionExecutor for ForceModeExecutor {
    async fn execute(&self, ctx: &ExecutorContext) -> Result<(), ActionError> {
        let target_mode = self.parse_mode()?;

        // Default to 60 minutes if not specified
        let duration = self.duration_minutes.unwrap_or(60);

        match ctx.context_engine.set_override(target_mode, duration, self.reason.clone()) {
            Some(_state) => {
                eprintln!(
                    "[executors] forced mode '{}' for {} minutes (reason: {})",
                    self.mode, duration, self.reason
                );
                Ok(())
            }
            None => Err(ActionError::new("failed to set mode override")),
        }
    }

    fn action_type(&self) -> &'static str {
        "force_mode"
    }

    fn can_handle(&self, action: &ActionDefinition) -> bool {
        matches!(action, ActionDefinition::ForceMode { .. })
    }
}

// =============================================================================
// AgentCommandExecutor
// =============================================================================

/// Executor for AgentCommand actions
/// Sends commands to agents via MQTT
pub struct AgentCommandExecutor {
    pub agent_id: String,
    pub command_type: String,
    pub parameters: Option<Value>,
}

impl AgentCommandExecutor {
    pub fn new(agent_id: String, command_type: String, parameters: Option<Value>) -> Self {
        Self { agent_id, command_type, parameters }
    }

    pub fn from_action(action: &ActionDefinition) -> Option<Self> {
        if let ActionDefinition::AgentCommand { agent_id, command_type, parameters, .. } = action {
            Some(Self::new(agent_id.clone(), command_type.clone(), parameters.clone()))
        } else {
            None
        }
    }
}

#[async_trait]
impl ActionExecutor for AgentCommandExecutor {
    async fn execute(&self, ctx: &ExecutorContext) -> Result<(), ActionError> {
        // Verify agent exists and is online (optional, can skip if we want fire-and-forget)
        if !ctx.agents.is_agent_online(&self.agent_id) {
            eprintln!(
                "[executors] warning: agent '{}' appears offline, sending command anyway",
                self.agent_id
            );
        }

        match ctx.agents.send_command(&self.agent_id, &self.command_type, self.parameters.clone()).await {
            Ok(command_id) => {
                eprintln!(
                    "[executors] command '{}' sent to agent '{}' (id: {})",
                    self.command_type, self.agent_id, command_id
                );
                Ok(())
            }
            Err(e) => Err(ActionError::recoverable(format!("agent command failed: {}", e))),
        }
    }

    fn action_type(&self) -> &'static str {
        "agent_command"
    }

    fn can_handle(&self, action: &ActionDefinition) -> bool {
        matches!(action, ActionDefinition::AgentCommand { .. })
    }
}

// =============================================================================
// DelayExecutor
// =============================================================================

/// Executor for Delay actions
/// Simple async sleep between actions
pub struct DelayExecutor {
    pub seconds: u32,
}

impl DelayExecutor {
    pub fn new(seconds: u32) -> Self {
        Self { seconds }
    }

    pub fn from_action(action: &ActionDefinition) -> Option<Self> {
        if let ActionDefinition::Delay { seconds } = action {
            Some(Self::new(*seconds))
        } else {
            None
        }
    }
}

#[async_trait]
impl ActionExecutor for DelayExecutor {
    async fn execute(&self, _ctx: &ExecutorContext) -> Result<(), ActionError> {
        eprintln!("[executors] waiting {} seconds", self.seconds);
        tokio::time::sleep(tokio::time::Duration::from_secs(self.seconds as u64)).await;
        Ok(())
    }

    fn action_type(&self) -> &'static str {
        "delay"
    }

    fn can_handle(&self, action: &ActionDefinition) -> bool {
        matches!(action, ActionDefinition::Delay { .. })
    }
}

// =============================================================================
// CustomActionExecutor
// =============================================================================

/// Executor for Custom plugin-defined actions
/// Forwards action execution to the appropriate plugin via HTTP/Unix socket
pub struct CustomActionExecutor {
    pub plugin_name: String,
    pub action_type_name: String,
    pub config: Value,
}

impl CustomActionExecutor {
    pub fn new(plugin_name: String, action_type_name: String, config: Value) -> Self {
        Self { plugin_name, action_type_name, config }
    }

    pub fn from_action(action: &ActionDefinition) -> Option<Self> {
        if let ActionDefinition::Custom { plugin_name, action_type, config, .. } = action {
            Some(Self::new(plugin_name.clone(), action_type.clone(), config.clone()))
        } else {
            None
        }
    }
}

#[async_trait]
impl ActionExecutor for CustomActionExecutor {
    async fn execute(&self, ctx: &ExecutorContext) -> Result<(), ActionError> {
        // Check if plugin registry is available
        let plugin_registry = match &ctx.plugin_registry {
            Some(registry) => registry,
            None => {
                return Err(ActionError::new(
                    "plugin registry not available for custom actions"
                ));
            }
        };

        // Check if the plugin is registered
        let plugins = plugin_registry.list_plugins().await;
        let plugin_exists = plugins.iter().any(|p| p.name == self.plugin_name);

        if !plugin_exists {
            return Err(ActionError::new(format!(
                "plugin '{}' not registered",
                self.plugin_name
            )));
        }

        // For now, log that custom actions will be implemented in Phase 5
        // In Phase 5, we would:
        // 1. Find the plugin's Unix socket
        // 2. Send an HTTP POST to /actions/{action_type} with config as body
        // 3. Wait for response and handle errors

        eprintln!(
            "[executors] custom action {}/{} not yet implemented (Phase 5)",
            self.plugin_name, self.action_type_name
        );

        // Return error since custom actions are not yet implemented
        Err(ActionError::new(format!(
            "custom action {}/{} not implemented (Phase 5)",
            self.plugin_name, self.action_type_name
        )))
    }

    fn action_type(&self) -> &'static str {
        "custom"
    }

    fn can_handle(&self, action: &ActionDefinition) -> bool {
        matches!(action, ActionDefinition::Custom { .. })
    }
}

// =============================================================================
// ActionExecutorRegistry
// =============================================================================

/// Central registry for executing actions
/// Provides a unified interface for executing any ActionDefinition
pub struct ActionExecutorRegistry;

impl ActionExecutorRegistry {
    /// Execute a single action and return the result
    pub async fn execute(
        action: &ActionDefinition,
        ctx: &ExecutorContext,
    ) -> ActionResult {
        let start = Instant::now();

        let (success, error) = match action {
            ActionDefinition::SendNotification { priority, title, body, .. } => {
                let executor = SendNotificationExecutor::new(
                    priority.clone(),
                    title.clone(),
                    body.clone(),
                );
                match executor.execute(ctx).await {
                    Ok(()) => (true, None),
                    Err(e) => (false, Some(e.message)),
                }
            }

            ActionDefinition::ForceMode { mode, duration_minutes, reason, .. } => {
                let executor = ForceModeExecutor::new(
                    mode.clone(),
                    *duration_minutes,
                    reason.clone(),
                );
                match executor.execute(ctx).await {
                    Ok(()) => (true, None),
                    Err(e) => (false, Some(e.message)),
                }
            }

            ActionDefinition::AgentCommand { agent_id, command_type, parameters, .. } => {
                let executor = AgentCommandExecutor::new(
                    agent_id.clone(),
                    command_type.clone(),
                    parameters.clone(),
                );
                match executor.execute(ctx).await {
                    Ok(()) => (true, None),
                    Err(e) => (false, Some(e.message)),
                }
            }

            ActionDefinition::Delay { seconds } => {
                let executor = DelayExecutor::new(*seconds);
                match executor.execute(ctx).await {
                    Ok(()) => (true, None),
                    Err(e) => (false, Some(e.message)),
                }
            }

            ActionDefinition::Custom { plugin_name, action_type, config, .. } => {
                let executor = CustomActionExecutor::new(
                    plugin_name.clone(),
                    action_type.clone(),
                    config.clone(),
                );
                match executor.execute(ctx).await {
                    Ok(()) => (true, None),
                    Err(e) => (false, Some(e.message)),
                }
            }

            ActionDefinition::SetFeature { .. } => {
                // SetFeature is handled by the engine via FeatureRegistry (engine.rs)
                // This executor path should not be reached
                (false, Some("set_feature must be executed via AutomationEngine".to_string()))
            }
        };

        let duration_ms = start.elapsed().as_millis() as u64;

        ActionResult {
            action_type: Self::action_type_name(action),
            success,
            error,
            duration_ms,
            // Legacy executor doesn't use DecisionEngine - these fields are set by AutomationEngine
            decision_id: None,
            trust_score: None,
            decision_outcome: None,
            blocked_reasons: None,
        }
    }

    /// Execute multiple actions sequentially and return all results
    pub async fn execute_all(
        actions: &[ActionDefinition],
        ctx: &ExecutorContext,
    ) -> Vec<ActionResult> {
        let mut results = Vec::with_capacity(actions.len());

        for action in actions {
            let result = Self::execute(action, ctx).await;

            // Log failures
            if !result.success {
                eprintln!(
                    "[executors] action {} failed: {:?}",
                    result.action_type,
                    result.error
                );
            }

            results.push(result);
        }

        results
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
            ActionDefinition::SetFeature { .. } => "set_feature".to_string(),
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
                        None => format!("Force mode '{}' for 60 minutes (default)", mode),
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
                ActionDefinition::SetFeature { feature_id, value, .. } => {
                    format!("Set feature '{}' = {}", feature_id, value)
                }
            })
            .collect()
    }
}

// =============================================================================
// Tests
// =============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_action_error_display() {
        let error = ActionError::new("test error");
        assert_eq!(error.to_string(), "test error");
        assert!(!error.recoverable);

        let recoverable = ActionError::recoverable("recoverable error");
        assert!(recoverable.recoverable);
    }

    #[test]
    fn test_force_mode_parse_mode() {
        let executor = ForceModeExecutor::new("pro".to_string(), None, "test".to_string());
        assert!(executor.parse_mode().is_ok());

        let executor = ForceModeExecutor::new("cravate".to_string(), None, "test".to_string());
        assert!(executor.parse_mode().is_ok()); // alias

        let executor = ForceModeExecutor::new("maison".to_string(), None, "test".to_string());
        assert!(executor.parse_mode().is_ok());

        let executor = ForceModeExecutor::new("intime".to_string(), None, "test".to_string());
        assert!(executor.parse_mode().is_ok()); // alias

        let executor = ForceModeExecutor::new("veille".to_string(), None, "test".to_string());
        assert!(executor.parse_mode().is_ok());

        let executor = ForceModeExecutor::new("neutre".to_string(), None, "test".to_string());
        assert!(executor.parse_mode().is_ok()); // alias

        let executor = ForceModeExecutor::new("invalid".to_string(), None, "test".to_string());
        assert!(executor.parse_mode().is_err());
    }

    #[test]
    fn test_action_type_name() {
        use crate::decision::ImpactLevel;

        let action = ActionDefinition::SendNotification {
            priority: "P1".to_string(),
            title: "Test".to_string(),
            body: "Body".to_string(),
            impact_level: ImpactLevel::Low,
        };
        assert_eq!(ActionExecutorRegistry::action_type_name(&action), "send_notification");

        let action = ActionDefinition::ForceMode {
            mode: "pro".to_string(),
            duration_minutes: Some(30),
            reason: "test".to_string(),
            impact_level: ImpactLevel::Medium,
            use_override: None,
        };
        assert_eq!(ActionExecutorRegistry::action_type_name(&action), "force_mode");

        let action = ActionDefinition::AgentCommand {
            agent_id: "agent1".to_string(),
            command_type: "shutdown".to_string(),
            parameters: None,
            impact_level: ImpactLevel::VeryHigh,
        };
        assert_eq!(ActionExecutorRegistry::action_type_name(&action), "agent_command");

        let action = ActionDefinition::Delay { seconds: 5 };
        assert_eq!(ActionExecutorRegistry::action_type_name(&action), "delay");

        let action = ActionDefinition::Custom {
            plugin_name: "lights".to_string(),
            action_type: "toggle".to_string(),
            config: serde_json::json!({}),
            impact_level: ImpactLevel::Medium,
        };
        assert_eq!(ActionExecutorRegistry::action_type_name(&action), "custom:lights/toggle");
    }

    #[test]
    fn test_preview_actions() {
        use crate::decision::ImpactLevel;

        let actions = vec![
            ActionDefinition::SendNotification {
                priority: "P1".to_string(),
                title: "Alert".to_string(),
                body: "Test".to_string(),
                impact_level: ImpactLevel::Low,
            },
            ActionDefinition::Delay { seconds: 5 },
            ActionDefinition::ForceMode {
                mode: "pro".to_string(),
                duration_minutes: Some(30),
                reason: "meeting".to_string(),
                impact_level: ImpactLevel::Medium,
                use_override: None,
            },
            ActionDefinition::ForceMode {
                mode: "maison".to_string(),
                duration_minutes: None,
                reason: "default".to_string(),
                impact_level: ImpactLevel::Medium,
                use_override: None,
            },
        ];

        let previews = ActionExecutorRegistry::preview_actions(&actions);
        assert_eq!(previews.len(), 4);
        assert!(previews[0].contains("Alert"));
        assert!(previews[1].contains("5 seconds"));
        assert!(previews[2].contains("30 minutes"));
        assert!(previews[3].contains("60 minutes (default)"));
    }

    #[test]
    fn test_executor_from_action() {
        use crate::decision::ImpactLevel;

        // SendNotification
        let action = ActionDefinition::SendNotification {
            priority: "P1".to_string(),
            title: "Test".to_string(),
            body: "Body".to_string(),
            impact_level: ImpactLevel::Low,
        };
        let executor = SendNotificationExecutor::from_action(&action);
        assert!(executor.is_some());
        let executor = executor.unwrap();
        assert_eq!(executor.priority, "P1");
        assert_eq!(executor.title, "Test");

        // ForceMode
        let action = ActionDefinition::ForceMode {
            mode: "pro".to_string(),
            duration_minutes: Some(30),
            reason: "test".to_string(),
            impact_level: ImpactLevel::Medium,
            use_override: None,
        };
        let executor = ForceModeExecutor::from_action(&action);
        assert!(executor.is_some());
        let executor = executor.unwrap();
        assert_eq!(executor.mode, "pro");
        assert_eq!(executor.duration_minutes, Some(30));

        // AgentCommand
        let action = ActionDefinition::AgentCommand {
            agent_id: "agent1".to_string(),
            command_type: "shutdown".to_string(),
            parameters: Some(serde_json::json!({"force": true})),
            impact_level: ImpactLevel::VeryHigh,
        };
        let executor = AgentCommandExecutor::from_action(&action);
        assert!(executor.is_some());
        let executor = executor.unwrap();
        assert_eq!(executor.agent_id, "agent1");
        assert_eq!(executor.command_type, "shutdown");

        // Delay
        let action = ActionDefinition::Delay { seconds: 10 };
        let executor = DelayExecutor::from_action(&action);
        assert!(executor.is_some());
        let executor = executor.unwrap();
        assert_eq!(executor.seconds, 10);

        // Custom
        let action = ActionDefinition::Custom {
            plugin_name: "lights".to_string(),
            action_type: "toggle".to_string(),
            config: serde_json::json!({"room": "salon"}),
            impact_level: ImpactLevel::Medium,
        };
        let executor = CustomActionExecutor::from_action(&action);
        assert!(executor.is_some());
        let executor = executor.unwrap();
        assert_eq!(executor.plugin_name, "lights");
        assert_eq!(executor.action_type_name, "toggle");
    }

    #[test]
    fn test_can_handle() {
        use crate::decision::ImpactLevel;

        let notification_executor = SendNotificationExecutor::new(
            "P1".to_string(),
            "Test".to_string(),
            "Body".to_string(),
        );

        let notification_action = ActionDefinition::SendNotification {
            priority: "P1".to_string(),
            title: "Test".to_string(),
            body: "Body".to_string(),
            impact_level: ImpactLevel::Low,
        };
        assert!(notification_executor.can_handle(&notification_action));

        let delay_action = ActionDefinition::Delay { seconds: 5 };
        assert!(!notification_executor.can_handle(&delay_action));
    }
}
