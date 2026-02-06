/**
 * SYMBION KERNEL - Context Intelligence Engine
 *
 * ROLE: Intelligent autonomous context adaptation system
 *
 * FEATURES:
 * - Multi-signal collection (time, agent activity, environment)
 * - Pattern learning from user behavior
 * - Mode prediction with confidence scores
 * - Feedback loop for continuous improvement
 * - Auto-creation of automations from learned patterns
 * - Drift detection and adaptation
 */

use std::collections::{HashMap, VecDeque};
use std::sync::Arc;
use parking_lot::RwLock;
use serde::{Deserialize, Serialize};
use time::OffsetDateTime;

use crate::agents::SharedAgentRegistry;
use crate::context::{ContextEngine, Mode, Theme};
use crate::sensors::SensorRegistry;

/// Shared type alias for ContextIntelligence
pub type SharedContextIntelligence = Arc<ContextIntelligence>;

// ============================================================================
// Configuration
// ============================================================================

/// Configuration for the intelligence engine
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IntelligenceConfig {
    /// Threshold for auto-applying mode changes without validation (0.0-1.0)
    /// Default: 0.90 (90% confidence required)
    pub auto_apply_threshold: f32,

    /// Threshold for suggesting mode changes via notification (0.0-1.0)
    /// Default: 0.70 (70% confidence required)
    pub suggestion_threshold: f32,

    /// Minimum occurrences of a pattern before learning it
    /// Default: 3
    pub min_pattern_occurrences: u32,

    /// Weights for different signal sources
    pub weights: SignalWeights,

    /// Enable auto-creation of automations from learned patterns
    pub auto_create_automations: bool,

    /// Enable automatic adaptation when habits change
    pub auto_adapt: bool,

    /// Check interval in seconds for the intelligence monitor
    pub check_interval_seconds: u64,

    // ========== v1.1.9 Stabilization Parameters ==========

    /// Decay coefficients for pattern aging [<7d, <30d, <90d, >90d]
    /// Default: [1.0, 0.9, 0.7, 0.4] - softer decay for seasonal patterns
    pub decay_coefficients: [f32; 4],

    /// Days before a dead pattern is eligible for purge
    /// Default: 90 (acceptable), 120 for margin
    pub purge_threshold_days: u32,

    /// Maximum push notifications per day
    /// Default: 5
    pub max_push_per_day: u32,

    /// Cooldown in minutes between suggestions for same mode
    /// Default: 60
    pub suggestion_cooldown_minutes: u32,

    /// Quiet hours start (23 = 23:00, no push except 0.9+ established)
    pub quiet_hours_start: u8,

    /// Quiet hours end (7 = 07:00)
    pub quiet_hours_end: u8,
}

impl Default for IntelligenceConfig {
    fn default() -> Self {
        Self {
            auto_apply_threshold: 0.60,  // v1.1.10: lowered for more responsiveness
            suggestion_threshold: 0.30,  // v1.1.10: lowered to show more suggestions
            min_pattern_occurrences: 3,
            weights: SignalWeights::default(),
            auto_create_automations: true,
            auto_adapt: true,
            check_interval_seconds: 30,
            // v1.1.9 stabilization
            decay_coefficients: [1.0, 0.9, 0.7, 0.4],  // Softer than 1.0/0.8/0.5/0.2
            purge_threshold_days: 90,
            max_push_per_day: 5,
            suggestion_cooldown_minutes: 60,
            quiet_hours_start: 23,
            quiet_hours_end: 7,
        }
    }
}

/// Weights for different signal sources in prediction
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SignalWeights {
    /// Weight for temporal signals (hour + day of week)
    pub temporal: f32,
    /// Weight for behavioral patterns (manual changes history)
    pub behavioral: f32,
    /// Weight for agent activity (CPU, processes, idle time)
    pub agent_activity: f32,
    /// Weight for environmental factors (temperature, humidity)
    pub environmental: f32,
    /// Weight for momentum (time in current mode)
    pub momentum: f32,
}

impl Default for SignalWeights {
    fn default() -> Self {
        Self {
            temporal: 0.35,       // Time patterns are reliable
            behavioral: 0.35,     // Learned patterns matter
            agent_activity: 0.15, // Increased: active apps are strong signal
            environmental: 0.05,
            momentum: 0.10,
        }
    }
}

// ============================================================================
// Signal Collection
// ============================================================================

/// Snapshot of all contextual signals at a point in time
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContextSignals {
    // Temporal
    pub hour: u8,                    // 0-23
    pub day_of_week: u8,             // 0-6 (Mon-Sun) - Monday-based indexing
    pub is_weekend: bool,
    pub is_holiday: bool,            // Future: API for holidays

    // Agent activity
    pub agent_idle_seconds: u64,     // Seconds since last activity
    pub cpu_usage: f32,              // 0-100
    pub active_processes: Vec<String>, // Top running processes
    pub is_screen_locked: bool,      // Future: Agent capability

    // Environment
    pub temperature: Option<f32>,
    pub humidity: Option<f32>,

    // Current context
    pub current_mode: String,
    pub time_in_current_mode_minutes: i64,
    #[serde(with = "time::serde::iso8601::option")]
    pub last_manual_change: Option<OffsetDateTime>,
}

// ============================================================================
// Prediction
// ============================================================================

/// Result of a mode prediction
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModePrediction {
    pub mode: String,               // Mode slug (pro, maison, veille)
    pub confidence: f32,            // 0.0 - 1.0
    pub reasons: Vec<String>,       // Human-readable explanations
    pub contributing_factors: Vec<(String, f32)>, // (signal_name, weight)
}

/// Single prediction from one signal source
#[derive(Debug, Clone)]
pub struct SinglePrediction {
    pub mode: String,
    pub confidence: f32,
    pub reason: String,
}

// ============================================================================
// Learning
// ============================================================================

/// A pattern learned from user behavior
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LearnedPattern {
    pub mode: String,
    pub day_of_week: u8,
    pub hour: u8,
    pub confidence: f32,
    pub occurrences: u32,
    #[serde(with = "time::serde::iso8601")]
    pub last_seen: OffsetDateTime,
    pub source: PatternSource,
}

/// Where a pattern came from
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum PatternSource {
    /// Detected from history analysis
    Historical,
    /// Learned from user correction
    UserCorrection,
    /// Imported from existing automation
    Automation,
}

/// Record of a prediction for learning purposes
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PredictionRecord {
    #[serde(with = "time::serde::iso8601")]
    pub timestamp: OffsetDateTime,
    pub predicted_mode: String,
    pub actual_mode: Option<String>,  // Set when user corrects
    pub confidence: f32,
    pub was_correct: Option<bool>,
    /// Source of outcome (v1.1.9): auto_applied, suggestion, ignored
    #[serde(default)]
    pub outcome_source: Option<PredictionOutcome>,
}

/// How a prediction was handled (v1.1.9)
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum PredictionOutcome {
    /// Auto-applied (high confidence + established)
    AutoApplied,
    /// Suggestion sent (push or silent)
    Suggestion,
    /// Ignored (confidence too low)
    Ignored,
}

/// User feedback on a prediction
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserFeedback {
    #[serde(with = "time::serde::iso8601")]
    pub timestamp: OffsetDateTime,
    pub predicted_mode: String,
    pub actual_mode: String,       // What the user chose
    pub signals_snapshot: ContextSignals,
    pub was_correction: bool,      // true if different from prediction
}

/// Pattern with computed decay for export/debug (v1.1.9)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PatternExport {
    pub mode: String,
    pub day_of_week: u8,
    pub hour: u8,
    pub confidence: f32,
    pub decayed_confidence: f32,
    pub occurrences: u32,
    #[serde(with = "time::serde::iso8601")]
    pub last_seen: OffsetDateTime,
    pub source: PatternSource,
    pub days_since_seen: u32,
}

// ============================================================================
// Decision Engine Feedback
// ============================================================================

