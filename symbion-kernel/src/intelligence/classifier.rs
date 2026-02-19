//! Process Classifier
//!
//! Categorizes running processes into semantic categories for context intelligence.
//! Uses external TOML configuration - NO hardcoded app names in core.
//!
//! ## Categories
//!
//! - `ide`: Development environments (JetBrains, VSCode, etc.)
//! - `media`: Entertainment apps (Netflix, Kodi, Spotify, etc.)
//! - `communication`: Chat/meeting apps (Slack, Teams, Discord, etc.)
//! - `browser`: Web browsers (neutral - not work or leisure specific)
//! - `gaming`: Games and game launchers
//!
//! ## Architecture
//!
//! ```text
//! Agent Heartbeat → process.list → Classifier → process.category.* features
//! ```

use std::collections::HashMap;
use std::path::Path;
use std::sync::Arc;
use parking_lot::RwLock;
use serde::{Deserialize, Serialize};

use super::features::{FeatureRegistry, FeatureValue, feature_ids, ttl};

// ============================================================================
// Configuration
// ============================================================================

/// Process classification rules loaded from TOML
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct ClassifierConfig {
    /// IDE/development tools
    #[serde(default)]
    pub ide: CategoryRules,

    /// Media/entertainment
    #[serde(default)]
    pub media: CategoryRules,

    /// Communication/collaboration
    #[serde(default)]
    pub communication: CategoryRules,

    /// Web browsers (neutral)
    #[serde(default)]
    pub browser: CategoryRules,

    /// Gaming
    #[serde(default)]
    pub gaming: CategoryRules,

    /// Office/productivity
    #[serde(default)]
    pub office: CategoryRules,
}

/// Rules for a single category
#[derive(Debug, Clone, Default, Deserialize, Serialize)]
pub struct CategoryRules {
    /// Process name patterns (case-insensitive substring match)
    #[serde(default)]
    pub patterns: Vec<String>,

    /// Exact process names (case-insensitive)
    #[serde(default)]
    pub exact: Vec<String>,

    /// Weight for this category in context decisions (default 1.0)
    #[serde(default = "default_weight")]
    pub weight: f32,
}

fn default_weight() -> f32 {
    1.0
}

impl ClassifierConfig {
    /// Check if any category weight is invalid (negative or > 2.0)
    fn has_invalid_weights(&self) -> bool {
        let weights = [self.ide.weight, self.gaming.weight, self.communication.weight, self.media.weight, self.browser.weight, self.office.weight];
        weights.iter().any(|w| *w < 0.0 || *w > 2.0)
    }
}

impl Default for ClassifierConfig {
    fn default() -> Self {
        Self {
            ide: CategoryRules {
                patterns: vec![
                    // JetBrains family
                    "jetbrains".to_string(),
                    "intellij".to_string(),
                    "pycharm".to_string(),
                    "webstorm".to_string(),
                    "phpstorm".to_string(),
                    "rustrover".to_string(),
                    "rider".to_string(),
                    "goland".to_string(),
                    "clion".to_string(),
                    "datagrip".to_string(),
                    // Other IDEs
                    "vscode".to_string(),
                    "code".to_string(),
                    "sublime".to_string(),
                    "atom".to_string(),
                    "vim".to_string(),
                    "nvim".to_string(),
                    "emacs".to_string(),
                    "android-studio".to_string(),
                    "xcode".to_string(),
                ],
                exact: vec![],
                weight: 1.0,
            },
            media: CategoryRules {
                patterns: vec![
                    "netflix".to_string(),
                    "kodi".to_string(),
                    "plex".to_string(),
                    "vlc".to_string(),
                    "mpv".to_string(),
                    "stremio".to_string(),
                    "jellyfin".to_string(),
                    "prime video".to_string(),
                    "disney".to_string(),
                ],
                exact: vec![],
                weight: 1.0,
            },
            communication: CategoryRules {
                patterns: vec![
                    "slack".to_string(),
                    "teams".to_string(),
                    "zoom".to_string(),
                    "meet".to_string(),
                    "webex".to_string(),
                    "skype".to_string(),
                ],
                exact: vec![],
                weight: 0.7, // Lower weight - can be work or personal
            },
            browser: CategoryRules {
                patterns: vec![
                    "firefox".to_string(),
                    "chrome".to_string(),
                    "chromium".to_string(),
                    "brave".to_string(),
                    "edge".to_string(),
                    "safari".to_string(),
                    "opera".to_string(),
                ],
                exact: vec![],
                weight: 0.3, // Neutral - browsers are ambiguous
            },
            gaming: CategoryRules {
                patterns: vec![
                    "steam".to_string(),
                    "epic".to_string(),
                    "gog".to_string(),
                    "lutris".to_string(),
                    "heroic".to_string(),
                    "retroarch".to_string(),
                ],
                exact: vec![],
                weight: 1.0,
            },
            office: CategoryRules {
                patterns: vec![
                    "libreoffice".to_string(),
                    "word".to_string(),
                    "excel".to_string(),
                    "powerpoint".to_string(),
                    "obsidian".to_string(),
                    "notion".to_string(),
                    "evernote".to_string(),
                    "onenote".to_string(),
                ],
                exact: vec![],
                weight: 0.8,
            },
        }
    }
}

