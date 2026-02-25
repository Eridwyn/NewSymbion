use super::AppState;
use axum::extract::{Path, Query, State};
use axum::http::StatusCode;
use axum::Json;
use serde::Deserialize;
use time::{Duration, OffsetDateTime, format_description::well_known::Rfc3339};
use utoipa::{IntoParams, ToSchema};
use crate::models::HostState;
use crate::wol::trigger_wol_udp;

// Types referenced by utoipa::path body annotations
use crate::contracts::Contract;
use crate::health::KernelHealth;

// ============================================================================
// Host Views
// ============================================================================

/// Serializable view of a host's current state for API responses.
#[derive(serde::Serialize, ToSchema)]
pub(crate) struct HostView {
    host_id: String,
    last_seen: String,       // format RFC3339 pour l'API
    stale: bool,             // true si > 90s
    stale_for_seconds: i64,  // âge en secondes
    cpu: Option<f32>,
    ram: Option<f32>,
    ip: Option<String>,
}

/// Convert a HostState into a HostView, computing staleness from the current time.
pub(super) fn to_view(h: &HostState) -> HostView {
    let now = OffsetDateTime::now_utc();
    let age = now - h.last_seen;
    let secs = age.whole_seconds().max(0);
    HostView {
        host_id: h.host_id.clone(),
        last_seen: h.last_seen.format(&Rfc3339).unwrap_or_default(),
        stale: age > Duration::seconds(90),
        stale_for_seconds: secs,
        cpu: h.cpu,
        ram: h.ram,
        ip: h.ip.clone(),
    }
}

// ============================================================================
// Wake Params
// ============================================================================

/// Query parameters for the Wake-on-LAN endpoint.
#[derive(Debug, Deserialize, ToSchema, IntoParams)]
pub(crate) struct WakeParams { pub host_id: String }

// ============================================================================
// Hosts / Contracts / Health Handlers
// ============================================================================

/// GET /hosts -- Return a list of all known hosts with their current state.
#[utoipa::path(
    get,
    path = "/hosts",
    tag = "System",
    security(("bearer_auth" = [])),
    responses(
        (status = 200, description = "Liste de tous les hosts connus", body = Vec<HostView>),
        (status = 401, description = "Non authentifié")
    )
)]
pub(super) async fn get_hosts(State(app): State<AppState>) -> Json<Vec<HostView>> {
    let list: Vec<HostView> = {
        let states = app.states.lock();
        states.values().map(to_view).collect()
    }; // Lock libéré immédiatement
    Json(list)
}

/// GET /hosts/:id -- Return a single host by ID, or 404 if not found.
#[utoipa::path(
    get,
    path = "/hosts/{id}",
    tag = "System",
    security(("bearer_auth" = [])),
    params(("id" = String, Path, description = "Identifiant unique du host")),
    responses(
        (status = 200, description = "Détail du host", body = HostView),
        (status = 404, description = "Host non trouvé"),
        (status = 401, description = "Non authentifié")
    )
)]
pub(super) async fn get_host(
    State(app): State<AppState>,
    Path(id): Path<String>,
) -> Result<Json<HostView>, StatusCode> {
    let host_view = {
        let map = app.states.lock();
        let Some(h) = map.get(&id) else { return Err(StatusCode::NOT_FOUND); };
        to_view(h)
    }; // Lock libéré immédiatement
    Ok(Json(host_view))
}