/// Signal type from Decision Engine outcomes
/// Used to provide feedback to Intelligence from automated decisions
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum DecisionSignal {
    /// Action approved automatically (high trust) → Strong positive reinforcement
    ApprovedAuto,
    /// Action approved after MFA validation → Weak positive (needed human confirmation)
    ApprovedMFA,
    /// Action denied by user (MFA refused) → Strong negative signal
    Denied,
    /// MFA validation expired (user didn't respond) → Ambiguous, no learning
    Expired,
    /// Action blocked by guards (context changed, expired, etc.) → Context was invalid
    Blocked,
}

// ============================================================================
// Drift Detection
// ============================================================================

/// Detected change in user habits
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HabitDrift {
    pub mode: String,
    pub day_of_week: u8,
    pub old_hour: u8,
    pub new_hour: u8,
    pub shift_hours: i8,
    pub suggestion: String,
}

// ============================================================================
// Status
// ============================================================================

/// Current status of the intelligence engine
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IntelligenceStatus {
    pub enabled: bool,
    pub config: IntelligenceConfig,
    pub learned_patterns_count: usize,
    pub auto_created_automations: usize,
    pub last_prediction: Option<ModePrediction>,
    pub accuracy_last_7_days: f32,
}

// ============================================================================
// Main Engine
// ============================================================================

/// The Context Intelligence Engine
pub struct ContextIntelligence {
    context_engine: Arc<ContextEngine>,
    agents: SharedAgentRegistry,
    sensors: Arc<SensorRegistry>,

    // Configuration
    pub config: RwLock<IntelligenceConfig>,

    // Learning state
    learned_patterns: RwLock<Vec<LearnedPattern>>,
    prediction_history: RwLock<VecDeque<PredictionRecord>>,
    feedback_history: RwLock<VecDeque<UserFeedback>>,

    // Last prediction (for status)
    last_prediction: RwLock<Option<ModePrediction>>,

    // Last collected signals (for Decision Engine feedback loop)
    last_signals_cache: RwLock<Option<ContextSignals>>,

    // Anti-spam for notifications (v1.1.9)
    // Per-mode: last suggestion time (60 min cooldown)
    suggestion_cooldowns: RwLock<std::collections::HashMap<String, OffsetDateTime>>,
    // Daily count: (date, count) - max 5/day
    daily_suggestion_count: RwLock<(time::Date, u32)>,

    // Health counters (v1.1.9) - reset daily
    // (date, push_sent, suggestions_generated, auto_applied, denied)
    health_counters: RwLock<HealthCounters>,
}

/// Health counters for observability (v1.1.9)
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct HealthCounters {
    pub date: Option<time::Date>,
    pub push_sent: u32,
    pub suggestions_generated: u32,
    pub auto_applied: u32,
    pub denied: u32,
    // Purge tracking (P0.5)
    #[serde(with = "time::serde::iso8601::option", default)]
    pub purge_last_run_at: Option<OffsetDateTime>,
    pub purge_removed_count_last_run: u32,
}

/// Detailed accuracy stats with denominators (v1.1.9 P0 fix)
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct AccuracyStats {
    /// Total predictions made in period
    pub predictions_total: u32,
    /// Predictions that received feedback (was_correct is Some)
    pub predictions_scored: u32,
    /// Predictions that were auto-applied
    pub predictions_auto_applied: u32,
    /// Predictions with user feedback (manual correction or approval)
    pub predictions_user_feedback: u32,
    /// Predictions ignored (confidence < suggestion_threshold)
    pub predictions_ignored: u32,
    /// Correct predictions out of scored
    pub correct_count: u32,
    /// Accuracy strict: correct / total (None if < 20 predictions = unreliable)
    pub accuracy_strict: Option<f32>,
    /// Accuracy feedback-only: correct / scored (None if < 20 predictions)
    pub accuracy_feedback: Option<f32>,
    /// Warning if sample size is too small
    pub warning: Option<String>,
    /// Minimum sample size for reliable accuracy (20)
    pub min_sample_size: u32,
}

impl ContextIntelligence {
    /// Create a new ContextIntelligence engine
    pub fn new(
        context_engine: Arc<ContextEngine>,
        agents: SharedAgentRegistry,
        sensors: Arc<SensorRegistry>,
    ) -> Self {
        let config = IntelligenceConfig::default();

        // Load learned patterns from disk if they exist
        let patterns_path = std::path::PathBuf::from("learned_patterns.json");
        let learned_patterns = if patterns_path.exists() {
            match std::fs::read_to_string(&patterns_path) {
                Ok(content) => {
                    match serde_json::from_str::<Vec<LearnedPattern>>(&content) {
                        Ok(p) => {
                            eprintln!("[intelligence] Loaded {} learned patterns from disk", p.len());
                            p
                        }
                        Err(e) => {
                            eprintln!("[intelligence] Failed to parse patterns file: {}", e);
                            Vec::new()
                        }
                    }
                }
                Err(e) => {
                    eprintln!("[intelligence] Failed to read patterns file: {}", e);
                    Vec::new()
                }
            }
        } else {
            eprintln!("[intelligence] No patterns file found, starting fresh");
            Vec::new()
        };

        Self {
            context_engine,
            agents,
            sensors,
            config: RwLock::new(config),
            learned_patterns: RwLock::new(learned_patterns),
            prediction_history: RwLock::new(VecDeque::with_capacity(1000)),
            feedback_history: RwLock::new(VecDeque::with_capacity(500)),
            last_prediction: RwLock::new(None),
            last_signals_cache: RwLock::new(None),
            // Anti-spam (v1.1.9)
            suggestion_cooldowns: RwLock::new(std::collections::HashMap::new()),
            daily_suggestion_count: RwLock::new((OffsetDateTime::now_utc().date(), 0)),
            // Health counters (v1.1.9)
            health_counters: RwLock::new(HealthCounters::default()),
        }
    }

    /// Get current configuration
    pub fn get_config(&self) -> IntelligenceConfig {
        self.config.read().clone()
    }

    /// Update configuration
    pub fn update_config(&self, new_config: IntelligenceConfig) {
        *self.config.write() = new_config;
        eprintln!("[intelligence] Configuration updated");
    }

    /// Get current status
    pub fn get_status(&self) -> IntelligenceStatus {
        let config = self.config.read().clone();
        let patterns_count = self.learned_patterns.read().len();
        let last_pred = self.last_prediction.read().clone();
        let accuracy = self.calculate_accuracy(7);

        IntelligenceStatus {
            enabled: true,
            config,
            learned_patterns_count: patterns_count,
            auto_created_automations: 0, // TODO: Track this
            last_prediction: last_pred,
            accuracy_last_7_days: accuracy,
        }
    }

    /// Get learned patterns
    pub fn get_patterns(&self) -> Vec<LearnedPattern> {
        self.learned_patterns.read().clone()
    }

    /// Get prediction history
    pub fn get_prediction_history(&self) -> Vec<PredictionRecord> {
        self.prediction_history.read().iter().cloned().collect()
    }

    /// Purge dead patterns (v1.1.9)
    /// Criteria: last_seen > purge_threshold_days AND decayed_confidence < 0.15 AND occurrences < 5
    /// Protections: UserCorrection with occurrences >= 3 are kept
    pub fn purge_dead_patterns(&self) -> usize {
        // Read config once before locking patterns
        let config = self.config.read();
        let decay_coeffs = config.decay_coefficients;
        let purge_days = config.purge_threshold_days as i64;
        drop(config);  // Release lock

        let mut patterns = self.learned_patterns.write();
        let now = OffsetDateTime::now_utc();
        let initial_count = patterns.len();

        patterns.retain(|p| {
            let days_since = (now - p.last_seen).whole_days();
            let decay = if days_since < 7 { decay_coeffs[0] }
                       else if days_since < 30 { decay_coeffs[1] }
                       else if days_since < 90 { decay_coeffs[2] }
                       else { decay_coeffs[3] };
            let decayed_conf = p.confidence * decay;

            // Purge criteria (uses config threshold)
            let is_dead = days_since > purge_days && decayed_conf < 0.15 && p.occurrences < 5;

            // Protection: UserCorrection with enough occurrences = intentional habit
            let is_protected = p.source == PatternSource::UserCorrection && p.occurrences >= 3;

            // Keep if not dead OR if protected
            !is_dead || is_protected
        });

        let purged = initial_count - patterns.len();
        drop(patterns);  // Release lock before updating counters

        // Record purge stats (P0.5)
        {
            let mut counters = self.health_counters.write();
            counters.purge_last_run_at = Some(now);
            counters.purge_removed_count_last_run = purged as u32;
        }

        if purged > 0 {
            eprintln!(
                "[intelligence] 🗑️ Purge: {} patterns supprimés (>{}j, <0.15 conf, <5 occ)",
                purged, purge_days
            );
        }
        purged
    }

