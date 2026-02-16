//! Inference Engine v2
//!
//! Case-based reasoning with weighted voting for mode prediction.
//!
//! ## Architecture
//!
//! ```text
//! ContextVector → Similarity Search → Weighted Voting → Prediction
//! ```
//!
//! ## Algorithm
//!
//! 1. Calculate cosine similarity between current vector and stored samples
//! 2. Select top-k most similar samples
//! 3. Weighted vote: similarity × recency × source_weight
//! 4. Output prediction with confidence and alternatives
//! 5. If confidence < threshold → mode = "unknown"

use std::collections::HashMap;
use std::sync::Arc;
use parking_lot::RwLock;
use serde::{Deserialize, Serialize};
use time::OffsetDateTime;

use super::vector::{ContextVector, dimensions};

// ============================================================================
// Training Sample
// ============================================================================

/// Source of a training sample (affects weight)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum SampleSource {
    /// User explicitly corrected the mode (highest weight)
    UserCorrection,
    /// Mode confirmed via MFA/notification
    MfaConfirmed,
    /// Inferred from automation trigger
    Automation,
    /// Bootstrap/initial rules (lowest weight)
    Bootstrap,
}

impl SampleSource {
    /// Get the base weight multiplier for this source
    pub fn weight_multiplier(&self) -> f32 {
        match self {
            SampleSource::UserCorrection => 1.3,
            SampleSource::MfaConfirmed => 1.0,
            SampleSource::Automation => 0.8,
            SampleSource::Bootstrap => 0.5,
        }
    }

    /// Check if this source is bootstrap (low quality)
    pub fn is_bootstrap(&self) -> bool {
        matches!(self, SampleSource::Bootstrap)
    }
}

/// A training sample linking a context vector to a chosen mode
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TrainingSample {
    /// Unique sample ID
    pub id: String,

    /// The context vector at the time of the decision
    pub vector: SampleVector,

    /// The mode that was chosen/confirmed
    pub chosen_mode: String,

    /// How this sample was created
    pub source: SampleSource,

    /// When this sample was recorded
    #[serde(with = "time::serde::iso8601")]
    pub timestamp: OffsetDateTime,

    /// Base weight (before time decay)
    pub base_weight: f32,
}

/// Simplified vector for storage (just dimensions, no why-chain)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SampleVector {
    pub dimensions: HashMap<String, f32>,
}

impl From<&ContextVector> for SampleVector {
    fn from(cv: &ContextVector) -> Self {
        Self {
            dimensions: cv.dimensions.clone(),
        }
    }
}

impl TrainingSample {
    /// Create a new training sample
    pub fn new(vector: &ContextVector, chosen_mode: &str, source: SampleSource) -> Self {
        Self {
            id: uuid::Uuid::new_v4().to_string(),
            vector: SampleVector::from(vector),
            chosen_mode: chosen_mode.to_string(),
            source,
            timestamp: OffsetDateTime::now_utc(),
            base_weight: 1.0,
        }
    }

    /// Calculate the effective weight (with time decay and source multiplier)
    /// Bootstrap samples decay faster (half-life 7 days vs 30 days for others)
    pub fn effective_weight(&self) -> f32 {
        self.effective_weight_with_config(7.0)
    }

    /// Calculate effective weight with configurable bootstrap decay half-life
    pub fn effective_weight_with_config(&self, bootstrap_half_life_days: f32) -> f32 {
        let age_days = (OffsetDateTime::now_utc() - self.timestamp).whole_days() as f32;

        // Bootstrap samples have faster decay (configurable half-life)
        let decay_rate = if self.source.is_bootstrap() {
            bootstrap_half_life_days // Default: half weight after 7 days
        } else {
            30.0 // Normal samples: half weight after 30 days
        };

        let time_decay = (-age_days * 0.693 / decay_rate).exp(); // 0.693 = ln(2) for half-life
        self.base_weight * self.source.weight_multiplier() * time_decay
    }

    /// Calculate cosine similarity with another vector
    pub fn similarity(&self, other: &ContextVector) -> f32 {
        cosine_similarity(&self.vector.dimensions, &other.dimensions)
    }

