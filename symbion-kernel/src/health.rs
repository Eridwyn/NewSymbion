/**
 * HEALTH MONITORING - Surveillance temps réel de l'infrastructure Symbion
 * 
 * RÔLE :
 * Ce module assure le monitoring interne du kernel Symbion : uptime, mémoire, 
 * état MQTT, contrats chargés. Il publie automatiquement ces métriques sur MQTT.
 * 
 * FONCTIONNEMENT :
 * - Tracking continu des métriques vitales du kernel
 * - Auto-publication toutes les 30s sur symbion/kernel/health@v1
 * - API REST /system/health pour interrogation à la demande
 * - Surveillance état connexion MQTT avec compteur de reconnexions
 * 
 * UTILITÉ DANS SYMBION :
 * 🎯 Observabilité : visibilité temps réel sur l'état du kernel
 * 🎯 Détection pannes : alertes si kernel devient instable  
 * 🎯 Dashboard admin : métriques d'infrastructure dans l'interface
 * 🎯 Debug : corrélation entre problems et état système
 * 
 * MÉTRIQUES SURVEILLÉES :
 * - uptime_seconds : temps de fonctionnement depuis démarrage
 * - contracts_loaded : nombre de contrats MQTT chargés
 * - agents_count : nombre d'agents enregistrés
 * - memory_usage_mb : consommation RAM du processus kernel
 * - mqtt_status : état connexion (connected/disconnected/reconnecting)
 * - mqtt_reconnects : nombre de tentatives de reconnexion
 * 
 * PUBLICATION AUTOMATIQUE :
 * Toutes les 30s → topic symbion/kernel/health@v1 via MQTT
 */

use serde::{Deserialize, Serialize};
use std::time::{Duration, Instant};
use std::sync::Arc;
use std::sync::atomic::{AtomicU32, AtomicU64, Ordering};
use crate::state::Shared;
use crate::config::HostsConfig;
use crate::contracts::ContractRegistry;
use rumqttc::{AsyncClient, MqttOptions, QoS};
use tokio::task;

/// Snapshot des métriques de santé du kernel à un instant T
/// Structure sérialisable exposée via API REST et MQTT
#[derive(Debug, Serialize, Deserialize)]
pub struct KernelHealth {
    /// Durée de fonctionnement en secondes depuis le démarrage
    pub uptime_seconds: u64,
    /// Nombre de contrats MQTT chargés depuis contracts/mqtt/
    pub contracts_loaded: u32,
    /// Nombre d'agents actuellement enregistrés
    pub agents_count: u32,
    /// Consommation mémoire du processus kernel en MB
    pub memory_usage_mb: f32,
    /// État actuel connexion MQTT (connected/disconnected/reconnecting)
    pub mqtt_status: String,
    /// Compteur total des reconnexions MQTT depuis démarrage
    pub mqtt_reconnects: u32,
    /// Nombre total de plugins découverts
    pub plugins_total: u32,
    /// Nombre de plugins actuellement actifs (Running)
    pub plugins_active: u32,
    /// Nombre de plugins en échec
    pub plugins_failed: u32,
    /// Messages MQTT par minute (activité temps réel)
    pub mqtt_messages_per_minute: f32,
    /// Total des messages MQTT depuis le démarrage
    pub mqtt_messages_total: u64,
}

/// Tracker persistent des métriques de santé kernel
/// Maintient l'état entre les interrogations et coordonne la publication automatique
#[derive(Clone)]
pub struct HealthTracker {
    /// Instant de démarrage du kernel pour calcul uptime
    start_time: Instant,
    /// Compteur atomique thread-safe des reconnexions MQTT
    mqtt_reconnects: Arc<AtomicU32>,
    /// État actuel de la connexion MQTT (partagé entre threads)
    mqtt_status: Arc<parking_lot::Mutex<String>>,
    /// Compteur total des messages MQTT
    mqtt_message_counter: Arc<AtomicU64>,
    /// Historique des timestamps pour calcul messages/minute
    message_timestamps: Arc<parking_lot::Mutex<Vec<Instant>>>,
}

impl HealthTracker {
    pub fn new() -> Self {
        Self {
            start_time: Instant::now(),
            mqtt_reconnects: Arc::new(AtomicU32::new(0)),
            mqtt_status: Arc::new(parking_lot::Mutex::new("connecting".to_string())),
            mqtt_message_counter: Arc::new(AtomicU64::new(0)),
            message_timestamps: Arc::new(parking_lot::Mutex::new(Vec::new())),
        }
    }

    #[allow(dead_code)]
    pub fn mark_mqtt_connected(&self) {
        *self.mqtt_status.lock() = "connected".to_string();
    }

    #[allow(dead_code)]
    pub fn mark_mqtt_disconnected(&self) {
        *self.mqtt_status.lock() = "disconnected".to_string();
    }

