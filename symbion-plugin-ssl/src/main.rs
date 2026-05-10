//! Symbion SSL Plugin v2
//!
//! Monitors SSL certificates and domain availability:
//! - Certificate expiry dates
//! - Certificate validity
//! - Domain online status
//! - Dynamic domain management via API
//! - Per-domain alert thresholds
//! - Fingerprint tracking for change detection
//! - Publishes features for Intelligence v2 automations

mod config;
mod mqtt;
mod ssl;
mod state;

use anyhow::{Context, Result};
use axum::{
    extract::{Path, State},
    http::StatusCode,
    routing::{delete, get, post, put},
    Json, Router,
};
use serde::Serialize;
use std::sync::Arc;
use symbion_plugin_common::PluginHttpServer;
use tokio::sync::{broadcast, RwLock};
use tokio::time::{interval_at, Duration, Instant};
use tracing::{error, info, warn, Level};
use tracing_subscriber::EnvFilter;

use config::Config;
use mqtt::{DomainStatus, MqttPublisher};
use ssl::{CertificateStatus, SslChecker};
use state::{CreateDomainRequest, DomainState, DynamicDomain, UpdateDomainRequest};

/// Plugin constants
const PLUGIN_ID: &str = "ssl";
const SPEC_VERSION: &str = "2.0";

/// Plugin state shared across tasks
struct PluginState {
    config: Config,
    checker: SslChecker,
    mqtt: MqttPublisher,
    domains: Arc<DomainState>,
    domain_statuses: RwLock<std::collections::HashMap<String, DomainStatus>>,
    health: RwLock<InternalHealth>,
    started_at: std::time::Instant,
    /// Channel to trigger immediate SSL check
    check_trigger: broadcast::Sender<()>,
}

/// Internal health tracking
#[derive(Debug, Clone, Default)]
struct InternalHealth {
    mqtt_connected: bool,
    last_ssl_check: Option<String>,
    last_online_check: Option<String>,
    domains_total: usize,
    domains_enabled: usize,
    domains_valid: usize,
    domains_expiring: usize,
    error: Option<String>,
}

/// Health response for kernel discovery (standard format)
#[derive(Debug, Clone, Serialize)]
struct HealthResponse {
    plugin_id: String,
    spec_version: String,
    status: String,
    uptime_seconds: u64,
    domains_total: usize,
    domains_enabled: usize,
}

/// Domains list response
#[derive(Debug, Clone, Serialize)]
struct DomainsResponse {
    domains: Vec<DomainWithStatus>,
    summary: SummaryInfo,
}

#[derive(Debug, Clone, Serialize)]
struct DomainWithStatus {
    #[serde(flatten)]
    config: DynamicDomain,
    #[serde(flatten)]
    status: Option<DomainStatus>,
}

#[derive(Debug, Clone, Serialize)]
struct SummaryInfo {
    total: usize,
    enabled: usize,
    valid: usize,
    expiring_soon: usize,
    critical: usize,
}

/// API error response
#[derive(Debug, Serialize)]
struct ApiError {
    error: String,
    code: String,
}

impl ApiError {
    fn new(error: impl Into<String>, code: impl Into<String>) -> Self {
        Self {
            error: error.into(),
            code: code.into(),
        }
    }
}

type ApiResult<T> = Result<Json<T>, (StatusCode, Json<ApiError>)>;

