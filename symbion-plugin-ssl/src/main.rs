//! Symbion SSL Plugin
//!
//! Monitors SSL certificates and domain availability:
//! - Certificate expiry dates
//! - Certificate validity
//! - Domain online status
//! - Publishes features for Intelligence v2 automations

mod config;
mod mqtt;
mod ssl;

use anyhow::{Context, Result};
use axum::{extract::State, routing::get, Json, Router};
use serde::Serialize;
use std::collections::HashMap;
use std::sync::Arc;
use symbion_plugin_common::PluginHttpServer;
use tokio::sync::RwLock;
use tokio::time::{interval_at, Duration, Instant};
use tracing::{error, info, warn, Level};
use tracing_subscriber::EnvFilter;

use config::Config;
use mqtt::{DomainStatus, MqttPublisher};
use ssl::{CertificateStatus, SslChecker};

/// Plugin constants
const PLUGIN_ID: &str = "ssl";
const SPEC_VERSION: &str = "1.0";

/// Plugin state shared across tasks
struct PluginState {
    config: Config,
    checker: SslChecker,
    mqtt: MqttPublisher,
    domain_statuses: RwLock<HashMap<String, DomainStatus>>,
    health: RwLock<InternalHealth>,
    started_at: std::time::Instant,
}

/// Internal health tracking
#[derive(Debug, Clone, Default)]
struct InternalHealth {
    mqtt_connected: bool,
    last_ssl_check: Option<String>,
    last_online_check: Option<String>,
    domains_checked: usize,
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
}

/// Domains list response
#[derive(Debug, Clone, Serialize)]
struct DomainsResponse {
    domains: Vec<DomainStatus>,
    summary: SummaryInfo,
}

#[derive(Debug, Clone, Serialize)]
struct SummaryInfo {
    total: usize,
    valid: usize,
    expiring_soon: usize,
    critical: usize,
}

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

    info!("Starting Symbion SSL Plugin v{}", env!("CARGO_PKG_VERSION"));

    // Load configuration
    let config_path = std::env::var("SSL_CONFIG")
        .unwrap_or_else(|_| "/opt/symbion/config/ssl.toml".to_string());

    let config = Config::load(&config_path)
        .with_context(|| format!("Failed to load config from {}", config_path))?;

    info!("Loaded configuration from {}", config_path);
    info!("Monitoring {} domains", config.domains.len());

    // Initialize SSL checker
    let checker = SslChecker::new();

    // Initialize MQTT publisher
    info!("Connecting to MQTT broker {}:{}...", config.mqtt.host, config.mqtt.port);
    let mqtt = MqttPublisher::connect(&config.mqtt)
        .await
        .context("Failed to connect to MQTT broker")?;

    info!("MQTT connected");

    // Create shared state
    let state = Arc::new(PluginState {
        config: config.clone(),
        checker,
        mqtt,
        domain_statuses: RwLock::new(HashMap::new()),
        health: RwLock::new(InternalHealth {
            mqtt_connected: true,
            ..Default::default()
        }),
        started_at: std::time::Instant::now(),
    });

    // Publish manifest
    let manifest = include_str!("../manifest.json");
    state.mqtt.publish_manifest(manifest).await?;
    info!("Manifest published to symbion/plugins/ssl/manifest");

    // Publish initial health
    state.mqtt.publish_health(true, "Plugin started").await?;

    // Start polling tasks
    let ssl_handle = tokio::spawn(ssl_check_loop(Arc::clone(&state)));
    let online_handle = tokio::spawn(online_check_loop(Arc::clone(&state)));

    // Start health endpoint
    let health_handle = tokio::spawn(health_server(Arc::clone(&state)));

    info!("All tasks started, plugin running");

    // Wait for any task to complete
    tokio::select! {
        r = ssl_handle => {
            error!("SSL check loop exited: {:?}", r);
        }
        r = online_handle => {
            error!("Online check loop exited: {:?}", r);
        }
        r = health_handle => {
            error!("Health server exited: {:?}", r);
        }
    }

    Ok(())
}

