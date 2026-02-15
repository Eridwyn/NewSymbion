//! State management for SSL plugin
//!
//! Handles dynamic domain storage with persistence

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::Path;
use tokio::sync::RwLock;

/// Path for state persistence
const DEFAULT_STATE_PATH: &str = "/opt/symbion/data/ssl-domains.json";

/// Dynamic domain configuration (can be added/modified via API)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DynamicDomain {
    /// Unique identifier (slug-friendly)
    pub id: String,

    /// Domain hostname (e.g., "www.markcha.fr")
    pub hostname: String,

    /// Port to check (default: 443)
    #[serde(default = "default_port")]
    pub port: u16,

    /// Display label for PWA
    #[serde(default)]
    pub label: Option<String>,

    /// Days before expiry to trigger warning (yellow)
    #[serde(default = "default_warning_days")]
    pub warning_days: i64,

    /// Days before expiry to trigger critical (red)
    #[serde(default = "default_critical_days")]
    pub critical_days: i64,

    /// Whether to check HTTP health too
    #[serde(default = "default_check_http")]
    pub check_http: bool,

    /// Whether this domain is enabled for monitoring
    #[serde(default = "default_enabled")]
    pub enabled: bool,

    /// Creation timestamp
    #[serde(default = "default_timestamp")]
    pub created_at: String,

    /// Last modification timestamp
    #[serde(default = "default_timestamp")]
    pub updated_at: String,

    /// Certificate fingerprint (SHA256) for change detection
    #[serde(default)]
    pub last_fingerprint: Option<String>,
}

fn default_port() -> u16 { 443 }
fn default_warning_days() -> i64 { 30 }
fn default_critical_days() -> i64 { 14 }
fn default_check_http() -> bool { true }
fn default_enabled() -> bool { true }
fn default_timestamp() -> String { chrono::Utc::now().to_rfc3339() }

/// Request to create a new domain
#[derive(Debug, Clone, Deserialize)]
pub struct CreateDomainRequest {
    /// Unique identifier (optional, will be derived from hostname if not provided)
    pub id: Option<String>,

    /// Domain hostname (required)
    pub hostname: String,

    /// Port to check (optional, default: 443)
    pub port: Option<u16>,

    /// Display label (optional)
    pub label: Option<String>,

    /// Warning threshold in days (optional, default: 30)
    pub warning_days: Option<i64>,

    /// Critical threshold in days (optional, default: 14)
    pub critical_days: Option<i64>,

    /// Check HTTP health (optional, default: true)
    pub check_http: Option<bool>,
}

/// Request to update a domain
#[derive(Debug, Clone, Deserialize)]
pub struct UpdateDomainRequest {
    /// Domain hostname (optional)
    pub hostname: Option<String>,

    /// Port to check (optional)
    pub port: Option<u16>,

    /// Display label (optional)
    pub label: Option<String>,

    /// Warning threshold in days (optional)
    pub warning_days: Option<i64>,

    /// Critical threshold in days (optional)
    pub critical_days: Option<i64>,

    /// Check HTTP health (optional)
    pub check_http: Option<bool>,

    /// Enabled status (optional)
    pub enabled: Option<bool>,
}

/// State file structure
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct StateFile {
    /// Version for migrations
    pub version: u32,

    /// Dynamic domains
    pub domains: HashMap<String, DynamicDomain>,

    /// Global default thresholds
    #[serde(default)]
    pub default_warning_days: Option<i64>,

    #[serde(default)]
    pub default_critical_days: Option<i64>,
}

/// Domain state manager
pub struct DomainState {
    /// File path for persistence
    path: String,

    /// In-memory state
    state: RwLock<StateFile>,

    /// Dirty flag for debounced saving
    dirty: std::sync::atomic::AtomicBool,
}

impl DomainState {
    /// Load state from file or create new
    pub fn load(path: Option<&str>) -> Result<Self> {
        let path = path.unwrap_or(DEFAULT_STATE_PATH).to_string();

        let state = if Path::new(&path).exists() {
            let content = std::fs::read_to_string(&path)
                .with_context(|| format!("Failed to read state file: {}", path))?;
            serde_json::from_str(&content)
                .with_context(|| "Failed to parse state file")?
        } else {
            StateFile {
                version: 1,
                domains: HashMap::new(),
                default_warning_days: Some(30),
                default_critical_days: Some(14),
            }
        };

        Ok(Self {
            path,
            state: RwLock::new(state),
            dirty: std::sync::atomic::AtomicBool::new(false),
        })
    }

    /// Import domains from static config
    pub async fn import_from_config(&self, domains: &HashMap<String, crate::config::DomainConfig>, alerts: &crate::config::AlertConfig) {
        let mut state = self.state.write().await;

        for (id, config) in domains {
            // Only import if not already present (don't overwrite dynamic changes)
            if !state.domains.contains_key(id) {
                state.domains.insert(id.clone(), DynamicDomain {
                    id: id.clone(),
                    hostname: config.hostname.clone(),
                    port: config.port,
                    label: config.label.clone(),
                    warning_days: alerts.warning_days,
                    critical_days: alerts.critical_days,
                    check_http: config.check_http,
                    enabled: true,
                    created_at: chrono::Utc::now().to_rfc3339(),
                    updated_at: chrono::Utc::now().to_rfc3339(),
                    last_fingerprint: None,
                });
            }
        }

        self.dirty.store(true, std::sync::atomic::Ordering::SeqCst);
    }