#[tokio::main]
async fn main() -> Result<()> {
    // Initialize logging
    tracing_subscriber::fmt()
        .with_env_filter(
            EnvFilter::from_default_env()
                .add_directive(Level::INFO.into())
                .add_directive("symbion_plugin_ssl=debug".parse().unwrap()),
        )
        .init();

    info!("Starting Symbion SSL Plugin v{} (spec {})", env!("CARGO_PKG_VERSION"), SPEC_VERSION);

    // Load configuration
    let config_path = std::env::var("SSL_CONFIG")
        .unwrap_or_else(|_| "/opt/symbion/config/ssl.toml".to_string());

    let config = Config::load(&config_path)
        .with_context(|| format!("Failed to load config from {}", config_path))?;

    info!("Loaded configuration from {}", config_path);

    // Load or create domain state
    let state_path = std::env::var("SSL_STATE")
        .unwrap_or_else(|_| "/opt/symbion/data/ssl-domains.json".to_string());

    let domain_state = Arc::new(DomainState::load(Some(&state_path))
        .with_context(|| format!("Failed to load state from {}", state_path))?);

    // Import static config domains into dynamic state
    domain_state.import_from_config(&config.domains, &config.alerts).await;
    domain_state.save().await?;

    let enabled_count = domain_state.list_enabled_domains().await.len();
    let total_count = domain_state.list_domains().await.len();
    info!("Managing {} domains ({} enabled)", total_count, enabled_count);

    // Initialize SSL checker
    let checker = SslChecker::new();

    // Initialize MQTT publisher
    info!("Connecting to MQTT broker {}:{}...", config.mqtt.host, config.mqtt.port);
    let mqtt = MqttPublisher::connect(&config.mqtt)
        .await
        .context("Failed to connect to MQTT broker")?;

    info!("MQTT connected");

    // Create check trigger channel
    let (check_trigger, _) = broadcast::channel::<()>(16);

    // Create shared state
    let state = Arc::new(PluginState {
        config: config.clone(),
        checker,
        mqtt,
        domains: domain_state,
        domain_statuses: RwLock::new(std::collections::HashMap::new()),
        health: RwLock::new(InternalHealth {
            mqtt_connected: true,
            domains_total: total_count,
            domains_enabled: enabled_count,
            ..Default::default()
        }),
        started_at: std::time::Instant::now(),
        check_trigger,
    });

    // Publish manifest
    let manifest = include_str!("../manifest.json");
    state.mqtt.publish_manifest(manifest).await?;
    info!("Manifest published to symbion/plugins/ssl/manifest");

    // Publish initial health
    state.mqtt.publish_health(true, "Plugin started (v2)").await?;

    // Start polling tasks
    let ssl_handle = tokio::spawn(ssl_check_loop(Arc::clone(&state)));
    let online_handle = tokio::spawn(online_check_loop(Arc::clone(&state)));
    let save_handle = tokio::spawn(periodic_save(Arc::clone(&state)));

    // Start HTTP API server
    let api_handle = tokio::spawn(api_server(Arc::clone(&state)));

    // Register with kernel + actions templates
    let socket_str = state.config.http.socket_path.clone();
    tokio::spawn(async move {
        use symbion_plugin_common::{PluginAction, PluginRegistrationBuilder};

        // Wait for socket to be ready
        tokio::time::sleep(std::time::Duration::from_secs(2)).await;

        if let Err(e) = PluginRegistrationBuilder::new(PLUGIN_ID, &socket_str)
            .route("/health")
            .route("/domains")
            .route("/check")
            .version(env!("CARGO_PKG_VERSION"))
            .description("SSL/TLS certificate monitoring")
            .action(PluginAction {
                name: "check_now".into(),
                label: "Vérifier les certificats maintenant".into(),
                description: Some("Force une vérification immédiate de tous les domaines (au lieu d'attendre le prochain cycle).".into()),
                icon: Some("🔍".into()),
                route: "check".into(),
                method: "POST".into(),
                impact_level: "Low".into(),
                wrap_protocol: None,  // route directe, pas Contract v1.0
                params: vec![],
            })
            .register()
            .await
        {
            warn!("Failed to register with kernel: {}", e);
        } else {
            info!("Registered with kernel + 1 action template");
        }
    });

    info!("All tasks started, plugin running");

    // Wait for any task to complete or graceful shutdown signal
    tokio::select! {
        r = ssl_handle => {
            error!("SSL check loop exited: {:?}", r);
        }
        r = online_handle => {
            error!("Online check loop exited: {:?}", r);
        }
        r = save_handle => {
            error!("Save loop exited: {:?}", r);
        }
        r = api_handle => {
            error!("API server exited: {:?}", r);
        }
        _ = tokio::signal::ctrl_c() => {
            info!("[ssl] Received SIGINT (Ctrl+C), shutting down gracefully...");
        }
        _ = async {
            #[cfg(unix)]
            {
                let mut sigterm = tokio::signal::unix::signal(tokio::signal::unix::SignalKind::terminate())
                    .expect("failed to install SIGTERM handler");
                sigterm.recv().await;
            }
            #[cfg(not(unix))]
            {
                // On non-Unix platforms, just wait forever (ctrl_c branch handles shutdown)
                std::future::pending::<()>().await;
            }
        } => {
            info!("[ssl] Received SIGTERM, shutting down gracefully...");
        }
    }

    // Final state save before exit
    if let Err(e) = state.domains.save().await {
        error!("Failed to save state on shutdown: {}", e);
    }
    info!("[ssl] Shutdown complete");

    Ok(())
}

