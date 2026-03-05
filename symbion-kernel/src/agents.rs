/**
 * AGENTS MANAGER - Gestion des agents système distribués sur le réseau LAN
 * 
 * RÔLE : Registration, persistance, télémétrie et contrôle des agents multi-OS.
 * Système de contrôle à distance avec Wake-on-LAN, power management, processus.
 * 
 * ARCHITECTURE : Registry agents avec persistance JSON + MQTT events + API REST.
 * UTILITÉ : Contrôle infrastructure réseau local depuis dashboard centralisé.
 */

use serde::{Deserialize, Serialize};
use std::collections::{HashMap, VecDeque};
use time::OffsetDateTime;
use tokio::sync::RwLock;
use std::sync::Arc;
use rumqttc::AsyncClient;
use uuid::Uuid;
use anyhow::Result;
use std::time::Duration;
use utoipa::ToSchema;

// Structures pour tracking des commandes en cours
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct PendingCommand {
    pub command_id: String,
    pub agent_id: String,
    pub command_type: String,
    #[schema(value_type = Option<Object>)]
    pub parameters: Option<serde_json::Value>,
    #[schema(value_type = String)]
    pub timestamp: OffsetDateTime,
    pub timeout: Duration,
    pub status: CommandStatus,
    #[schema(value_type = Option<Object>)]
    pub output: Option<serde_json::Value>,
    #[schema(value_type = Option<Object>)]
    pub error: Option<serde_json::Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub enum CommandStatus {
    Sent,
    Acknowledged,
    InProgress,
    Completed,
    Failed,
    Cancelled,
    TimedOut,
}

// Structure pour les réponses des agents (agents.response@v1)
// Uses serde(default) for optional fields to handle mismatches with agent format
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct AgentResponse {
    pub command_id: String,
    pub agent_id: String,
    pub status: String, // "success", "error", "in_progress", "cancelled"
    #[serde(default)]
    #[schema(value_type = Option<Object>)]
    pub output: Option<serde_json::Value>,
    #[serde(default)]
    #[schema(value_type = Option<Object>)]
    pub error: Option<serde_json::Value>,
    #[serde(default)]
    pub progress: Option<u32>,
    #[serde(default)]
    #[schema(value_type = Option<Object>)]
    pub metadata: Option<serde_json::Value>,
    /// Accepts any timestamp format (chrono DateTime or time crate ISO 8601)
    #[serde(default)]
    pub timestamp: serde_json::Value,
}

// Structures basées sur les contrats agents.registration@v1 et agents.heartbeat@v1
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct Agent {
    pub agent_id: String,           // MAC sans colons (ex: a1b2c3d4e5f6)
    pub hostname: String,
    pub os: String,                 // linux, windows, android, macos
    pub architecture: String,       // x86_64, aarch64, arm, i686
    pub capabilities: Vec<String>,  // power_management, process_control, etc.
    pub network: AgentNetwork,
    pub version: Option<String>,
    pub status: AgentStatus,
    #[schema(value_type = String)]
    pub last_seen: OffsetDateTime,
    #[schema(value_type = String)]
    pub registration_time: OffsetDateTime,
    /// Timestamp Unix de soft-delete (None = actif, Some = supprimé)
    /// Purge automatique après 7 jours
    #[serde(skip_serializing_if = "Option::is_none")]
    pub deleted_at: Option<i64>,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct AgentNetwork {
    pub primary_mac: String,        // Format avec colons (ex: a1:b2:c3:d4:e5:f6)
    pub interfaces: Vec<AgentInterface>,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct AgentInterface {
    pub name: String,               // eth0, wlan0, etc.
    pub mac: String,
    pub ip: String,
    #[serde(rename = "type")]
    pub interface_type: String,     // ethernet, wireless, loopback, other
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct AgentStatus {
    pub status: String,             // online, idle, busy, maintenance
    #[schema(value_type = Option<String>)]
    pub last_heartbeat: Option<OffsetDateTime>,
    pub system: Option<AgentSystemMetrics>,
    pub processes: Option<AgentProcesses>,
    pub services: Option<Vec<AgentService>>,
    /// Health score 0-100 (computed by watchdog every 60s)
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub health_score: Option<u8>,
    /// Detailed health score breakdown
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub health_details: Option<HealthScoreDetails>,
    /// Agent-side watchdog report (v2.5+)
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub watchdog: Option<AgentWatchdogReport>,
    /// Plugin data from agent plugins (v2.5+)
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub plugin_data: Option<HashMap<String, serde_json::Value>>,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct AgentSystemMetrics {
    pub uptime_seconds: u64,
    pub cpu: AgentCpuMetrics,
    pub memory: AgentMemoryMetrics,
    pub disk: Option<Vec<AgentDiskMetrics>>,
    pub network: Option<AgentNetworkMetrics>,
    pub temperature: Option<AgentTemperatureMetrics>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub gpu: Option<AgentGpuMetrics>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub disk_io: Option<AgentDiskIoMetrics>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub network_advanced: Option<AgentNetworkAdvancedMetrics>,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct AgentCpuMetrics {
    pub percent: f32,
    pub load_avg: Option<[f32; 3]>,  // [1min, 5min, 15min]
    pub core_count: Option<u32>,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct AgentMemoryMetrics {
    pub total_mb: u64,
    pub used_mb: u64,
    pub available_mb: Option<u64>,
    pub percent_used: f32,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct AgentDiskMetrics {
    pub path: String,
    pub total_gb: f64,
    pub used_gb: f64,
    pub free_gb: Option<f64>,
    pub percent_used: f32,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct AgentNetworkMetrics {
    pub interfaces: Vec<AgentNetworkInterface>,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct AgentNetworkInterface {
    pub name: String,
    pub bytes_sent: Option<u64>,
    pub bytes_recv: Option<u64>,
    pub packets_sent: Option<u64>,
    pub packets_recv: Option<u64>,
    pub is_up: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct AgentTemperatureMetrics {
    pub cpu_celsius: Option<f32>,
    pub sensors: Option<Vec<AgentTemperatureSensor>>,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct AgentTemperatureSensor {
    pub name: String,
    pub value: f32,
    pub unit: String,
    pub critical: Option<f32>,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct AgentGpuMetrics {
    pub gpus: Vec<AgentGpuInfo>,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct AgentGpuInfo {
    pub name: String,
    pub vendor: String,
    pub temperature_celsius: Option<f32>,
    pub utilization_percent: Option<f32>,
    pub memory_used_mb: Option<u64>,
    pub memory_total_mb: Option<u64>,
    pub fan_speed_percent: Option<f32>,
    pub power_watts: Option<f32>,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct AgentDiskIoMetrics {
    pub disks: Vec<AgentDiskIoInfo>,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct AgentDiskIoInfo {
    pub device: String,
    pub read_bytes_per_sec: u64,
    pub write_bytes_per_sec: u64,
    pub read_iops: u64,
    pub write_iops: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct AgentNetworkAdvancedMetrics {
    pub gateway_latency_ms: Option<f64>,
    pub dns_latency_ms: Option<f64>,
    pub active_connections: Option<u32>,
    pub interfaces: Option<Vec<AgentInterfaceBandwidth>>,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct AgentInterfaceBandwidth {
    pub name: String,
    pub rx_bytes_per_sec: u64,
    pub tx_bytes_per_sec: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct AgentProcesses {
    pub total_count: u32,
    pub running_count: u32,
    pub top_cpu: Option<Vec<AgentProcess>>,
    pub top_memory: Option<Vec<AgentProcess>>,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct AgentProcess {
    pub pid: u32,
    pub name: String,
    pub cpu_percent: f32,
    pub memory_mb: f32,
    pub user: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct AgentService {
    pub name: String,
    pub status: String,             // active, inactive, failed, unknown
    pub enabled: Option<bool>,      // peut être null si non déterminable
}

// Messages MQTT pour les commandes (kernel → agent)
#[derive(Debug, Serialize, ToSchema)]
pub struct AgentCommand {
    pub command_id: String,
    pub agent_id: String,
    pub command_type: String,       // shutdown, reboot, hibernate, kill_process, run_command, get_metrics
    #[schema(value_type = Option<Object>)]
    pub parameters: Option<serde_json::Value>,
    pub timeout_seconds: Option<u32>,
    pub timestamp: String,
}

/// MQTT contract: agent command response payload (fields required for deserialization)
#[derive(Debug, Deserialize, ToSchema)]
#[allow(dead_code)]
pub struct AgentCommandResponse {
    pub command_id: String,
    pub agent_id: String,
    pub status: String,             // success, error, timeout
    #[schema(value_type = Option<Object>)]
    pub result: Option<serde_json::Value>,
    pub error_message: Option<String>,
    pub timestamp: String,
}

// Messages MQTT entrants (agent → kernel)
#[derive(Debug, Deserialize, ToSchema)]
pub struct AgentRegistrationMessage {
    pub agent_id: String,
    pub hostname: String,
    pub os: String,
    pub architecture: String,
    pub capabilities: Vec<String>,
    pub network: AgentNetwork,
    pub version: Option<String>,
    #[allow(dead_code)] // MQTT contract: required for deserialization
    pub timestamp: String,
}

#[derive(Debug, Deserialize, ToSchema)]
pub struct AgentHeartbeatMessage {
    pub agent_id: String,
    pub status: String,
    pub system: AgentSystemMetrics,
    pub processes: Option<AgentProcesses>,
    pub services: Option<Vec<AgentService>>,
    #[allow(dead_code)] // MQTT contract: required for deserialization
    pub last_command: Option<AgentLastCommand>,
    /// Agent-side watchdog report (v2.5+)
    #[serde(default)]
    pub watchdog: Option<AgentWatchdogReport>,
    /// Plugin data from agent plugins (v2.5+)
    #[serde(default)]
    pub plugin_data: Option<HashMap<String, serde_json::Value>>,
    #[allow(dead_code)] // MQTT contract: required for deserialization
    pub timestamp: String,
}

/// MQTT contract: last command status from agent (fields required for deserialization)
#[derive(Debug, Deserialize, ToSchema)]
#[allow(dead_code)]
pub struct AgentLastCommand {
    pub command_id: String,
    pub command_type: String,
    pub status: String,
    pub timestamp: String,
}

// ========== Health Score (Watchdog B1) ==========

/// Detailed breakdown of an agent's health score (0-100).
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct HealthScoreDetails {
    pub heartbeat_score: u8,  // 0-25: regularity of heartbeats (jitter)
    pub command_score: u8,    // 0-25: success/failure ratio
    pub resource_score: u8,   // 0-25: CPU<90%, mem<95%, disk<95%
    pub uptime_score: u8,     // 0-25: online and stable
}

/// Internal tracking for health score computation.
#[derive(Debug, Default)]
struct AgentHealthTracking {
    /// Recent heartbeat interval timestamps (ring buffer, max 10)
    heartbeat_timestamps: VecDeque<OffsetDateTime>,
    /// Command success/failure counters
    commands_success: u32,
    commands_failed: u32,
}

// ========== Agent Watchdog Report (v2.5+) ==========

/// Watchdog subsystem health report from agent-host v2.5+.
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct AgentWatchdogReport {
    pub status: String,
    pub mqtt_status: String,
    pub metrics_status: String,
    pub heartbeat_status: String,
    pub recovery_attempts: u32,
}

// ========== Agent Logs (Log Streaming B2) ==========

/// Log entry received from an agent via MQTT.
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct AgentLogEntry {
    pub timestamp: String,
    pub level: String,
    pub message: String,
    pub module: Option<String>,
    /// Log source: "agent", "os_journal", "event_viewer" (v2.5+)
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub source: Option<String>,
}

/// MQTT payload for agent log forwarding.
#[derive(Debug, Deserialize)]
pub struct AgentLogMessage {
    pub agent_id: String,
    pub entries: Vec<AgentLogEntry>,
    #[allow(dead_code)]
    pub timestamp: String,
}

pub type AgentsMap = HashMap<String, Agent>;

pub struct AgentRegistry {
    agents: Arc<RwLock<AgentsMap>>,
    data_file: String,
    mqtt_client: Option<AsyncClient>,
    pending_commands: Arc<RwLock<HashMap<String, PendingCommand>>>,
    dirty: Arc<std::sync::atomic::AtomicBool>,
    /// Dispatcher pour événements automations
    automation_dispatcher: Arc<tokio::sync::RwLock<Option<crate::automations::EventDispatcher>>>,
    /// SQLite database (None = JSON-only fallback mode)
    db: Option<crate::database::SharedDatabase>,
    /// Health tracking data per agent (for watchdog score computation)
    health_tracking: Arc<RwLock<HashMap<String, AgentHealthTracking>>>,
    /// Agent logs ring buffer (max 100 entries per agent)
    agent_logs: Arc<RwLock<HashMap<String, VecDeque<AgentLogEntry>>>>,
}

impl AgentRegistry {
    pub fn new(data_file: &str) -> Self {
        Self {
            agents: Arc::new(RwLock::new(HashMap::new())),
            data_file: data_file.to_string(),
            mqtt_client: None,
            pending_commands: Arc::new(RwLock::new(HashMap::new())),
            dirty: Arc::new(std::sync::atomic::AtomicBool::new(false)),
            automation_dispatcher: Arc::new(tokio::sync::RwLock::new(None)),
            db: None,
            health_tracking: Arc::new(RwLock::new(HashMap::new())),
            agent_logs: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// Set database for SQLite persistence (called after construction).
    pub async fn set_database(&mut self, db: crate::database::SharedDatabase) {
        let count = crate::database::agent_queries::count_agents(&db).unwrap_or(0);
        if count > 0 {
            let rows = crate::database::agent_queries::list_agents(&db).unwrap_or_default();
            let mut agents = HashMap::new();
            for row in rows {
                let capabilities: Vec<String> = serde_json::from_str(&row.capabilities_json)
                    .unwrap_or_default();
                let network: AgentNetwork = serde_json::from_str(&row.network_json)
                    .unwrap_or_else(|_| AgentNetwork { primary_mac: String::new(), interfaces: vec![] });
                let status: AgentStatus = serde_json::from_str(&row.status_json)
                    .unwrap_or_else(|_| AgentStatus { status: "unknown".to_string(), last_heartbeat: None, system: None, processes: None, services: None, health_score: None, health_details: None, watchdog: None, plugin_data: None });
                let last_seen = OffsetDateTime::parse(&row.last_seen,
                    &time::format_description::well_known::Rfc3339).unwrap_or_else(|_| OffsetDateTime::now_utc());
                let registration_time = OffsetDateTime::parse(&row.registration_time,
                    &time::format_description::well_known::Rfc3339).unwrap_or_else(|_| OffsetDateTime::now_utc());

                agents.insert(row.agent_id.clone(), Agent {
                    agent_id: row.agent_id,
                    hostname: row.hostname,
                    os: row.os,
                    architecture: row.architecture,
                    capabilities,
                    network,
                    version: row.version,
                    status,
                    last_seen,
                    registration_time,
                    deleted_at: row.deleted_at.as_deref()
                        .and_then(|s| s.parse::<i64>().ok()),
                });
            }
            eprintln!("[agents] Loaded {} agents from SQLite", agents.len());
            *self.agents.write().await = agents;
        } else {
            // Seed DB from in-memory data
            self.persist_to_db(&db);
        }
        self.db = Some(db);
    }

    /// Persist all agents to SQLite (sync — DB calls are fast under parking_lot::Mutex)
    fn persist_to_db(&self, db: &crate::database::SharedDatabase) {
        // Use try_read to avoid deadlocks in sync context; skip if can't acquire
        let agents = match self.agents.try_read() {
            Ok(guard) => guard,
            Err(_) => return,
        };
        for a in agents.values() {
            let capabilities_json = serde_json::to_string(&a.capabilities).unwrap_or_else(|_| "[]".to_string());
            let network_json = serde_json::to_string(&a.network).unwrap_or_else(|_| "{}".to_string());
            let status_json = serde_json::to_string(&a.status).unwrap_or_else(|_| "{}".to_string());
            let last_seen = a.last_seen.format(&time::format_description::well_known::Rfc3339).unwrap_or_default();
            let registration_time = a.registration_time.format(&time::format_description::well_known::Rfc3339).unwrap_or_default();

            let row = crate::database::agent_queries::AgentRow {
                agent_id: a.agent_id.clone(),
                hostname: a.hostname.clone(),
                os: a.os.clone(),
                architecture: a.architecture.clone(),
                capabilities_json,
                network_json,
                version: a.version.clone(),
                status_json,
                last_seen,
                registration_time,
                deleted_at: a.deleted_at.map(|ts| ts.to_string()),
            };
            let _ = crate::database::agent_queries::upsert_agent(db, &row);
        }
    }

    pub fn with_mqtt_client(mut self, client: AsyncClient) -> Self {
        self.mqtt_client = Some(client);
        self
    }

    /// Set automation dispatcher for event dispatching
    pub async fn set_automation_dispatcher(&self, dispatcher: crate::automations::EventDispatcher) {
        let mut d = self.automation_dispatcher.write().await;
        *d = Some(dispatcher);
        eprintln!("[agents] automation dispatcher attached");
    }

    /// Charge les agents depuis le fichier JSON de persistance
    pub async fn load_agents(&mut self) -> Result<()> {
        if !std::path::Path::new(&self.data_file).exists() {
            println!("[agents] no existing agents file, starting fresh");
            return Ok(());
        }

        let content = tokio::fs::read_to_string(&self.data_file).await?;
        let agents: AgentsMap = serde_json::from_str(&content)?;
        
        let mut agents_map = self.agents.write().await;
        *agents_map = agents;
        
        println!("[agents] loaded {} agents from {}", agents_map.len(), self.data_file);
        Ok(())
    }

    /// Sauvegarde les agents (DB-primary, JSON-fallback)
    pub async fn save_agents(&self) -> Result<()> {
        // Try SQLite first
        if let Some(ref db) = self.db {
            self.persist_to_db(db);
            // Always write JSON as backup
            let _ = self.save_agents_json().await;
            return Ok(());
        }
        self.save_agents_json().await
    }

    /// JSON-only save (fallback)
    async fn save_agents_json(&self) -> Result<()> {
        // Clone data snapshot AVANT I/O pour minimiser durée du lock
        let agents_snapshot = {
            let agents_map = self.agents.read().await;
            agents_map.clone()
        }; // Libère le read lock immédiatement

        // Sérialisation et I/O SANS tenir de lock
        let content = serde_json::to_string_pretty(&agents_snapshot)?;
        tokio::fs::write(&self.data_file, content).await?;
        Ok(())
    }

    /// Traite un message de registration d'agent
    pub async fn handle_agent_registration(&self, msg: AgentRegistrationMessage) -> Result<()> {
        let now = OffsetDateTime::now_utc();
        
        let agent = Agent {
            agent_id: msg.agent_id.clone(),
            hostname: msg.hostname,
            os: msg.os,
            architecture: msg.architecture,
            capabilities: msg.capabilities,
            network: msg.network,
            version: msg.version,
            status: AgentStatus {
                status: "online".to_string(),
                last_heartbeat: Some(now),
                system: None,
                processes: None,
                services: None,
                health_score: None,
                health_details: None,
                watchdog: None,
                plugin_data: None,
            },
            last_seen: now,
            registration_time: now,
            deleted_at: None,
        };

        let hostname = agent.hostname.clone();
        
        {
            let mut agents_map = self.agents.write().await;
            agents_map.insert(msg.agent_id.clone(), agent);
        }

        if let Err(e) = self.save_agents().await {
            eprintln!("[agents] failed to save agents after registration: {}", e);
        }

        println!("[agents] registered agent {} ({})", msg.agent_id, hostname);
        Ok(())
    }

    /// Traite un message de heartbeat d'agent
    pub async fn handle_agent_heartbeat(&self, msg: AgentHeartbeatMessage) -> Result<()> {
        let now = OffsetDateTime::now_utc();
        let mut status_changed_to_online = false;
        let mut previous_status: Option<String> = None;

        {
            let mut agents_map = self.agents.write().await;
            if let Some(agent) = agents_map.get_mut(&msg.agent_id) {
                println!("[agents] updating heartbeat for agent {} - status: {}", msg.agent_id, msg.status);

                // Détecter si l'agent passe de offline à online
                if agent.status.status != msg.status {
                    previous_status = Some(agent.status.status.clone());
                    if agent.status.status == "offline" && msg.status == "online" {
                        status_changed_to_online = true;
                    }
                }

                agent.status.status = msg.status.clone();
                agent.status.last_heartbeat = Some(now);

                // Merge enriched metrics: keep previous values if new heartbeat doesn't include them
                if let Some(ref mut existing) = agent.status.system {
                    if msg.system.gpu.is_some() {
                        existing.gpu = msg.system.gpu;
                    }
                    if msg.system.disk_io.is_some() {
                        existing.disk_io = msg.system.disk_io;
                    }
                    if msg.system.network_advanced.is_some() {
                        existing.network_advanced = msg.system.network_advanced;
                    }
                    // Always update core metrics
                    existing.uptime_seconds = msg.system.uptime_seconds;
                    existing.cpu = msg.system.cpu;
                    existing.memory = msg.system.memory;
                    existing.disk = msg.system.disk;
                    existing.network = msg.system.network;
                    existing.temperature = msg.system.temperature;
                } else {
                    agent.status.system = Some(msg.system);
                }

                agent.status.processes = msg.processes;
                agent.status.services = msg.services;

                // Store v2.5 watchdog report
                if msg.watchdog.is_some() {
                    agent.status.watchdog = msg.watchdog;
                }

                // Store v2.5 plugin data (merge to keep previous values)
                if let Some(new_pd) = msg.plugin_data {
                    match &mut agent.status.plugin_data {
                        Some(existing) => {
                            for (k, v) in new_pd {
                                existing.insert(k, v);
                            }
                        }
                        None => {
                            agent.status.plugin_data = Some(new_pd);
                        }
                    }
                }

                agent.last_seen = now;
                println!("[agents] agent {} updated - last_seen: {}", msg.agent_id, now);
            } else {
                println!("[agents] ❌ received heartbeat from UNKNOWN agent {} - not registered!", msg.agent_id);
                return Ok(());
            }
        }

        // Track heartbeat timestamp for health score computation
        {
            let mut tracking = self.health_tracking.write().await;
            let entry = tracking.entry(msg.agent_id.clone()).or_default();
            entry.heartbeat_timestamps.push_back(now);
            if entry.heartbeat_timestamps.len() > 10 {
                entry.heartbeat_timestamps.pop_front();
            }
        }

        // Marquer comme dirty pour sauvegarde périodique
        self.dirty.store(true, std::sync::atomic::Ordering::Relaxed);

        // Dispatcher événement pour automations si status a changé vers online
        if status_changed_to_online {
            let dispatcher = self.automation_dispatcher.read().await;
            if let Some(ref d) = *dispatcher {
                d.dispatch_agent_status(&msg.agent_id, "online", previous_status.as_deref());
            }
        }

        Ok(())
    }

    /// Liste tous les agents actifs (exclut les soft-deleted)
    pub async fn list_agents(&self) -> AgentsMap {
        self.agents.read().await
            .iter()
            .filter(|(_, agent)| agent.deleted_at.is_none())
            .map(|(k, v)| (k.clone(), v.clone()))
            .collect()
    }

    /// Obtient le nombre d'agents actifs de façon synchrone (pour health check)
    pub fn agents_count(&self) -> u32 {
        self.agents.try_read()
            .map(|agents| agents.values().filter(|a| a.deleted_at.is_none()).count() as u32)
            .unwrap_or(0)
    }

    /// Récupère un agent spécifique (même si soft-deleted pour afficher info)
    pub async fn get_agent(&self, agent_id: &str) -> Option<Agent> {
        self.agents.read().await.get(agent_id).cloned()
    }

    /// Vérifie si un agent est online (sync, pour automations)
    pub fn is_agent_online(&self, agent_id: &str) -> bool {
        // Use try_read to avoid blocking - if lock unavailable, assume offline
        match self.agents.try_read() {
            Ok(agents) => agents
                .get(agent_id)
                .map(|a| a.status.status == "online" && a.deleted_at.is_none())
                .unwrap_or(false),
            Err(_) => false,
        }
    }

    /// Soft-delete un agent (sera purgé après 7 jours)
    pub async fn soft_delete_agent(&self, agent_id: &str) -> Result<bool> {
        let deleted = {
            let mut agents_map = self.agents.write().await;
            if let Some(agent) = agents_map.get_mut(agent_id) {
                if agent.deleted_at.is_some() {
                    return Ok(false); // Déjà supprimé
                }
                agent.deleted_at = Some(OffsetDateTime::now_utc().unix_timestamp());
                true
            } else {
                false
            }
        };

        if deleted {
            self.dirty.store(true, std::sync::atomic::Ordering::Relaxed);
            println!("[agents] soft-deleted agent {} (will be purged in 7 days)", agent_id);
        }
        Ok(deleted)
    }

    /// Purge les agents soft-deleted depuis plus de 7 jours
    pub async fn purge_deleted_agents(&self) -> usize {
        let now = OffsetDateTime::now_utc().unix_timestamp();
        let seven_days_secs = 7 * 24 * 60 * 60; // 7 jours en secondes

        let mut agents_map = self.agents.write().await;
        let initial_count = agents_map.len();

        agents_map.retain(|agent_id, agent| {
            if let Some(deleted_at) = agent.deleted_at {
                if now - deleted_at > seven_days_secs {
                    println!("[agents] purging agent {} (deleted {} days ago)",
                        agent_id, (now - deleted_at) / 86400);
                    return false;
                }
            }
            true
        });

        let purged = initial_count - agents_map.len();
        if purged > 0 {
            self.dirty.store(true, std::sync::atomic::Ordering::Relaxed);
        }
        purged
    }

    /// Envoie une commande à un agent via MQTT
    pub async fn send_command(&self, agent_id: &str, command_type: &str, parameters: Option<serde_json::Value>) -> Result<String> {
        let command_id = Uuid::new_v4().to_string();
        
        let command = AgentCommand {
            command_id: command_id.clone(),
            agent_id: agent_id.to_string(),
            command_type: command_type.to_string(),
            parameters: parameters.clone(),
            timeout_seconds: Some(30),
            timestamp: OffsetDateTime::now_utc().format(&time::format_description::well_known::Iso8601::DEFAULT)?,
        };

        if let Some(mqtt_client) = &self.mqtt_client {
            // Créer la commande en attente
            let pending_command = PendingCommand {
                command_id: command_id.clone(),
                agent_id: agent_id.to_string(),
                command_type: command_type.to_string(),
                parameters: parameters.clone(),
                timestamp: OffsetDateTime::now_utc(),
                timeout: Duration::from_secs(30),
                status: CommandStatus::Sent,
                output: None,
                error: None,
            };
            
            // Stocker la commande
            {
                let mut pending = self.pending_commands.write().await;
                pending.insert(command_id.clone(), pending_command);
            }
            
            let topic = "symbion/agents/command@v1";
            let payload = serde_json::to_string(&command)?;
            
            mqtt_client.publish(topic, rumqttc::QoS::AtLeastOnce, false, payload).await?;
            println!("[agents] sent command {} to agent {}: {}", command_id, agent_id, command_type);

            // Dual-write to SQLite command history
            if let Some(ref db) = self.db {
                let now_str = OffsetDateTime::now_utc()
                    .format(&time::format_description::well_known::Rfc3339)
                    .unwrap_or_default();
                let row = crate::database::command_history_queries::CommandHistoryRow {
                    command_id: command_id.clone(),
                    agent_id: agent_id.to_string(),
                    command_type: command_type.to_string(),
                    parameters_json: parameters.as_ref().map(|p| p.to_string()),
                    status: "Sent".to_string(),
                    output_json: None,
                    error_json: None,
                    timeout_seconds: 30,
                    created_at: now_str.clone(),
                    updated_at: now_str,
                    completed_at: None,
                };
                if let Err(e) = crate::database::command_history_queries::insert_command(db, &row) {
                    eprintln!("[agents] command history insert failed (non-fatal): {}", e);
                }
            }

            Ok(command_id)
        } else {
            Err(anyhow::anyhow!("MQTT client not configured"))
        }
    }

    /// Annule une commande en cours
    pub async fn cancel_command(&self, command_id: &str) -> Result<bool> {
        let mut pending = self.pending_commands.write().await;
        if let Some(command) = pending.get_mut(command_id) {
            match command.status {
                CommandStatus::Sent | CommandStatus::Acknowledged => {
                    command.status = CommandStatus::Cancelled;
                    
                    // Envoyer commande d'annulation à l'agent si MQTT disponible
                    if let Some(mqtt_client) = &self.mqtt_client {
                        let cancel_command = AgentCommand {
                            command_id: Uuid::new_v4().to_string(),
                            agent_id: command.agent_id.clone(),
                            command_type: "cancel".to_string(),
                            parameters: Some(serde_json::json!({"cancelled_command_id": command_id})),
                            timeout_seconds: Some(10),
                            timestamp: OffsetDateTime::now_utc().format(&time::format_description::well_known::Iso8601::DEFAULT).unwrap_or_default(),
                        };
                        
                        let topic = "symbion/agents/command@v1";
                        let payload = serde_json::to_string(&cancel_command)?;
                        mqtt_client.publish(topic, rumqttc::QoS::AtLeastOnce, false, payload).await?;
                    }
                    
                    println!("[agents] cancelled command {} for agent {}", command_id, command.agent_id);
                    Ok(true)
                }
                _ => {
                    println!("[agents] cannot cancel command {} - already {:?}", command_id, command.status);
                    Ok(false)
                }
            }
        } else {
            Err(anyhow::anyhow!("Command {} not found", command_id))
        }
    }

    /// Récupère l'état d'une commande (auto-timeout après 30s si toujours Sent/Acknowledged)
    pub async fn get_command_status(&self, command_id: &str) -> Option<PendingCommand> {
        // Check if command needs timeout first
        {
            let mut pending = self.pending_commands.write().await;
            if let Some(command) = pending.get_mut(command_id) {
                let elapsed = OffsetDateTime::now_utc() - command.timestamp;
                let timeout_secs = command.timeout.as_secs() as i64;
                match command.status {
                    CommandStatus::Sent | CommandStatus::Acknowledged => {
                        if elapsed.whole_seconds() > timeout_secs {
                            command.status = CommandStatus::TimedOut;
                            command.error = Some(serde_json::json!({
                                "code": "TIMEOUT",
                                "message": format!("Command timed out after {}s with no response", timeout_secs)
                            }));
                            println!("[agents] command {} timed out ({}s elapsed)", command_id, elapsed.whole_seconds());
                        }
                    }
                    _ => {}
                }
            }
        }

        let pending = self.pending_commands.read().await;
        pending.get(command_id).cloned()
    }

    /// Liste toutes les commandes en cours pour un agent
    pub async fn get_agent_pending_commands(&self, agent_id: &str) -> Vec<PendingCommand> {
        let pending = self.pending_commands.read().await;
        pending.values()
            .filter(|cmd| cmd.agent_id == agent_id)
            .cloned()
            .collect()
    }

    /// Traite une réponse d'agent (agents.response@v1)
    pub async fn handle_agent_response(&self, response: AgentResponse) -> Result<()> {
        println!("[debug] handle_agent_response called for command: {}", response.command_id);
        println!("[debug] response status: {}", response.status);
        println!("[debug] response output: {:?}", response.output);
        println!("[debug] response error: {:?}", response.error);
        let mut pending = self.pending_commands.write().await;
        
        if let Some(command) = pending.get_mut(&response.command_id) {
            // Mettre à jour le statut selon la réponse
            command.status = match response.status.as_str() {
                "success" => CommandStatus::Completed,
                "error" => CommandStatus::Failed,
                "in_progress" => CommandStatus::InProgress,
                "cancelled" => CommandStatus::Cancelled,
                _ => CommandStatus::Failed,
            };
            
            // Stocker la sortie et l'erreur
            command.output = response.output;
            command.error = response.error;
            
            println!("[agents] updated command {} status: {:?}", response.command_id, command.status);

            // Track command stats for health score
            let is_success = matches!(command.status, CommandStatus::Completed);
            let is_failure = matches!(command.status, CommandStatus::Failed | CommandStatus::TimedOut);
            let agent_id_clone = command.agent_id.clone();
            if is_success || is_failure {
                let mut tracking = self.health_tracking.write().await;
                let entry = tracking.entry(agent_id_clone).or_default();
                if is_success {
                    entry.commands_success += 1;
                } else {
                    entry.commands_failed += 1;
                }
            }

            // Dual-write status update to SQLite
            if let Some(ref db) = self.db {
                let status_str = format!("{:?}", command.status);
                let output_str = command.output.as_ref().map(|v| v.to_string());
                let error_str = command.error.as_ref().map(|v| v.to_string());
                let completed_at = match command.status {
                    CommandStatus::Completed | CommandStatus::Failed | CommandStatus::Cancelled | CommandStatus::TimedOut => {
                        Some(OffsetDateTime::now_utc()
                            .format(&time::format_description::well_known::Rfc3339)
                            .unwrap_or_default())
                    }
                    _ => None,
                };
                if let Err(e) = crate::database::command_history_queries::update_command_status(
                    db, &response.command_id, &status_str,
                    output_str.as_deref(), error_str.as_deref(),
                    completed_at.as_deref(),
                ) {
                    eprintln!("[agents] command history update failed (non-fatal): {}", e);
                }
            }

            Ok(())
        } else {
            // Commande inconnue, peut-être déjà supprimée ou timeout
            println!("[agents] received response for unknown command: {}", response.command_id);
            Ok(())
        }
    }

    /// Nettoie les commandes anciennes (timeout ou terminées)
    pub async fn cleanup_old_commands(&self, max_age_minutes: i64) -> Result<usize> {
        let cutoff = OffsetDateTime::now_utc() - time::Duration::minutes(max_age_minutes);
        let mut pending = self.pending_commands.write().await;
        
        let initial_count = pending.len();
        pending.retain(|_, cmd| {
            match cmd.status {
                CommandStatus::Completed | CommandStatus::Failed | CommandStatus::Cancelled => {
                    // Garder les commandes terminées pendant au moins 5 minutes pour historique
                    cmd.timestamp > cutoff - time::Duration::minutes(5)
                }
                CommandStatus::Sent | CommandStatus::Acknowledged | CommandStatus::InProgress => {
                    // Timeout les commandes en cours après max_age_minutes
                    cmd.timestamp > cutoff
                }
                CommandStatus::TimedOut => false, // Supprimer les timeouts
            }
        });
        
        let removed = initial_count - pending.len();
        if removed > 0 {
            println!("[agents] cleaned up {} old commands", removed);
        }
        
        Ok(removed)
    }

    /// Marque un agent comme offline après timeout
    pub async fn mark_agent_offline(&self, agent_id: &str) {
        let was_online = {
            let mut agents_map = self.agents.write().await;
            if let Some(agent) = agents_map.get_mut(agent_id) {
                let previous_status = agent.status.status.clone();
                agent.status.status = "offline".to_string();
                println!("[agents] marked agent {} as offline", agent_id);
                previous_status == "online"
            } else {
                false
            }
        }; // Libère le write lock AVANT toute opération I/O

        // Dispatcher événement pour automations si status a changé
        if was_online {
            let dispatcher = self.automation_dispatcher.read().await;
            if let Some(ref d) = *dispatcher {
                d.dispatch_agent_status(agent_id, "offline", Some("online"));
            }
        }
    }

    /// Supprime les agents qui n'ont pas donné signe de vie depuis trop longtemps
    #[allow(dead_code)]
    pub async fn cleanup_stale_agents(&self, max_age_hours: i64) -> Result<()> {
        let cutoff = OffsetDateTime::now_utc() - time::Duration::hours(max_age_hours);
        let mut removed_count = 0;
        
        {
            let mut agents_map = self.agents.write().await;
            agents_map.retain(|agent_id, agent| {
                if agent.last_seen < cutoff {
                    println!("[agents] removing stale agent {} (last seen: {})", agent_id, agent.last_seen);
                    removed_count += 1;
                    false
                } else {
                    true
                }
            });
        }
        
        if removed_count > 0 {
            self.save_agents().await?;
            println!("[agents] cleaned up {} stale agents", removed_count);
        }
        
        Ok(())
    }

    // ========== Agent Logs (Log Streaming B2) ==========

    /// Store log entries from an agent (ring buffer, max 100 per agent).
    pub async fn store_agent_logs(&self, agent_id: &str, entries: Vec<AgentLogEntry>) {
        let mut logs_map = self.agent_logs.write().await;
        let buffer = logs_map.entry(agent_id.to_string()).or_insert_with(|| VecDeque::with_capacity(100));
        for entry in entries {
            if buffer.len() >= 100 {
                buffer.pop_front();
            }
            buffer.push_back(entry);
        }
    }

    /// Retrieve log entries for an agent, optionally filtered by level.
    pub async fn get_agent_logs(&self, agent_id: &str, level_filter: Option<&str>) -> Vec<AgentLogEntry> {
        let logs_map = self.agent_logs.read().await;
        match logs_map.get(agent_id) {
            Some(buffer) => {
                if let Some(level) = level_filter {
                    let level_upper = level.to_uppercase();
                    buffer.iter()
                        .filter(|e| e.level.to_uppercase() == level_upper)
                        .cloned()
                        .collect()
                } else {
                    buffer.iter().cloned().collect()
                }
            }
            None => vec![],
        }
    }

    // ========== Command History (A2) ==========

    /// Get command history from SQLite for an agent.
    pub fn get_command_history(&self, agent_id: &str, limit: i64, offset: i64) -> Vec<crate::database::command_history_queries::CommandHistoryRow> {
        if let Some(ref db) = self.db {
            crate::database::command_history_queries::get_agent_history(db, agent_id, limit, offset)
                .unwrap_or_default()
        } else {
            vec![]
        }
    }

    /// Cleanup old command history entries (>30 days).
    pub fn cleanup_command_history(&self) {
        if let Some(ref db) = self.db {
            match crate::database::command_history_queries::cleanup_old_entries(db, 30) {
                Ok(n) if n > 0 => println!("[agents] cleaned up {} old command history entries", n),
                Err(e) => eprintln!("[agents] command history cleanup failed: {}", e),
                _ => {}
            }
        }
    }

    // ========== Health Score (Watchdog B1) ==========

    /// Compute and update health scores for all agents.
    pub async fn compute_health_scores(&self) {
        let tracking = self.health_tracking.read().await;
        let mut agents_map = self.agents.write().await;

        for (agent_id, agent) in agents_map.iter_mut() {
            if agent.deleted_at.is_some() {
                continue;
            }

            let track = tracking.get(agent_id);

            // uptime_score (0-25): online + recent heartbeat = 25, offline = 0
            let uptime_score = if agent.status.status == "online" {
                if let Some(hb) = agent.status.last_heartbeat {
                    let age = (OffsetDateTime::now_utc() - hb).whole_seconds();
                    if age < 120 { 25 } else if age < 300 { 15 } else { 5 }
                } else {
                    10
                }
            } else {
                0
            };

            // heartbeat_score (0-25): std dev of intervals, jitter < 5s = 25
            let heartbeat_score = if let Some(t) = track {
                compute_heartbeat_score(&t.heartbeat_timestamps)
            } else {
                if agent.status.status == "online" { 15 } else { 0 }
            };

            // command_score (0-25): success ratio. No commands = 25 (healthy)
            let command_score = if let Some(t) = track {
                let total = t.commands_success + t.commands_failed;
                if total == 0 {
                    25
                } else {
                    let ratio = t.commands_success as f32 / total as f32;
                    (ratio * 25.0) as u8
                }
            } else {
                25
            };

            // resource_score (0-25): deduct for high resource usage
            let resource_score = if let Some(ref sys) = agent.status.system {
                let mut score: i8 = 25;
                if sys.cpu.percent > 90.0 { score -= 10; }
                if sys.memory.percent_used > 95.0 { score -= 10; }
                if let Some(ref disks) = sys.disk {
                    if disks.iter().any(|d| d.percent_used > 95.0) { score -= 5; }
                }
                score.max(0) as u8
            } else {
                if agent.status.status == "online" { 20 } else { 0 }
            };

            let total = uptime_score + heartbeat_score + command_score + resource_score;
            agent.status.health_score = Some(total);
            agent.status.health_details = Some(HealthScoreDetails {
                heartbeat_score,
                command_score,
                resource_score,
                uptime_score,
            });
        }
    }

    /// Lance une tâche périodique de sauvegarde débounced (toutes les 5 min si dirty)
    pub fn start_periodic_save(registry: SharedAgentRegistry) {
        tokio::spawn(async move {
            let mut interval = tokio::time::interval(tokio::time::Duration::from_secs(300)); // 5 min
            loop {
                interval.tick().await;

                // Si dirty, sauvegarder
                if registry.dirty.swap(false, std::sync::atomic::Ordering::Relaxed) {
                    if let Err(e) = registry.save_agents().await {
                        eprintln!("[agents] periodic save failed: {}", e);
                    } else {
                        println!("[agents] ✅ periodic save completed");
                    }
                }
            }
        });
    }

    /// Surveille périodiquement les agents et marque ceux inactifs comme offline
    pub fn start_agent_monitoring(registry: SharedAgentRegistry, timeout_minutes: i64) {
        println!("[agents] starting agent monitoring (timeout: {}min)", timeout_minutes);

        tokio::spawn(async move {
            let mut interval = tokio::time::interval(std::time::Duration::from_secs(60)); // Check toutes les minutes
            let mut purge_counter = 0u32; // Compteur pour purge horaire

            loop {
                interval.tick().await;
                purge_counter += 1;

                let now = OffsetDateTime::now_utc();
                let timeout_threshold = now - time::Duration::minutes(timeout_minutes);
                let mut agents_to_mark_offline = Vec::new();

                // Identifier les agents qui ont timeout - Lock minimal
                {
                    let agents_map = registry.agents.read().await;
                    for (agent_id, agent) in agents_map.iter() {
                        // Ignorer les agents soft-deleted
                        if agent.deleted_at.is_some() {
                            continue;
                        }
                        if agent.status.status == "online" && agent.last_seen < timeout_threshold {
                            agents_to_mark_offline.push(agent_id.clone());
                        }
                    }
                } // Libère le read lock immédiatement

                // Marquer les agents timeout comme offline (déjà optimisé avec scope interne)
                for agent_id in agents_to_mark_offline {
                    registry.mark_agent_offline(&agent_id).await;
                }

                // Compute health scores every tick (60s)
                registry.compute_health_scores().await;

                // Purge des agents soft-deleted après 7 jours (toutes les heures = 60 ticks)
                if purge_counter >= 60 {
                    purge_counter = 0;
                    let purged = registry.purge_deleted_agents().await;
                    if purged > 0 {
                        println!("[agents] purged {} agents older than 7 days", purged);
                    }
                    // Also cleanup old command history (30 day retention)
                    registry.cleanup_command_history();
                }

                // Sauvegarder les changements SANS tenir de lock
                if let Err(e) = registry.save_agents().await {
                    eprintln!("[agents] failed to save agents during monitoring: {}", e);
                }
            }
        });
    }

    /// Proactive timeout checker: scans pending commands every 5s and marks timed-out ones.
    /// Also sends a cancel signal to the agent via MQTT so it can abort the running process.
    pub fn start_command_timeout_checker(registry: SharedAgentRegistry) {
        println!("[agents] starting proactive command timeout checker (5s interval)");

        tokio::spawn(async move {
            let mut interval = tokio::time::interval(std::time::Duration::from_secs(5));

            loop {
                interval.tick().await;

                let now = OffsetDateTime::now_utc();
                let mut timed_out_commands: Vec<(String, String)> = Vec::new(); // (command_id, agent_id)

                // Scan and mark timed-out commands
                {
                    let mut pending = registry.pending_commands.write().await;
                    for (cmd_id, command) in pending.iter_mut() {
                        match command.status {
                            CommandStatus::Sent | CommandStatus::Acknowledged => {
                                let elapsed = now - command.timestamp;
                                let timeout_secs = command.timeout.as_secs() as i64;
                                if elapsed.whole_seconds() > timeout_secs {
                                    command.status = CommandStatus::TimedOut;
                                    command.error = Some(serde_json::json!({
                                        "code": "TIMEOUT",
                                        "message": format!("Command timed out after {}s with no response", timeout_secs)
                                    }));
                                    timed_out_commands.push((cmd_id.clone(), command.agent_id.clone()));
                                    println!("[agents] proactive timeout: command {} ({}s elapsed)", cmd_id, elapsed.whole_seconds());
                                }
                            }
                            _ => {}
                        }
                    }
                }

                // Send cancel signals to agents for timed-out commands
                if let Some(mqtt_client) = &registry.mqtt_client {
                    for (cmd_id, agent_id) in &timed_out_commands {
                        let cancel_command = AgentCommand {
                            command_id: Uuid::new_v4().to_string(),
                            agent_id: agent_id.clone(),
                            command_type: "cancel".to_string(),
                            parameters: Some(serde_json::json!({"cancelled_command_id": cmd_id})),
                            timeout_seconds: Some(10),
                            timestamp: now.format(&time::format_description::well_known::Iso8601::DEFAULT).unwrap_or_default(),
                        };
                        if let Ok(payload) = serde_json::to_string(&cancel_command) {
                            let _ = mqtt_client.publish("symbion/agents/command@v1", rumqttc::QoS::AtLeastOnce, false, payload).await;
                        }
                    }
                }

                // Update SQLite for timed-out commands
                if let Some(ref db) = registry.db {
                    for (cmd_id, _) in &timed_out_commands {
                        let now_str = now.format(&time::format_description::well_known::Rfc3339).unwrap_or_default();
                        let error_str = serde_json::json!({"code": "TIMEOUT", "message": "Command timed out"}).to_string();
                        let _ = crate::database::command_history_queries::update_command_status(
                            db, cmd_id, "TimedOut",
                            None,
                            Some(&error_str),
                            Some(&now_str),
                        );
                    }
                }

                // Cleanup old completed/failed/timed-out commands (>5 minutes old)
                {
                    let mut pending = registry.pending_commands.write().await;
                    let before = pending.len();
                    pending.retain(|_, cmd| {
                        match cmd.status {
                            CommandStatus::Completed | CommandStatus::Failed | CommandStatus::TimedOut | CommandStatus::Cancelled => {
                                let elapsed = now - cmd.timestamp;
                                elapsed.whole_seconds() < 300 // Keep for 5 minutes
                            }
                            _ => true, // Keep active commands
                        }
                    });
                    let removed = before - pending.len();
                    if removed > 0 {
                        println!("[agents] cleaned up {} stale pending commands", removed);
                    }
                }
            }
        });
    }
}

/// Compute heartbeat regularity score from timestamps (0-25).
/// Expected interval is ~30s. Jitter < 5s = 25, > 30s = 0.
fn compute_heartbeat_score(timestamps: &VecDeque<OffsetDateTime>) -> u8 {
    if timestamps.len() < 2 {
        return 15; // Not enough data, assume moderate
    }

    let ts_vec: Vec<&OffsetDateTime> = timestamps.iter().collect();
    let intervals: Vec<f64> = ts_vec.windows(2)
        .map(|w| (*w[1] - *w[0]).whole_seconds() as f64)
        .collect();

    if intervals.is_empty() {
        return 15;
    }

    let mean = intervals.iter().sum::<f64>() / intervals.len() as f64;
    let variance = intervals.iter()
        .map(|&x| (x - mean).powi(2))
        .sum::<f64>() / intervals.len() as f64;
    let std_dev = variance.sqrt();

    // Jitter is the std deviation from expected 30s interval
    if std_dev < 5.0 { 25 }
    else if std_dev < 10.0 { 20 }
    else if std_dev < 20.0 { 15 }
    else if std_dev < 30.0 { 10 }
    else { 0 }
}

pub type SharedAgentRegistry = Arc<AgentRegistry>;