    /// Save learned patterns to disk (with periodic purge)
    pub fn save_patterns(&self) {
        // Purge dead patterns before saving (keeps JSON clean)
        self.purge_dead_patterns();

        let patterns = self.learned_patterns.read();
        let path = std::path::PathBuf::from("learned_patterns.json");

        match serde_json::to_string_pretty(&*patterns) {
            Ok(json) => {
                if let Err(e) = std::fs::write(&path, json) {
                    eprintln!("[intelligence] Failed to save patterns: {}", e);
                }
            }
            Err(e) => {
                eprintln!("[intelligence] Failed to serialize patterns: {}", e);
            }
        }
    }

    /// Calculate prediction accuracy over the last N days
    pub fn calculate_accuracy(&self, days: i64) -> f32 {
        let history = self.prediction_history.read();
        let cutoff = OffsetDateTime::now_utc() - time::Duration::days(days);

        let recent: Vec<&PredictionRecord> = history
            .iter()
            .filter(|r| r.timestamp > cutoff && r.was_correct.is_some())
            .collect();

        if recent.is_empty() {
            return 0.0;
        }

        let correct = recent.iter().filter(|r| r.was_correct == Some(true)).count();
        (correct as f32 / recent.len() as f32) * 100.0
    }

    /// Calculate detailed accuracy stats with all denominators (v1.1.9 P0)
    pub fn calculate_accuracy_detailed(&self, days: i64) -> AccuracyStats {
        let history = self.prediction_history.read();
        let config = self.config.read();
        let cutoff = OffsetDateTime::now_utc() - time::Duration::days(days);

        let recent: Vec<&PredictionRecord> = history
            .iter()
            .filter(|r| r.timestamp > cutoff)
            .collect();

        let predictions_total = recent.len() as u32;

        // Count by outcome source
        let predictions_auto_applied = recent.iter()
            .filter(|r| r.outcome_source == Some(PredictionOutcome::AutoApplied))
            .count() as u32;

        let predictions_suggested = recent.iter()
            .filter(|r| r.outcome_source == Some(PredictionOutcome::Suggestion))
            .count() as u32;

        let predictions_ignored = recent.iter()
            .filter(|r| r.outcome_source == Some(PredictionOutcome::Ignored)
                || r.confidence < config.suggestion_threshold)
            .count() as u32;

        // Scored = has feedback
        let scored: Vec<&&PredictionRecord> = recent.iter()
            .filter(|r| r.was_correct.is_some())
            .collect();
        let predictions_scored = scored.len() as u32;

        // User feedback = was_correct set AND actual_mode differs from predicted
        let predictions_user_feedback = recent.iter()
            .filter(|r| r.was_correct.is_some() && r.actual_mode.is_some())
            .count() as u32;

        let correct_count = scored.iter()
            .filter(|r| r.was_correct == Some(true))
            .count() as u32;

        const MIN_SAMPLE_SIZE: u32 = 20;

        // Two accuracy metrics - None if sample too small (< 20 = unreliable)
        let (accuracy_feedback, accuracy_strict) = if predictions_total >= MIN_SAMPLE_SIZE {
            let feedback = if predictions_scored > 0 {
                Some((correct_count as f32 / predictions_scored as f32) * 100.0)
            } else {
                None
            };
            let strict = Some((correct_count as f32 / predictions_total as f32) * 100.0);
            (feedback, strict)
        } else {
            // Sample too small - don't report accuracy, it's misleading
            (None, None)
        };

        // Warning explains why accuracy is null or suspicious
        let warning = if predictions_total < MIN_SAMPLE_SIZE {
            Some(format!("Échantillon insuffisant ({}/{} min) - accuracy non fiable",
                predictions_total, MIN_SAMPLE_SIZE))
        } else if predictions_scored < 5 {
            Some(format!("Peu de feedback ({}/{} scorées)", predictions_scored, predictions_total))
        } else if accuracy_feedback.map(|a| a > 95.0).unwrap_or(false) && predictions_scored < 30 {
            Some("Accuracy élevée avec peu de données - méfiance".to_string())
        } else {
            None
        };

        AccuracyStats {
            predictions_total,
            predictions_scored,
            predictions_auto_applied,
            predictions_user_feedback,
            predictions_ignored,
            correct_count,
            accuracy_strict,
            accuracy_feedback,
            warning,
            min_sample_size: MIN_SAMPLE_SIZE,
        }
    }

    /// Initialize patterns from existing context history
    /// Only runs if no patterns were loaded from disk
    pub fn init_patterns_from_history(&self) {
        // Check if patterns were already loaded from disk
        {
            let patterns = self.learned_patterns.read();
            if !patterns.is_empty() {
                eprintln!("[intelligence] Patterns already loaded from disk ({} patterns), skipping history bootstrap", patterns.len());
                return;
            }
        }

        // Analyze context history directly
        let history = self.context_engine.get_history();
        let manual_changes: Vec<_> = history.iter().filter(|e| e.was_manual).collect();

        if manual_changes.len() < 2 {
            eprintln!("[intelligence] Not enough manual changes in history for pattern detection");
            return;
        }

        // Group by (mode, day_of_week, hour)
        use std::collections::HashMap;
        let mut pattern_map: HashMap<(String, u8, u8), u32> = HashMap::new();

        for entry in manual_changes {
            let mode_slug = match entry.mode {
                Mode::Cravate => "pro",
                Mode::Intime => "maison",
                Mode::Neutre => "veille",
            };
            let weekday = entry.timestamp.weekday().number_from_monday() as u8 - 1; // 0=Mon, 6=Sun
            let hour = entry.timestamp.hour();
            let key = (mode_slug.to_string(), weekday, hour);

            *pattern_map.entry(key).or_insert(0) += 1;
        }

        // Create learned patterns (min 2 occurrences)
        let mut patterns = self.learned_patterns.write();

        for ((mode, day, hour), count) in pattern_map {
            if count >= 2 {
                // More aggressive confidence: /5 instead of /10, max 0.85 for bootstrap patterns
                let confidence = (count as f32 / 5.0).min(0.85);
                patterns.push(LearnedPattern {
                    mode,
                    day_of_week: day,
                    hour,
                    confidence,
                    occurrences: count,
                    last_seen: OffsetDateTime::now_utc(),
                    source: PatternSource::Historical,
                });
            }
        }

        // Sort by confidence
        patterns.sort_by(|a, b| b.confidence.partial_cmp(&a.confidence).unwrap_or(std::cmp::Ordering::Equal));

        eprintln!("[intelligence] Bootstrapped {} patterns from history", patterns.len());
        drop(patterns);

        self.save_patterns();
    }

    // ========================================================================
    // Phase 3: Signal Collection
    // ========================================================================