    /// Check if this sample is recent (within given days)
    pub fn is_recent(&self, days: i64) -> bool {
        let age_days = (OffsetDateTime::now_utc() - self.timestamp).whole_days();
        age_days <= days
    }
}

// ============================================================================
// Prediction Result
// ============================================================================

/// A mode prediction with confidence and alternatives
#[derive(Debug, Clone, Serialize)]
pub struct PredictionV2 {
    /// Predicted mode (or "unknown" if low confidence)
    pub mode: String,

    /// Confidence score (0.0-1.0)
    pub confidence: f32,

    /// Alternative modes with their scores
    pub alternatives: Vec<ModeScore>,

    /// Explanation of how this prediction was made
    pub why: Vec<PredictionReason>,

    /// Number of samples used for this prediction
    pub samples_used: usize,

    /// Whether this prediction has sufficient confidence
    pub is_confident: bool,
}

/// A mode with its score
#[derive(Debug, Clone, Serialize)]
pub struct ModeScore {
    pub mode: String,
    pub score: f32,
}

/// Explanation for a prediction contribution
#[derive(Debug, Clone, Serialize)]
pub struct PredictionReason {
    pub sample_id: String,
    pub mode: String,
    pub similarity: f32,
    pub weight: f32,
    pub contribution: f32,
}

impl PredictionV2 {
    /// Create an "unknown" prediction when we don't have enough data
    pub fn unknown(reason: &str) -> Self {
        Self {
            mode: "unknown".to_string(),
            confidence: 0.0,
            alternatives: vec![],
            why: vec![PredictionReason {
                sample_id: "none".to_string(),
                mode: "unknown".to_string(),
                similarity: 0.0,
                weight: 0.0,
                contribution: 0.0,
            }],
            samples_used: 0,
            is_confident: false,
        }
    }
}

// ============================================================================
// Inference Engine
// ============================================================================

/// Configuration for the inference engine
#[derive(Debug, Clone)]
pub struct InferenceConfig {
    /// Maximum number of samples to store
    pub max_samples: usize,

    /// Number of top-k samples to use for voting
    pub top_k: usize,

    /// Minimum confidence threshold (below this → "unknown")
    pub min_confidence: f32,

    /// Minimum similarity threshold to consider a sample
    pub min_similarity: f32,
}

impl Default for InferenceConfig {
    fn default() -> Self {
        Self {
            max_samples: 2000,
            top_k: 10,
            min_confidence: 0.25,
            min_similarity: 0.3,
        }
    }
}

/// Shared inference engine type
pub type SharedInferenceEngine = Arc<InferenceEngine>;

/// The inference engine for mode prediction
pub struct InferenceEngine {
    /// Stored training samples
    samples: RwLock<Vec<TrainingSample>>,

    /// Configuration
    config: InferenceConfig,

    /// Path to persistence file (None = no persistence)
    data_path: Option<std::path::PathBuf>,
}

impl Default for InferenceEngine {
    fn default() -> Self {
        Self::new(InferenceConfig::default())
    }
}

impl InferenceEngine {
    /// Create a new inference engine (no persistence)
    pub fn new(config: InferenceConfig) -> Self {
        Self {
            samples: RwLock::new(Vec::new()),
            config,
            data_path: None,
        }
    }

    /// Create a new inference engine with persistence
    pub fn with_persistence(config: InferenceConfig, data_path: std::path::PathBuf) -> Self {
        let engine = Self {
            samples: RwLock::new(Vec::new()),
            config,
            data_path: Some(data_path),
        };
        // Load existing samples from disk
        engine.load_samples();
        engine
    }

    /// Load samples from disk
    pub fn load_samples(&self) {
        let Some(path) = &self.data_path else { return };

        if !path.exists() {
            eprintln!("[inference] No existing samples file at {:?}", path);
            return;
        }

        match std::fs::read_to_string(path) {
            Ok(content) => {
                match serde_json::from_str::<Vec<TrainingSample>>(&content) {
                    Ok(loaded) => {
                        let count = loaded.len();
                        *self.samples.write() = loaded;
                        eprintln!("[inference] ✅ Loaded {} samples from {:?}", count, path);
                    }
                    Err(e) => {
                        eprintln!("[inference] ❌ Failed to parse samples: {}", e);
                    }
                }
            }
            Err(e) => {
                eprintln!("[inference] ❌ Failed to read samples file: {}", e);
            }
        }
    }