/// POST /wake -- Send a Wake-on-LAN magic packet to the specified host.
#[utoipa::path(
    post,
    path = "/wake",
    tag = "System",
    security(("bearer_auth" = [])),
    params(WakeParams),
    responses(
        (status = 200, description = "Magic packet envoyé avec succès", body = Object),
        (status = 400, description = "Paramètres invalides"),
        (status = 401, description = "Non authentifié")
    )
)]
pub(super) async fn wake(
    State(app): State<AppState>,
    Query(params): Query<WakeParams>,
) -> (StatusCode, Json<serde_json::Value>) {
    // D'abord essayer avec les agents (système moderne)
    let agents = app.agents.list_agents().await;
    for agent in agents.values() {
        if agent.agent_id == params.host_id {
            // Utiliser l'adresse MAC de l'agent pour WoL
            let mac_str = format!("{}:{}:{}:{}:{}:{}",
                &params.host_id[0..2], &params.host_id[2..4], &params.host_id[4..6],
                &params.host_id[6..8], &params.host_id[8..10], &params.host_id[10..12]
            );

            return send_magic_packet(&mac_str).await;
        }
    }

    // Fallback vers ancien système hosts
    let cfg = app.cfg.lock().clone();
    let (code, msg) = trigger_wol_udp(&cfg, &params.host_id).await;
    (code, Json(serde_json::json!({ "ok": code == StatusCode::OK, "msg": msg })))
}

/// Envoie un magic packet WoL pour l'adresse MAC donnée
pub(super) async fn send_magic_packet(mac: &str) -> (StatusCode, Json<serde_json::Value>) {
    use std::net::{Ipv4Addr, SocketAddrV4, UdpSocket};

    // Parse MAC address
    let hex: String = mac.chars().filter(|c| c.is_ascii_hexdigit()).collect();
    if hex.len() != 12 {
        return (StatusCode::BAD_REQUEST, Json(serde_json::json!({"ok": false, "msg": "invalid mac length"})));
    }

    let mut mac_bytes = [0u8; 6];
    for i in 0..6 {
        match u8::from_str_radix(&hex[i*2..i*2+2], 16) {
            Ok(byte) => mac_bytes[i] = byte,
            Err(_) => return (StatusCode::BAD_REQUEST, Json(serde_json::json!({"ok": false, "msg": "invalid mac format"})))
        }
    }

    // Create magic packet (6 x 0xFF + 16 x MAC)
    let mut packet = [0u8; 102];
    for i in 0..6 { packet[i] = 0xFF; }
    for i in 0..16 {
        let base = 6 + i*6;
        packet[base..base+6].copy_from_slice(&mac_bytes);
    }

    // Send UDP broadcast on ports 9 and 7
    let sock = match UdpSocket::bind(("0.0.0.0", 0)) {
        Ok(s) => s,
        Err(_) => return (StatusCode::INTERNAL_SERVER_ERROR, Json(serde_json::json!({"ok": false, "msg": "failed to bind socket"})))
    };

    if sock.set_broadcast(true).is_err() {
        return (StatusCode::INTERNAL_SERVER_ERROR, Json(serde_json::json!({"ok": false, "msg": "failed to enable broadcast"})));
    }

    let broadcast = Ipv4Addr::new(255, 255, 255, 255);
    let mut success = false;

    for port in [9u16, 7u16] {
        let addr = SocketAddrV4::new(broadcast, port);
        if sock.send_to(&packet, addr).is_ok() {
            success = true;
        }
    }

    if success {
        (StatusCode::OK, Json(serde_json::json!({"ok": true, "msg": "magic packet sent"})))
    } else {
        (StatusCode::INTERNAL_SERVER_ERROR, Json(serde_json::json!({"ok": false, "msg": "failed to send magic packet"})))
    }
}

/// GET /contracts -- Return the list of all registered MQTT contract names.
#[utoipa::path(
    get,
    path = "/contracts",
    tag = "System",
    security(("bearer_auth" = [])),
    responses(
        (status = 200, description = "Liste des noms de contrats MQTT", body = Vec<String>),
        (status = 401, description = "Non authentifié")
    )
)]
pub(super) async fn list_contracts(State(app): State<AppState>) -> Json<Vec<String>> {
    Json(app.contracts.list_contracts())
}

/// GET /contracts/{name} -- Return a single MQTT contract by name, or 404 if not found.
#[utoipa::path(
    get,
    path = "/contracts/{name}",
    tag = "System",
    security(("bearer_auth" = [])),
    params(("name" = String, Path, description = "Nom du contrat MQTT")),
    responses(
        (status = 200, description = "Détail du contrat MQTT", body = Contract),
        (status = 404, description = "Contrat non trouvé"),
        (status = 401, description = "Non authentifié")
    )
)]
pub(super) async fn get_contract(
    State(app): State<AppState>,
    Path(name): Path<String>,
) -> Result<Json<crate::contracts::Contract>, StatusCode> {
    match app.contracts.get_contract(&name) {
        Some(contract) => Ok(Json(contract.clone())),
        None => Err(StatusCode::NOT_FOUND),
    }
}