    /// Collect all available contextual signals
    pub async fn collect_signals(&self) -> ContextSignals {
        let now = OffsetDateTime::now_utc();

        // Temporal signals
        let hour = now.hour();
        let day_of_week = now.weekday().number_from_monday() as u8 - 1; // 0=Mon, 6=Sun
        let is_weekend = day_of_week >= 5; // 5=Sat, 6=Sun

        // Agent activity (from primary agent's heartbeat)
        let (idle_seconds, cpu_usage, active_processes) = self.get_agent_metrics().await;

        // Environment (from sensors)
        let (temperature, humidity) = self.get_environment_readings().await;

        // Current context state
        let (current_mode, time_in_mode) = match self.context_engine.get_state() {
            Some(current) => {
                let mode = current.mode_slug.clone().unwrap_or_else(|| "veille".to_string());
                let duration = now - current.changed_at;
                (mode, duration.whole_minutes())
            }
            None => ("veille".to_string(), 0),
        };
        let last_manual = self.get_last_manual_change();

        let signals = ContextSignals {
            hour,
            day_of_week,
            is_weekend,
            is_holiday: false, // TODO: External holiday API
            agent_idle_seconds: idle_seconds,
            cpu_usage,
            active_processes,
            is_screen_locked: false, // TODO: Agent capability
            temperature,
            humidity,
            current_mode,
            time_in_current_mode_minutes: time_in_mode,
            last_manual_change: last_manual,
        };

        // Cache signals for Decision Engine feedback loop
        *self.last_signals_cache.write() = Some(signals.clone());

        signals
    }

    /// Get metrics from the primary user agent (Windows PC preferred, excludes server)
    async fn get_agent_metrics(&self) -> (u64, f32, Vec<String>) {
        let agents = self.agents.list_agents().await;

        // Find primary USER agent (excludes the kernel server):
        // 1. Windows agent that's online (user's primary workstation)
        // 2. Any online agent that's NOT the kernel server (hostname != "symbion")
        // Note: We explicitly do NOT fallback to the server - showing server CPU would be misleading
        let primary = agents.iter()
            .find(|(_, a)| a.os == "windows" && a.status.status == "online")
            .or_else(|| agents.iter().find(|(_, a)| {
                a.status.status == "online" && a.hostname != "symbion"
            }));

        if let Some((_, agent)) = primary {
            // Calculate idle from last heartbeat
            let idle = agent.status.last_heartbeat
                .map(|hb| {
                    let duration = OffsetDateTime::now_utc() - hb;
                    duration.whole_seconds().max(0) as u64
                })
                .unwrap_or(0);

            // CPU from latest metrics
            let cpu = agent.status.system.as_ref()
                .map(|s| s.cpu.percent)
                .unwrap_or(0.0);

            // Top processes if available (get process names from top_cpu)
            let processes = agent.status.processes.as_ref()
                .and_then(|p| p.top_cpu.as_ref())
                .map(|procs| procs.iter().map(|p| p.name.clone()).collect())
                .unwrap_or_default();

            (idle, cpu, processes)
        } else {
            (0, 0.0, Vec::new())
        }
    }

    /// Get environment readings from sensors
    async fn get_environment_readings(&self) -> (Option<f32>, Option<f32>) {
        // Try to get readings from any room with recent data
        let rooms = self.sensors.list_rooms();

        for room_id in rooms {
            if let Some(env) = self.sensors.get_environment_by_room(&room_id) {
                // Only use if recent (< 5 minutes old)
                let age_seconds = (chrono::Utc::now() - env.current.timestamp).num_seconds();
                if age_seconds < 300 {
                    return (env.current.temperature_c, env.current.humidity_pct);
                }
            }
        }

        (None, None)
    }

    /// Get timestamp of last manual mode change
    fn get_last_manual_change(&self) -> Option<OffsetDateTime> {
        let history = self.context_engine.get_history();
        history.iter()
            .filter(|e| e.was_manual)
            .map(|e| e.timestamp)
            .max() // Return most recent, not first
    }

    // ========================================================================
    // Phase 4: Prediction Engine
    // ========================================================================

    /// Predict optimal mode based on all signals
    pub fn predict_mode(&self, signals: &ContextSignals) -> ModePrediction {
        let config = self.config.read();
        let mut scores: HashMap<String, f32> = HashMap::new();
        let mut reasons: Vec<String> = Vec::new();
        let mut factors: Vec<(String, f32)> = Vec::new();

        // Initialize mode scores (focus = travail PC, pro = extérieur)
        for mode in ["pro", "focus", "maison", "veille"] {
            scores.insert(mode.to_string(), 0.0);
        }

        // 1. TEMPORAL SIGNAL (35%)
        let temporal = self.predict_from_temporal(signals);
        self.add_weighted_score(&mut scores, &temporal, config.weights.temporal);
        if temporal.confidence > 0.2 {  // Lowered from 0.5 to show more factors
            reasons.push(temporal.reason.clone());
            factors.push(("temporal".into(), temporal.confidence));
        }

        // 2. BEHAVIORAL PATTERNS (25%)
        let behavioral = self.predict_from_patterns(signals);
        self.add_weighted_score(&mut scores, &behavioral, config.weights.behavioral);
        if behavioral.confidence > 0.2 {  // Lowered from 0.5
            reasons.push(behavioral.reason.clone());
            factors.push(("behavioral".into(), behavioral.confidence));
        }

        // 3. AGENT ACTIVITY (20%)
        let activity = self.predict_from_agent_activity(signals);
        self.add_weighted_score(&mut scores, &activity, config.weights.agent_activity);
        if activity.confidence > 0.2 {  // Lowered from 0.5
            reasons.push(activity.reason.clone());
            factors.push(("agent_activity".into(), activity.confidence));
        }

        // 4. ENVIRONMENT (10%)
        let environment = self.predict_from_environment(signals);
        self.add_weighted_score(&mut scores, &environment, config.weights.environmental);
        if environment.confidence > 0.1 {  // Lowered from 0.3
            factors.push(("environment".into(), environment.confidence));
        }

        // 5. MOMENTUM (10%) - Prefer staying in current mode
        let momentum = self.predict_from_momentum(signals);
        self.add_weighted_score(&mut scores, &momentum, config.weights.momentum);

        drop(config);

        // Select mode with highest score
        let (best_mode, best_score) = scores
            .iter()
            .max_by(|a, b| a.1.partial_cmp(b.1).unwrap_or(std::cmp::Ordering::Equal))
            .map(|(m, s)| (m.clone(), *s))
            .unwrap_or(("veille".into(), 0.0));

        let prediction = ModePrediction {
            mode: best_mode,
            confidence: best_score.min(1.0),
            reasons,
            contributing_factors: factors,
        };

        // Store for status
        *self.last_prediction.write() = Some(prediction.clone());

        prediction
    }

    /// Add weighted score from a single prediction
    fn add_weighted_score(
        &self,
        scores: &mut HashMap<String, f32>,
        prediction: &SinglePrediction,
        weight: f32,
    ) {
        if !prediction.mode.is_empty() && prediction.confidence > 0.0 {
            if let Some(score) = scores.get_mut(&prediction.mode) {
                *score += prediction.confidence * weight;
            }
        }
    }

    // ========================================================================
    // Individual Predictors
    // ========================================================================

    /// Predict based on time patterns (hour + day of week)
    fn predict_from_temporal(&self, signals: &ContextSignals) -> SinglePrediction {
        // Check learned patterns first
        let patterns = self.learned_patterns.read();

        // Find matching pattern (same day, hour ±1)
        // UNIFIED (v1.1.9): Sort by confidence only - occurrences kept for diagnostics
        let matching = patterns.iter()
            .filter(|p| {
                p.day_of_week == signals.day_of_week &&
                (p.hour as i8 - signals.hour as i8).abs() <= 1
            })
            .max_by(|a, b| {
                a.confidence.partial_cmp(&b.confidence)
                    .unwrap_or(std::cmp::Ordering::Equal)
            });

        if let Some(pattern) = matching {
            // Apply temporal decay: older patterns have less influence
            let days_since = (OffsetDateTime::now_utc() - pattern.last_seen).whole_days();
            let decay = self.calculate_decay(days_since);
            let effective_confidence = pattern.confidence * decay;

            return SinglePrediction {
                mode: pattern.mode.clone(),
                confidence: effective_confidence,
                reason: format!(
                    "Pattern détecté: {} à {}h le {} ({} occ{})",
                    mode_display_name(&pattern.mode),
                    pattern.hour,
                    day_name(pattern.day_of_week),
                    pattern.occurrences,
                    if days_since > 7 { format!(", decay {:.0}%", decay * 100.0) } else { String::new() }
                ),
            };
        }

        drop(patterns);

        // Fallback to default temporal rules
        let (mode, confidence, reason) = match (signals.is_weekend, signals.hour) {
            (_, 23..=23 | 0..=6) => ("veille", 0.6, "Heures de nuit"),
            (true, _) => ("maison", 0.5, "Week-end"),
            (false, 8..=12) => ("pro", 0.5, "Matinée en semaine"),
            (false, 13..=17) => ("pro", 0.5, "Après-midi en semaine"),
            (false, 18..=22) => ("maison", 0.5, "Soirée en semaine"),
            _ => ("veille", 0.4, "Horaire par défaut"),
        };

        SinglePrediction {
            mode: mode.to_string(),
            confidence,
            reason: reason.to_string(),
        }
    }

