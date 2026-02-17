//! Session Manager v2
//!
//! Manages context mode sessions with hysteresis to prevent oscillations.
//!
//! ## Features
//!
//! - Hysteresis: Entry threshold > Exit threshold
//! - Minimum duration before mode switch
//! - Cooldown after manual changes
//! - Override expiry handling
//! - Stability tracking

use std::sync::Arc;
use parking_lot::RwLock;
use serde::{Deserialize, Serialize};
use time::{Duration, OffsetDateTime};

// ============================================================================
// Session Source
// ============================================================================

/// How the current session was established
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum SessionSource {
    /// Predicted by the intelligence engine
    Predicted,
    /// Manually set by user
    Manual,
    /// Temporary override (expires)
    Override,
    /// System default (startup/fallback)
    Default,
}

impl SessionSource {
    /// Whether this source blocks automatic transitions
    pub fn blocks_auto_transition(&self) -> bool {
        matches!(self, SessionSource::Manual | SessionSource::Override)
    }

    /// Get the cooldown duration for this source
    pub fn cooldown_minutes(&self) -> i64 {
        match self {
            SessionSource::Manual => 30,
            SessionSource::Override => 0, // Override has explicit expiry
            SessionSource::Predicted => 5,
            SessionSource::Default => 0,
        }
    }
}

// ============================================================================
// Active Session
// ============================================================================

/// Current active mode session with metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ActiveSession {
    /// Current mode (e.g., "focus", "maison", "pro")
    pub mode: String,

    /// When this session started
    #[serde(with = "time::serde::iso8601")]
    pub started_at: OffsetDateTime,

    /// Last time the mode was confirmed (prediction matched or user action)
    #[serde(with = "time::serde::iso8601")]
    pub last_confirmed_at: OffsetDateTime,

    /// How this session was established
    pub source: SessionSource,

    /// Session stability score (0.0-1.0)
    /// Increases over time, decreases on conflicting predictions
    pub stability_score: f32,

    /// Number of consecutive predictions matching this mode
    pub consecutive_matches: u32,

    /// Override expiry time (if source is Override)
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[serde(with = "option_iso8601")]
    pub override_expires_at: Option<OffsetDateTime>,
}

/// Custom serialization for Option<OffsetDateTime>
mod option_iso8601 {
    use serde::{Deserialize, Deserializer, Serialize, Serializer};
    use time::OffsetDateTime;

    pub fn serialize<S>(opt: &Option<OffsetDateTime>, s: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        match opt {
            Some(dt) => dt.serialize(s),
            None => s.serialize_none(),
        }
    }

    pub fn deserialize<'de, D>(d: D) -> Result<Option<OffsetDateTime>, D::Error>
    where
        D: Deserializer<'de>,
    {
        Option::<String>::deserialize(d)?
            .map(|s| OffsetDateTime::parse(&s, &time::format_description::well_known::Iso8601::DEFAULT))
            .transpose()
            .map_err(serde::de::Error::custom)
    }
}

impl ActiveSession {
    /// Create a new session
    pub fn new(mode: &str, source: SessionSource) -> Self {
        let now = now_paris();
        Self {
            mode: mode.to_string(),
            started_at: now,
            last_confirmed_at: now,
            source,
            stability_score: 0.3, // Start with modest stability
            consecutive_matches: 0,
            override_expires_at: None,
        }
    }

    /// Create an override session with expiry
    pub fn new_override(mode: &str, duration_minutes: i64) -> Self {
        let now = now_paris();
        Self {
            mode: mode.to_string(),
            started_at: now,
            last_confirmed_at: now,
            source: SessionSource::Override,
            stability_score: 1.0, // Override is fully stable
            consecutive_matches: 0,
            override_expires_at: Some(now + Duration::minutes(duration_minutes)),
        }
    }

    /// Duration of the current session (clamped to zero if clock skew)
    pub fn duration(&self) -> Duration {
        let d = now_paris() - self.started_at;
        if d.is_negative() { Duration::ZERO } else { d }
    }

    /// Duration in minutes
    pub fn duration_minutes(&self) -> i64 {
        self.duration().whole_minutes()
    }