/// GET /system/health -- Return full infrastructure health status (MQTT, agents, plugins).
#[utoipa::path(
    get,
    path = "/system/health",
    tag = "System",
    responses(
        (status = 200, description = "Santé complète de l'infrastructure", body = KernelHealth)
    )
)]
pub(super) async fn get_system_health(State(app): State<AppState>) -> Json<crate::health::KernelHealth> {
    let health = app.health_tracker.get_health(&app.contracts, &app.agents, &app.plugin_registry);
    Json(health)
}

/// GET /health/ready — Readiness probe pour monitoring externe (healthcheck.io, UptimeRobot, k8s)
/// Vérifie que le kernel est prêt à servir des requêtes (MQTT connecté, contracts chargés)
#[utoipa::path(
    get,
    path = "/health/ready",
    tag = "System",
    responses(
        (status = 200, description = "Kernel prêt à servir des requêtes", body = Object),
        (status = 503, description = "Kernel pas encore prêt")
    )
)]
pub(super) async fn health_readiness_check(State(app): State<AppState>) -> Result<Json<serde_json::Value>, StatusCode> {
    let health = app.health_tracker.get_health(&app.contracts, &app.agents, &app.plugin_registry);

    let mqtt_ok = health.mqtt_status == "connected";
    let ready = mqtt_ok;

    if ready {
        Ok(Json(serde_json::json!({
            "status": "ready",
            "mqtt": health.mqtt_status,
            "contracts": health.contracts_loaded,
            "uptime": health.uptime_seconds,
            "agents": health.agents_count,
            "plugins": health.plugins_active,
        })))
    } else {
        Err(StatusCode::SERVICE_UNAVAILABLE)
    }
}

// ============================================================================
// Metrics API Endpoints (PR4 - P1)
// ============================================================================

/// GET /v1/metrics/agents - Per-agent metrics in JSON format
/// Returns detailed telemetry for each agent: CPU, RAM, disk, network, processes
#[derive(serde::Serialize, ToSchema)]
pub(crate) struct AgentMetrics {
    agent_id: String,
    hostname: String,
    status: String,
    last_seen: i64, // Unix timestamp
    uptime_seconds: u64,
    cpu: AgentCpuMetrics,
    memory: AgentMemoryMetrics,
    disk: Vec<AgentDiskMetrics>,
    network: Vec<AgentNetworkMetrics>,
    processes: AgentProcessMetrics,
}

/// CPU telemetry for a single agent (percent, load average, core count).
#[derive(serde::Serialize, ToSchema)]
pub(crate) struct AgentCpuMetrics {
    percent: f32,
    load_avg: Vec<f32>,
    core_count: u32,
}

/// Memory telemetry for a single agent (total, used, available, percent).
#[derive(serde::Serialize, ToSchema)]
pub(crate) struct AgentMemoryMetrics {
    total_mb: u64,
    used_mb: u64,
    available_mb: u64,
    percent_used: f32,
}

/// Disk usage telemetry for a single mount point on an agent.
#[derive(serde::Serialize, ToSchema)]
pub(crate) struct AgentDiskMetrics {
    path: String,
    total_gb: f64,
    used_gb: f64,
    free_gb: f64,
    percent_used: f32,
}

/// Network interface telemetry for a single agent (bytes sent/received, link status).
#[derive(serde::Serialize, ToSchema)]
pub(crate) struct AgentNetworkMetrics {
    name: String,
    bytes_sent: u64,
    bytes_recv: u64,
    is_up: bool,
}

/// Process count summary for a single agent (total and running).
#[derive(serde::Serialize, ToSchema)]
pub(crate) struct AgentProcessMetrics {
    total_count: u32,
    running_count: u32,
}