    /// Predict based on learned behavioral patterns
    fn predict_from_patterns(&self, signals: &ContextSignals) -> SinglePrediction {
        let patterns = self.learned_patterns.read();

        // Find high-confidence pattern for current context
        let high_confidence = patterns.iter()
            .filter(|p| {
                p.day_of_week == signals.day_of_week &&
                p.hour == signals.hour &&
                p.occurrences >= 3 &&
                p.confidence > 0.6
            })
            .max_by(|a, b| a.occurrences.cmp(&b.occurrences));

        if let Some(pattern) = high_confidence {
            SinglePrediction {
                mode: pattern.mode.clone(),
                confidence: pattern.confidence,
                reason: format!(
                    "Habitude apprise: {} ({} occurrences)",
                    mode_display_name(&pattern.mode),
                    pattern.occurrences
                ),
            }
        } else {
            SinglePrediction {
                mode: String::new(),
                confidence: 0.0,
                reason: String::new(),
            }
        }
    }

    /// Predict based on agent activity (CPU, processes, idle time)
    fn predict_from_agent_activity(&self, signals: &ContextSignals) -> SinglePrediction {
        // Detect work apps (IDEs, terminals, communication pro)
        let work_apps = ["code", "rider", "intellij", "vscode", "terminal", "slack", "teams", "rustrover", "idea", "obsidian", "notion", "jetbrains", "webstorm", "pycharm", "goland", "clion", "datagrip"];
        let has_work_apps = signals.active_processes.iter()
            .any(|p| work_apps.iter().any(|w| p.to_lowercase().contains(w)));

        // Detect leisure apps (streaming media uniquement - Steam tourne souvent en fond donc exclu)
        let leisure_apps = ["netflix", "kodi", "plex", "stremio"];
        let has_leisure_apps = signals.active_processes.iter()
            .any(|p| leisure_apps.iter().any(|l| p.to_lowercase().contains(l)));

        // Check if idle
        let is_idle = signals.agent_idle_seconds > 300; // 5 minutes
        let is_very_idle = signals.agent_idle_seconds > 1800; // 30 minutes

        let (mode, confidence, reason) = if is_very_idle && (signals.hour >= 23 || signals.hour <= 6) {
            ("veille", 0.8, "Inactif tard le soir".to_string())
        } else if is_very_idle && signals.cpu_usage < 5.0 {
            ("veille", 0.6, "Système très idle".to_string())
        } else if has_work_apps && !has_leisure_apps {
            let app_names: Vec<&str> = signals.active_processes.iter()
                .filter(|p| work_apps.iter().any(|w| p.to_lowercase().contains(w)))
                .take(2)
                .map(|s| s.as_str())
                .collect();
            // "focus" pour travail sur PC (pro = extérieur/réunions sans PC)
            ("focus", 0.7, format!("Apps de travail: {}", app_names.join(", ")))
        } else if has_leisure_apps && !has_work_apps {
            ("maison", 0.6, "Apps de détente détectées".to_string())
        } else if is_idle {
            ("veille", 0.4, "Système idle".to_string())
        } else {
            ("", 0.0, String::new())
        };

        SinglePrediction {
            mode: mode.to_string(),
            confidence,
            reason,
        }
    }

    /// Predict based on environment (temperature, humidity)
    fn predict_from_environment(&self, signals: &ContextSignals) -> SinglePrediction {
        // Environmental factors are weak signals
        // Cold temperature might indicate sleeping/away
        if let Some(temp) = signals.temperature {
            if temp < 17.0 && signals.hour >= 22 {
                return SinglePrediction {
                    mode: "veille".to_string(),
                    confidence: 0.3,
                    reason: format!("Température basse ({:.1}°C) le soir", temp),
                };
            }
        }

        SinglePrediction {
            mode: String::new(),
            confidence: 0.0,
            reason: String::new(),
        }
    }

    /// Predict based on momentum (time in current mode)
    fn predict_from_momentum(&self, signals: &ContextSignals) -> SinglePrediction {
        let minutes = signals.time_in_current_mode_minutes;

        // Strong preference to stay if just changed
        let confidence = if minutes < 15 {
            0.8 // Just changed - strong momentum
        } else if minutes < 60 {
            0.5 // Less than an hour
        } else if minutes < 240 {
            0.3 // Less than 4 hours
        } else {
            0.1 // Long time - weak momentum
        };

        SinglePrediction {
            mode: signals.current_mode.clone(),
            confidence,
            reason: format!("En mode {} depuis {}min", signals.current_mode, minutes),
        }
    }

    // ========================================================================
    // Phase 5: Feedback Loop (Learning)
    // ========================================================================

    /// Record user feedback when they manually change mode
    pub fn record_feedback(&self, chosen_mode: &str, signals: ContextSignals) {
        let last_prediction = self.last_prediction.read().clone();

        let predicted_mode = last_prediction
            .as_ref()
            .map(|p| p.mode.clone())
            .unwrap_or_default();

        let was_correction = !predicted_mode.is_empty() && predicted_mode != chosen_mode;

        let feedback = UserFeedback {
            timestamp: OffsetDateTime::now_utc(),
            predicted_mode: predicted_mode.clone(),
            actual_mode: chosen_mode.to_string(),
            signals_snapshot: signals.clone(),
            was_correction,
        };

        // Store feedback
        {
            let mut history = self.feedback_history.write();
            if history.len() >= 500 {
                history.pop_front();
            }
            history.push_back(feedback.clone());
        }

        // Update prediction history
        {
            let mut pred_history = self.prediction_history.write();
            if let Some(last) = pred_history.back_mut() {
                if last.predicted_mode == predicted_mode {
                    last.actual_mode = Some(chosen_mode.to_string());
                    last.was_correct = Some(!was_correction);
                }
            }
        }

        // ALWAYS learn from manual changes (not just corrections)
        // This reinforces patterns even when user confirms our prediction
        self.learn_from_manual_change(chosen_mode, &feedback.signals_snapshot, was_correction);
    }

    /// Learn from any manual mode change (correction or confirmation)
    fn learn_from_manual_change(&self, chosen_mode: &str, signals: &ContextSignals, was_correction: bool) {
        if was_correction {
            eprintln!(
                "[intelligence] 📚 Correction apprise: {} (prédit différent)",
                chosen_mode
            );
        } else {
            eprintln!(
                "[intelligence] 📚 Confirmation apprise: {} (prédit correct)",
                chosen_mode
            );
        }

        let new_pattern = LearnedPattern {
            mode: chosen_mode.to_string(),
            day_of_week: signals.day_of_week,
            hour: signals.hour,
            confidence: if was_correction { 0.3 } else { 0.2 }, // Corrections have slightly higher initial weight
            occurrences: 1,
            last_seen: OffsetDateTime::now_utc(),
            source: PatternSource::UserCorrection,
        };

        self.merge_or_create_pattern(new_pattern);
    }

