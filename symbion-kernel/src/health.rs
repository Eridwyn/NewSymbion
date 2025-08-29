use serde::{Deserialize, Serialize};
use std::time::{Duration, Instant};
use crate::state::Shared;
use crate::config::HostsConfig;
use crate::contracts::ContractRegistry;
use crate::models::HostsMap;
use rumqttc::{AsyncClient, MqttOptions, QoS};
use tokio::task;

#[derive(Debug, Serialize, Deserialize)]
pub struct KernelHealth {
    pub uptime_seconds: u64,
    pub contracts_loaded: u32,
    pub hosts_tracked: u32,
    pub memory_usage_mb: f32,
    pub mqtt_status: String,
    pub mqtt_reconnects: u32,
}

#[derive(Clone)]
pub struct HealthTracker {
    start_time: Instant,
    mqtt_reconnects: std::sync::Arc<std::sync::atomic::AtomicU32>,
    mqtt_status: std::sync::Arc<parking_lot::Mutex<String>>,
}

impl HealthTracker {
    pub fn new() -> Self {
        Self {
            start_time: Instant::now(),
            mqtt_reconnects: std::sync::Arc::new(std::sync::atomic::AtomicU32::new(0)),
            mqtt_status: std::sync::Arc::new(parking_lot::Mutex::new("connecting".to_string())),
        }
    }

    pub fn mark_mqtt_connected(&self) {
        *self.mqtt_status.lock() = "connected".to_string();
    }

    pub fn mark_mqtt_disconnected(&self) {
        *self.mqtt_status.lock() = "disconnected".to_string();
    }

    pub fn increment_reconnects(&self) {
        self.mqtt_reconnects.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        *self.mqtt_status.lock() = "reconnecting".to_string();
    }

    pub fn get_health(&self, contracts: &ContractRegistry, hosts: &Shared<HostsMap>) -> KernelHealth {
        let uptime = self.start_time.elapsed().as_secs();
        let contracts_count = contracts.list_contracts().len() as u32;
        let hosts_count = hosts.lock().len() as u32;
        let memory_mb = get_memory_usage_mb();
        let mqtt_status = self.mqtt_status.lock().clone();
        let reconnects = self.mqtt_reconnects.load(std::sync::atomic::Ordering::Relaxed);

        KernelHealth {
            uptime_seconds: uptime,
            contracts_loaded: contracts_count,
            hosts_tracked: hosts_count,
            memory_usage_mb: memory_mb,
            mqtt_status,
            mqtt_reconnects: reconnects,
        }
    }

    /// Démarre la publication auto du health kernel
    pub fn spawn_health_publisher(
        &self,
        config: Shared<HostsConfig>,
        contracts: ContractRegistry,
        hosts: Shared<HostsMap>,
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
                        let health = health_tracker.get_health(&contracts, &hosts);
                        if let Ok(payload) = serde_json::to_string(&health) {
                            if let Err(e) = client.publish("symbion/kernel/health@v1", QoS::AtLeastOnce, false, payload).await {
                                eprintln!("[health] failed to publish: {:?}", e);
                            } else {
                                println!("[health] published kernel health (uptime: {}s, hosts: {})", 
                                        health.uptime_seconds, health.hosts_tracked);
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