/// GET /v1/metrics/agents -- Return per-agent telemetry (CPU, RAM, disk, network, processes).
#[utoipa::path(
    get,
    path = "/v1/metrics/agents",
    tag = "System",
    responses(
        (status = 200, description = "Métriques détaillées par agent", body = Vec<AgentMetrics>)
    )
)]
pub(super) async fn get_metrics_agents(
    State(app): State<AppState>,
) -> Json<Vec<AgentMetrics>> {
    let agents_map = app.agents.list_agents().await;

    let mut metrics = Vec::new();
    for (agent_id, agent) in agents_map.iter() {
        let agent_metric = AgentMetrics {
            agent_id: agent_id.clone(),
            hostname: agent.hostname.clone(),
            status: agent.status.status.clone(),
            last_seen: agent.last_seen.unix_timestamp(),
            uptime_seconds: agent.status.system.as_ref()
                .map(|s| s.uptime_seconds)
                .unwrap_or(0),
            cpu: AgentCpuMetrics {
                percent: agent.status.system.as_ref()
                    .map(|s| s.cpu.percent)
                    .unwrap_or(0.0),
                load_avg: agent.status.system.as_ref()
                    .and_then(|s| s.cpu.load_avg.map(|arr| arr.to_vec()))
                    .unwrap_or_default(),
                core_count: agent.status.system.as_ref()
                    .and_then(|s| s.cpu.core_count)
                    .unwrap_or(0),
            },
            memory: AgentMemoryMetrics {
                total_mb: agent.status.system.as_ref()
                    .map(|s| s.memory.total_mb)
                    .unwrap_or(0),
                used_mb: agent.status.system.as_ref()
                    .map(|s| s.memory.used_mb)
                    .unwrap_or(0),
                available_mb: agent.status.system.as_ref()
                    .and_then(|s| s.memory.available_mb)
                    .unwrap_or(0),
                percent_used: agent.status.system.as_ref()
                    .map(|s| s.memory.percent_used)
                    .unwrap_or(0.0),
            },
            disk: agent.status.system.as_ref()
                .and_then(|s| s.disk.as_ref())
                .map(|disks| disks.iter().map(|d| AgentDiskMetrics {
                    path: d.path.clone(),
                    total_gb: d.total_gb,
                    used_gb: d.used_gb,
                    free_gb: d.free_gb.unwrap_or(0.0),
                    percent_used: d.percent_used,
                }).collect())
                .unwrap_or_default(),
            network: agent.status.system.as_ref()
                .and_then(|s| s.network.as_ref())
                .map(|n| n.interfaces.iter().map(|i| AgentNetworkMetrics {
                    name: i.name.clone(),
                    bytes_sent: i.bytes_sent.unwrap_or(0),
                    bytes_recv: i.bytes_recv.unwrap_or(0),
                    is_up: i.is_up,
                }).collect())
                .unwrap_or_default(),
            processes: AgentProcessMetrics {
                total_count: agent.status.processes.as_ref()
                    .map(|p| p.total_count)
                    .unwrap_or(0),
                running_count: agent.status.processes.as_ref()
                    .map(|p| p.running_count)
                    .unwrap_or(0),
            },
        };
        metrics.push(agent_metric);
    }

    Json(metrics)
}

/// GET /v1/metrics/system - Kernel performance metrics in JSON format
/// Returns kernel runtime stats: uptime, memory, MQTT, plugins, context
#[derive(serde::Serialize, ToSchema)]
pub(crate) struct SystemMetrics {
    kernel: KernelRuntimeMetrics,
    mqtt: MqttMetrics,
    agents: AgentsSummaryMetrics,
    plugins: PluginsMetrics,
    context: ContextMetrics,
    decision_engine: DecisionEngineMetrics,
}

/// Kernel runtime stats (uptime, memory usage, contracts loaded).
#[derive(serde::Serialize, ToSchema)]
pub(crate) struct KernelRuntimeMetrics {
    uptime_seconds: u64,
    memory_usage_mb: f32,
    contracts_loaded: u32,
}