    /// Merge with existing pattern or create new
    /// UNIFIED (v1.1.9): Uses additive confidence with adaptive modifier (no occurrences/5 formula)
    fn merge_or_create_pattern(&self, new_pattern: LearnedPattern) {
        let mut patterns = self.learned_patterns.write();

        // Find similar pattern
        if let Some(existing) = patterns.iter_mut().find(|p| {
            p.mode == new_pattern.mode &&
            p.day_of_week == new_pattern.day_of_week &&
            (p.hour as i8 - new_pattern.hour as i8).abs() <= 1
        }) {
            // Reinforce existing pattern
            existing.occurrences += 1; // Keep for diagnostics
            // UNIFIED: Additive confidence with adaptive modifier
            let was_correction = new_pattern.source == PatternSource::UserCorrection;
            let base_modifier = if was_correction { 0.25 } else { 0.15 };
            let effective_modifier = adaptive_modifier(base_modifier, existing.confidence);
            let old_conf = existing.confidence;
            existing.confidence = (existing.confidence + effective_modifier).clamp(0.05, 0.95);
            existing.last_seen = new_pattern.last_seen;
            eprintln!(
                "[intelligence] 📈 Pattern renforcé: {} à {}h ({} occ, {:.0}% → {:.0}% conf, modifier {:.2})",
                existing.mode, existing.hour, existing.occurrences, old_conf * 100.0, existing.confidence * 100.0, effective_modifier
            );
        } else {
            // Create new pattern
            eprintln!(
                "[intelligence] 🆕 Nouveau pattern: {} à {}h le {}",
                new_pattern.mode, new_pattern.hour, day_name(new_pattern.day_of_week)
            );
            patterns.push(new_pattern);
        }

        drop(patterns);
        self.save_patterns();
    }

    /// Record a prediction for accuracy tracking
    pub fn record_prediction(&self, prediction: &ModePrediction) {
        let record = PredictionRecord {
            timestamp: OffsetDateTime::now_utc(),
            predicted_mode: prediction.mode.clone(),
            actual_mode: None,
            confidence: prediction.confidence,
            was_correct: None,
            outcome_source: None,  // Updated later by monitor loop
        };

        let mut history = self.prediction_history.write();
        if history.len() >= 1000 {
            history.pop_front();
        }
        history.push_back(record);
    }

    /// Mark the last prediction as correct (called when auto-apply succeeds)
    pub fn mark_last_prediction_correct(&self) {
        let mut history = self.prediction_history.write();
        if let Some(last) = history.back_mut() {
            if last.was_correct.is_none() {
                last.was_correct = Some(true);
                last.actual_mode = Some(last.predicted_mode.clone());
            }
        }
    }

    // ========================================================================
    // Pattern Queries
    // ========================================================================

    /// Check if there's an established pattern (occurrences >= min) for the given mode/day/hour
    /// Used as guard-fou for auto-apply: confidence alone isn't enough for automatic execution
    pub fn has_established_pattern(&self, mode: &str, day_of_week: u8, hour: u8, min_occurrences: u32) -> bool {
        let patterns = self.learned_patterns.read();
        patterns.iter().any(|p| {
            p.mode == mode &&
            p.day_of_week == day_of_week &&
            (p.hour as i8 - hour as i8).abs() <= 1 &&
            p.occurrences >= min_occurrences
        })
    }

    /// Check if pattern is established AND return days since last seen (v1.1.9)
    /// Returns (is_established, days_since_seen) - None if no pattern found
    pub fn get_pattern_recency(&self, mode: &str, day_of_week: u8, hour: u8, min_occurrences: u32) -> (bool, Option<i64>) {
        let patterns = self.learned_patterns.read();
        let now = OffsetDateTime::now_utc();

        let matching = patterns.iter().find(|p| {
            p.mode == mode &&
            p.day_of_week == day_of_week &&
            (p.hour as i8 - hour as i8).abs() <= 1
        });

        match matching {
            Some(p) => {
                let days_since = (now - p.last_seen).whole_days();
                let is_established = p.occurrences >= min_occurrences;
                (is_established, Some(days_since))
            }
            None => (false, None)
        }
    }

    /// Get pattern with decayed confidence for export/debug
    pub fn get_patterns_with_decay(&self) -> Vec<PatternExport> {
        // Read config coefficients once
        let decay_coeffs = self.config.read().decay_coefficients;
        let patterns = self.learned_patterns.read();
        let now = OffsetDateTime::now_utc();

        patterns.iter().map(|p| {
            let days_since = (now - p.last_seen).whole_days();
            let decay = if days_since < 7 { decay_coeffs[0] }
                       else if days_since < 30 { decay_coeffs[1] }
                       else if days_since < 90 { decay_coeffs[2] }
                       else { decay_coeffs[3] };

            PatternExport {
                mode: p.mode.clone(),
                day_of_week: p.day_of_week,
                hour: p.hour,
                confidence: p.confidence,
                decayed_confidence: p.confidence * decay,
                occurrences: p.occurrences,
                last_seen: p.last_seen,
                source: p.source.clone(),
                days_since_seen: days_since as u32,
            }
        }).collect()
    }

    // ========================================================================
    // Decision Engine Feedback Loop
    // ========================================================================

    /// Get the last collected signals (for external use by Decision Engine)
    pub fn last_signals(&self) -> Option<ContextSignals> {
        self.last_signals_cache.read().clone()
    }

    /// Record feedback from Decision Engine outcome
    /// This closes the loop: Intelligence learns from automated decisions, not just user corrections
    /// target_mode: The mode the automation was trying to achieve (None = no learning)
    /// blocked_categories: For Blocked signals, categories determine learning modifier
    pub fn record_decision_outcome(
        &self,
        signal: DecisionSignal,
        target_mode: Option<&str>,
        signals: &ContextSignals,
        blocked_categories: Option<&[crate::decision::BlockedReasonCategory]>,
    ) {
        // No target mode = no learning (rule: intent → goal_mode → None, NOT current_mode)
        let mode = match target_mode {
            Some(m) if !m.is_empty() => m,
            _ => {
                eprintln!("[intelligence] 🎯 No explicit intent (target_mode=None), skipping feedback");
                return;
            }
        };

        let modifier: f32 = match signal {
            DecisionSignal::ApprovedAuto => 0.3,    // Strong positive: system was right
            DecisionSignal::ApprovedMFA => 0.15,    // Weak positive: needed human validation
            DecisionSignal::Denied => {
                // Health counter: denied
                self.increment_counter(|c| c.denied += 1);
                -0.3  // Strong negative: human said no
            }
            DecisionSignal::Expired => 0.0,         // Ambiguous: no learning
            DecisionSignal::Blocked => {
                // Use category-specific modifier if available
                blocked_categories
                    .map(|cats| {
                        cats.iter()
                            .map(|c| c.learning_modifier())
                            .min_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal))
                            .unwrap_or(-0.1)
                    })
                    .unwrap_or(-0.1)
            }
        };

        if modifier.abs() < 0.01 {
            eprintln!("[intelligence] 🎯 Decision outcome: {:?} (no learning - modifier ~0)", signal);
            return;
        }

        self.apply_decision_modifier(mode, signals, modifier, signal);
    }

    /// Apply a modifier to an existing pattern based on Decision Engine feedback
    /// ADAPTIVE (v1.1.9): Uses diminishing returns at extremes
    fn apply_decision_modifier(
        &self,
        mode: &str,
        signals: &ContextSignals,
        base_modifier: f32,
        signal: DecisionSignal,
    ) {
        let mut patterns = self.learned_patterns.write();

        // Find matching pattern (same day, hour ±1)
        if let Some(existing) = patterns.iter_mut().find(|p| {
            p.mode == mode &&
            p.day_of_week == signals.day_of_week &&
            (p.hour as i8 - signals.hour as i8).abs() <= 1
        }) {
            // ADAPTIVE: Apply diminishing returns at extremes
            let effective_modifier = adaptive_modifier(base_modifier, existing.confidence);
            let old_conf = existing.confidence;
            existing.confidence = (existing.confidence + effective_modifier).clamp(0.05, 0.95);
            existing.last_seen = OffsetDateTime::now_utc();

            let action = if base_modifier > 0.0 { "reinforced" } else { "penalized" };
            eprintln!(
                "[intelligence] 🎯 Decision feedback ({:?}): {} pattern {} {:.0}% → {:.0}% (adaptive: {:.2})",
                signal, mode, action, old_conf * 100.0, existing.confidence * 100.0, effective_modifier
            );
        } else if base_modifier > 0.0 {
            // Create new pattern only for positive signals
            let new_pattern = LearnedPattern {
                mode: mode.to_string(),
                day_of_week: signals.day_of_week,
                hour: signals.hour,
                confidence: base_modifier.min(0.3),
                occurrences: 1,
                last_seen: OffsetDateTime::now_utc(),
                source: PatternSource::Automation,
            };
            patterns.push(new_pattern);
            eprintln!(
                "[intelligence] 🎯 Decision feedback ({:?}): new pattern {} created at {}h",
                signal, mode, signals.hour
            );
        } else {
            // Negative signal but no existing pattern to penalize
            eprintln!(
                "[intelligence] 🎯 Decision feedback ({:?}): no pattern to penalize for {} at {}h",
                signal, mode, signals.hour
            );
        }

        drop(patterns);
        self.save_patterns();
    }
}

