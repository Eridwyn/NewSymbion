/**
 * SYMBION KERNEL - Trust Score Tracker
 *
 * ROLE: Track action execution history to evolve trust scores over time
 *
 * ARCHITECTURE:
 * - Records success/failure for each action type and agent
 * - Calculates trust modifiers based on historical performance
 * - Persists statistics to JSON file
 * - Provides modifiers to TrustCalculator for dynamic scoring
 *
 * FORMULA:
 * - Success: modifier += 0.01 (capped at +0.2)
 * - Failure: modifier -= 0.05 (capped at -0.2)
 * - This creates a "trust but verify" system where repeated success
 *   slowly builds trust, but failures quickly reduce it
 */

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;
use std::sync::{Arc, RwLock};
use time::OffsetDateTime;

/// Statistics for a specific action type
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ActionTrustStats {
    pub action_type: String,
    pub total_executions: u64,
    pub successful: u64,
    pub failed: u64,
    pub blocked: u64,
    /// Trust modifier based on history (-0.2 to +0.2)
    pub current_trust_modifier: f32,
    #[serde(with = "time::serde::rfc3339")]
    pub last_updated: OffsetDateTime,
}

impl ActionTrustStats {
    pub fn new(action_type: &str) -> Self {
        Self {
            action_type: action_type.to_string(),
            total_executions: 0,
            successful: 0,
            failed: 0,
            blocked: 0,
            current_trust_modifier: 0.0,
            last_updated: OffsetDateTime::now_utc(),
        }
    }

    /// Success rate as percentage (0.0 - 1.0)
    pub fn success_rate(&self) -> f32 {
        if self.total_executions == 0 {
            return 1.0; // No history = neutral
        }
        self.successful as f32 / self.total_executions as f32
    }
}

/// Statistics for a specific agent
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentTrustStats {
    pub agent_id: String,
    pub total_commands: u64,
    pub successful: u64,
    pub failed: u64,
    pub current_trust_modifier: f32,
    #[serde(with = "time::serde::rfc3339")]
    pub last_updated: OffsetDateTime,
}

impl AgentTrustStats {
    pub fn new(agent_id: &str) -> Self {
        Self {
            agent_id: agent_id.to_string(),
            total_commands: 0,
            successful: 0,
            failed: 0,
            current_trust_modifier: 0.0,
            last_updated: OffsetDateTime::now_utc(),
        }
    }

    pub fn success_rate(&self) -> f32 {
        if self.total_commands == 0 {
            return 1.0;
        }
        self.successful as f32 / self.total_commands as f32
    }
}

/// Aggregated trust statistics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TrustStats {
    pub action_stats: HashMap<String, ActionTrustStats>,
    pub agent_stats: HashMap<String, AgentTrustStats>,
    #[serde(with = "time::serde::rfc3339")]
    pub last_updated: OffsetDateTime,
    /// Total decisions recorded
    pub total_decisions: u64,
}

impl Default for TrustStats {
    fn default() -> Self {
        Self {
            action_stats: HashMap::new(),
            agent_stats: HashMap::new(),
            last_updated: OffsetDateTime::now_utc(),
            total_decisions: 0,
        }
    }
}

/// Trust Tracker - manages trust score evolution
pub struct TrustTracker {
    stats: Arc<RwLock<TrustStats>>,
    data_file: PathBuf,
    /// Modifier increment on success
    success_increment: f32,
    /// Modifier decrement on failure (negative)
    failure_decrement: f32,
    /// Maximum positive modifier
    max_modifier: f32,
    /// Minimum negative modifier
    min_modifier: f32,
    /// Half-life in days for temporal decay of modifiers
    decay_half_life_days: f32,
    /// SQLite database (None = JSON-only fallback mode)
    db: Option<crate::database::SharedDatabase>,
}

