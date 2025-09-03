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
    pub command_id: String,
    pub agent_id: String,
    pub status: String,             // success, error, timeout
    pub result: Option<serde_json::Value>,
    pub error_message: Option<String>,
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
    pub timestamp: String,
}

#[derive(Debug, Deserialize)]
pub struct AgentHeartbeatMessage {
    pub agent_id: String,
    pub status: String,
    pub system: AgentSystemMetrics,
    pub processes: Option<AgentProcesses>,
    pub services: Option<Vec<AgentService>>,
    pub last_command: Option<AgentLastCommand>,
    pub timestamp: String,
}

#[derive(Debug, Deserialize)]
pub struct AgentLastCommand {
    pub command_id: String,
    pub command_type: String,
    pub status: String,
    pub timestamp: String,
}

pub type AgentsMap = HashMap<String, Agent>;

pub struct AgentRegistry {
    agents: Arc<RwLock<AgentsMap>>,
    data_file: String,
    mqtt_client: Option<AsyncClient>,
}

impl AgentRegistry {
    pub fn new(data_file: &str) -> Self {
        Self {
            agents: Arc::new(RwLock::new(HashMap::new())),
            data_file: data_file.to_string(),
            mqtt_client: None,
        }
    }

    pub fn with_mqtt_client(mut self, client: AsyncClient) -> Self {
        self.mqtt_client = Some(client);
        self
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
        let agents_map = self.agents.read().await;
        let content = serde_json::to_string_pretty(&*agents_map)?;
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
        
        {
            let mut agents_map = self.agents.write().await;
            if let Some(agent) = agents_map.get_mut(&msg.agent_id) {
                agent.status.status = msg.status;
                agent.status.last_heartbeat = Some(now);
                agent.status.system = Some(msg.system);
                agent.status.processes = msg.processes;
                agent.status.services = msg.services;
                agent.last_seen = now;
            } else {
                println!("[agents] received heartbeat from unknown agent {}", msg.agent_id);
                return Ok(());
            }
        }

        // Sauvegarde périodique moins fréquente (on ne sauvegarde pas chaque heartbeat)
        // La sauvegarde sera fait par un job périodique ou lors d'events importants
        Ok(())
    }

    /// Liste tous les agents
    pub async fn list_agents(&self) -> AgentsMap {
        self.agents.read().await.clone()
    }

    /// Récupère un agent spécifique
    pub async fn get_agent(&self, agent_id: &str) -> Option<Agent> {
        self.agents.read().await.get(agent_id).cloned()
    }

    /// Envoie une commande à un agent via MQTT
    pub async fn send_command(&self, agent_id: &str, command_type: &str, parameters: Option<serde_json::Value>) -> Result<String> {
        let command_id = Uuid::new_v4().to_string();
        
        let command = AgentCommand {
            command_id: command_id.clone(),
            agent_id: agent_id.to_string(),
            command_type: command_type.to_string(),
            parameters,
            timeout_seconds: Some(30),
            timestamp: OffsetDateTime::now_utc().format(&time::format_description::well_known::Iso8601::DEFAULT)?,
        };

        if let Some(mqtt_client) = &self.mqtt_client {
            let topic = "symbion/agents/command@v1";
            let payload = serde_json::to_string(&command)?;
            
            mqtt_client.publish(topic, rumqttc::QoS::AtLeastOnce, false, payload).await?;
            println!("[agents] sent command {} to agent {}: {}", command_id, agent_id, command_type);
            
            Ok(command_id)
        } else {
            Err(anyhow::anyhow!("MQTT client not configured"))
        }
    }

    /// Marque un agent comme offline après timeout
    pub async fn mark_agent_offline(&self, agent_id: &str) {
        let mut agents_map = self.agents.write().await;
        if let Some(agent) = agents_map.get_mut(agent_id) {
            agent.status.status = "offline".to_string();
            println!("[agents] marked agent {} as offline", agent_id);
        }
    }

    /// Supprime les agents qui n'ont pas donné signe de vie depuis trop longtemps
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

    /// Surveille périodiquement les agents et marque ceux inactifs comme offline
    pub fn start_agent_monitoring(registry: SharedAgentRegistry, timeout_minutes: i64) {
        println!("[agents] starting agent monitoring (timeout: {}min)", timeout_minutes);
        
        tokio::spawn(async move {
            let mut interval = tokio::time::interval(std::time::Duration::from_secs(60)); // Check toutes les minutes
            
            loop {
                interval.tick().await;
                
                let now = OffsetDateTime::now_utc();
                let timeout_threshold = now - time::Duration::minutes(timeout_minutes);
                let mut agents_to_mark_offline = Vec::new();
                
                // Identifier les agents qui ont timeout
                {
                    let agents_map = registry.agents.read().await;
                    for (agent_id, agent) in agents_map.iter() {
                        if agent.status.status == "online" && agent.last_seen < timeout_threshold {
                            agents_to_mark_offline.push(agent_id.clone());
                        }
                    }
                }
                
                // Marquer les agents timeout comme offline
                for agent_id in agents_to_mark_offline {
                    registry.mark_agent_offline(&agent_id).await;
                }
                
                // Sauvegarder les changements
                if let Err(e) = registry.save_agents().await {
                    eprintln!("[agents] failed to save agents during monitoring: {}", e);
                }
            }
        });
    }
}

pub type SharedAgentRegistry = Arc<AgentRegistry>;