// ============================================================================
// Helper Functions
// ============================================================================

/// Adaptive modifier with diminishing returns at extremes (v1.1.9)
///
/// Prevents oscillations by damping changes when confidence is already extreme.
/// Examples:
/// - current=0.50 → damping=1.0  → +0.30 * 1.0 = +0.30 (full effect)
/// - current=0.70 → damping=0.6  → +0.30 * 0.6 = +0.18
/// - current=0.90 → damping=0.3  → +0.30 * 0.3 = +0.09 (minimum)
fn adaptive_modifier(base: f32, current: f32) -> f32 {
    // Distance from center (0.5)
    let distance = (current - 0.5).abs();  // 0.0 to 0.45
    // Damping: near center = full effect, extremes = attenuated (min 0.3)
    let damping = (1.0 - distance * 2.0).max(0.3);
    base * damping
}

/// Convert day number to French name
/// Uses Monday-based indexing: 0=Lundi, 6=Dimanche (matches number_from_monday() - 1)
fn day_name(day: u8) -> &'static str {
    match day {
        0 => "lundi",
        1 => "mardi",
        2 => "mercredi",
        3 => "jeudi",
        4 => "vendredi",
        5 => "samedi",
        6 => "dimanche",
        _ => "inconnu",
    }
}

fn mode_display_name(mode: &str) -> &'static str {
    match mode {
        "pro" | "cravate" | "focus" => "Professionnel",
        "maison" | "intime" => "Maison",
        "veille" | "neutre" => "Veille",
        _ => "Inconnu",
    }
}

// ============================================================================
// Intelligence Monitor
// ============================================================================

impl ContextIntelligence {
    /// Spawn the intelligence monitor background task
    /// Runs every 30 seconds to collect signals, predict mode, and auto-apply/suggest
    pub fn spawn_intelligence_monitor(
        intelligence: SharedContextIntelligence,
        mode_registry: crate::modes::SharedModeRegistry,
        notifications_manager: crate::notifications::SharedNotificationManager,
    ) {
        tokio::spawn(async move {
            eprintln!("[intelligence] 🧠 Intelligence monitor started");

            loop {
                let check_interval = {
                    let config = intelligence.config.read();
                    std::time::Duration::from_secs(config.check_interval_seconds)
                };

                tokio::time::sleep(check_interval).await;

                // 1. Collect signals
                let signals = intelligence.collect_signals().await;

                // 2. Predict optimal mode
                let prediction = intelligence.predict_mode(&signals);

                // Record the prediction for accuracy tracking
                intelligence.record_prediction(&prediction);

                // 3. Compare with current mode
                let current_mode = signals.current_mode.clone();

                // Log prediction for debugging
                eprintln!(
                    "[intelligence] Prediction: {} (conf: {:.0}%) | Current: {} | {}",
                    prediction.mode,
                    prediction.confidence * 100.0,
                    current_mode,
                    if prediction.mode == current_mode { "SAME" } else { "DIFFERENT" }
                );

                // Auto-mark prediction as correct if mode is stable for 30+ minutes
                if prediction.mode == current_mode && signals.time_in_current_mode_minutes > 30 {
                    intelligence.mark_last_prediction_correct();
                }

                // Check for active override - NEVER change mode while override is active
                let has_active_override = intelligence.context_engine.get_state()
                    .and_then(|s| s.manual_override)
                    .map(|o| o.until > time::OffsetDateTime::now_utc())
                    .unwrap_or(false);

                if has_active_override {
                    eprintln!(
                        "[intelligence] 🔒 Override actif - aucun changement de mode autorisé"
                    );
                    continue; // Skip this cycle entirely
                }

                if prediction.mode != current_mode {
                    let config = intelligence.config.read().clone();

                    // Check if there was a recent manual change (cooldown period)
                    let manual_cooldown_minutes = 30;
                    let recent_manual_change = signals.last_manual_change
                        .map(|ts| {
                            let now = time::OffsetDateTime::now_utc();
                            (now - ts).whole_minutes() < manual_cooldown_minutes
                        })
                        .unwrap_or(false);

                    // v1.1.9: Auto-apply requires BOTH high confidence AND established pattern
                    let (has_established, days_since_seen) = intelligence.get_pattern_recency(
                        &prediction.mode,
                        signals.day_of_week,
                        signals.hour,
                        config.min_pattern_occurrences,
                    );

                    if prediction.confidence >= config.auto_apply_threshold && !recent_manual_change && has_established {
                        // AUTO-APPLY: High confidence + established pattern + no recent manual change
                        eprintln!(
                            "[intelligence] 🎯 Auto-apply: {} → {} (confiance {:.0}%, pattern établi)",
                            current_mode, prediction.mode, prediction.confidence * 100.0
                        );

                        // Get theme from mode registry and convert ModeTheme -> Theme
                        let mode_theme = mode_registry.get_by_slug(&prediction.mode)
                            .map(|m| m.theme.clone());
                        let theme = match mode_theme {
                            Some(mt) => Theme {
                                primary: mt.primary,
                                bg: mt.background,
                                accent: mt.accent,
                            },
                            None => Theme {
                                primary: "#6b7280".to_string(),
                                bg: "#f9fafb".to_string(),
                                accent: "#4b5563".to_string(),
                            },
                        };

                        let reason = if prediction.reasons.is_empty() {
                            "Intelligence contextuelle".to_string()
                        } else {
                            prediction.reasons.join(" + ")
                        };

                        intelligence.context_engine.set_mode_natural(
                            prediction.mode.clone(),
                            theme,
                            reason,
                        );

                        // Mark prediction as correct for accuracy tracking
                        intelligence.mark_last_prediction_correct();

                        // Health counter: auto-applied
                        intelligence.increment_counter(|c| c.auto_applied += 1);

                    } else if prediction.confidence >= config.suggestion_threshold {
                        // SUGGEST: Medium confidence OR missing established pattern
                        let block_reason = if recent_manual_change {
                            "changement manuel récent"
                        } else if prediction.confidence >= config.auto_apply_threshold && !has_established {
                            "pattern non établi (<3 occ)"
                        } else {
                            ""
                        };

                        // v1.1.9: Anti-spam rules for push notifications
                        let (can_push, spam_reason) = intelligence.can_send_push_notification(
                            &prediction.mode,
                            prediction.confidence,
                            has_established,
                            days_since_seen,
                        );

                        if can_push {
                            // Log and send push notification
                            if !block_reason.is_empty() {
                                eprintln!(
                                    "[intelligence] ⏸️ Auto-apply bloqué ({}) → push: {} → {} (confiance {:.0}%)",
                                    block_reason, current_mode, prediction.mode, prediction.confidence * 100.0
                                );
                            } else {
                                eprintln!(
                                    "[intelligence] 💡 Suggestion push: {} → {} (confiance {:.0}%)",
                                    current_mode, prediction.mode, prediction.confidence * 100.0
                                );
                            }

                            Self::send_suggestion_notification(
                                &notifications_manager,
                                &prediction,
                                &current_mode,
                            ).await;

                            // Record for rate limiting (also increments push_sent counter)
                            intelligence.record_notification_sent(&prediction.mode);
                        } else {
                            // Silent suggestion (visible in PWA status, no push)
                            eprintln!(
                                "[intelligence] 💭 Suggestion silencieuse: {} → {} (confiance {:.0}%, pas de push: {})",
                                current_mode, prediction.mode, prediction.confidence * 100.0, spam_reason
                            );
                        }

                        // Health counter: suggestion generated (both push and silent)
                        intelligence.increment_counter(|c| c.suggestions_generated += 1);
                    } else if recent_manual_change {
                        // OBSERVE: Confidence too low but we log that we're respecting manual choice
                        eprintln!(
                            "[intelligence] 👀 Observer (changement manuel récent): {} prédit, confiance {:.0}%",
                            prediction.mode, prediction.confidence * 100.0
                        );
                    }
                    // If confidence < suggestion_threshold and no recent manual: observe silently
                }
            }
        });
    }