impl TrustTracker {
    /// Create a new TrustTracker with configurable trust evolution parameters.
    /// Env overrides: SYMBION_TRUST_SUCCESS_INCREMENT, SYMBION_TRUST_FAILURE_DECREMENT,
    /// SYMBION_TRUST_MAX_MODIFIER, SYMBION_TRUST_DECAY_HALF_LIFE_DAYS
    pub fn new(data_dir: &str) -> Self {
        let data_file = PathBuf::from(data_dir).join("trust_stats.json");
        let stats = Self::load_or_default(&data_file);

        let success_increment = std::env::var("SYMBION_TRUST_SUCCESS_INCREMENT")
            .ok().and_then(|v| v.parse().ok()).unwrap_or(0.01);
        let failure_decrement = std::env::var("SYMBION_TRUST_FAILURE_DECREMENT")
            .ok().and_then(|v| v.parse().ok()).unwrap_or(0.05);
        let max_modifier = std::env::var("SYMBION_TRUST_MAX_MODIFIER")
            .ok().and_then(|v| v.parse().ok()).unwrap_or(0.2);
        let decay_half_life_days = std::env::var("SYMBION_TRUST_DECAY_HALF_LIFE_DAYS")
            .ok().and_then(|v| v.parse().ok()).unwrap_or(30.0);

        Self {
            stats: Arc::new(RwLock::new(stats)),
            data_file,
            success_increment,
            failure_decrement,
            max_modifier,
            min_modifier: -max_modifier,
            decay_half_life_days,
            db: None,
        }
    }

    /// Attach a database for SQLite persistence.
    pub fn with_database(mut self, db: crate::database::SharedDatabase) -> Self {
        let action_count = crate::database::trust_queries::count_action_stats(&db).unwrap_or(0);
        if action_count > 0 {
            // Load from DB
            let action_rows = crate::database::trust_queries::list_action_stats(&db).unwrap_or_default();
            let agent_rows = crate::database::trust_queries::list_agent_stats(&db).unwrap_or_default();

            let mut stats = self.stats.write().unwrap();
            stats.action_stats.clear();
            for row in action_rows {
                stats.action_stats.insert(row.action_type.clone(), ActionTrustStats {
                    action_type: row.action_type,
                    total_executions: row.total_executions as u64,
                    successful: row.successful as u64,
                    failed: row.failed as u64,
                    blocked: row.blocked as u64,
                    current_trust_modifier: row.current_trust_modifier as f32,
                    last_updated: OffsetDateTime::parse(&row.last_updated,
                        &time::format_description::well_known::Rfc3339).unwrap_or_else(|_| OffsetDateTime::now_utc()),
                });
            }
            stats.agent_stats.clear();
            for row in agent_rows {
                stats.agent_stats.insert(row.agent_id.clone(), AgentTrustStats {
                    agent_id: row.agent_id,
                    total_commands: row.total_commands as u64,
                    successful: row.successful as u64,
                    failed: row.failed as u64,
                    current_trust_modifier: row.current_trust_modifier as f32,
                    last_updated: OffsetDateTime::parse(&row.last_updated,
                        &time::format_description::well_known::Rfc3339).unwrap_or_else(|_| OffsetDateTime::now_utc()),
                });
            }

            // Load global counters
            if let Ok(Some(val)) = crate::database::trust_queries::get_trust_global(&db, "total_decisions") {
                stats.total_decisions = val.parse().unwrap_or(0);
            }
            if let Ok(Some(val)) = crate::database::trust_queries::get_trust_global(&db, "last_updated") {
                stats.last_updated = OffsetDateTime::parse(&val,
                    &time::format_description::well_known::Rfc3339).unwrap_or_else(|_| OffsetDateTime::now_utc());
            }

            eprintln!("[trust_tracker] Loaded {} action stats + {} agent stats from SQLite",
                stats.action_stats.len(), stats.agent_stats.len());
        } else {
            // Seed DB from in-memory data
            self.persist_to_db(&db);
        }
        self.db = Some(db);
        self
    }

    /// Persist all stats to SQLite
    fn persist_to_db(&self, db: &crate::database::SharedDatabase) {
        let stats = self.stats.read().unwrap();
        for s in stats.action_stats.values() {
            let row = crate::database::trust_queries::ActionStatsRow {
                action_type: s.action_type.clone(),
                total_executions: s.total_executions as i64,
                successful: s.successful as i64,
                failed: s.failed as i64,
                blocked: s.blocked as i64,
                current_trust_modifier: s.current_trust_modifier as f64,
                last_updated: s.last_updated.format(&time::format_description::well_known::Rfc3339)
                    .unwrap_or_default(),
            };
            let _ = crate::database::trust_queries::upsert_action_stats(db, &row);
        }
        for s in stats.agent_stats.values() {
            let row = crate::database::trust_queries::AgentStatsRow {
                agent_id: s.agent_id.clone(),
                total_commands: s.total_commands as i64,
                successful: s.successful as i64,
                failed: s.failed as i64,
                current_trust_modifier: s.current_trust_modifier as f64,
                last_updated: s.last_updated.format(&time::format_description::well_known::Rfc3339)
                    .unwrap_or_default(),
            };
            let _ = crate::database::trust_queries::upsert_agent_stats(db, &row);
        }
        let _ = crate::database::trust_queries::set_trust_global(db, "total_decisions", &stats.total_decisions.to_string());
        let _ = crate::database::trust_queries::set_trust_global(db, "last_updated",
            &stats.last_updated.format(&time::format_description::well_known::Rfc3339).unwrap_or_default());
    }