impl ClassifierConfig {
    /// Load config from TOML file, or return defaults
    pub fn load_or_default(path: &Path) -> Self {
        match std::fs::read_to_string(path) {
            Ok(content) => {
                match toml::from_str::<Self>(&content) {
                    Ok(config) => {
                        if config.has_invalid_weights() {
                            eprintln!("[classifier] invalid weights in {}, using defaults", path.display());
                            return Self::default();
                        }
                        eprintln!("[classifier] loaded config from {}", path.display());
                        config
                    }
                    Err(e) => {
                        eprintln!("[classifier] failed to parse {}: {}, using defaults", path.display(), e);
                        Self::default()
                    }
                }
            }
            Err(_) => {
                eprintln!("[classifier] no config at {}, using defaults", path.display());
                Self::default()
            }
        }
    }

    /// Save current config to TOML file
    pub fn save(&self, path: &Path) -> Result<(), std::io::Error> {
        let content = toml::to_string_pretty(self)
            .map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e))?;
        std::fs::write(path, content)
    }
}

// ============================================================================
// Classification Result
// ============================================================================

/// Result of classifying a process list
#[derive(Debug, Clone, Default, Serialize)]
pub struct ClassificationResult {
    /// Active categories with their confidence scores
    pub categories: HashMap<String, CategoryMatch>,

    /// Number of processes analyzed
    pub process_count: usize,

    /// Number of processes matched
    pub matched_count: usize,
}

/// A matched category
#[derive(Debug, Clone, Serialize)]
pub struct CategoryMatch {
    /// Whether this category is active
    pub active: bool,

    /// Confidence score (0.0-1.0)
    pub confidence: f32,

    /// Matching process names
    pub matching_processes: Vec<String>,
}

// ============================================================================
// Process Classifier
// ============================================================================

/// Classifies processes into categories
#[derive(Debug)]
pub struct ProcessClassifier {
    config: RwLock<ClassifierConfig>,
    config_path: Option<std::path::PathBuf>,
}

/// Thread-safe shared classifier
pub type SharedProcessClassifier = Arc<ProcessClassifier>;

impl Default for ProcessClassifier {
    fn default() -> Self {
        Self {
            config: RwLock::new(ClassifierConfig::default()),
            config_path: None,
        }
    }
}

impl ProcessClassifier {
    /// Create a new classifier with config from path
    pub fn new(config_path: std::path::PathBuf) -> Self {
        let config = ClassifierConfig::load_or_default(&config_path);
        Self {
            config: RwLock::new(config),
            config_path: Some(config_path),
        }
    }

    /// Get current config
    pub fn config(&self) -> ClassifierConfig {
        self.config.read().clone()
    }

    /// Update config
    pub fn update_config(&self, config: ClassifierConfig) {
        *self.config.write() = config;
    }

    /// Reload config from file
    pub fn reload_config(&self) {
        if let Some(ref path) = self.config_path {
            let config = ClassifierConfig::load_or_default(path);
            *self.config.write() = config;
            eprintln!("[classifier] config reloaded");
        }
    }

