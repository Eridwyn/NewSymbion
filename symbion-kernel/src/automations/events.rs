/**
 * SYMBION KERNEL - Automations Event System
 *
 * ROLE: Event dispatcher for automation triggers
 *
 * ARCHITECTURE:
 * - AutomationEvent enum: All possible trigger events
 * - EventDispatcher: Sends events via broadcast channel
 * - Event sources hook into this to dispatch events
 *
 * EVENT SOURCES:
 * - ContextEngine: Mode changes (cravate/intime/neutre)
 * - EnvironmentAlertMonitor: Sensor alert level changes
 * - AgentRegistry: Agent online/offline status changes
 */

use serde::{Deserialize, Serialize};
use tokio::sync::broadcast;
use time::OffsetDateTime;

/// All automation trigger events
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum AutomationEvent {
    /// Context mode changed
    ModeChange {
        from_mode: String,
        to_mode: String,
        reason: String,
        #[serde(with = "time::serde::iso8601")]
        timestamp: OffsetDateTime,
    },

    /// Sensor alert level changed
    SensorAlert {
        room_id: String,
        sensor_id: String,
        alert_level: String,  // "normal", "moderate", "high", "critical"
        previous_level: Option<String>,
        temperature: Option<f32>,
        humidity: Option<f32>,
        #[serde(with = "time::serde::iso8601")]
        timestamp: OffsetDateTime,
    },

    /// Agent status changed
    AgentStatus {
        agent_id: String,
        status: String,  // "online", "offline"
        previous_status: Option<String>,
        #[serde(with = "time::serde::iso8601")]
        timestamp: OffsetDateTime,
    },

    /// Manual trigger via API
    Manual {
        automation_id: String,
        triggered_by: Option<String>,  // username
        #[serde(with = "time::serde::iso8601")]
        timestamp: OffsetDateTime,
    },

    /// Plugin health status changed
    PluginHealth {
        plugin_name: String,
        status: String,  // "healthy", "unhealthy", "recovery_attempt", "recovery_failed", "recovery_success"
        previous_status: Option<String>,
        #[serde(with = "time::serde::iso8601")]
        timestamp: OffsetDateTime,
    },

    /// Scheduled/polling trigger fired
    Scheduled {
        automation_id: String,
        automation_name: String,
        #[serde(with = "time::serde::iso8601")]
        timestamp: OffsetDateTime,
    },
}

impl AutomationEvent {
    /// Get event type as string (for matching against triggers)
    pub fn event_type(&self) -> &'static str {
        match self {
            AutomationEvent::ModeChange { .. } => "mode_change",
            AutomationEvent::SensorAlert { .. } => "sensor_alert",
            AutomationEvent::AgentStatus { .. } => "agent_status",
            AutomationEvent::Manual { .. } => "manual",
            AutomationEvent::PluginHealth { .. } => "plugin_health",
            AutomationEvent::Scheduled { .. } => "scheduled",
        }
    }

    /// D7: Discriminant tag matching Trigger::event_type_tag() for fast pre-filtering
    pub fn trigger_type_tag(&self) -> &'static str {
        match self {
            AutomationEvent::ModeChange { .. } => "ModeChange",
            AutomationEvent::SensorAlert { .. } => "SensorAlert",
            AutomationEvent::AgentStatus { .. } => "AgentStatus",
            AutomationEvent::Manual { .. } => "Manual",
            AutomationEvent::PluginHealth { .. } => "PluginHealth",
            AutomationEvent::Scheduled { .. } => "Scheduled",
        }
    }

    /// Get timestamp of event
    pub fn timestamp(&self) -> OffsetDateTime {
        match self {
            AutomationEvent::ModeChange { timestamp, .. } => *timestamp,
            AutomationEvent::SensorAlert { timestamp, .. } => *timestamp,
            AutomationEvent::AgentStatus { timestamp, .. } => *timestamp,
            AutomationEvent::Manual { timestamp, .. } => *timestamp,
            AutomationEvent::PluginHealth { timestamp, .. } => *timestamp,
            AutomationEvent::Scheduled { timestamp, .. } => *timestamp,
        }
    }
}

/// Event dispatcher - sends events to automation engine
#[derive(Clone)]
pub struct EventDispatcher {
    sender: broadcast::Sender<AutomationEvent>,
}

impl EventDispatcher {
    /// Create new dispatcher with channel
    pub fn new() -> (Self, broadcast::Receiver<AutomationEvent>) {
        // Buffer 512 events - prevents silent event loss under high load
        let (sender, receiver) = broadcast::channel(512);
        (Self { sender }, receiver)
    }

    /// Create dispatcher from existing sender (for cloning to multiple sources)
    pub fn from_sender(sender: broadcast::Sender<AutomationEvent>) -> Self {
        Self { sender }
    }

    /// Get a new receiver for this dispatcher
    pub fn subscribe(&self) -> broadcast::Receiver<AutomationEvent> {
        self.sender.subscribe()
    }

    /// Dispatch mode change event
    pub fn dispatch_mode_change(&self, from_mode: &str, to_mode: &str, reason: &str) {
        let event = AutomationEvent::ModeChange {
            from_mode: from_mode.to_string(),
            to_mode: to_mode.to_string(),
            reason: reason.to_string(),
            timestamp: OffsetDateTime::now_utc(),
        };

        match self.sender.send(event) {
            Ok(n) => eprintln!("[automations] dispatched mode_change event ({} receivers)", n),
            Err(_) => eprintln!("[automations] no receivers for mode_change event"),
        }
    }