    /// Load stats from file or create default
    fn load_or_default(path: &PathBuf) -> TrustStats {
        if path.exists() {
            match fs::read_to_string(path) {
                Ok(content) => {
                    match serde_json::from_str(&content) {
                        Ok(stats) => {
                            eprintln!("[trust_tracker] loaded stats from {:?}", path);
                            return stats;
                        }
                        Err(e) => {
                            eprintln!("[trust_tracker] failed to parse stats: {}", e);
                        }
                    }
                }
                Err(e) => {
                    eprintln!("[trust_tracker] failed to read stats file: {}", e);
                }
            }
        }
        eprintln!("[trust_tracker] using default stats");
        TrustStats::default()
    }

    /// Record an action execution result
    pub fn record_action(&self, action_type: &str, agent_id: Option<&str>, success: bool) {
        let modifier_value: f32;

        {
            let mut stats = self.stats.write().unwrap();

            // Update action stats
            let action_stats = stats.action_stats
                .entry(action_type.to_string())
                .or_insert_with(|| ActionTrustStats::new(action_type));

            action_stats.total_executions += 1;
            if success {
                action_stats.successful += 1;
                action_stats.current_trust_modifier =
                    (action_stats.current_trust_modifier + self.success_increment).min(self.max_modifier);
            } else {
                action_stats.failed += 1;
                action_stats.current_trust_modifier =
                    (action_stats.current_trust_modifier - self.failure_decrement).max(self.min_modifier);
            }
            action_stats.last_updated = OffsetDateTime::now_utc();
            modifier_value = action_stats.current_trust_modifier;

            // Update agent stats if applicable
            if let Some(agent) = agent_id {
                let agent_stats = stats.agent_stats
                    .entry(agent.to_string())
                    .or_insert_with(|| AgentTrustStats::new(agent));

                agent_stats.total_commands += 1;
                if success {
                    agent_stats.successful += 1;
                    agent_stats.current_trust_modifier =
                        (agent_stats.current_trust_modifier + self.success_increment).min(self.max_modifier);
                } else {
                    agent_stats.failed += 1;
                    agent_stats.current_trust_modifier =
                        (agent_stats.current_trust_modifier - self.failure_decrement).max(self.min_modifier);
                }
                agent_stats.last_updated = OffsetDateTime::now_utc();
            }

            stats.total_decisions += 1;
            stats.last_updated = OffsetDateTime::now_utc();
        } // Release lock before logging and saving

        // Log update
        eprintln!(
            "[trust_tracker] recorded {} for action '{}'{}: modifier now {:.3}",
            if success { "success" } else { "failure" },
            action_type,
            agent_id.map(|a| format!(" (agent: {})", a)).unwrap_or_default(),
            modifier_value
        );

        // Persist
        self.save();
    }

    /// Record a blocked action
    pub fn record_blocked(&self, action_type: &str) {
        {
            let mut stats = self.stats.write().unwrap();

            let action_stats = stats.action_stats
                .entry(action_type.to_string())
                .or_insert_with(|| ActionTrustStats::new(action_type));

            action_stats.blocked += 1;
            action_stats.last_updated = OffsetDateTime::now_utc();

            stats.total_decisions += 1;
            stats.last_updated = OffsetDateTime::now_utc();
        }

        eprintln!("[trust_tracker] recorded blocked for action '{}'", action_type);
        self.save();
    }

    /// Apply temporal decay to a modifier based on time since last update.
    /// Formula: modifier × 2^(-age_days / half_life)
    /// After 30 days of inactivity, modifier is halved. After 60 days, quartered.
    fn decay_modifier(&self, modifier: f32, last_updated: OffsetDateTime) -> f32 {
        if modifier.abs() < 1e-6 {
            return 0.0;
        }
        let age_days = (OffsetDateTime::now_utc() - last_updated).whole_seconds() as f32 / 86400.0;
        if age_days <= 0.0 {
            return modifier;
        }
        modifier * (-age_days * 0.693 / self.decay_half_life_days).exp()
    }