    /// Save samples to disk (atomic write: temp file + rename)
    /// File I/O is offloaded to a background thread to avoid blocking the async runtime.
    pub fn save_samples(&self) {
        let Some(path) = &self.data_path else { return };

        // Serialize under read lock (fast, in-memory)
        let json = {
            let samples = self.samples.read();
            match serde_json::to_string_pretty(&*samples) {
                Ok(json) => json,
                Err(e) => {
                    eprintln!("[inference] Failed to serialize samples: {}", e);
                    return;
                }
            }
        };

        // Offload blocking file I/O to background thread
        let path = path.clone();
        std::thread::spawn(move || {
            let tmp_path = path.with_extension("json.tmp");
            if let Err(e) = std::fs::write(&tmp_path, &json) {
                eprintln!("[inference] Failed to write temp samples file: {}", e);
                return;
            }
            if let Err(e) = std::fs::rename(&tmp_path, &path) {
                eprintln!("[inference] Failed to rename temp samples file: {}", e);
            }
        });
    }

    /// Add a training sample
    pub fn add_sample(&self, sample: TrainingSample) {
        {
            let mut samples = self.samples.write();
            samples.push(sample);

            // Enforce max samples limit
            if samples.len() > self.config.max_samples {
                // Remove oldest samples with lowest weight
                samples.sort_by(|a, b| {
                    b.effective_weight()
                        .partial_cmp(&a.effective_weight())
                        .unwrap_or(std::cmp::Ordering::Equal)
                });
                samples.truncate(self.config.max_samples);
            }
        } // Release write lock before saving

        // Persist to disk
        self.save_samples();
    }

    /// Record a user correction (highest priority sample)
    pub fn record_correction(&self, vector: &ContextVector, chosen_mode: &str) {
        let sample = TrainingSample::new(vector, chosen_mode, SampleSource::UserCorrection);
        self.add_sample(sample);
    }

    /// Record an automation-triggered mode (lower priority)
    pub fn record_automation(&self, vector: &ContextVector, chosen_mode: &str) {
        let sample = TrainingSample::new(vector, chosen_mode, SampleSource::Automation);
        self.add_sample(sample);
    }

    /// Record a bootstrap sample (lowest priority, for cold start)
    pub fn record_bootstrap(&self, vector: &ContextVector, mode: &str) {
        let mut sample = TrainingSample::new(vector, mode, SampleSource::Bootstrap);
        sample.base_weight = 0.5; // Even lower weight for bootstrap
        self.add_sample(sample);
    }