/// MQTT broker connection and throughput metrics.
#[derive(serde::Serialize, ToSchema)]
pub(crate) struct MqttMetrics {
    status: String, // "connected", "disconnected", "reconnecting"
    reconnects_total: u32,
    messages_per_minute: f32,
    messages_total: u64,
}

/// Summary counts of agents by online/offline status.
#[derive(serde::Serialize, ToSchema)]
pub(crate) struct AgentsSummaryMetrics {
    total: usize,
    online: usize,
    offline: usize,
}

/// Plugin system status counts (total, running, failed).
#[derive(serde::Serialize, ToSchema)]
pub(crate) struct PluginsMetrics {
    total: u32,
    running: u32,
    failed: u32,
}

/// Context engine state (current mode and detection confidence).
#[derive(serde::Serialize, ToSchema)]
pub(crate) struct ContextMetrics {
    current_mode: String, // "veille", "pro", "maison"
    confidence: f32,
}

/// Decision engine counters (total, approved, blocked, pending validations, active overrides).
#[derive(serde::Serialize, ToSchema)]
pub(crate) struct DecisionEngineMetrics {
    decisions_total: u64,
    decisions_approved: u64,
    decisions_blocked: u64,
    validations_pending: usize,
    overrides_active: usize,
}

/// GET /v1/metrics/system -- Return aggregated kernel performance metrics (runtime, MQTT, agents, plugins, context, decisions).
#[utoipa::path(
    get,
    path = "/v1/metrics/system",
    tag = "System",
    responses(
        (status = 200, description = "Métriques agrégées du kernel", body = SystemMetrics)
    )
)]
pub(super) async fn get_metrics_system(
    State(app): State<AppState>,
) -> Json<SystemMetrics> {
    // Get kernel health
    let kernel_health = app.health_tracker.get_health(&app.contracts, &app.agents, &app.plugin_registry);

    // Get agent summary
    let agents_map = app.agents.list_agents().await;
    let total_agents = agents_map.len();
    let online_agents = agents_map.values()
        .filter(|a| a.status.status == "online")
        .count();

    // Get context state
    let context_state = app.context_engine.get_state();
    let (mode_name, mode_confidence) = if let Some(state) = context_state {
        use crate::context::Mode;
        let name = match state.mode {
            Mode::Veille => "veille",
            Mode::Pro => "pro",
            Mode::Maison => "maison",
        };
        (name.to_string(), state.confidence)
    } else {
        ("unknown".to_string(), 0.0)
    };

    // Get decision engine stats
    let validation_stats = app.decision_validation_manager.stats();
    let override_stats = app.decision_override_manager.stats();

    // Decision metrics counters
    let decisions_total = app.decision_metrics.get_decisions_total();
    let decisions_approved = app.decision_metrics.get_decisions_approved();
    let decisions_blocked = app.decision_metrics.get_decisions_blocked();

    let metrics = SystemMetrics {
        kernel: KernelRuntimeMetrics {
            uptime_seconds: kernel_health.uptime_seconds,
            memory_usage_mb: kernel_health.memory_usage_mb,
            contracts_loaded: kernel_health.contracts_loaded,
        },
        mqtt: MqttMetrics {
            status: kernel_health.mqtt_status,
            reconnects_total: kernel_health.mqtt_reconnects,
            messages_per_minute: kernel_health.mqtt_messages_per_minute,
            messages_total: kernel_health.mqtt_messages_total,
        },
        agents: AgentsSummaryMetrics {
            total: total_agents,
            online: online_agents,
            offline: total_agents - online_agents,
        },
        plugins: PluginsMetrics {
            total: 0, // Legacy plugin system removed - now using plugin_proxy
            running: 0,
            failed: 0,
        },
        context: ContextMetrics {
            current_mode: mode_name,
            confidence: mode_confidence,
        },
        decision_engine: DecisionEngineMetrics {
            decisions_total,
            decisions_approved,
            decisions_blocked,
            validations_pending: validation_stats.pending,
            overrides_active: override_stats.active,
        },
    };

    Json(metrics)
}