/// SSL certificate check loop
async fn ssl_check_loop(state: Arc<PluginState>) {
    // Subscribe to trigger channel
    let mut trigger_rx = state.check_trigger.subscribe();

    // Run immediately on start, then at configured interval
    let mut ticker = interval_at(
        Instant::now(),
        Duration::from_secs(state.config.polling.ssl_seconds),
    );

    loop {
        tokio::select! {
            _ = ticker.tick() => {
                run_ssl_check(&state).await;
            }
            _ = trigger_rx.recv() => {
                info!("Manual SSL check triggered");
                run_ssl_check(&state).await;
            }
        }
    }
}

/// Run SSL check for all enabled domains
async fn run_ssl_check(state: &Arc<PluginState>) {
    info!("Running SSL certificate check...");

    let domains = state.domains.list_enabled_domains().await;

    let mut valid_count = 0;
    let mut expiring_count = 0;
    let mut statuses = Vec::new();

    for domain in &domains {
        info!("Checking {} ({}:{})", domain.id, domain.hostname, domain.port);

        // Run SSL check in blocking context (native-tls is sync)
        let hostname = domain.hostname.clone();
        let port = domain.port;
        let checker = SslChecker::new();

        let cert_status = tokio::task::spawn_blocking(move || {
            checker.check(&hostname, port)
        })
        .await
        .unwrap_or_else(|e| CertificateStatus {
            hostname: domain.hostname.clone(),
            port: domain.port,
            valid: false,
            expiry_date: None,
            days_remaining: None,
            issuer: None,
            subject: None,
            fingerprint: None,
            error: Some(format!("Task error: {}", e)),
            checked_at: chrono::Utc::now(),
        });

        // Log result
        if cert_status.valid {
            valid_count += 1;
            if let Some(days) = cert_status.days_remaining {
                if days <= domain.warning_days {
                    expiring_count += 1;
                    warn!(
                        "{}: Certificate expires in {} days (warning threshold: {})",
                        domain.hostname, days, domain.warning_days
                    );
                } else {
                    info!(
                        "{}: Certificate valid ({} days remaining)",
                        domain.hostname, days
                    );
                }
            }
        } else {
            error!(
                "{}: Certificate INVALID - {:?}",
                domain.hostname, cert_status.error
            );
        }

        // Check fingerprint change
        if let Some(ref fingerprint) = cert_status.fingerprint {
            if let Some(old_fp) = state.domains.check_fingerprint_change(&domain.id, fingerprint).await {
                warn!(
                    "{}: Certificate fingerprint CHANGED! Old: {}..., New: {}...",
                    domain.hostname,
                    &old_fp[..16],
                    &fingerprint[..16]
                );
                if let Err(e) = state.mqtt.publish_fingerprint_change(
                    &domain.id, &domain.hostname, &old_fp, fingerprint,
                ).await {
                    warn!("Failed to publish fingerprint change: {}", e);
                }
            }
            // Update fingerprint in state
            let _ = state.domains.update_fingerprint(&domain.id, fingerprint).await;
        }

        // Build alert config from domain thresholds
        let domain_alerts = config::AlertConfig {
            warning_days: domain.warning_days,
            critical_days: domain.critical_days,
        };

        // Publish to MQTT
        if let Err(e) = state
            .mqtt
            .publish_certificate(&domain.id, &cert_status, &domain_alerts)
            .await
        {
            warn!("Failed to publish certificate status: {}", e);
        }

        // Build domain status
        let status_level = match cert_status.days_remaining {
            Some(days) if days < 0 => "expired",
            Some(days) if days <= domain.critical_days => "critical",
            Some(days) if days <= domain.warning_days => "warning",
            Some(_) => "ok",
            None => "error",
        };

        let domain_status = DomainStatus {
            domain_id: domain.id.clone(),
            hostname: cert_status.hostname.clone(),
            port: cert_status.port,
            online: true,
            ssl_valid: cert_status.valid,
            days_remaining: cert_status.days_remaining,
            expiry_date: cert_status.expiry_date.map(|d| d.format("%Y-%m-%d").to_string()),
            issuer: cert_status.issuer.clone(),
            status_level: status_level.to_string(),
            error: cert_status.error.clone(),
            checked_at: cert_status.checked_at.to_rfc3339(),
        };

        statuses.push(domain_status.clone());

        // Update internal cache
        state
            .domain_statuses
            .write()
            .await
            .insert(domain.id.clone(), domain_status);
    }

    // Publish summary
    if let Err(e) = state.mqtt.publish_summary(&statuses).await {
        warn!("Failed to publish summary: {}", e);
    }

    // Update health
    let mut health = state.health.write().await;
    health.last_ssl_check = Some(chrono::Utc::now().to_rfc3339());
    health.domains_total = state.domains.list_domains().await.len();
    health.domains_enabled = domains.len();
    health.domains_valid = valid_count;
    health.domains_expiring = expiring_count;

    info!(
        "SSL check complete: {}/{} valid, {} expiring soon",
        valid_count,
        domains.len(),
        expiring_count
    );
}

