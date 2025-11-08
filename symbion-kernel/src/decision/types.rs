// Types fondamentaux du Decision Engine
// Spec: PR3 P0 v3.1 REFINED

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use time::OffsetDateTime;

/// Niveau d'impact d'une action
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ImpactLevel {
    Low,       // Actions locales, reversibles, sans risque
    Medium,    // Impact modere sur environnement
    High,      // Critiques: securite, vie privee, argent
    VeryHigh,  // Actions physiques/securite (serrures, cameras, etc.)
}

/// Action a evaluer par le Decision Engine
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Action {
    pub action_type: String,
    pub agent_id: String,
    pub impact_level: ImpactLevel,
    pub trace_id: String,                       // Idempotence
    #[serde(with = "time::serde::rfc3339::option")]
    pub expires_at: Option<OffsetDateTime>,     // TTL
    pub dry_run: bool,                          // Evaluation sans execution
    pub expected_mode: Option<String>,          // Mode contextuel attendu
    pub expected_ssid: Option<String>,          // SSID attendu
}

/// Contexte de decision
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DecisionContext {
    pub mode: String,           // Mode contextuel actuel (Cravate, Intime, Neutre)
    pub ssid: String,           // SSID actuel
    pub agents: HashMap<String, AgentState>,  // Etats agents
}

/// Etat d'un agent
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentState {
    pub id: String,
    #[serde(with = "time::serde::rfc3339")]
    pub last_seen: OffsetDateTime,
    pub metrics: AgentMetrics,
    pub maintenance_mode: bool,
    #[serde(with = "time::serde::rfc3339::option")]
    pub last_reconnect: Option<OffsetDateTime>,
}

/// Metriques agent
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentMetrics {
    pub cpu_usage: f32,              // 0.0-1.0
    pub memory_usage_percent: f32,   // 0.0-1.0
}

/// Resultat d'une decision
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DecisionResult {
    pub decision_id: String,
    pub outcome: DecisionOutcome,
    pub trace_id: String,
    pub warnings: Vec<GuardWarning>,
}

/// Issue d'une decision
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum DecisionOutcome {
    Approved {
        trust_score: f32,
        auto: bool,
    },
    RequireValidation {
        reasons: Vec<String>,
        explanation_codes: Vec<String>,
        human_reasons: Vec<String>,
    },
    Blocked {
        reasons: Vec<String>,
        explanation_codes: Vec<String>,
    },
    DryRun {
        would_approve: bool,
        trust_score: f32,
        threshold: f32,
        guards_passed: bool,
        blocks: Vec<GuardBlock>,
    },
}

/// Avertissement d'un guard
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GuardWarning {
    pub reason: String,
    pub explanation_code: String,
    pub human_reason: String,
}

/// Blocage d'un guard
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GuardBlock {
    pub reason: String,
    pub explanation_code: String,
    pub human_reason: String,
}

/// Validation requise d'un guard
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GuardRequire {
    pub reason: String,
    pub explanation_code: String,
    pub human_reason: String,
}

/// Evaluation de tous les guards
#[derive(Debug, Clone)]
pub struct GuardsEvaluation {
    pub passed: bool,
    pub blocks: Vec<GuardBlock>,
    pub requires_validation: Vec<GuardRequire>,
    pub warnings: Vec<GuardWarning>,
}

/// Criteres de calcul trust score
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TrustCriteria {
    pub context_match: f32,          // 0.0-1.0
    pub temporal_consistency: f32,   // 0.0-1.0
    pub agent_health: f32,           // 0.0-1.0
    pub recent_success_rate: f32,    // 0.0-1.0
    pub user_approval_history: f32,  // 0.0-1.0
}

/// Trust score calcule
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TrustScore {
    pub score: f32,
    pub breakdown: TrustCriteria,
    pub config_version: u64,
}

/// Record de decision persiste
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DecisionRecord {
    pub decision_id: String,
    pub trace_id: String,
    pub action_type: String,
    pub agent_id: String,
    pub impact_level: ImpactLevel,
    pub outcome: DecisionOutcome,
    pub trust_score: Option<TrustScore>,
    #[serde(with = "time::serde::rfc3339")]
    pub timestamp: OffsetDateTime,
    pub config_version: u64,
}

/// Configuration de decision (hot-reloadable)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DecisionConfig {
    pub version: u64,
    pub trust_weights: TrustWeights,
    pub impact_thresholds: ImpactThresholds,
    pub agent_health_mapping: AgentHealthMapping,
}

/// Poids trust score
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TrustWeights {
    pub context_match: f32,
    pub temporal_consistency: f32,
    pub agent_health: f32,
    pub recent_success_rate: f32,
    pub user_approval_history: f32,
}

/// Seuils par impact level
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ImpactThresholds {
    pub low: f32,
    pub medium: f32,
    pub high: f32,
    pub very_high: f32,  // > 1.0 = impossible auto-approve
}

/// Mapping agent health
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct AgentHealthMapping {
    pub online_min_score: f32,
    pub active_min_score: f32,
    pub idle_min_score: f32,
    pub degraded_min_score: f32,
    pub degraded_consecutive_threshold: u32,
    pub stale_max_age_secs: u64,
}

impl Default for DecisionConfig {
    fn default() -> Self {
        Self {
            version: 1,
            trust_weights: TrustWeights {
                context_match: 0.25,
                temporal_consistency: 0.20,
                agent_health: 0.25,
                recent_success_rate: 0.15,
                user_approval_history: 0.15,
            },
            impact_thresholds: ImpactThresholds {
                low: 0.3,
                medium: 0.7,
                high: 0.9,
                very_high: 1.1,  // Impossible auto-approve
            },
            agent_health_mapping: AgentHealthMapping {
                online_min_score: 0.9,
                active_min_score: 0.85,
                idle_min_score: 0.7,
                degraded_min_score: 0.5,
                degraded_consecutive_threshold: 3,
                stale_max_age_secs: 300,
            },
        }
    }
}