    /// Dispatch sensor alert event
    pub fn dispatch_sensor_alert(
        &self,
        room_id: &str,
        sensor_id: &str,
        alert_level: &str,
        previous_level: Option<&str>,
        temperature: Option<f32>,
        humidity: Option<f32>,
    ) {
        let event = AutomationEvent::SensorAlert {
            room_id: room_id.to_string(),
            sensor_id: sensor_id.to_string(),
            alert_level: alert_level.to_string(),
            previous_level: previous_level.map(|s| s.to_string()),
            temperature,
            humidity,
            timestamp: OffsetDateTime::now_utc(),
        };

        match self.sender.send(event) {
            Ok(n) => eprintln!("[automations] dispatched sensor_alert event for {} ({} receivers)", room_id, n),
            Err(_) => eprintln!("[automations] no receivers for sensor_alert event"),
        }
    }

    /// Dispatch agent status event
    pub fn dispatch_agent_status(
        &self,
        agent_id: &str,
        status: &str,
        previous_status: Option<&str>,
    ) {
        let event = AutomationEvent::AgentStatus {
            agent_id: agent_id.to_string(),
            status: status.to_string(),
            previous_status: previous_status.map(|s| s.to_string()),
            timestamp: OffsetDateTime::now_utc(),
        };

        match self.sender.send(event) {
            Ok(n) => eprintln!("[automations] dispatched agent_status event for {} ({} receivers)", agent_id, n),
            Err(_) => eprintln!("[automations] no receivers for agent_status event"),
        }
    }

    /// Dispatch manual trigger event
    pub fn dispatch_manual(&self, automation_id: &str, triggered_by: Option<&str>) {
        let event = AutomationEvent::Manual {
            automation_id: automation_id.to_string(),
            triggered_by: triggered_by.map(|s| s.to_string()),
            timestamp: OffsetDateTime::now_utc(),
        };

        match self.sender.send(event) {
            Ok(n) => eprintln!("[automations] dispatched manual trigger for {} ({} receivers)", automation_id, n),
            Err(_) => eprintln!("[automations] no receivers for manual trigger"),
        }
    }

    /// Dispatch plugin health event
    pub fn dispatch_plugin_health(
        &self,
        plugin_name: &str,
        status: &str,
        previous_status: Option<&str>,
    ) {
        let event = AutomationEvent::PluginHealth {
            plugin_name: plugin_name.to_string(),
            status: status.to_string(),
            previous_status: previous_status.map(|s| s.to_string()),
            timestamp: OffsetDateTime::now_utc(),
        };

        match self.sender.send(event) {
            Ok(n) => eprintln!("[automations] dispatched plugin_health event for {} ({} receivers)", plugin_name, n),
            Err(_) => eprintln!("[automations] no receivers for plugin_health event"),
        }
    }

    /// Dispatch scheduled trigger event
    pub fn dispatch_scheduled(&self, automation_id: &str, automation_name: &str) {
        let event = AutomationEvent::Scheduled {
            automation_id: automation_id.to_string(),
            automation_name: automation_name.to_string(),
            timestamp: OffsetDateTime::now_utc(),
        };

        match self.sender.send(event) {
            Ok(n) => eprintln!("[scheduler] dispatched scheduled event for {} ({} receivers)", automation_name, n),
            Err(_) => eprintln!("[scheduler] no receivers for scheduled event"),
        }
    }

    /// Get sender for sharing with other components
    pub fn get_sender(&self) -> broadcast::Sender<AutomationEvent> {
        self.sender.clone()
    }
}

impl Default for EventDispatcher {
    fn default() -> Self {
        let (dispatcher, _) = Self::new();
        dispatcher
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_mode_change_dispatch() {
        let (dispatcher, mut receiver) = EventDispatcher::new();

        dispatcher.dispatch_mode_change("veille", "pro", "test reason");

        let event = receiver.recv().await.unwrap();
        match event {
            AutomationEvent::ModeChange { from_mode, to_mode, reason, .. } => {
                assert_eq!(from_mode, "veille");
                assert_eq!(to_mode, "pro");
                assert_eq!(reason, "test reason");
            }
            _ => panic!("Wrong event type"),
        }
    }

    #[tokio::test]
    async fn test_sensor_alert_dispatch() {
        let (dispatcher, mut receiver) = EventDispatcher::new();

        dispatcher.dispatch_sensor_alert(
            "chambre",
            "esp32-001",
            "moderate",
            Some("normal"),
            Some(22.5),
            Some(75.0),
        );

        let event = receiver.recv().await.unwrap();
        match event {
            AutomationEvent::SensorAlert { room_id, alert_level, humidity, .. } => {
                assert_eq!(room_id, "chambre");
                assert_eq!(alert_level, "moderate");
                assert_eq!(humidity, Some(75.0));
            }
            _ => panic!("Wrong event type"),
        }
    }

    #[tokio::test]
    async fn test_multiple_receivers() {
        let (dispatcher, mut receiver1) = EventDispatcher::new();
        let mut receiver2 = dispatcher.subscribe();

        dispatcher.dispatch_agent_status("agent-001", "offline", Some("online"));

        // Both receivers should get the event
        let event1 = receiver1.recv().await.unwrap();
        let event2 = receiver2.recv().await.unwrap();

        assert_eq!(event1.event_type(), "agent_status");
        assert_eq!(event2.event_type(), "agent_status");
    }
}