/// Online check loop (more frequent)
async fn online_check_loop(state: Arc<PluginState>) {
    let mut ticker = interval_at(
        Instant::now() + Duration::from_secs(5), // Start 5s after SSL check
        Duration::from_secs(state.config.polling.online_seconds),
    );

    loop {
        ticker.tick().await;

        let domains = state.domains.list_enabled_domains().await;

        for domain in &domains {
            let hostname = domain.hostname.clone();
            let port = domain.port;
            let checker = SslChecker::new();

            let online = tokio::task::spawn_blocking(move || {
                checker.check_online(&hostname, port)
            })
            .await
            .unwrap_or(false);

            // Publish online status
            if let Err(e) = state
                .mqtt
                .publish_online(&domain.id, &domain.hostname, online)
                .await
            {
                warn!("Failed to publish online status: {}", e);
            }
        }

        // Update health
        let mut health = state.health.write().await;
        health.last_online_check = Some(chrono::Utc::now().to_rfc3339());
    }
}

/// Periodic state save (debounced)
async fn periodic_save(state: Arc<PluginState>) {
    let mut ticker = interval_at(
        Instant::now() + Duration::from_secs(60),
        Duration::from_secs(60),
    );

    loop {
        ticker.tick().await;

        if let Err(e) = state.domains.save().await {
            error!("Failed to save state: {}", e);
        }
    }
}

/// HTTP API server (Unix socket)
async fn api_server(state: Arc<PluginState>) -> Result<()> {
    let socket_path = state.config.http.socket_path.clone();

    // Create parent directory if needed
    if let Some(parent) = std::path::Path::new(&socket_path).parent() {
        std::fs::create_dir_all(parent)?;
    }

    info!("API server listening on {}", socket_path);

    let app = Router::new()
        // Health endpoint
        .route("/health", get(health_handler))
        // Domain CRUD
        .route("/domains", get(list_domains_handler))
        .route("/domains", post(create_domain_handler))
        .route("/domains/:id", get(get_domain_handler))
        .route("/domains/:id", put(update_domain_handler))
        .route("/domains/:id", delete(delete_domain_handler))
        // Actions
        .route("/check", post(trigger_check_handler))
        .with_state(state);

    let server = PluginHttpServer::new(&socket_path, app);
    server
        .serve()
        .await
        .map_err(|e| anyhow::anyhow!("API server error: {}", e))?;

    Ok(())
}

// === API Handlers ===

async fn health_handler(State(state): State<Arc<PluginState>>) -> Json<HealthResponse> {
    let internal = state.health.read().await;
    let status = if internal.mqtt_connected && internal.error.is_none() {
        "healthy"
    } else {
        "degraded"
    };

    Json(HealthResponse {
        plugin_id: PLUGIN_ID.to_string(),
        spec_version: SPEC_VERSION.to_string(),
        status: status.to_string(),
        uptime_seconds: state.started_at.elapsed().as_secs(),
        domains_total: internal.domains_total,
        domains_enabled: internal.domains_enabled,
    })
}