// ============================================================================
// Prometheus Metrics Endpoint (PR4 - P0)
// ============================================================================

/// GET /metrics - Prometheus scraping endpoint (public, no auth required)
/// Exports all kernel metrics in Prometheus exposition format:
/// - Decision Engine metrics (decisions, guards, validations)
/// - Agent telemetry (count, online/offline status)
/// - System metrics (MQTT status, uptime)
/// - HTTP metrics (placeholder for future request counters)
#[utoipa::path(
    get,
    path = "/metrics",
    tag = "System",
    responses(
        (status = 200, description = "Métriques au format Prometheus exposition", body = String)
    )
)]
pub(super) async fn prometheus_metrics_endpoint(
    State(app): State<AppState>,
) -> Result<String, StatusCode> {
    let mut output = String::new();

    // ========== Decision Engine Metrics ==========
    let audit_stats = app.decision_audit_manager.stats();
    let validation_stats = app.decision_validation_manager.stats();
    let override_stats = app.decision_override_manager.stats();
    let agent_health_stats = app.decision_agent_health_manager.stats();

    output.push_str(&app.decision_metrics.export_prometheus(
        &audit_stats,
        &validation_stats,
        &override_stats,
        &agent_health_stats,
    ));

    // ========== System Metrics ==========
    // Get kernel health for MQTT status
    let kernel_health = app.health_tracker.get_health(&app.contracts, &app.agents, &app.plugin_registry);

    // MQTT Connection Status
    let mqtt_connected = if kernel_health.mqtt_status == "connected" { 1 } else { 0 };
    output.push_str("# HELP symbion_mqtt_connected MQTT broker connection status (1=connected, 0=disconnected)\n");
    output.push_str("# TYPE symbion_mqtt_connected gauge\n");
    output.push_str(&format!("symbion_mqtt_connected {}\n", mqtt_connected));

    output.push_str("# HELP symbion_mqtt_reconnects_total Total number of MQTT reconnections since startup\n");
    output.push_str("# TYPE symbion_mqtt_reconnects_total counter\n");
    output.push_str(&format!("symbion_mqtt_reconnects_total {}\n", kernel_health.mqtt_reconnects));

    output.push_str("# HELP symbion_mqtt_messages_per_minute MQTT messages received per minute\n");
    output.push_str("# TYPE symbion_mqtt_messages_per_minute gauge\n");
    output.push_str(&format!("symbion_mqtt_messages_per_minute {:.2}\n", kernel_health.mqtt_messages_per_minute));

    output.push_str("# HELP symbion_mqtt_messages_total Total MQTT messages since startup\n");
    output.push_str("# TYPE symbion_mqtt_messages_total counter\n");
    output.push_str(&format!("symbion_mqtt_messages_total {}\n", kernel_health.mqtt_messages_total));

    // Agent Metrics
    let agents_map = app.agents.list_agents().await;
    let total_agents = agents_map.len();
    let online_agents = agents_map.values()
        .filter(|a| a.status.status == "online")
        .count();
    let offline_agents = total_agents - online_agents;

    output.push_str("# HELP symbion_kernel_agents_total Total number of registered agents\n");
    output.push_str("# TYPE symbion_kernel_agents_total gauge\n");
    output.push_str(&format!("symbion_kernel_agents_total {}\n", total_agents));

    output.push_str("# HELP symbion_kernel_agents_online Number of agents currently online\n");
    output.push_str("# TYPE symbion_kernel_agents_online gauge\n");
    output.push_str(&format!("symbion_kernel_agents_online {}\n", online_agents));

    output.push_str("# HELP symbion_kernel_agents_offline Number of agents currently offline\n");
    output.push_str("# TYPE symbion_kernel_agents_offline gauge\n");
    output.push_str(&format!("symbion_kernel_agents_offline {}\n", offline_agents));

    // Context Engine Metrics
    if let Some(context_state) = app.context_engine.get_state() {
        use crate::context::Mode;
        let mode_value = match context_state.mode {
            Mode::Veille => 0,
            Mode::Pro => 1,
            Mode::Maison => 2,
        };
        output.push_str("# HELP symbion_context_mode Current context mode (0=veille, 1=pro, 2=maison)\n");
        output.push_str("# TYPE symbion_context_mode gauge\n");
        output.push_str(&format!("symbion_context_mode {}\n", mode_value));

        output.push_str("# HELP symbion_context_confidence Context detection confidence (0.0-1.0)\n");
        output.push_str("# TYPE symbion_context_confidence gauge\n");
        output.push_str(&format!("symbion_context_confidence {:.2}\n", context_state.confidence));
    }

    // Plugin Metrics - Legacy system removed, now using plugin_proxy
    output.push_str("# HELP symbion_plugins_total Total number of registered plugins (legacy - always 0)\n");
    output.push_str("# TYPE symbion_plugins_total gauge\n");
    output.push_str("symbion_plugins_total 0\n");

    output.push_str("# HELP symbion_plugins_running Number of plugins currently running (legacy - always 0)\n");
    output.push_str("# TYPE symbion_plugins_running gauge\n");
    output.push_str("symbion_plugins_running 0\n");

    output.push_str("# HELP symbion_plugins_failed Number of plugins in failed state (legacy - always 0)\n");
    output.push_str("# TYPE symbion_plugins_failed gauge\n");
    output.push_str("symbion_plugins_failed 0\n");

    // Kernel Runtime Metrics
    output.push_str("# HELP symbion_kernel_uptime_seconds Kernel uptime in seconds since startup\n");
    output.push_str("# TYPE symbion_kernel_uptime_seconds gauge\n");
    output.push_str(&format!("symbion_kernel_uptime_seconds {}\n", kernel_health.uptime_seconds));

    output.push_str("# HELP symbion_kernel_memory_usage_mb Kernel memory usage in megabytes\n");
    output.push_str("# TYPE symbion_kernel_memory_usage_mb gauge\n");
    output.push_str(&format!("symbion_kernel_memory_usage_mb {:.2}\n", kernel_health.memory_usage_mb));

    output.push_str("# HELP symbion_contracts_loaded Number of MQTT contracts loaded\n");
    output.push_str("# TYPE symbion_contracts_loaded gauge\n");
    output.push_str(&format!("symbion_contracts_loaded {}\n", kernel_health.contracts_loaded));

    // TODO: Add HTTP request metrics (counter, latency histogram)
    // Requires instrumentation with prometheus middleware

    Ok(output)
}