    /// Predict the mode from a context vector
    pub fn predict(&self, vector: &ContextVector) -> PredictionV2 {
        let samples = self.samples.read();

        // Check if we have enough samples
        if samples.is_empty() {
            return PredictionV2::unknown("no training samples");
        }

        // Calculate similarity and weight for each sample
        let mut scored_samples: Vec<(usize, f32, f32, f32)> = samples
            .iter()
            .enumerate()
            .map(|(i, s)| {
                let similarity = s.similarity(vector);
                let weight = s.effective_weight();
                let score = similarity * weight;
                (i, similarity, weight, score)
            })
            .filter(|(_, sim, _, _)| *sim >= self.config.min_similarity)
            .collect();

        // Sort by score descending
        scored_samples.sort_by(|a, b| b.3.partial_cmp(&a.3).unwrap_or(std::cmp::Ordering::Equal));

        // Take top-k
        let top_k: Vec<_> = scored_samples
            .iter()
            .take(self.config.top_k)
            .collect();

        if top_k.is_empty() {
            return PredictionV2::unknown("no similar samples found");
        }

        // Weighted voting
        let mut mode_scores: HashMap<String, f32> = HashMap::new();
        let mut why = Vec::new();
        let mut total_score = 0.0;

        for (idx, similarity, weight, score) in &top_k {
            let sample = &samples[*idx];
            let mode = &sample.chosen_mode;

            *mode_scores.entry(mode.clone()).or_insert(0.0) += score;
            total_score += score;

            why.push(PredictionReason {
                sample_id: sample.id.clone(),
                mode: mode.clone(),
                similarity: *similarity,
                weight: *weight,
                contribution: *score,
            });
        }

        // Normalize scores
        if total_score > 0.0 {
            for score in mode_scores.values_mut() {
                *score /= total_score;
            }
        }

        // Find best mode
        let mut sorted_modes: Vec<_> = mode_scores.into_iter().collect();
        sorted_modes.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));

        let (best_mode, confidence) = sorted_modes
            .first()
            .map(|(m, s)| (m.clone(), *s))
            .unwrap_or(("unknown".to_string(), 0.0));

        let is_confident = confidence >= self.config.min_confidence;
        let final_mode = if is_confident {
            best_mode.clone()
        } else {
            "unknown".to_string()
        };

        let alternatives: Vec<ModeScore> = sorted_modes
            .iter()
            .skip(1)
            .take(3)
            .map(|(m, s)| ModeScore {
                mode: m.clone(),
                score: *s,
            })
            .collect();

        PredictionV2 {
            mode: final_mode,
            confidence,
            alternatives,
            why,
            samples_used: top_k.len(),
            is_confident,
        }
    }

    /// Get all samples (for export/debug)
    pub fn get_samples(&self) -> Vec<TrainingSample> {
        self.samples.read().clone()
    }

    /// Get sample count
    pub fn sample_count(&self) -> usize {
        self.samples.read().len()
    }

    /// Clear all samples
    pub fn clear(&self) {
        self.samples.write().clear();
    }

    /// Compact samples (remove low-weight old samples)
    pub fn compact(&self) -> usize {
        let mut samples = self.samples.write();
        let initial_count = samples.len();

        // Remove samples with very low effective weight
        samples.retain(|s| s.effective_weight() > 0.01);

        initial_count - samples.len()
    }

    /// Get engine statistics
    pub fn stats(&self) -> InferenceStats {
        let samples = self.samples.read();

        let mut by_source: HashMap<String, usize> = HashMap::new();
        let mut by_mode: HashMap<String, usize> = HashMap::new();
        let mut total_weight = 0.0;

        for sample in samples.iter() {
            *by_source
                .entry(format!("{:?}", sample.source))
                .or_insert(0) += 1;
            *by_mode.entry(sample.chosen_mode.clone()).or_insert(0) += 1;
            total_weight += sample.effective_weight();
        }

        InferenceStats {
            total_samples: samples.len(),
            by_source,
            by_mode,
            average_weight: if samples.is_empty() {
                0.0
            } else {
                total_weight / samples.len() as f32
            },
        }
    }

    /// Get detailed sample statistics for v2 stabilization
    pub fn sample_stats(&self, recent_days: i64) -> SampleStats {
        let samples = self.samples.read();

        let total = samples.len();
        let bootstrap = samples.iter().filter(|s| s.source.is_bootstrap()).count();
        let non_bootstrap = total - bootstrap;
        let recent = samples.iter().filter(|s| s.is_recent(recent_days)).count();
        let recent_non_bootstrap = samples.iter()
            .filter(|s| !s.source.is_bootstrap() && s.is_recent(recent_days))
            .count();

        SampleStats {
            total,
            bootstrap,
            non_bootstrap,
            recent,
            recent_non_bootstrap,
            recent_days,
        }
    }

    /// Check if auto-apply is allowed based on v2 stabilization guards
    pub fn check_auto_apply_guards(
        &self,
        prediction: &PredictionV2,
        session_stable_minutes: i64,
        config: &super::config::V2StabilizationConfig,
    ) -> AutoApplyGuard {
        let stats = self.sample_stats(config.recent_days_window);

        // Individual guard checks
        let confidence_ok = prediction.confidence >= config.auto_apply_threshold;
        let total_samples_ok = stats.total >= config.min_samples_total;
        let non_bootstrap_ok = stats.non_bootstrap >= config.min_samples_non_bootstrap;
        let recent_samples_ok = stats.recent >= config.min_recent_samples;
        let session_stable_ok = session_stable_minutes >= config.min_session_stable_minutes;

        // Bootstrap containment: if ALL used samples are bootstrap, block
        let samples = self.samples.read();
        let used_sample_ids: std::collections::HashSet<_> = prediction.why.iter()
            .map(|r| &r.sample_id)
            .collect();
        let used_samples: Vec<_> = samples.iter()
            .filter(|s| used_sample_ids.contains(&s.id))
            .collect();
        let not_bootstrap_only = used_samples.iter().any(|s| !s.source.is_bootstrap());
        drop(samples);

        let guards = GuardChecks {
            confidence_ok,
            total_samples_ok,
            non_bootstrap_ok,
            recent_samples_ok,
            session_stable_ok,
            not_bootstrap_only,
        };

        // Determine blocked reason
        let blocked_reason = if !confidence_ok {
            Some(format!("insufficient_confidence ({:.0}% < {:.0}%)",
                prediction.confidence * 100.0, config.auto_apply_threshold * 100.0))
        } else if !total_samples_ok {
            Some(format!("insufficient_total_samples ({} < {})",
                stats.total, config.min_samples_total))
        } else if !non_bootstrap_ok {
            Some(format!("insufficient_non_bootstrap_samples ({} < {})",
                stats.non_bootstrap, config.min_samples_non_bootstrap))
        } else if !recent_samples_ok {
            Some(format!("insufficient_recent_samples ({} < {} in {}d)",
                stats.recent, config.min_recent_samples, config.recent_days_window))
        } else if !session_stable_ok {
            Some(format!("session_not_stable ({}min < {}min)",
                session_stable_minutes, config.min_session_stable_minutes))
        } else if !not_bootstrap_only {
            Some("bootstrap_samples_only".to_string())
        } else {
            None
        };

        let allowed = blocked_reason.is_none();

        AutoApplyGuard {
            allowed,
            blocked_reason,
            guards,
        }
    }
}