    /// Check if we can send a push notification (anti-spam rules v1.1.9)
    /// Returns (can_send, reason)
    /// days_since_seen: None if no pattern, Some(days) if pattern exists
    fn can_send_push_notification(&self, mode: &str, confidence: f32, has_established: bool, days_since_seen: Option<i64>) -> (bool, &'static str) {
        let now = OffsetDateTime::now_utc();
        let today = now.date();
        let config = self.config.read();

        // Rule 0: Quiet hours (23h-7h by default) - no push except very strong established + recent
        let hour = now.hour();
        let in_quiet_hours = if config.quiet_hours_start > config.quiet_hours_end {
            // Wraps around midnight (e.g., 23-7)
            hour >= config.quiet_hours_start || hour < config.quiet_hours_end
        } else {
            hour >= config.quiet_hours_start && hour < config.quiet_hours_end
        };
        if in_quiet_hours {
            // Exception: strong established pattern (0.80+) AND seen recently (< 14 days)
            // v1.1.10: lowered from 0.90 to 0.80
            let is_recent = days_since_seen.map(|d| d < 14).unwrap_or(false);
            if !(has_established && confidence >= 0.80 && is_recent) {
                return (false, "quiet hours (23h-7h)");
            }
        }

        // Rule 1: Check daily limit
        {
            let mut daily = self.daily_suggestion_count.write();
            if daily.0 != today {
                // New day, reset counter
                *daily = (today, 0);
            }
            if daily.1 >= config.max_push_per_day {
                return (false, "quota journalier atteint");
            }
        }

        // Rule 2: Check per-mode cooldown
        {
            let cooldowns = self.suggestion_cooldowns.read();
            if let Some(last_time) = cooldowns.get(mode) {
                let minutes_since = (now - *last_time).whole_minutes();
                if minutes_since < config.suggestion_cooldown_minutes as i64 {
                    return (false, "cooldown mode");
                }
            }
        }

        // Rule 3: Require minimum confidence for push
        // v1.1.10: unified threshold at 50% for both established and non-established
        if confidence >= 0.50 {
            if has_established {
                return (true, "pattern établi");
            } else {
                return (true, "confiance suffisante");
            }
        }

        (false, "confiance < 0.50")
    }

    /// Record that a notification was sent (update anti-spam counters)
    fn record_notification_sent(&self, mode: &str) {
        let now = OffsetDateTime::now_utc();

        // Update per-mode cooldown
        self.suggestion_cooldowns.write().insert(mode.to_string(), now);

        // Update daily count
        let mut daily = self.daily_suggestion_count.write();
        if daily.0 == now.date() {
            daily.1 += 1;
        } else {
            *daily = (now.date(), 1);
        }

        // Update health counters
        self.increment_counter(|c| c.push_sent += 1);
    }

    /// Increment a health counter (auto-resets daily)
    fn increment_counter<F: FnOnce(&mut HealthCounters)>(&self, updater: F) {
        let today = OffsetDateTime::now_utc().date();
        let mut counters = self.health_counters.write();
        if counters.date != Some(today) {
            // New day, reset all counters
            *counters = HealthCounters {
                date: Some(today),
                ..Default::default()
            };
        }
        updater(&mut counters);
    }

    /// Get current health counters (24h)
    pub fn get_health_counters(&self) -> HealthCounters {
        let today = OffsetDateTime::now_utc().date();
        let counters = self.health_counters.read();
        if counters.date == Some(today) {
            counters.clone()
        } else {
            // New day, return empty
            HealthCounters {
                date: Some(today),
                ..Default::default()
            }
        }
    }

    /// Calculate decay coefficient for a given number of days since last seen
    /// Uses config.decay_coefficients: [<7d, <30d, <90d, >90d]
    fn calculate_decay(&self, days_since: i64) -> f32 {
        let config = self.config.read();
        let coeffs = config.decay_coefficients;
        if days_since < 7 { coeffs[0] }
        else if days_since < 30 { coeffs[1] }
        else if days_since < 90 { coeffs[2] }
        else { coeffs[3] }
    }

    /// Send a suggestion notification to the user
    async fn send_suggestion_notification(
        notifications_manager: &crate::notifications::SharedNotificationManager,
        prediction: &ModePrediction,
        current_mode: &str,
    ) {
        let title = format!("💡 Suggestion de mode");
        let body = format!(
            "Passer en mode {} ? (confiance: {:.0}%)\nRaison: {}",
            mode_display_name(&prediction.mode),
            prediction.confidence * 100.0,
            prediction.reasons.first().cloned().unwrap_or_default()
        );

        let notification = crate::notifications::Notification {
            id: uuid::Uuid::new_v4().to_string(),
            priority: crate::notifications::NotificationPriority::P2, // Normal priority suggestion
            title,
            body,
            source: "context_intelligence".to_string(),
            timestamp: time::OffsetDateTime::now_utc(),
            acknowledged: false,
            acknowledged_at: None,
            actions: vec![
                crate::notifications::NotificationAction {
                    id: "apply".to_string(),
                    label: format!("Appliquer {}", mode_display_name(&prediction.mode)),
                    action_type: crate::notifications::ActionType::Custom("apply_mode".to_string()),
                },
                crate::notifications::NotificationAction {
                    id: "dismiss".to_string(),
                    label: "Ignorer".to_string(),
                    action_type: crate::notifications::ActionType::Reject,
                },
            ],
            data: Some(serde_json::json!({
                "type": "intelligence_suggestion",
                "suggested_mode": prediction.mode,
                "current_mode": current_mode,
                "confidence": prediction.confidence,
            })),
        };

        if let Err(e) = notifications_manager.send(notification).await {
            eprintln!("[intelligence] Failed to send suggestion notification: {}", e);
        }
    }
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_config() {
        let config = IntelligenceConfig::default();
        assert_eq!(config.auto_apply_threshold, 0.60);  // v1.1.10: lowered for responsiveness
        assert_eq!(config.suggestion_threshold, 0.30);  // v1.1.10: lowered
        assert_eq!(config.min_pattern_occurrences, 3);
        assert!(config.auto_create_automations);
    }

    #[test]
    fn test_signal_weights_sum_to_one() {
        let weights = SignalWeights::default();
        let sum = weights.temporal + weights.behavioral + weights.agent_activity +
                  weights.environmental + weights.momentum;
        assert!((sum - 1.0).abs() < 0.001, "Weights should sum to 1.0, got {}", sum);
    }

    #[test]
    fn test_pattern_source_serialization() {
        let source = PatternSource::UserCorrection;
        let json = serde_json::to_string(&source).unwrap();
        assert!(json.contains("UserCorrection"));

        let parsed: PatternSource = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed, PatternSource::UserCorrection);
    }
}