    /// Check if the session is in cooldown period
    pub fn is_in_cooldown(&self) -> bool {
        let cooldown = Duration::minutes(self.source.cooldown_minutes());
        let elapsed = now_paris() - self.last_confirmed_at;
        if elapsed.is_negative() { return true; } // Clock skew: stay in cooldown
        elapsed < cooldown
    }

    /// Check if override has expired
    pub fn is_override_expired(&self) -> bool {
        match (self.source, self.override_expires_at) {
            (SessionSource::Override, Some(expiry)) => now_paris() > expiry,
            _ => false,
        }
    }

    /// Confirm the current mode (resets cooldown, increases stability)
    pub fn confirm(&mut self) {
        self.last_confirmed_at = now_paris();
        self.consecutive_matches += 1;
        // Increase stability (diminishing returns)
        self.stability_score = (self.stability_score + 0.1).min(1.0);
    }

    /// Record a conflicting prediction (decreases stability)
    pub fn record_conflict(&mut self) {
        self.consecutive_matches = 0;
        // Decrease stability
        self.stability_score = (self.stability_score - 0.05).max(0.0);
    }

    /// Get stability score with time-based decay.
    /// Decays toward 0.0 based on time since last confirmation.
    /// Half-life: 60 minutes (stability halves every hour without confirmation).
    pub fn decayed_stability(&self) -> f32 {
        let elapsed = now_paris() - self.last_confirmed_at;
        let minutes = if elapsed.is_negative() { 0.0 } else { elapsed.whole_minutes() as f32 };
        let decay = (-minutes * 0.693 / 60.0).exp(); // half-life 60 min
        self.stability_score * decay
    }
}

// ============================================================================
// Session Config
// ============================================================================

/// Configuration for session management
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionConfig {
    /// Minimum session duration before auto-switch (minutes)
    pub min_duration_minutes: i64,

    /// Confidence threshold to ENTER a new mode
    pub entry_threshold: f32,

    /// Confidence threshold to EXIT current mode (lower = more sticky)
    pub exit_threshold: f32,

    /// Minimum stability required to consider switching
    pub min_stability_for_switch: f32,

    /// Number of consecutive predictions required before switch
    pub required_consecutive: u32,

    /// Default mode when no prediction is confident
    pub default_mode: String,
}

impl Default for SessionConfig {
    fn default() -> Self {
        Self {
            min_duration_minutes: 5,
            entry_threshold: 0.50,
            exit_threshold: 0.35,
            min_stability_for_switch: 0.2,
            required_consecutive: 3,
            default_mode: "maison".to_string(),
        }
    }
}

// ============================================================================
// Transition Decision
// ============================================================================

/// Decision about whether to transition to a new mode
#[derive(Debug, Clone, Serialize)]
pub enum TransitionDecision {
    /// Stay in current mode
    Stay {
        reason: String,
    },
    /// Suggest a mode change (notification)
    Suggest {
        new_mode: String,
        confidence: f32,
        reason: String,
    },
    /// Auto-apply the mode change
    Apply {
        new_mode: String,
        confidence: f32,
        reason: String,
    },
    /// Override has expired, need to recalculate
    OverrideExpired {
        previous_mode: String,
    },
}

// ============================================================================
// Session Manager
// ============================================================================

/// Manages context mode sessions with hysteresis
#[derive(Debug)]
pub struct SessionManager {
    /// Current active session
    session: RwLock<ActiveSession>,

    /// Configuration
    config: RwLock<SessionConfig>,

    /// Pending prediction (waiting for consecutive matches)
    pending: RwLock<Option<PendingTransition>>,
}

#[derive(Debug, Clone)]
struct PendingTransition {
    mode: String,
    confidence: f32,
    consecutive_count: u32,
    first_seen: OffsetDateTime,
}

/// Thread-safe shared session manager
pub type SharedSessionManager = Arc<SessionManager>;

impl Default for SessionManager {
    fn default() -> Self {
        Self::new(SessionConfig::default())
    }
}