    /// Save state to file
    pub async fn save(&self) -> Result<()> {
        if !self.dirty.load(std::sync::atomic::Ordering::SeqCst) {
            return Ok(());
        }

        let state = self.state.read().await;
        let content = serde_json::to_string_pretty(&*state)?;

        // Ensure parent directory exists
        if let Some(parent) = Path::new(&self.path).parent() {
            std::fs::create_dir_all(parent)?;
        }

        std::fs::write(&self.path, content)
            .with_context(|| format!("Failed to write state file: {}", self.path))?;

        self.dirty.store(false, std::sync::atomic::Ordering::SeqCst);
        Ok(())
    }

    /// Get all domains
    pub async fn list_domains(&self) -> Vec<DynamicDomain> {
        let state = self.state.read().await;
        state.domains.values().cloned().collect()
    }

    /// Get enabled domains only
    pub async fn list_enabled_domains(&self) -> Vec<DynamicDomain> {
        let state = self.state.read().await;
        state.domains.values()
            .filter(|d| d.enabled)
            .cloned()
            .collect()
    }

    /// Get a specific domain
    pub async fn get_domain(&self, id: &str) -> Option<DynamicDomain> {
        let state = self.state.read().await;
        state.domains.get(id).cloned()
    }

    /// Create a new domain
    pub async fn create_domain(&self, req: CreateDomainRequest) -> Result<DynamicDomain> {
        let mut state = self.state.write().await;

        // Generate ID from hostname if not provided
        let id = req.id.unwrap_or_else(|| {
            req.hostname
                .replace('.', "-")
                .replace(':', "-")
                .to_lowercase()
        });

        // Check for duplicates
        if state.domains.contains_key(&id) {
            anyhow::bail!("Domain with ID '{}' already exists", id);
        }

        // Check for duplicate hostname
        if state.domains.values().any(|d| d.hostname == req.hostname && d.port == req.port.unwrap_or(443)) {
            anyhow::bail!("Domain '{}:{}' already exists", req.hostname, req.port.unwrap_or(443));
        }

        let now = chrono::Utc::now().to_rfc3339();
        let domain = DynamicDomain {
            id: id.clone(),
            hostname: req.hostname,
            port: req.port.unwrap_or(443),
            label: req.label,
            warning_days: req.warning_days.unwrap_or(state.default_warning_days.unwrap_or(30)),
            critical_days: req.critical_days.unwrap_or(state.default_critical_days.unwrap_or(14)),
            check_http: req.check_http.unwrap_or(true),
            enabled: true,
            created_at: now.clone(),
            updated_at: now,
            last_fingerprint: None,
        };

        state.domains.insert(id, domain.clone());
        self.dirty.store(true, std::sync::atomic::Ordering::SeqCst);

        Ok(domain)
    }

    /// Update an existing domain
    pub async fn update_domain(&self, id: &str, req: UpdateDomainRequest) -> Result<DynamicDomain> {
        let mut state = self.state.write().await;

        let domain = state.domains.get_mut(id)
            .ok_or_else(|| anyhow::anyhow!("Domain '{}' not found", id))?;

        // Apply updates
        if let Some(hostname) = req.hostname {
            domain.hostname = hostname;
        }
        if let Some(port) = req.port {
            domain.port = port;
        }
        if let Some(label) = req.label {
            domain.label = Some(label);
        }
        if let Some(warning_days) = req.warning_days {
            domain.warning_days = warning_days;
        }
        if let Some(critical_days) = req.critical_days {
            domain.critical_days = critical_days;
        }
        if let Some(check_http) = req.check_http {
            domain.check_http = check_http;
        }
        if let Some(enabled) = req.enabled {
            domain.enabled = enabled;
        }

        domain.updated_at = chrono::Utc::now().to_rfc3339();
        self.dirty.store(true, std::sync::atomic::Ordering::SeqCst);

        Ok(domain.clone())
    }

    /// Delete a domain
    pub async fn delete_domain(&self, id: &str) -> Result<DynamicDomain> {
        let mut state = self.state.write().await;

        let domain = state.domains.remove(id)
            .ok_or_else(|| anyhow::anyhow!("Domain '{}' not found", id))?;

        self.dirty.store(true, std::sync::atomic::Ordering::SeqCst);

        Ok(domain)
    }

    /// Update fingerprint for a domain
    pub async fn update_fingerprint(&self, id: &str, fingerprint: &str) -> Result<()> {
        let mut state = self.state.write().await;

        if let Some(domain) = state.domains.get_mut(id) {
            domain.last_fingerprint = Some(fingerprint.to_string());
            domain.updated_at = chrono::Utc::now().to_rfc3339();
            self.dirty.store(true, std::sync::atomic::Ordering::SeqCst);
        }

        Ok(())
    }

    /// Check if fingerprint changed (returns old fingerprint if changed)
    pub async fn check_fingerprint_change(&self, id: &str, new_fingerprint: &str) -> Option<String> {
        let state = self.state.read().await;

        if let Some(domain) = state.domains.get(id) {
            if let Some(ref old) = domain.last_fingerprint {
                if old != new_fingerprint {
                    return Some(old.clone());
                }
            }
        }

        None
    }
}