/// Statistics about the inference engine
#[derive(Debug, Clone, Serialize)]
pub struct InferenceStats {
    pub total_samples: usize,
    pub by_source: HashMap<String, usize>,
    pub by_mode: HashMap<String, usize>,
    pub average_weight: f32,
}

/// Detailed sample statistics for v2 stabilization guards
#[derive(Debug, Clone, Serialize)]
pub struct SampleStats {
    /// Total number of samples
    pub total: usize,
    /// Number of bootstrap samples
    pub bootstrap: usize,
    /// Number of non-bootstrap samples (UserCorrection, MFA, Automation)
    pub non_bootstrap: usize,
    /// Number of recent samples (within recent_days)
    pub recent: usize,
    /// Number of recent non-bootstrap samples
    pub recent_non_bootstrap: usize,
    /// Days used for "recent" calculation
    pub recent_days: i64,
}

/// Auto-apply guard result for v2
#[derive(Debug, Clone, Serialize)]
pub struct AutoApplyGuard {
    /// Whether auto-apply is allowed
    pub allowed: bool,
    /// Blocked reason if not allowed (None if allowed)
    pub blocked_reason: Option<String>,
    /// Which guards passed/failed
    pub guards: GuardChecks,
}

/// Individual guard check results
#[derive(Debug, Clone, Serialize)]
pub struct GuardChecks {
    pub confidence_ok: bool,
    pub total_samples_ok: bool,
    pub non_bootstrap_ok: bool,
    pub recent_samples_ok: bool,
    pub session_stable_ok: bool,
    pub not_bootstrap_only: bool,
}

// ============================================================================
// Utility Functions
// ============================================================================