    /// Classify a list of process names
    pub fn classify(&self, processes: &[String]) -> ClassificationResult {
        let config = self.config.read();
        let mut result = ClassificationResult {
            process_count: processes.len(),
            ..Default::default()
        };

        // Check each category
        let categories = [
            ("ide", &config.ide),
            ("media", &config.media),
            ("communication", &config.communication),
            ("browser", &config.browser),
            ("gaming", &config.gaming),
            ("office", &config.office),
        ];

        for (name, rules) in categories {
            let matching = self.find_matches(processes, rules);
            let active = !matching.is_empty();
            let confidence = if active {
                // Confidence based on number of matches and category weight
                // Normalize by typical active process count per category
                const PROCESS_COUNT_NORMALIZATION: f32 = 3.0;
                let base = (matching.len() as f32 / PROCESS_COUNT_NORMALIZATION).min(1.0);
                base * rules.weight
            } else {
                0.0
            };

            if active {
                result.matched_count += matching.len();
            }

            result.categories.insert(name.to_string(), CategoryMatch {
                active,
                confidence,
                matching_processes: matching,
            });
        }

        result
    }

    /// Find processes matching a category's rules
    fn find_matches(&self, processes: &[String], rules: &CategoryRules) -> Vec<String> {
        let mut matches = Vec::new();

        for process in processes {
            let lower = process.to_lowercase();

            // Check exact matches
            if rules.exact.iter().any(|e| e.to_lowercase() == lower) {
                matches.push(process.clone());
                continue;
            }

            // Check pattern matches
            if rules.patterns.iter().any(|p| lower.contains(&p.to_lowercase())) {
                matches.push(process.clone());
            }
        }

        matches
    }

    /// Classify and update feature registry.
    /// A process can match multiple categories (e.g. Slack = communication + productivity) — by design.
    pub fn classify_and_update(&self, processes: &[String], registry: &FeatureRegistry) {
        let result = self.classify(processes);

        // Update features for each category
        for (category, match_info) in &result.categories {
            let feature_id = format!("process.category.{}", category);

            registry.set_feature(
                &feature_id,
                FeatureValue::Bool(match_info.active),
                "classifier.process",
                match_info.confidence.max(0.5), // Minimum 0.5 confidence when detected
                ttl::PROCESS, // 60 seconds TTL
            );
        }

        // Also set summary features
        registry.set_feature(
            feature_ids::PROCESS_WORK_ACTIVE,
            FeatureValue::Bool(
                result.categories.get("ide").map(|c| c.active).unwrap_or(false) ||
                result.categories.get("office").map(|c| c.active).unwrap_or(false)
            ),
            "classifier.process",
            0.8,
            ttl::PROCESS,
        );

        registry.set_feature(
            feature_ids::PROCESS_LEISURE_ACTIVE,
            FeatureValue::Bool(
                result.categories.get("media").map(|c| c.active).unwrap_or(false) ||
                result.categories.get("gaming").map(|c| c.active).unwrap_or(false)
            ),
            "classifier.process",
            0.8,
            ttl::PROCESS,
        );
    }
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_classify_ide() {
        let classifier = ProcessClassifier::default();
        let processes = vec![
            "rustrover".to_string(),
            "firefox".to_string(),
            "slack".to_string(),
        ];

        let result = classifier.classify(&processes);

        assert!(result.categories.get("ide").unwrap().active);
        assert!(result.categories.get("browser").unwrap().active);
        assert!(result.categories.get("communication").unwrap().active);
        assert!(!result.categories.get("media").unwrap().active);
    }

    #[test]
    fn test_classify_media() {
        let classifier = ProcessClassifier::default();
        let processes = vec!["netflix".to_string(), "vlc".to_string()];

        let result = classifier.classify(&processes);

        assert!(result.categories.get("media").unwrap().active);
        assert!(!result.categories.get("ide").unwrap().active);
    }

    #[test]
    fn test_case_insensitive() {
        let classifier = ProcessClassifier::default();
        let processes = vec!["RUSTROVER".to_string(), "FireFox".to_string()];

        let result = classifier.classify(&processes);

        assert!(result.categories.get("ide").unwrap().active);
        assert!(result.categories.get("browser").unwrap().active);
    }

    #[test]
    fn test_default_config() {
        let config = ClassifierConfig::default();
        assert!(!config.ide.patterns.is_empty());
        assert!(!config.media.patterns.is_empty());
    }
}
