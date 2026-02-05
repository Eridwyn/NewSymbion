/**
 * SYMBION KERNEL - Automation Scheduler
 *
 * ROLE: Periodic polling for scheduled triggers
 *
 * ARCHITECTURE:
 * - Runs every 30 seconds
 * - Checks all automations with Scheduled triggers
 * - Dispatches events when interval has elapsed
 * - Respects active_hours configuration
 */

use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;
use tokio::time::{interval, Instant};
use chrono::{Timelike, Local};

use crate::automations::events::EventDispatcher;
use crate::automations::persistence::AutomationStore;
use crate::automations::types::{Trigger, TriggerItem};

/// Scheduler for polling-based automation triggers
pub struct AutomationScheduler {
    store: Arc<AutomationStore>,
    dispatcher: EventDispatcher,
    /// Tracks last execution time per automation ID
    last_runs: HashMap<String, Instant>,
}

impl AutomationScheduler {
    /// Create a new scheduler
    pub fn new(store: Arc<AutomationStore>, dispatcher: EventDispatcher) -> Self {
        Self {
            store,
            dispatcher,
            last_runs: HashMap::new(),
        }
    }

    /// Spawn the scheduler as a background task
    /// Checks every 30 seconds for scheduled automations
    pub fn spawn(mut self) {
        tokio::spawn(async move {
            let mut tick = interval(Duration::from_secs(30));
            println!("[scheduler] Automation scheduler started (30s check interval)");

            loop {
                tick.tick().await;
                self.check_scheduled_automations();
            }
        });
    }

    /// Check all automations for scheduled triggers that should fire
    fn check_scheduled_automations(&mut self) {
        let automations = self.store.list();

        let now = Instant::now();
        let current_hour = Local::now().hour() as u8;

        for auto in automations {
            // Skip disabled automations
            if !auto.enabled {
                continue;
            }

            // Check all triggers in the trigger group
            let trigger_group = auto.get_trigger_group();
            for trigger_item in &trigger_group.triggers {
                self.check_trigger_item(&auto.id, &auto.name, trigger_item, now, current_hour);
            }
        }
    }

    /// Recursively check a trigger item (handles nested groups)
    fn check_trigger_item(
        &mut self,
        automation_id: &str,
        automation_name: &str,
        item: &TriggerItem,
        now: Instant,
        current_hour: u8,
    ) {
        match item {
            TriggerItem::Single(trigger) => {
                if let Trigger::Scheduled { interval_seconds, active_hours } = trigger {
                    // Check active hours if configured
                    if let Some((start, end)) = active_hours {
                        if !Self::is_hour_in_range(current_hour, *start, *end) {
                            return; // Outside active hours
                        }
                    }

                    // Check if enough time has passed since last run
                    let should_run = match self.last_runs.get(automation_id) {
                        Some(last) => {
                            now.duration_since(*last).as_secs() >= (*interval_seconds).max(60) as u64
                        }
                        None => true, // First run after startup
                    };

                    if should_run {
                        println!("[scheduler] Firing scheduled trigger for: {} (interval: {}s)",
                            automation_name, interval_seconds);
                        self.dispatcher.dispatch_scheduled(automation_id, automation_name);
                        self.last_runs.insert(automation_id.to_string(), now);
                    }
                }
            }
            TriggerItem::Group(nested_group) => {
                // Recursively check nested groups
                for nested_item in &nested_group.triggers {
                    self.check_trigger_item(automation_id, automation_name, nested_item, now, current_hour);
                }
            }
        }
    }

    /// Check if current hour is within the specified range
    /// Handles overnight ranges (e.g., 22-6 means 10pm to 6am)
    fn is_hour_in_range(hour: u8, start: u8, end: u8) -> bool {
        if start <= end {
            // Normal range: e.g., 9-18 (9am to 6pm)
            hour >= start && hour < end
        } else {
            // Overnight range: e.g., 22-6 (10pm to 6am)
            hour >= start || hour < end
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_hour_in_range_normal() {
        // 9am-6pm range
        assert!(AutomationScheduler::is_hour_in_range(9, 9, 18));
        assert!(AutomationScheduler::is_hour_in_range(12, 9, 18));
        assert!(AutomationScheduler::is_hour_in_range(17, 9, 18));
        assert!(!AutomationScheduler::is_hour_in_range(8, 9, 18));
        assert!(!AutomationScheduler::is_hour_in_range(18, 9, 18));
        assert!(!AutomationScheduler::is_hour_in_range(22, 9, 18));
    }

    #[test]
    fn test_hour_in_range_overnight() {
        // 10pm-6am range (overnight)
        assert!(AutomationScheduler::is_hour_in_range(22, 22, 6));
        assert!(AutomationScheduler::is_hour_in_range(23, 22, 6));
        assert!(AutomationScheduler::is_hour_in_range(0, 22, 6));
        assert!(AutomationScheduler::is_hour_in_range(5, 22, 6));
        assert!(!AutomationScheduler::is_hour_in_range(6, 22, 6));
        assert!(!AutomationScheduler::is_hour_in_range(12, 22, 6));
        assert!(!AutomationScheduler::is_hour_in_range(21, 22, 6));
    }
}