    /// Get trust modifier for an action type (with temporal decay)
    pub fn get_action_modifier(&self, action_type: &str) -> f32 {
        let stats = self.stats.read().unwrap();
        stats.action_stats
            .get(action_type)
            .map(|s| self.decay_modifier(s.current_trust_modifier, s.last_updated))
            .unwrap_or(0.0)
    }

    /// Get trust modifier for an agent (with temporal decay)
    pub fn get_agent_modifier(&self, agent_id: &str) -> f32 {
        let stats = self.stats.read().unwrap();
        stats.agent_stats
            .get(agent_id)
            .map(|s| self.decay_modifier(s.current_trust_modifier, s.last_updated))
            .unwrap_or(0.0)
    }

    /// Get combined modifier for action + agent
    pub fn get_combined_modifier(&self, action_type: &str, agent_id: Option<&str>) -> f32 {
        let action_mod = self.get_action_modifier(action_type);
        let agent_mod = agent_id
            .map(|a| self.get_agent_modifier(a))
            .unwrap_or(0.0);

        // Average the modifiers, capped to max/min
        let combined = (action_mod + agent_mod) / 2.0;
        combined.clamp(self.min_modifier, self.max_modifier)
    }

    /// Get all statistics (for API/display)
    pub fn get_stats(&self) -> TrustStats {
        self.stats.read().unwrap().clone()
    }

    /// Get stats for a specific action type
    pub fn get_action_stats(&self, action_type: &str) -> Option<ActionTrustStats> {
        let stats = self.stats.read().unwrap();
        stats.action_stats.get(action_type).cloned()
    }

    /// Get stats for a specific agent
    pub fn get_agent_stats(&self, agent_id: &str) -> Option<AgentTrustStats> {
        let stats = self.stats.read().unwrap();
        stats.agent_stats.get(agent_id).cloned()
    }

    /// Reset modifier for an action type (admin function)
    pub fn reset_action_modifier(&self, action_type: &str) {
        let found = {
            let mut stats = self.stats.write().unwrap();
            if let Some(action_stats) = stats.action_stats.get_mut(action_type) {
                action_stats.current_trust_modifier = 0.0;
                action_stats.last_updated = OffsetDateTime::now_utc();
                true
            } else {
                false
            }
        };
        if found {
            eprintln!("[trust_tracker] reset modifier for action '{}'", action_type);
            self.save();
        }
    }

    /// Reset modifier for an agent (admin function)
    pub fn reset_agent_modifier(&self, agent_id: &str) {
        let found = {
            let mut stats = self.stats.write().unwrap();
            if let Some(agent_stats) = stats.agent_stats.get_mut(agent_id) {
                agent_stats.current_trust_modifier = 0.0;
                agent_stats.last_updated = OffsetDateTime::now_utc();
                true
            } else {
                false
            }
        };
        if found {
            eprintln!("[trust_tracker] reset modifier for agent '{}'", agent_id);
            self.save();
        }
    }

    /// Save stats to disk (DB-primary, JSON-fallback)
    fn save(&self) {
        // Try SQLite first
        if let Some(ref db) = self.db {
            self.persist_to_db(db);
            // Always write JSON as backup
            let _ = self.save_json();
            return;
        }
        let _ = self.save_json();
    }

    /// JSON-only save (fallback)
    fn save_json(&self) -> Result<(), String> {
        let stats = self.stats.read().unwrap();
        match serde_json::to_string_pretty(&*stats) {
            Ok(json) => {
                if let Err(e) = fs::write(&self.data_file, json) {
                    eprintln!("[trust_tracker] failed to save stats: {}", e);
                    return Err(e.to_string());
                }
            }
            Err(e) => {
                eprintln!("[trust_tracker] failed to serialize stats: {}", e);
                return Err(e.to_string());
            }
        }
        Ok(())
    }
}