/// SSL certificate check loop
async fn ssl_check_loop(state: Arc<PluginState>) {
    // Run immediately on start, then at configured interval
    let mut ticker = interval_at(
        Instant::now(),
        Duration::from_secs(state.config.polling.ssl_seconds),
    );

    loop {
        ticker.tick().await;

        info!("Running SSL certificate check...");

        let mut valid_count = 0;
        let mut expiring_count = 0;
        let mut statuses = Vec::new();

        for (domain_id, domain_config) in &state.config.domains {
            info!("Checking {} ({}:{})", domain_id, domain_config.hostname, domain_config.port);

            // Run SSL check in blocking context (native-tls is sync)
            let hostname = domain_config.hostname.clone();
            let port = domain_config.port;
            let checker = SslChecker::new();

            let cert_status = tokio::task::spawn_blocking(move || {
                checker.check(&hostname, port)
            })
            .await
            .unwrap_or_else(|e| CertificateStatus {
                hostname: domain_config.hostname.clone(),
                port: domain_config.port,
                valid: false,
                expiry_date: None,
                days_remaining: None,
                issuer: None,
                subject: None,
                error: Some(format!("Task error: {}", e)),
                checked_at: chrono::Utc::now(),
            });

            // Log result
            if cert_status.valid {
                valid_count += 1;
                if let Some(days) = cert_status.days_remaining {
                    if days <= state.config.alerts.warning_days {
                        expiring_count += 1;
                        warn!(
                            "{}: Certificate expires in {} days",
                            domain_config.hostname, days
                        );
                    } else {
                        info!(
                            "{}: Certificate valid ({} days remaining)",
                            domain_config.hostname, days
                        );
                    }
                }
            } else {
                error!(
                    "{}: Certificate INVALID - {:?}",
                    domain_config.hostname, cert_status.error
                );
            }

            // Publish to MQTT
            if let Err(e) = state
                .mqtt
                .publish_certificate(domain_id, &cert_status, &state.config.alerts)
                .await
            {
                warn!("Failed to publish certificate status: {}", e);
            }

            // Build domain status
            let status_level = match cert_status.days_remaining {
                Some(days) if days < 0 => "expired",
                Some(days) if days <= state.config.alerts.critical_days => "critical",
                Some(days) if days <= state.config.alerts.warning_days => "warning",
                Some(_) => "ok",
                None => "error",
            };

            let domain_status = DomainStatus {
                domain_id: domain_id.clone(),
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
                .insert(domain_id.clone(), domain_status);
        }

        // Publish summary
        if let Err(e) = state.mqtt.publish_summary(&statuses).await {
            warn!("Failed to publish summary: {}", e);
        }

        // Update health
        let mut health = state.health.write().await;
        health.last_ssl_check = Some(chrono::Utc::now().to_rfc3339());
        health.domains_checked = state.config.domains.len();
        health.domains_valid = valid_count;
        health.domains_expiring = expiring_count;

        info!(
            "SSL check complete: {}/{} valid, {} expiring soon",
            valid_count,
            state.config.domains.len(),
            expiring_count
        );
    }
}

/// Online check loop (more frequent)
async fn online_check_loop(state: Arc<PluginState>) {
    let mut ticker = interval_at(
        Instant::now() + Duration::from_secs(5), // Start 5s after SSL check
        Duration::from_secs(state.config.polling.online_seconds),
    );

    loop {
        ticker.tick().await;

        for (domain_id, domain_config) in &state.config.domains {
            let hostname = domain_config.hostname.clone();
            let port = domain_config.port;
            let checker = SslChecker::new();

            let online = tokio::task::spawn_blocking(move || {
                checker.check_online(&hostname, port)
            })
            .await
            .unwrap_or(false);

            // Publish online status
            if let Err(e) = state
                .mqtt
                .publish_online(domain_id, &domain_config.hostname, online)
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

/// Health endpoint server (Unix socket)
async fn health_server(state: Arc<PluginState>) -> Result<()> {
    let socket_path = state.config.http.socket_path.clone();

    // Create parent directory if needed
    if let Some(parent) = std::path::Path::new(&socket_path).parent() {
        std::fs::create_dir_all(parent)?;
    }

    info!("Health endpoint listening on {}", socket_path);

    let app = Router::new()
        .route("/health", get(health_handler))
        .route("/domains", get(domains_handler))
        .with_state(state);

    let server = PluginHttpServer::new(&socket_path, app);
    server
        .serve()
        .await
        .map_err(|e| anyhow::anyhow!("Health server error: {}", e))?;

    Ok(())
}

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
    })
}

async fn domains_handler(State(state): State<Arc<PluginState>>) -> Json<DomainsResponse> {
    let statuses = state.domain_statuses.read().await;
    let domains: Vec<DomainStatus> = statuses.values().cloned().collect();

    let valid = domains.iter().filter(|d| d.ssl_valid).count();
    let expiring = domains
        .iter()
        .filter(|d| {
            d.days_remaining
                .map(|days| days <= state.config.alerts.warning_days)
                .unwrap_or(false)
        })
        .count();
    let critical = domains
        .iter()
        .filter(|d| {
            d.days_remaining
                .map(|days| days <= state.config.alerts.critical_days)
                .unwrap_or(false)
        })
        .count();

    Json(DomainsResponse {
        domains,
        summary: SummaryInfo {
            total: statuses.len(),
            valid,
            expiring_soon: expiring,
            critical,
        },
    })
}
