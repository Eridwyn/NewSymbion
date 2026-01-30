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
use std::collections::HashMap;
use time::OffsetDateTime;
use tokio::sync::RwLock;
use std::sync::Arc;
use rumqttc::AsyncClient;
use uuid::Uuid;
use anyhow::Result;
use std::time::Duration;

// Structures pour tracking des commandes en cours
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PendingCommand {
    pub command_id: String,
    pub agent_id: String,
    pub command_type: String,
    pub parameters: Option<serde_json::Value>,
    pub timestamp: OffsetDateTime,
    pub timeout: Duration,
    pub status: CommandStatus,
    pub output: Option<serde_json::Value>,
    pub error: Option<serde_json::Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
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
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentResponse {
    pub command_id: String,
    pub agent_id: String,
    pub status: String, // "success", "error", "in_progress", "cancelled"
    pub output: Option<serde_json::Value>,
    pub error: Option<serde_json::Value>,
    pub progress: Option<u32>,
    pub metadata: Option<serde_json::Value>,
    pub timestamp: String,
}

// Structures basées sur les contrats agents.registration@v1 et agents.heartbeat@v1
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Agent {
    pub agent_id: String,           // MAC sans colons (ex: a1b2c3d4e5f6)
    pub hostname: String,
    pub os: String,                 // linux, windows, android, macos
    pub architecture: String,       // x86_64, aarch64, arm, i686
    pub capabilities: Vec<String>,  // power_management, process_control, etc.
    pub network: AgentNetwork,
    pub version: Option<String>,
    pub status: AgentStatus,
    pub last_seen: OffsetDateTime,
    pub registration_time: OffsetDateTime,
    /// Timestamp Unix de soft-delete (None = actif, Some = supprimé)
    /// Purge automatique après 7 jours
    #[serde(skip_serializing_if = "Option::is_none")]
    pub deleted_at: Option<i64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentNetwork {
    pub primary_mac: String,        // Format avec colons (ex: a1:b2:c3:d4:e5:f6)
    pub interfaces: Vec<AgentInterface>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentInterface {
    pub name: String,               // eth0, wlan0, etc.
    pub mac: String,
    pub ip: String,
    #[serde(rename = "type")]
    pub interface_type: String,     // ethernet, wireless, loopback, other
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentStatus {
    pub status: String,             // online, idle, busy, maintenance
    pub last_heartbeat: Option<OffsetDateTime>,
    pub system: Option<AgentSystemMetrics>,
    pub processes: Option<AgentProcesses>,
    pub services: Option<Vec<AgentService>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentSystemMetrics {
    pub uptime_seconds: u64,
    pub cpu: AgentCpuMetrics,
    pub memory: AgentMemoryMetrics,
    pub disk: Option<Vec<AgentDiskMetrics>>,
    pub network: Option<AgentNetworkMetrics>,
    pub temperature: Option<AgentTemperatureMetrics>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentCpuMetrics {
    pub percent: f32,
    pub load_avg: Option<[f32; 3]>,  // [1min, 5min, 15min]
    pub core_count: Option<u32>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentMemoryMetrics {
    pub total_mb: u64,
    pub used_mb: u64,
    pub available_mb: Option<u64>,
    pub percent_used: f32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentDiskMetrics {
    pub path: String,
    pub total_gb: f64,
    pub used_gb: f64,
    pub free_gb: Option<f64>,
    pub percent_used: f32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentNetworkMetrics {
    pub interfaces: Vec<AgentNetworkInterface>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentNetworkInterface {
    pub name: String,
    pub bytes_sent: Option<u64>,
    pub bytes_recv: Option<u64>,
    pub packets_sent: Option<u64>,
    pub packets_recv: Option<u64>,
    pub is_up: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentTemperatureMetrics {
    pub cpu_celsius: Option<f32>,
    pub sensors: Option<Vec<AgentTemperatureSensor>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentTemperatureSensor {
    pub name: String,
    pub value: f32,
    pub unit: String,
    pub critical: Option<f32>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentProcesses {
    pub total_count: u32,
    pub running_count: u32,
    pub top_cpu: Option<Vec<AgentProcess>>,
    pub top_memory: Option<Vec<AgentProcess>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentProcess {
    pub pid: u32,
    pub name: String,
    pub cpu_percent: f32,
    pub memory_mb: f32,
    pub user: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentService {
    pub name: String,
    pub status: String,             // active, inactive, failed, unknown
    pub enabled: Option<bool>,      // peut être null si non déterminable
}

// Messages MQTT pour les commandes (kernel → agent)
#[derive(Debug, Serialize)]
pub struct AgentCommand {
    pub command_id: String,
    pub agent_id: String,
    pub command_type: String,       // shutdown, reboot, hibernate, kill_process, run_command, get_metrics
    pub parameters: Option<serde_json::Value>,
    pub timeout_seconds: Option<u32>,
    pub timestamp: String,
}

#[derive(Debug, Deserialize)]
pub struct AgentCommandResponse {
    #[allow(dead_code)]
    pub command_id: String,
    #[allow(dead_code)]
    pub agent_id: String,
    #[allow(dead_code)]
    pub status: String,             // success, error, timeout
    #[allow(dead_code)]
    pub result: Option<serde_json::Value>,
    #[allow(dead_code)]
    pub error_message: Option<String>,
    #[allow(dead_code)]
    pub timestamp: String,
}

// Messages MQTT entrants (agent → kernel)
#[derive(Debug, Deserialize)]
pub struct AgentRegistrationMessage {
    pub agent_id: String,
    pub hostname: String,
    pub os: String,
    pub architecture: String,
    pub capabilities: Vec<String>,
    pub network: AgentNetwork,
    pub version: Option<String>,
    #[allow(dead_code)]
    pub timestamp: String,
}

#[derive(Debug, Deserialize)]
pub struct AgentHeartbeatMessage {
    pub agent_id: String,
    pub status: String,
    pub system: AgentSystemMetrics,
    pub processes: Option<AgentProcesses>,
    pub services: Option<Vec<AgentService>>,
    #[allow(dead_code)]
    pub last_command: Option<AgentLastCommand>,
    #[allow(dead_code)]
    pub timestamp: String,
}

#[derive(Debug, Deserialize)]
pub struct AgentLastCommand {
    #[allow(dead_code)]
    pub command_id: String,
    #[allow(dead_code)]
    pub command_type: String,
    #[allow(dead_code)]
    pub status: String,
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

    /// Sauvegarde les agents dans le fichier JSON
    pub async fn save_agents(&self) -> Result<()> {
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
                agent.status.system = Some(msg.system);
                agent.status.processes = msg.processes;
                agent.status.services = msg.services;
                agent.last_seen = now;
                println!("[agents] agent {} updated - last_seen: {}", msg.agent_id, now);
            } else {
                println!("[agents] ❌ received heartbeat from UNKNOWN agent {} - not registered!", msg.agent_id);
                return Ok(());
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

    /// Récupère l'état d'une commande
    pub async fn get_command_status(&self, command_id: &str) -> Option<PendingCommand> {
        println!("[debug] get_command_status called for command: {}", command_id);
        let pending = self.pending_commands.read().await;
        let result = pending.get(command_id).cloned();
        println!("[debug] get_command_status result: {:?}", result.is_some());
        result
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
            
            // Si commande terminée, on pourrait la supprimer après un délai
            // ou la garder pour historique selon les besoins
            
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

                // Purge des agents soft-deleted après 7 jours (toutes les heures = 60 ticks)
                if purge_counter >= 60 {
                    purge_counter = 0;
                    let purged = registry.purge_deleted_agents().await;
                    if purged > 0 {
                        println!("[agents] purged {} agents older than 7 days", purged);
                    }
                }

                // Sauvegarder les changements SANS tenir de lock
                if let Err(e) = registry.save_agents().await {
                    eprintln!("[agents] failed to save agents during monitoring: {}", e);
                }
            }
        });
    }
}

pub type SharedAgentRegistry = Arc<AgentRegistry>;