async fn list_domains_handler(State(state): State<Arc<PluginState>>) -> Json<DomainsResponse> {
    let configs = state.domains.list_domains().await;
    let statuses = state.domain_statuses.read().await;

    let domains: Vec<DomainWithStatus> = configs
        .into_iter()
        .map(|config| {
            let status = statuses.get(&config.id).cloned();
            DomainWithStatus { config, status }
        })
        .collect();

    let enabled = domains.iter().filter(|d| d.config.enabled).count();
    let valid = domains.iter().filter(|d| {
        d.status.as_ref().map(|s| s.ssl_valid).unwrap_or(false)
    }).count();
    let expiring = domains.iter().filter(|d| {
        d.status.as_ref().and_then(|s| s.days_remaining)
            .map(|days| days <= d.config.warning_days)
            .unwrap_or(false)
    }).count();
    let critical = domains.iter().filter(|d| {
        d.status.as_ref().and_then(|s| s.days_remaining)
            .map(|days| days <= d.config.critical_days)
            .unwrap_or(false)
    }).count();

    Json(DomainsResponse {
        summary: SummaryInfo {
            total: domains.len(),
            enabled,
            valid,
            expiring_soon: expiring,
            critical,
        },
        domains,
    })
}

async fn get_domain_handler(
    State(state): State<Arc<PluginState>>,
    Path(id): Path<String>,
) -> ApiResult<DomainWithStatus> {
    let config = state.domains.get_domain(&id).await
        .ok_or_else(|| (
            StatusCode::NOT_FOUND,
            Json(ApiError::new(format!("Domain '{}' not found", id), "NOT_FOUND"))
        ))?;

    let status = state.domain_statuses.read().await.get(&id).cloned();

    Ok(Json(DomainWithStatus { config, status }))
}

async fn create_domain_handler(
    State(state): State<Arc<PluginState>>,
    Json(req): Json<CreateDomainRequest>,
) -> ApiResult<DynamicDomain> {
    let domain = state.domains.create_domain(req).await
        .map_err(|e| (
            StatusCode::BAD_REQUEST,
            Json(ApiError::new(e.to_string(), "VALIDATION_ERROR"))
        ))?;

    // Save immediately
    if let Err(e) = state.domains.save().await {
        warn!("Failed to save SSL domain state: {}", e);
    }

    // Trigger SSL check for new domain
    let _ = state.check_trigger.send(());

    info!("Created domain: {} ({})", domain.id, domain.hostname);

    Ok(Json(domain))
}

async fn update_domain_handler(
    State(state): State<Arc<PluginState>>,
    Path(id): Path<String>,
    Json(req): Json<UpdateDomainRequest>,
) -> ApiResult<DynamicDomain> {
    let domain = state.domains.update_domain(&id, req).await
        .map_err(|e| (
            StatusCode::NOT_FOUND,
            Json(ApiError::new(e.to_string(), "NOT_FOUND"))
        ))?;

    // Save immediately
    if let Err(e) = state.domains.save().await {
        warn!("Failed to save SSL domain state: {}", e);
    }

    info!("Updated domain: {} ({})", domain.id, domain.hostname);

    Ok(Json(domain))
}

async fn delete_domain_handler(
    State(state): State<Arc<PluginState>>,
    Path(id): Path<String>,
) -> ApiResult<DynamicDomain> {
    let domain = state.domains.delete_domain(&id).await
        .map_err(|e| (
            StatusCode::NOT_FOUND,
            Json(ApiError::new(e.to_string(), "NOT_FOUND"))
        ))?;

    // Remove from status cache
    state.domain_statuses.write().await.remove(&id);

    // Save immediately
    if let Err(e) = state.domains.save().await {
        warn!("Failed to save SSL domain state: {}", e);
    }

    info!("Deleted domain: {} ({})", domain.id, domain.hostname);

    Ok(Json(domain))
}

async fn trigger_check_handler(State(state): State<Arc<PluginState>>) -> Json<serde_json::Value> {
    let _ = state.check_trigger.send(());

    Json(serde_json::json!({
        "status": "triggered",
        "message": "SSL check triggered"
    }))
}