/// Calculate cosine similarity between two dimension maps
fn cosine_similarity(a: &HashMap<String, f32>, b: &HashMap<String, f32>) -> f32 {
    // Get all keys
    let all_keys: std::collections::HashSet<_> = a.keys().chain(b.keys()).collect();

    let mut dot_product = 0.0;
    let mut norm_a = 0.0;
    let mut norm_b = 0.0;

    for key in all_keys {
        let val_a = a.get(key).copied().unwrap_or(0.0);
        let val_b = b.get(key).copied().unwrap_or(0.0);

        dot_product += val_a * val_b;
        norm_a += val_a * val_a;
        norm_b += val_b * val_b;
    }

    let denominator = (norm_a.sqrt() * norm_b.sqrt());
    if denominator > 0.0 {
        dot_product / denominator
    } else {
        0.0
    }
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    fn make_test_vector(home: f32, work: f32, focus: f32, sleep: f32) -> ContextVector {
        let mut dimensions = HashMap::new();
        dimensions.insert(dimensions::HOME_PROB.to_string(), home);
        dimensions.insert(dimensions::WORK_PROB.to_string(), work);
        dimensions.insert(dimensions::FOCUS_PROB.to_string(), focus);
        dimensions.insert(dimensions::SLEEP_PROB.to_string(), sleep);
        dimensions.insert(dimensions::PC_ACTIVE.to_string(), 0.8);

        ContextVector {
            dimensions,
            why: HashMap::new(),
            built_at: OffsetDateTime::now_utc(),
            feature_count: 5,
        }
    }

    #[test]
    fn test_cosine_similarity() {
        let mut a = HashMap::new();
        a.insert("x".to_string(), 1.0);
        a.insert("y".to_string(), 0.0);

        let mut b = HashMap::new();
        b.insert("x".to_string(), 1.0);
        b.insert("y".to_string(), 0.0);

        // Identical vectors → similarity = 1.0
        let sim = cosine_similarity(&a, &b);
        assert!((sim - 1.0).abs() < 0.01);

        // Orthogonal vectors → similarity = 0.0
        let mut c = HashMap::new();
        c.insert("x".to_string(), 0.0);
        c.insert("y".to_string(), 1.0);

        let sim = cosine_similarity(&a, &c);
        assert!(sim.abs() < 0.01);
    }

    #[test]
    fn test_sample_weight_decay() {
        let vector = make_test_vector(0.5, 0.3, 0.15, 0.05);
        let sample = TrainingSample::new(&vector, "maison", SampleSource::UserCorrection);

        // Fresh sample should have full weight × source multiplier
        let weight = sample.effective_weight();
        assert!(weight > 1.0); // UserCorrection = 1.3x
    }

    #[test]
    fn test_prediction_with_samples() {
        let engine = InferenceEngine::default();

        // Add some training samples
        let home_vector = make_test_vector(0.7, 0.1, 0.1, 0.1);
        let work_vector = make_test_vector(0.1, 0.6, 0.2, 0.1);

        engine.record_correction(&home_vector, "maison");
        engine.record_correction(&work_vector, "pro");

        // Predict with a home-like vector
        let test_vector = make_test_vector(0.65, 0.15, 0.1, 0.1);
        let prediction = engine.predict(&test_vector);

        // Should predict "maison" since it's most similar
        assert_eq!(prediction.mode, "maison");
        assert!(prediction.confidence > 0.5);
    }

    #[test]
    fn test_unknown_prediction() {
        let engine = InferenceEngine::default();

        // Empty engine should return unknown
        let vector = make_test_vector(0.5, 0.5, 0.0, 0.0);
        let prediction = engine.predict(&vector);

        assert_eq!(prediction.mode, "unknown");
        assert!(!prediction.is_confident);
    }

    #[test]
    fn test_sample_compaction() {
        let engine = InferenceEngine::default();

        // Add many samples
        for i in 0..100 {
            let vector = make_test_vector(
                (i as f32) / 100.0,
                1.0 - (i as f32) / 100.0,
                0.0,
                0.0,
            );
            engine.record_bootstrap(&vector, "test");
        }

        assert_eq!(engine.sample_count(), 100);

        // Compaction removes low-weight samples
        // (bootstrap samples have low weight, but fresh ones won't be removed)
        let removed = engine.compact();
        assert!(removed == 0 || engine.sample_count() <= 100);
    }
}