// ============================================================================
// Logs API
// ============================================================================

/// Query parameters for the logs endpoint (level filter, search, limit, time range, trace ID).
#[derive(Deserialize, ToSchema, IntoParams)]
pub(crate) struct LogsQuery {
    level: Option<String>,    // comma-separated: "info,warn,error"
    search: Option<String>,
    limit: Option<u32>,       // default 200, max 1000
    since: Option<String>,    // "5m", "15m", "1h", "6h", "24h"
    trace_id: Option<String>, // filter by trace_id in message
}

/// A single parsed and sanitized log entry from journalctl output.
#[derive(serde::Serialize, ToSchema)]
pub(crate) struct LogEntry {
    timestamp: String,
    level: String,
    component: String,
    message: String,
    source: String,    // "kernel"
    #[schema(value_type = Object)]
    raw: serde_json::Value,
}

/// GET /logs - Récupère les logs kernel depuis journalctl
#[utoipa::path(
    get,
    path = "/logs",
    tag = "System",
    security(("bearer_auth" = [])),
    params(LogsQuery),
    responses(
        (status = 200, description = "Entrées de logs du kernel", body = Object),
        (status = 401, description = "Non authentifié"),
        (status = 500, description = "Erreur lors de la lecture des logs")
    )
)]
pub(super) async fn get_logs(
    Query(params): Query<LogsQuery>,
) -> Result<Json<serde_json::Value>, (StatusCode, String)> {
    let limit = params.limit.unwrap_or(200).min(1000);
    let since = params.since.as_deref().unwrap_or("1h");

    // Convert since shorthand to journalctl format (whitelist only)
    let since_arg = match since {
        "5m" => "5 minutes ago",
        "15m" => "15 minutes ago",
        "1h" => "1 hour ago",
        "6h" => "6 hours ago",
        "24h" => "24 hours ago",
        _ => "1 hour ago", // fallback safe — pas d'input arbitraire
    };

    let output = tokio::process::Command::new("journalctl")
        .args([
            "--output=json",
            "-u", "symbion-kernel",
            "--no-pager",
            "-n", &limit.to_string(),
            "--since", since_arg,
        ])
        .output()
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, format!("Failed to run journalctl: {}", e)))?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err((StatusCode::INTERNAL_SERVER_ERROR, format!("journalctl failed: {}", stderr)));
    }

    let stdout = String::from_utf8_lossy(&output.stdout);

    // Parse level filter
    let level_filter: Option<Vec<String>> = params.level.as_ref().map(|l| {
        l.split(',').map(|s| s.trim().to_lowercase()).collect()
    });

    let search_lower = params.search.as_ref().map(|s| s.to_lowercase());

    let mut entries: Vec<LogEntry> = Vec::new();

    for line in stdout.lines() {
        if line.trim().is_empty() {
            continue;
        }

        let raw: serde_json::Value = match serde_json::from_str(line) {
            Ok(v) => v,
            Err(_) => continue,
        };

        // Parse priority (syslog: 0=emerg .. 7=debug)
        let priority = raw.get("PRIORITY")
            .and_then(|v| v.as_str())
            .unwrap_or("6");
        let level = match priority {
            "0" | "1" | "2" => "critical",
            "3" => "error",
            "4" => "warning",
            "5" => "notice",
            "6" => "info",
            "7" => "debug",
            _ => "info",
        };

        // Apply level filter
        if let Some(ref filters) = level_filter {
            if !filters.iter().any(|f| f == level) {
                continue;
            }
        }

        // Parse message and extract component from [prefix]
        let message = raw.get("MESSAGE")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string();

        let component = if let Some(start) = message.find('[') {
            if let Some(end) = message[start..].find(']') {
                message[start + 1..start + end].to_string()
            } else {
                "kernel".to_string()
            }
        } else {
            "kernel".to_string()
        };

        // Apply search filter
        if let Some(ref search) = search_lower {
            if !message.to_lowercase().contains(search) && !component.to_lowercase().contains(search) {
                continue;
            }
        }

        // Apply trace_id filter
        if let Some(ref trace_id) = params.trace_id {
            if !message.contains(trace_id.as_str()) {
                continue;
            }
        }

        // Parse timestamp (microseconds since epoch)
        let timestamp = raw.get("__REALTIME_TIMESTAMP")
            .and_then(|v| v.as_str())
            .and_then(|ts| ts.parse::<i64>().ok())
            .map(|us| {
                let secs = us / 1_000_000;
                let nanos = ((us % 1_000_000) * 1000) as u32;
                let dt = chrono::DateTime::from_timestamp(secs, nanos)
                    .unwrap_or_default();
                dt.format("%Y-%m-%dT%H:%M:%S%.3fZ").to_string()
            })
            .unwrap_or_default();

        // Redact raw journalctl: keep only safe metadata fields
        let safe_raw = serde_json::json!({
            "PRIORITY": raw.get("PRIORITY"),
            "SYSLOG_IDENTIFIER": raw.get("SYSLOG_IDENTIFIER"),
            "_PID": raw.get("_PID"),
            "_SYSTEMD_UNIT": raw.get("_SYSTEMD_UNIT"),
            "__REALTIME_TIMESTAMP": raw.get("__REALTIME_TIMESTAMP"),
        });

        entries.push(LogEntry {
            timestamp,
            level: level.to_string(),
            component,
            message,
            source: "kernel".to_string(),
            raw: safe_raw,
        });
    }

    Ok(Json(serde_json::json!({
        "entries": entries,
        "total": entries.len()
    })))
}