    pub fn increment_reconnects(&self) {
        self.mqtt_reconnects.fetch_add(1, Ordering::Relaxed);
        *self.mqtt_status.lock() = "reconnecting".to_string();
    }

    pub fn record_mqtt_message(&self) {
        self.mqtt_message_counter.fetch_add(1, Ordering::Relaxed);
        let now = Instant::now();
        let mut timestamps = self.message_timestamps.lock();
        
        // Garder seulement les messages de la dernière minute
        timestamps.retain(|t| now.duration_since(*t).as_secs() < 60);
        timestamps.push(now);
    }

    pub fn get_health(&self, contracts: &ContractRegistry, agents: &crate::agents::SharedAgentRegistry, plugins: &Shared<crate::plugins::PluginManager>) -> KernelHealth {
        let uptime = self.start_time.elapsed().as_secs();
        let contracts_count = contracts.list_contracts().len() as u32;
        let agents_count = agents.agents_count();
        let memory_mb = get_memory_usage_mb();
        let mqtt_status = self.mqtt_status.lock().clone();
        let reconnects = self.mqtt_reconnects.load(Ordering::Relaxed);
        let total_messages = self.mqtt_message_counter.load(Ordering::Relaxed);
        
        // Calculer messages par minute
        let now = Instant::now();
        let timestamps = self.message_timestamps.lock();
        let recent_messages = timestamps.iter()
            .filter(|t| now.duration_since(**t).as_secs() < 60)
            .count();
        let messages_per_minute = recent_messages as f32;

        // Statistiques des plugins
        let plugin_infos = plugins.lock().list_plugins();
        let plugins_total = plugin_infos.len() as u32;
        let plugins_active = plugin_infos.iter()
            .filter(|p| matches!(p.status, crate::plugins::PluginStatus::Running))
            .count() as u32;
        let plugins_failed = plugin_infos.iter()
            .filter(|p| matches!(p.status, crate::plugins::PluginStatus::Failed(_)))
            .count() as u32;

        KernelHealth {
            uptime_seconds: uptime,
            contracts_loaded: contracts_count,
            agents_count,
            memory_usage_mb: memory_mb,
            mqtt_status,
            mqtt_reconnects: reconnects,
            plugins_total,
            plugins_active,
            plugins_failed,
            mqtt_messages_per_minute: messages_per_minute,
            mqtt_messages_total: total_messages,
        }
    }

    /// Démarre la publication auto du health kernel
    pub fn spawn_health_publisher(
        &self,
        config: Shared<HostsConfig>,
        contracts: ContractRegistry,
        agents: crate::agents::SharedAgentRegistry,
        plugins: Shared<crate::plugins::PluginManager>,
    ) {
        let health_tracker = self.clone();
        
        task::spawn(async move {
            // Setup MQTT client pour publish
            let cfg = config.lock().clone();
            let mqtt_cfg = cfg.mqtt.unwrap_or_else(|| crate::config::MqttConf { 
                host: "localhost".into(), 
                port: 1883 
            });
            
            let mut opts = MqttOptions::new("symbion-kernel-health", &mqtt_cfg.host, mqtt_cfg.port);
            opts.set_keep_alive(Duration::from_secs(15));
            
            let (client, mut eventloop) = AsyncClient::new(opts, 10);
            
            // Boucle principale : publish health toutes les 30s
            let mut interval = tokio::time::interval(Duration::from_secs(30));
            
            loop {
                tokio::select! {
                    _ = interval.tick() => {
                        let health = health_tracker.get_health(&contracts, &agents, &plugins);
                        if let Ok(payload) = serde_json::to_string(&health) {
                            if let Err(e) = client.publish("symbion/kernel/health@v1", QoS::AtLeastOnce, false, payload).await {
                                eprintln!("[health] failed to publish: {:?}", e);
                            } else {
                                println!("[health] published kernel health (uptime: {}s, agents: {})", 
                                        health.uptime_seconds, health.agents_count);
                            }
                        }
                    },
                    event = eventloop.poll() => {
                        match event {
                            Ok(_) => {}, // Ignore normal MQTT events
                            Err(e) => {
                                eprintln!("[health] MQTT error: {:?}", e);
                                health_tracker.increment_reconnects();
                                tokio::time::sleep(Duration::from_secs(2)).await;
                            }
                        }
                    }
                }
            }
        });
    }
}

fn get_memory_usage_mb() -> f32 {
    // Simple approximation - en production on pourrait utiliser sysinfo
    let pid = std::process::id();
    
    #[cfg(target_os = "linux")]
    {
        if let Ok(status) = std::fs::read_to_string(format!("/proc/{}/status", pid)) {
            for line in status.lines() {
                if line.starts_with("VmRSS:") {
                    if let Some(kb_str) = line.split_whitespace().nth(1) {
                        if let Ok(kb) = kb_str.parse::<u64>() {
                            return (kb as f32) / 1024.0; // KB -> MB
                        }
                    }
                }
            }
        }
    }
    
    // Fallback approximatif
    12.0
}