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
 *
 * NOTE: Types and config are now in crate::intelligence module
 */

use std::collections::{HashMap, VecDeque};
use std::sync::Arc;
use parking_lot::RwLock;
use time::OffsetDateTime;

use crate::agents::SharedAgentRegistry;
use crate::context::{ContextEngine, Mode, Theme};
use crate::sensors::SensorRegistry;

// Re-export types from intelligence module for backward compatibility
pub use crate::intelligence::{
    AccuracyStats,
    ContextSignals,
    DecisionSignal,
    HabitDrift,
    HealthCounters,
    IntelligenceConfig,
    IntelligenceStatus,
    LearnedPattern,
    ModePrediction,
    PatternExport,
    PatternSource,
    PredictionOutcome,
    PredictionRecord,
    SignalWeights,
    SinglePrediction,
    UserFeedback,
    adaptive_modifier,
    day_name,
    mode_display_name,
};

/// Shared type alias for ContextIntelligence
pub type SharedContextIntelligence = Arc<ContextIntelligence>;

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

    // Shadow mode statistics (v2 stabilization)
    shadow_stats: RwLock<crate::intelligence::ShadowStats>,
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
            // Shadow stats (v2 stabilization)
            shadow_stats: RwLock::new(crate::intelligence::ShadowStats {
                tracking_since: Some(OffsetDateTime::now_utc()),
                ..Default::default()
            }),
        }
    }

    /// Get shadow mode statistics (v1 vs v2 comparison)
    pub fn get_shadow_stats(&self) -> crate::intelligence::ShadowStats {
        self.shadow_stats.read().clone()
    }

    /// Update shadow stats with a comparison result
    fn record_shadow_comparison(&self, v1_mode: &str, v2_mode: &str, guard_result: &crate::intelligence::AutoApplyGuard) {
        let now = OffsetDateTime::now_utc();
        let mut stats = self.shadow_stats.write();

        // Initialize tracking_since if not set
        if stats.tracking_since.is_none() {
            stats.tracking_since = Some(now);
        }

        stats.total_comparisons += 1;
        stats.last_comparison_at = Some(now);

        if v1_mode == v2_mode {
            stats.agreements += 1;
        } else {
            stats.disagreements += 1;
        }

        // Update agreement rate
        if stats.total_comparisons > 0 {
            stats.agreement_rate = stats.agreements as f32 / stats.total_comparisons as f32;
        }

        // Track would-apply / blocked
        if guard_result.allowed {
            stats.v2_would_apply_count += 1;
        } else {
            stats.v2_blocked_count += 1;
            if let Some(reason) = &guard_result.blocked_reason {
                // Extract category from reason (e.g., "insufficient_confidence (...)" -> "insufficient_confidence")
                let category = reason.split(' ').next().unwrap_or(reason).to_string();
                *stats.blocked_reasons.entry(category).or_insert(0) += 1;
            }
        }

        // Update 24h stats (simple rolling - reset if last comparison > 24h ago)
        // For simplicity, we just track recent comparisons
        stats.last_24h.comparisons += 1;
        if v1_mode == v2_mode {
            stats.last_24h.agreements += 1;
        } else {
            stats.last_24h.disagreements += 1;
        }
        if stats.last_24h.comparisons > 0 {
            stats.last_24h.agreement_rate = stats.last_24h.agreements as f32 / stats.last_24h.comparisons as f32;
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
            let base_decay = if days_since < 7 { decay_coeffs[0] }
                       else if days_since < 30 { decay_coeffs[1] }
                       else if days_since < 90 { decay_coeffs[2] }
                       else { decay_coeffs[3] };

            // INVARIANT 3: Source-based decay modifier (same as calculate_decay)
            let source_multiplier = match &p.source {
                PatternSource::UserCorrection => 1.3,
                PatternSource::Historical => 1.0,
                PatternSource::Automation => 1.0,
            };
            let decay = (base_decay * source_multiplier).min(1.0);
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

        let _predictions_suggested = recent.iter()
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
        let (agent_online, idle_seconds, cpu_usage, active_processes) = self.get_agent_metrics().await;

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
            agent_online,
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
    /// Returns: (agent_online, idle_seconds, cpu_usage, active_processes)
    async fn get_agent_metrics(&self) -> (bool, u64, f32, Vec<String>) {
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

            (true, idle, cpu, processes)  // Agent is online
        } else {
            (false, 0, 0.0, Vec::new())   // No user agent online
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

        // Initialize mode scores dynamiquement depuis les patterns appris + mode actuel
        // Cela permet de supporter les modes créés par l'utilisateur via PWA
        {
            let patterns = self.learned_patterns.read();
            for pattern in patterns.iter() {
                scores.entry(pattern.mode.clone()).or_insert(0.0);
            }
        }
        // Toujours inclure le mode actuel (momentum)
        if !signals.current_mode.is_empty() {
            scores.entry(signals.current_mode.clone()).or_insert(0.0);
        }
        // Modes de base au cas où aucun pattern
        for mode in ["focus", "maison", "veille"] {
            scores.entry(mode.to_string()).or_insert(0.0);
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

        // Sort scores descending and take top 3
        let mut sorted_scores: Vec<(String, f32)> = scores.into_iter().collect();
        sorted_scores.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));

        // Normalize to percentages (sum = 100%)
        let total: f32 = sorted_scores.iter().map(|(_, s)| s.max(0.0)).sum();
        let top_modes: Vec<(String, f32)> = sorted_scores
            .iter()
            .take(3)
            .map(|(m, s)| {
                let normalized = if total > 0.0 { (s / total * 100.0).round() } else { 0.0 };
                (m.clone(), normalized)
            })
            .collect();

        // Best mode and confidence
        let (best_mode, best_score) = sorted_scores
            .first()
            .map(|(m, s)| (m.clone(), *s))
            .unwrap_or(("veille".into(), 0.0));

        // Uncertain if confidence too low (< 0.25) or top 2 modes are too close
        let is_uncertain = best_score < 0.25 || {
            if let Some((_, second_score)) = sorted_scores.get(1) {
                (best_score - second_score).abs() < 0.1 // Top 2 within 10%
            } else {
                false
            }
        };

        let prediction = ModePrediction {
            mode: if is_uncertain { "unknown".to_string() } else { best_mode },
            confidence: best_score.min(1.0),
            reasons,
            contributing_factors: factors,
            top_modes,
            is_uncertain,
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
            // INVARIANT 3: UserCorrection patterns decay slower
            let days_since = (OffsetDateTime::now_utc() - pattern.last_seen).whole_days();
            let decay = self.calculate_decay(days_since, &pattern.source);
            let effective_confidence = pattern.confidence * decay;

            // INVARIANT 3: Clear source in reason for debugging
            let source_label = match &pattern.source {
                PatternSource::UserCorrection => "user",
                PatternSource::Historical => "bootstrap",
                PatternSource::Automation => "auto",
            };
            return SinglePrediction {
                mode: pattern.mode.clone(),
                confidence: effective_confidence,
                reason: format!(
                    "Pattern détecté: {} à {}h le {} ({} occ, src={}{})",
                    mode_display_name(&pattern.mode),
                    pattern.hour,
                    day_name(pattern.day_of_week),
                    pattern.occurrences,
                    source_label,
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
        // Agent offline is a strong signal - user is away or PC is off
        if !signals.agent_online {
            let confidence = if signals.hour >= 23 || signals.hour <= 6 {
                0.75  // Night time + no agent = very likely sleeping
            } else if signals.is_weekend {
                0.5   // Weekend + no agent = maybe away
            } else {
                0.6   // Weekday + no agent = at work or out
            };
            return SinglePrediction {
                mode: "veille".to_string(),
                confidence,
                reason: "Agent utilisateur déconnecté".to_string(),
            };
        }

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

        // INVARIANT 3: Source label for logs
        let source_label = match &new_pattern.source {
            PatternSource::UserCorrection => "user",
            PatternSource::Historical => "bootstrap",
            PatternSource::Automation => "auto",
        };

        // Find similar pattern
        if let Some(existing) = patterns.iter_mut().find(|p| {
            p.mode == new_pattern.mode &&
            p.day_of_week == new_pattern.day_of_week &&
            (p.hour as i8 - new_pattern.hour as i8).abs() <= 1
        }) {
            // Reinforce existing pattern
            existing.occurrences += 1; // Keep for diagnostics
            // UNIFIED: Additive confidence with adaptive modifier
            // INVARIANT 3: UserCorrection has higher base modifier
            let was_correction = new_pattern.source == PatternSource::UserCorrection;
            let base_modifier = if was_correction { 0.25 } else { 0.15 };
            let effective_modifier = adaptive_modifier(base_modifier, existing.confidence);
            let old_conf = existing.confidence;
            existing.confidence = (existing.confidence + effective_modifier).clamp(0.05, 0.95);
            existing.last_seen = new_pattern.last_seen;
            // INVARIANT 3: Clear source in logs
            let existing_source = match &existing.source {
                PatternSource::UserCorrection => "user",
                PatternSource::Historical => "bootstrap",
                PatternSource::Automation => "auto",
            };
            eprintln!(
                "[intelligence] 📈 Pattern renforcé: {} à {}h ({} occ, src={}, input={}, {:.0}% → {:.0}%)",
                existing.mode, existing.hour, existing.occurrences, existing_source, source_label, old_conf * 100.0, existing.confidence * 100.0
            );
        } else {
            // Create new pattern
            // INVARIANT 3: Clear source in logs
            eprintln!(
                "[intelligence] 🆕 Nouveau pattern: {} à {}h le {} (src={})",
                new_pattern.mode, new_pattern.hour, day_name(new_pattern.day_of_week), source_label
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
    /// INVARIANT 3: Applies source-based decay modifier
    pub fn get_patterns_with_decay(&self) -> Vec<PatternExport> {
        // Read config coefficients once
        let decay_coeffs = self.config.read().decay_coefficients;
        let patterns = self.learned_patterns.read();
        let now = OffsetDateTime::now_utc();

        patterns.iter().map(|p| {
            let days_since = (now - p.last_seen).whole_days();
            let base_decay = if days_since < 7 { decay_coeffs[0] }
                       else if days_since < 30 { decay_coeffs[1] }
                       else if days_since < 90 { decay_coeffs[2] }
                       else { decay_coeffs[3] };

            // INVARIANT 3: Source-based decay modifier
            let source_multiplier = match &p.source {
                PatternSource::UserCorrection => 1.3,
                PatternSource::Historical => 1.0,
                PatternSource::Automation => 1.0,
            };
            let decay = (base_decay * source_multiplier).min(1.0);

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
// Intelligence Monitor
// ============================================================================

impl ContextIntelligence {
    /// Spawn the intelligence monitor background task
    /// Runs every 30 seconds to collect signals, predict mode, and auto-apply/suggest
    /// Also runs v2 predictions in shadow mode for comparison
    pub fn spawn_intelligence_monitor(
        intelligence: SharedContextIntelligence,
        mode_registry: crate::modes::SharedModeRegistry,
        notifications_manager: crate::notifications::SharedNotificationManager,
        feature_registry: crate::intelligence::SharedFeatureRegistry,
        inference_engine: crate::intelligence::SharedInferenceEngine,
        agent_registry: crate::agents::SharedAgentRegistry,
    ) {
        tokio::spawn(async move {
            eprintln!("[intelligence] 🧠 Intelligence monitor started (with v2 shadow mode)");
            let mut cycle_counter: u64 = 0;

            loop {
                cycle_counter += 1;
                let check_interval = {
                    let config = intelligence.config.read();
                    std::time::Duration::from_secs(config.check_interval_seconds)
                };

                tokio::time::sleep(check_interval).await;

                // 0. Sync all agents status to FeatureRegistry
                let all_agents = agent_registry.list_agents().await;
                eprintln!("[intelligence] Syncing {} agents to FeatureRegistry", all_agents.len());

                for (agent_id, agent) in all_agents.iter() {
                    let source = format!("agent.{}", agent_id);
                    let is_online = agent.status.status == "online";

                    // Set per-agent online status
                    feature_registry.set_feature(
                        &format!("agent.{}.online", agent_id),
                        crate::intelligence::FeatureValue::Bool(is_online),
                        &source,
                        1.0,
                        120, // 2 minutes TTL
                    );

                    // Set hostname for identification
                    feature_registry.set_feature(
                        &format!("agent.{}.hostname", agent_id),
                        crate::intelligence::FeatureValue::String(agent.hostname.clone()),
                        &source,
                        1.0,
                        300, // 5 minutes TTL
                    );

                    eprintln!("[intelligence]   - {} ({}) = {}", agent_id, agent.hostname, if is_online { "online" } else { "offline" });
                }

                // Track count of online agents
                let online_count = all_agents.values().filter(|a| a.status.status == "online").count();
                feature_registry.set_feature(
                    "agents.online_count",
                    crate::intelligence::FeatureValue::Int(online_count as i64),
                    "kernel",
                    1.0,
                    120,
                );
                eprintln!("[intelligence] Total online: {}/{}", online_count, all_agents.len());

                // 1. Collect signals
                let signals = intelligence.collect_signals().await;

                // 2. Predict optimal mode
                let prediction = intelligence.predict_mode(&signals);

                // Record the prediction for accuracy tracking
                intelligence.record_prediction(&prediction);

                // 3. Compare with current mode
                let current_mode = signals.current_mode.clone();

                // Log v1 prediction for debugging
                eprintln!(
                    "[intelligence] v1: {} (conf: {:.0}%) | Current: {} | {}",
                    prediction.mode,
                    prediction.confidence * 100.0,
                    current_mode,
                    if prediction.mode == current_mode { "SAME" } else { "DIFFERENT" }
                );

                // Shadow mode: Run v2 prediction and compare
                let vector = crate::intelligence::VectorBuilder::new(&feature_registry).build();
                let prediction_v2 = inference_engine.predict(&vector);
                let v2_mode = &prediction_v2.mode;
                let v2_conf = prediction_v2.confidence;

                // Get v2 config for guards
                let v2_config = intelligence.config.read().v2.clone();

                // Check auto-apply guards with stabilization rules
                let guard_result = inference_engine.check_auto_apply_guards(
                    &prediction_v2,
                    signals.time_in_current_mode_minutes,
                    &v2_config,
                );

                // Record shadow comparison stats
                intelligence.record_shadow_comparison(&prediction.mode, v2_mode, &guard_result);

                // Get sample stats for logging
                let sample_stats = inference_engine.sample_stats(v2_config.recent_days_window);

                // Detailed v2 prediction log
                eprintln!(
                    "[intelligence] v2: {} (conf: {:.0}%) | samples: {}/{} (bootstrap: {}, recent: {}) | {}",
                    v2_mode,
                    v2_conf * 100.0,
                    prediction_v2.samples_used,
                    sample_stats.total,
                    sample_stats.bootstrap,
                    sample_stats.recent,
                    if v2_mode == &prediction.mode { "AGREE" } else { "DISAGREE" }
                );

                // Log guard result
                if guard_result.allowed {
                    eprintln!(
                        "[intelligence] ✅ v2 guards: ALL PASSED (would auto-apply if enabled)"
                    );
                } else {
                    eprintln!(
                        "[intelligence] 🛡️ v2 guards: BLOCKED - {}",
                        guard_result.blocked_reason.as_deref().unwrap_or("unknown")
                    );
                }

                // Log top features contributing to vector
                let top_dims: Vec<_> = vector.dimensions.iter()
                    .filter(|(_, v)| **v > 0.1)
                    .take(3)
                    .map(|(k, v)| format!("{}={:.2}", k, v))
                    .collect();
                if !top_dims.is_empty() {
                    eprintln!(
                        "[intelligence] v2 vector: [{}] ({} features)",
                        top_dims.join(", "),
                        vector.feature_count
                    );
                }

                // Log comparison when predictions differ
                if v2_mode != &prediction.mode {
                    eprintln!(
                        "[intelligence] 🔄 Shadow comparison: v1={} vs v2={} | v2 has {} samples",
                        prediction.mode,
                        v2_mode,
                        prediction_v2.samples_used
                    );
                }

                // v2 Auto-apply check with STRICT guards
                let v2_would_apply = guard_result.allowed
                    && v2_mode != &current_mode
                    && v2_config.auto_apply_enabled;

                if v2_would_apply {
                    eprintln!(
                        "[intelligence] ✨ v2 AUTO-APPLY: {} → {} (conf: {:.0}%, {} samples, all guards passed)",
                        current_mode,
                        v2_mode,
                        v2_conf * 100.0,
                        prediction_v2.samples_used
                    );
                    // TODO: Actually apply when v2_config.auto_apply_enabled = true
                } else if guard_result.allowed && v2_mode != &current_mode {
                    // Would apply but auto_apply_enabled = false
                    eprintln!(
                        "[intelligence] 💭 v2 WOULD auto-apply (disabled): {} → {} | guards: ✅",
                        current_mode,
                        v2_mode
                    );
                }

                // Periodic compaction: remove decayed samples every ~100 cycles (~50 min)
                if cycle_counter % 100 == 0 {
                    let removed = inference_engine.compact();
                    if removed > 0 {
                        eprintln!("[intelligence] 🧹 Compacted {} decayed samples", removed);
                    }
                }

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
    /// INVARIANT 3: UserCorrection patterns decay slower (persist longer)
    fn calculate_decay(&self, days_since: i64, source: &PatternSource) -> f32 {
        let config = self.config.read();
        let coeffs = config.decay_coefficients;
        let base_decay = if days_since < 7 { coeffs[0] }
            else if days_since < 30 { coeffs[1] }
            else if days_since < 90 { coeffs[2] }
            else { coeffs[3] };

        // INVARIANT 3: Source-based decay modifier
        // UserCorrection = explicit user intent → slower decay (higher multiplier)
        // Historical/Automation = inferred → normal decay
        let source_multiplier = match source {
            PatternSource::UserCorrection => 1.3,  // 30% slower decay
            PatternSource::Historical => 1.0,      // Normal decay
            PatternSource::Automation => 1.0,      // Normal decay
        };

        // Apply multiplier (capped at 1.0 for coefficients > 1.0 edge case)
        (base_decay * source_multiplier).min(1.0)
    }

    /// Send a suggestion notification to the user
    async fn send_suggestion_notification(
        notifications_manager: &crate::notifications::SharedNotificationManager,
        prediction: &ModePrediction,
        current_mode: &str,
    ) {
        let title = "💡 Suggestion de mode".to_string();
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