/// Shared reference to TrustTracker
pub type SharedTrustTracker = Arc<TrustTracker>;

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    fn create_test_tracker() -> (TrustTracker, TempDir) {
        let temp_dir = TempDir::new().unwrap();
        let tracker = TrustTracker::new(temp_dir.path().to_str().unwrap());
        (tracker, temp_dir)
    }

    #[test]
    fn test_record_success_increases_modifier() {
        let (tracker, _dir) = create_test_tracker();

        tracker.record_action("send_notification", None, true);
        let modifier = tracker.get_action_modifier("send_notification");

        assert!(modifier > 0.0);
        assert!((modifier - 0.01).abs() < 0.001);
    }

    #[test]
    fn test_record_failure_decreases_modifier() {
        let (tracker, _dir) = create_test_tracker();

        tracker.record_action("agent_command", Some("pc-bureau"), false);
        let modifier = tracker.get_action_modifier("agent_command");

        assert!(modifier < 0.0);
        assert!((modifier - (-0.05)).abs() < 0.001);
    }

    #[test]
    fn test_modifier_caps() {
        let (tracker, _dir) = create_test_tracker();

        // Record many successes
        for _ in 0..50 {
            tracker.record_action("test_action", None, true);
        }

        let modifier = tracker.get_action_modifier("test_action");
        assert!(modifier <= 0.2);
        assert!((modifier - 0.2).abs() < 0.001);

        // Now record many failures
        for _ in 0..100 {
            tracker.record_action("test_action", None, false);
        }

        let modifier = tracker.get_action_modifier("test_action");
        assert!(modifier >= -0.2);
        assert!((modifier - (-0.2)).abs() < 0.001);
    }

    #[test]
    fn test_agent_stats() {
        let (tracker, _dir) = create_test_tracker();

        tracker.record_action("agent_command", Some("pc-salon"), true);
        tracker.record_action("agent_command", Some("pc-salon"), true);
        tracker.record_action("agent_command", Some("pc-salon"), false);

        let agent_stats = tracker.get_agent_stats("pc-salon").unwrap();
        assert_eq!(agent_stats.total_commands, 3);
        assert_eq!(agent_stats.successful, 2);
        assert_eq!(agent_stats.failed, 1);
    }

    #[test]
    fn test_combined_modifier() {
        let (tracker, _dir) = create_test_tracker();

        // Build up some history
        for _ in 0..10 {
            tracker.record_action("agent_command", Some("good-agent"), true);
        }
        for _ in 0..5 {
            tracker.record_action("agent_command", Some("bad-agent"), false);
        }

        let good_combined = tracker.get_combined_modifier("agent_command", Some("good-agent"));
        let bad_combined = tracker.get_combined_modifier("agent_command", Some("bad-agent"));

        assert!(good_combined > bad_combined);
    }

    #[test]
    fn test_stats_persistence() {
        let temp_dir = TempDir::new().unwrap();
        let path = temp_dir.path().to_str().unwrap();

        // Create tracker and record some data
        {
            let tracker = TrustTracker::new(path);
            tracker.record_action("test_action", None, true);
            tracker.record_action("test_action", None, true);
        }

        // Create new tracker from same path - should load data
        {
            let tracker = TrustTracker::new(path);
            let stats = tracker.get_action_stats("test_action").unwrap();
            assert_eq!(stats.successful, 2);
        }
    }

    #[test]
    fn test_modifier_decay() {
        let (tracker, _dir) = create_test_tracker();

        // Record failures to build negative modifier
        for _ in 0..4 {
            tracker.record_action("old_action", Some("old-agent"), false);
        }

        let raw_modifier = tracker.get_action_modifier("old_action");
        assert!(raw_modifier < -0.1, "Should have negative modifier: {}", raw_modifier);

        // Simulate age by directly modifying last_updated in stats
        {
            let mut stats = tracker.stats.write().unwrap();
            let action_stats = stats.action_stats.get_mut("old_action").unwrap();
            // Set last_updated to 60 days ago (2 half-lives)
            action_stats.last_updated = OffsetDateTime::now_utc() - time::Duration::days(60);

            let agent_stats = stats.agent_stats.get_mut("old-agent").unwrap();
            agent_stats.last_updated = OffsetDateTime::now_utc() - time::Duration::days(60);
        }

        // After 60 days (2 half-lives), modifier should be ~25% of original
        let decayed = tracker.get_action_modifier("old_action");
        let expected = raw_modifier * 0.25; // 2^(-60/30) = 0.25
        assert!(
            (decayed - expected).abs() < 0.01,
            "After 60 days, modifier {:.4} should be ~{:.4} (25% of {:.4})",
            decayed, expected, raw_modifier
        );

        // Agent modifier should also decay
        let agent_decayed = tracker.get_agent_modifier("old-agent");
        assert!(
            agent_decayed.abs() < raw_modifier.abs(),
            "Agent modifier should have decayed"
        );
    }

    #[test]
    fn test_blocked_recording() {
        let (tracker, _dir) = create_test_tracker();

        tracker.record_blocked("dangerous_action");

        let stats = tracker.get_action_stats("dangerous_action").unwrap();
        assert_eq!(stats.blocked, 1);
        assert_eq!(stats.total_executions, 0); // Blocked doesn't count as execution
    }
}