impl SessionManager {
    /// Create a new session manager with config
    pub fn new(config: SessionConfig) -> Self {
        let default_mode = config.default_mode.clone();
        Self {
            session: RwLock::new(ActiveSession::new(&default_mode, SessionSource::Default)),
            config: RwLock::new(config),
            pending: RwLock::new(None),
        }
    }

    /// Get current session (clone)
    pub fn current_session(&self) -> ActiveSession {
        self.session.read().clone()
    }

    /// Get current mode
    pub fn current_mode(&self) -> String {
        self.session.read().mode.clone()
    }

    /// Get config
    pub fn config(&self) -> SessionConfig {
        self.config.read().clone()
    }

    /// Update config
    pub fn update_config(&self, config: SessionConfig) {
        *self.config.write() = config;
    }

    /// Process a prediction and decide on transition
    pub fn process_prediction(&self, predicted_mode: &str, confidence: f32) -> TransitionDecision {
        let config = self.config.read().clone();
        let mut session = self.session.write();

        // Check override expiry first
        if session.is_override_expired() {
            let previous = session.mode.clone();
            // Reset to default, let next prediction decide
            *session = ActiveSession::new(&config.default_mode, SessionSource::Default);
            *self.pending.write() = None;
            return TransitionDecision::OverrideExpired { previous_mode: previous };
        }

        // If in manual/override, don't auto-switch
        if session.source.blocks_auto_transition() && !session.is_override_expired() {
            session.record_conflict(); // Still track conflicts
            return TransitionDecision::Stay {
                reason: format!(
                    "Session {} active (source: {:?}), auto-transition blocked",
                    session.mode, session.source
                ),
            };
        }

        // Same mode prediction - confirm and stay
        if predicted_mode == session.mode {
            session.confirm();
            *self.pending.write() = None; // Clear any pending transition
            return TransitionDecision::Stay {
                reason: format!(
                    "Prediction matches current mode '{}' (stability: {:.0}%)",
                    session.mode,
                    session.decayed_stability() * 100.0
                ),
            };
        }

        // Different mode - check hysteresis
        let session_duration = session.duration_minutes();

        // Too soon to switch?
        if session_duration < config.min_duration_minutes {
            session.record_conflict();
            return TransitionDecision::Stay {
                reason: format!(
                    "Session too short ({} min < {} min minimum)",
                    session_duration, config.min_duration_minutes
                ),
            };
        }

        // In cooldown?
        if session.is_in_cooldown() {
            session.record_conflict();
            return TransitionDecision::Stay {
                reason: format!(
                    "In cooldown period ({} min remaining)",
                    config.min_duration_minutes - session_duration
                ),
            };
        }

        // Confidence too low to exit current mode?
        if confidence < config.exit_threshold {
            session.record_conflict();
            return TransitionDecision::Stay {
                reason: format!(
                    "Confidence {:.0}% below exit threshold {:.0}%",
                    confidence * 100.0,
                    config.exit_threshold * 100.0
                ),
            };
        }

        // Update pending transition tracker
        let mut pending = self.pending.write();
        let consecutive = if let Some(ref mut p) = *pending {
            if p.mode == predicted_mode {
                p.consecutive_count += 1;
                p.confidence = confidence; // Update to latest
                p.consecutive_count
            } else {
                // Different mode, reset
                *pending = Some(PendingTransition {
                    mode: predicted_mode.to_string(),
                    confidence,
                    consecutive_count: 1,
                    first_seen: now_paris(),
                });
                1
            }
        } else {
            *pending = Some(PendingTransition {
                mode: predicted_mode.to_string(),
                confidence,
                consecutive_count: 1,
                first_seen: now_paris(),
            });
            1
        };

        // Not enough consecutive predictions?
        if consecutive < config.required_consecutive {
            session.record_conflict();
            return TransitionDecision::Stay {
                reason: format!(
                    "Waiting for consecutive predictions ({}/{} for '{}')",
                    consecutive, config.required_consecutive, predicted_mode
                ),
            };
        }

        // High confidence = auto-apply
        if confidence >= config.entry_threshold {
            return TransitionDecision::Apply {
                new_mode: predicted_mode.to_string(),
                confidence,
                reason: format!(
                    "Confidence {:.0}% >= entry threshold {:.0}%, {} consecutive predictions",
                    confidence * 100.0,
                    config.entry_threshold * 100.0,
                    consecutive
                ),
            };
        }

        // Medium confidence = suggest
        TransitionDecision::Suggest {
            new_mode: predicted_mode.to_string(),
            confidence,
            reason: format!(
                "Confidence {:.0}% between thresholds (exit={:.0}%, entry={:.0}%)",
                confidence * 100.0,
                config.exit_threshold * 100.0,
                config.entry_threshold * 100.0
            ),
        }
    }

    /// Apply a mode transition
    pub fn apply_transition(&self, new_mode: &str, source: SessionSource) {
        let mut session = self.session.write();
        *session = ActiveSession::new(new_mode, source);
        *self.pending.write() = None;

        eprintln!(
            "[sessions] Transition applied: {} → {} (source: {:?})",
            session.mode, new_mode, source
        );
    }

    /// Set manual override with optional expiry
    pub fn set_manual_override(&self, mode: &str, duration_minutes: Option<i64>) {
        let mut session = self.session.write();
        *session = if let Some(duration) = duration_minutes {
            ActiveSession::new_override(mode, duration)
        } else {
            ActiveSession::new(mode, SessionSource::Manual)
        };
        *self.pending.write() = None;

        eprintln!(
            "[sessions] Manual override set: {} (duration: {:?} min)",
            mode, duration_minutes
        );
    }

    /// Get session statistics
    pub fn stats(&self) -> SessionStats {
        let session = self.session.read();
        let pending = self.pending.read();

        SessionStats {
            current_mode: session.mode.clone(),
            source: session.source,
            duration_minutes: session.duration_minutes(),
            stability_score: session.decayed_stability(),
            consecutive_matches: session.consecutive_matches,
            is_in_cooldown: session.is_in_cooldown(),
            is_override_expired: session.is_override_expired(),
            pending_mode: pending.as_ref().map(|p| p.mode.clone()),
            pending_consecutive: pending.as_ref().map(|p| p.consecutive_count).unwrap_or(0),
        }
    }
}

/// Session statistics for API exposure
#[derive(Debug, Clone, Serialize)]
pub struct SessionStats {
    pub current_mode: String,
    pub source: SessionSource,
    pub duration_minutes: i64,
    pub stability_score: f32,
    pub consecutive_matches: u32,
    pub is_in_cooldown: bool,
    pub is_override_expired: bool,
    pub pending_mode: Option<String>,
    pub pending_consecutive: u32,
}

// ============================================================================
// Helpers
// ============================================================================

/// Get current time in configured local timezone
fn now_paris() -> OffsetDateTime {
    super::local_now()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_session_creation() {
        let session = ActiveSession::new("focus", SessionSource::Predicted);
        assert_eq!(session.mode, "focus");
        assert_eq!(session.source, SessionSource::Predicted);
        assert!(session.stability_score > 0.0);
    }

    #[test]
    fn test_override_creation() {
        let session = ActiveSession::new_override("veille", 60);
        assert_eq!(session.mode, "veille");
        assert_eq!(session.source, SessionSource::Override);
        assert!(session.override_expires_at.is_some());
    }

    #[test]
    fn test_session_manager_same_mode() {
        let manager = SessionManager::default();
        // Process prediction for same mode
        let decision = manager.process_prediction("maison", 0.8);
        assert!(matches!(decision, TransitionDecision::Stay { .. }));
    }

    #[test]
    fn test_hysteresis_consecutive() {
        let config = SessionConfig {
            min_duration_minutes: 0, // Disable for test
            required_consecutive: 2,
            ..Default::default()
        };
        let manager = SessionManager::new(config);

        // First prediction - should wait
        let decision = manager.process_prediction("focus", 0.8);
        assert!(matches!(decision, TransitionDecision::Stay { .. }));

        // Second prediction - should apply
        let decision = manager.process_prediction("focus", 0.8);
        assert!(matches!(decision, TransitionDecision::Apply { .. }));
    }
}
