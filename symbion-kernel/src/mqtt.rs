/**
 * EVENT BUS MQTT - Réception des événements des plugins Symbion
 * 
 * RÔLE : Écoute continue du broker MQTT pour traiter les heartbeats des hosts.
 * Maintient l'état temps réel des machines connectées au système.
 * 
 * FONCTIONNEMENT : Client MQTT async, parsing JSON, mise à jour thread-safe des états.
 * UTILITÉ : Télémétrie centralisée, monitoring distribué, resilience réseau.
 */

use crate::models::{HeartbeatIn, HostState, HostsMap};
use crate::state::Shared;
use crate::config::HostsConfig;
use crate::notes_bridge::{SharedNotesBridge, NoteResponse};
use crate::agents::{SharedAgentRegistry, AgentRegistrationMessage, AgentHeartbeatMessage, AgentResponse};
use crate::sensors::{SharedSensorRegistry, SensorRegistrationMessage, SensorEnvMessage};
use crate::notifications::SharedNotificationManager;
use crate::intelligence::{SharedFeatureRegistry, FeatureValue, feature_ids, ttl};
use crate::wol::trigger_wol_udp;
use rumqttc::{AsyncClient, Event, MqttOptions, QoS};
use serde::Deserialize;
use time::OffsetDateTime;
use tokio::task;

/// Message Wake-on-LAN via MQTT
#[derive(Debug, Deserialize)]
struct WakeRequest {
    agent_id: String,
}

/// Message d'acquittement notification via MQTT (PWA toast)
#[derive(Debug, Deserialize)]
struct NotificationAckRequest {
    notification_id: String,
}

/// Crée un client MQTT configuré pour le kernel avec son eventloop
pub fn create_mqtt_client(config: &HostsConfig) -> Result<AsyncClient, Box<dyn std::error::Error + Send + Sync>> {
    let mqtt_cfg = config.mqtt.clone().unwrap_or_else(|| crate::config::MqttConf { 
        host: "localhost".into(), 
        port: 1883 
    });
    
    let mut opts = MqttOptions::new("symbion-kernel-bridge", &mqtt_cfg.host, mqtt_cfg.port);
    opts.set_keep_alive(std::time::Duration::from_secs(15));
    opts.set_max_packet_size(1024 * 1024, 1024 * 1024); // 1 MB max pour gros payloads (notes, etc.)
    let (client, mut eventloop) = AsyncClient::new(opts, 10);
    
    // Lancer l'eventloop du client bridge en arrière-plan
    tokio::spawn(async move {
        loop {
            if let Err(e) = eventloop.poll().await {
                eprintln!("[mqtt-bridge] eventloop error: {:?}", e);
                tokio::time::sleep(std::time::Duration::from_secs(1)).await;
            }
        }
    });
    
    Ok(client)
}

pub fn spawn_mqtt_listener(states: Shared<HostsMap>, config: Shared<HostsConfig>, notes_bridge: Option<SharedNotesBridge>, agents: Option<SharedAgentRegistry>, sensors: Option<SharedSensorRegistry>, health_tracker: Option<crate::health::HealthTracker>, dashboard_events: Option<crate::dashboard_events::DashboardEventPublisher>, mqtt_watchdog: Option<crate::mqtt_watchdog::SharedMqttWatchdog>, notifications_manager: Option<SharedNotificationManager>, feature_registry: Option<SharedFeatureRegistry>) {
    task::spawn(async move {
        let cfg = config.lock().clone();
        let mqtt_cfg = cfg.mqtt.clone().unwrap_or_else(|| crate::config::MqttConf {
            host: "localhost".into(),
            port: 1883
        });
        
        let mut opts = MqttOptions::new("symbion-kernel-listener", &mqtt_cfg.host, mqtt_cfg.port);
        opts.set_keep_alive(std::time::Duration::from_secs(15));
        opts.set_max_packet_size(1024 * 1024, 1024 * 1024); // 1 MB max pour gros payloads (notes, etc.)
        let (client, mut eventloop) = AsyncClient::new(opts, 200); // Buffer increased for streaming (100+ notes)
        
        if let Err(e) = client.subscribe("symbion/hosts/heartbeat@v2", QoS::AtLeastOnce).await {
            eprintln!("[kernel] subscribe MQTT failed: {e:?}");
            return;
        }
        
        // S'abonner aux réponses des notes si bridge disponible
        if notes_bridge.is_some() {
            if let Err(e) = client.subscribe("symbion/notes/response@v1", QoS::AtLeastOnce).await {
                eprintln!("[kernel] subscribe notes responses failed: {e:?}");
            }
        }

        // S'abonner aux événements agents si registry disponible
        if agents.is_some() {
            if let Err(e) = client.subscribe("symbion/agents/registration@v1", QoS::AtLeastOnce).await {
                eprintln!("[kernel] subscribe agents registration failed: {e:?}");
            }
            if let Err(e) = client.subscribe("symbion/agents/heartbeat@v1", QoS::AtLeastOnce).await {
                eprintln!("[kernel] subscribe agents heartbeat failed: {e:?}");
            }
            if let Err(e) = client.subscribe("symbion/agents/response@v1", QoS::AtLeastOnce).await {
                eprintln!("[kernel] subscribe agents response failed: {e:?}");
            }
            // Wake-on-LAN via MQTT
            if let Err(e) = client.subscribe("symbion/agents/wake@v1", QoS::AtLeastOnce).await {
                eprintln!("[kernel] subscribe agents wake failed: {e:?}");
            }
        }

        // F1: S'abonner aux événements sensors si registry disponible
        if sensors.is_some() {
            if let Err(e) = client.subscribe("symbion/sensors/registration@v1", QoS::AtLeastOnce).await {
                eprintln!("[kernel] subscribe sensors registration failed: {e:?}");
            }
            if let Err(e) = client.subscribe("symbion/sensors/+/env@v1", QoS::AtLeastOnce).await {
                eprintln!("[kernel] subscribe sensors env readings failed: {e:?}");
            }
        }

        // S'abonner aux acquittements notifications PWA (toast)
        if notifications_manager.is_some() {
            if let Err(e) = client.subscribe("symbion/notifications/acknowledge@v1", QoS::AtLeastOnce).await {
                eprintln!("[kernel] subscribe notifications acknowledge failed: {e:?}");
            }
        }

        // Marquer MQTT comme connecté après subscriptions réussies
        if let Some(ref tracker) = health_tracker {
            tracker.mark_mqtt_connected();
            println!("[kernel] MQTT connected and subscriptions active");
        }

        loop {
            match eventloop.poll().await {
                Ok(Event::Incoming(rumqttc::Incoming::Publish(p))) => {
                    // Enregistrer l'activité MQTT
                    if let Some(ref tracker) = health_tracker {
                        tracker.record_mqtt_message();
                    }
                    // Notifier le watchdog qu'un message a été reçu
                    if let Some(ref watchdog) = mqtt_watchdog {
                        watchdog.message_received();
                    }
                    
                    if p.topic == "symbion/hosts/heartbeat@v2" {
                    if let Ok(txt) = String::from_utf8(p.payload.to_vec()) {
                        match serde_json::from_str::<HeartbeatIn>(&txt) {
                            Ok(hb) => {
                                let st = HostState {
                                    host_id: hb.host_id,
                                    last_seen: OffsetDateTime::now_utc(),
                                    cpu: Some(hb.metrics.cpu),
                                    ram: Some(hb.metrics.ram),
                                    ip: Some(hb.net.ip),
                                };
                                states.lock().insert(st.host_id.clone(), st);
                            }
                            Err(_) => eprintln!("[kernel] heartbeat JSON invalide: {txt}"),
                        }
                    }
                } else if p.topic == "symbion/notes/response@v1" {
                    if let Some(ref bridge) = notes_bridge {
                        if let Ok(txt) = String::from_utf8(p.payload.to_vec()) {
                            match serde_json::from_str::<NoteResponse>(&txt) {
                                Ok(response) => {
                                    bridge.handle_response(response);
                                }
                                Err(e) => eprintln!("[kernel] notes response JSON invalide: {}, error: {}", &txt[..txt.len().min(200)], e),
                            }
                        }
                    }
                } else if p.topic == "symbion/agents/registration@v1" {
                    if let Some(ref agent_registry) = agents {
                        if let Ok(txt) = String::from_utf8(p.payload.to_vec()) {
                            println!("[kernel] received registration MQTT: {}", txt);
                            match serde_json::from_str::<AgentRegistrationMessage>(&txt) {
                                Ok(registration) => {
                                    println!("[kernel] registration parsed for agent: {}", registration.agent_id);
                                    if let Err(e) = agent_registry.handle_agent_registration(registration).await {
                                        eprintln!("[kernel] failed to handle agent registration: {}", e);
                                    } else {
                                        println!("[kernel] registration handled successfully");
                                    }
                                }
                                Err(e) => eprintln!("[kernel] agent registration JSON invalide: {txt}, error: {}", e),
                            }
                        }
                    }
                } else if p.topic == "symbion/agents/heartbeat@v1" {
                    if let Some(ref agent_registry) = agents {
                        if let Ok(txt) = String::from_utf8(p.payload.to_vec()) {
                            println!("[kernel] received heartbeat MQTT: {}", txt);
                            match serde_json::from_str::<AgentHeartbeatMessage>(&txt) {
                                Ok(heartbeat) => {
                                    println!("[kernel] heartbeat parsed for agent: {}", heartbeat.agent_id);

                                    // Extraire données pour features AVANT consommation
                                    let agent_id = heartbeat.agent_id.clone();
                                    let cpu_percent = heartbeat.system.cpu.percent;
                                    let memory_used = heartbeat.system.memory.used_mb;
                                    let memory_total = heartbeat.system.memory.total_mb;
                                    let process_names: Vec<String> = heartbeat.processes.as_ref()
                                        .map(|p| {
                                            let mut names = Vec::new();
                                            if let Some(top_cpu) = &p.top_cpu {
                                                names.extend(top_cpu.iter().map(|proc| proc.name.clone()));
                                            }
                                            if let Some(top_mem) = &p.top_memory {
                                                names.extend(top_mem.iter().map(|proc| proc.name.clone()));
                                            }
                                            names
                                        })
                                        .unwrap_or_default();

                                    if let Err(e) = agent_registry.handle_agent_heartbeat(heartbeat).await {
                                        eprintln!("[kernel] failed to handle agent heartbeat: {}", e);
                                    } else {
                                        println!("[kernel] heartbeat handled successfully");

                                        // Publier la liste des agents sur le dashboard topic
                                        if let Some(ref dash_events) = dashboard_events {
                                            let agents_map = agent_registry.list_agents().await;
                                            let agents_list: Vec<crate::agents::Agent> = agents_map.values().cloned().collect();
                                            if let Err(e) = dash_events.publish_agents_update(&agents_list).await {
                                                eprintln!("[kernel] failed to publish agents update to dashboard: {}", e);
                                            }
                                        }

                                        // Alimenter le FeatureRegistry avec les métriques agent
                                        if let Some(ref features) = feature_registry {
                                            let source = format!("agent.{}", agent_id);

                                            // Agent online
                                            features.set_feature(
                                                feature_ids::AGENT_ONLINE,
                                                FeatureValue::Bool(true),
                                                &source,
                                                1.0,
                                                ttl::AGENT,
                                            );

                                            // CPU usage
                                            features.set_feature(
                                                feature_ids::AGENT_CPU_USAGE,
                                                FeatureValue::Float(cpu_percent as f64),
                                                &source,
                                                1.0,
                                                ttl::AGENT,
                                            );

                                            // Memory usage (percentage)
                                            let memory_pct = if memory_total > 0 {
                                                (memory_used as f64 / memory_total as f64) * 100.0
                                            } else {
                                                0.0
                                            };
                                            features.set_feature(
                                                feature_ids::AGENT_MEMORY_USAGE,
                                                FeatureValue::Float(memory_pct),
                                                &source,
                                                1.0,
                                                ttl::AGENT,
                                            );

                                            // Process list for classification
                                            if !process_names.is_empty() {
                                                features.set_feature(
                                                    "process.list",
                                                    FeatureValue::StringList(process_names),
                                                    &source,
                                                    1.0,
                                                    ttl::PROCESS,
                                                );
                                            }

                                            println!("[kernel] features updated from agent {}", agent_id);
                                        }
                                    }
                                }
                                Err(e) => eprintln!("[kernel] agent heartbeat JSON invalide: {txt}, error: {}", e),
                            }
                        }
                    }
                } else if p.topic == "symbion/agents/response@v1" {
                    if let Some(ref agent_registry) = agents {
                        if let Ok(txt) = String::from_utf8(p.payload.to_vec()) {
                            match serde_json::from_str::<AgentResponse>(&txt) {
                                Ok(response) => {
                                    if let Err(e) = agent_registry.handle_agent_response(response).await {
                                        eprintln!("[kernel] failed to handle agent response: {}", e);
                                    }
                                }
                                Err(e) => eprintln!("[kernel] agent response JSON invalide: {txt}, error: {}", e),
                            }
                        }
                    }
                } else if p.topic == "symbion/sensors/registration@v1" {
                    // F1: Sensor auto-registration
                    if let Some(ref sensor_registry) = sensors {
                        if let Ok(txt) = String::from_utf8(p.payload.to_vec()) {
                            println!("[kernel] received sensor registration MQTT: {}", txt);
                            match serde_json::from_str::<SensorRegistrationMessage>(&txt) {
                                Ok(registration) => {
                                    if let Err(e) = sensor_registry.handle_registration(registration) {
                                        eprintln!("[kernel] failed to handle sensor registration: {}", e);
                                    } else {
                                        println!("[kernel] sensor registration handled successfully");
                                    }
                                }
                                Err(e) => eprintln!("[kernel] sensor registration JSON invalide: {txt}, error: {}", e),
                            }
                        }
                    }
                } else if p.topic.starts_with("symbion/sensors/") && p.topic.ends_with("/env@v1") {
                    // F1: Environment sensor readings (topic pattern: symbion/sensors/{sensor_id}/env@v1)
                    if let Some(ref sensor_registry) = sensors {
                        if let Ok(txt) = String::from_utf8(p.payload.to_vec()) {
                            match serde_json::from_str::<SensorEnvMessage>(&txt) {
                                Ok(msg) => {
                                    println!("[kernel] received env reading from sensor {}: {}°C, {}%",
                                        msg.sensor_id, msg.temperature_c, msg.humidity_pct);

                                    // Extraire données pour features AVANT consommation
                                    let sensor_id = msg.sensor_id.clone();
                                    let temperature = msg.temperature_c;
                                    let humidity = msg.humidity_pct;

                                    if let Err(e) = sensor_registry.handle_env_reading(msg) {
                                        eprintln!("[kernel] failed to handle env reading: {}", e);
                                    } else {
                                        // Alimenter le FeatureRegistry avec les données environnementales
                                        if let Some(ref features) = feature_registry {
                                            let source = format!("sensor.{}", sensor_id);

                                            // Temperature
                                            features.set_feature(
                                                feature_ids::ENV_TEMPERATURE,
                                                FeatureValue::Float(temperature as f64),
                                                &source,
                                                1.0,
                                                ttl::ENVIRONMENT,
                                            );

                                            // Humidity
                                            features.set_feature(
                                                feature_ids::ENV_HUMIDITY,
                                                FeatureValue::Float(humidity as f64),
                                                &source,
                                                1.0,
                                                ttl::ENVIRONMENT,
                                            );

                                            println!("[kernel] env features updated from sensor {}", sensor_id);
                                        }
                                    }
                                }
                                Err(e) => eprintln!("[kernel] sensor env JSON invalide: {txt}, error: {}", e),
                            }
                        }
                    }
                } else if p.topic == "symbion/agents/wake@v1" {
                    // Wake-on-LAN via MQTT
                    if let Ok(txt) = String::from_utf8(p.payload.to_vec()) {
                        println!("[kernel] received wake request MQTT: {}", txt);
                        match serde_json::from_str::<WakeRequest>(&txt) {
                            Ok(req) => {
                                let (status, msg) = trigger_wol_udp(&cfg, &req.agent_id).await;
                                if status.is_success() {
                                    println!("[kernel] WoL sent successfully for agent: {}", req.agent_id);
                                } else {
                                    eprintln!("[kernel] WoL failed for agent {}: {}", req.agent_id, msg);
                                }
                            }
                            Err(e) => eprintln!("[kernel] wake request JSON invalide: {txt}, error: {}", e),
                        }
                    }
                } else if p.topic == "symbion/notifications/acknowledge@v1" {
                    // Acquittement notification PWA (toast)
                    if let Some(ref notif_mgr) = notifications_manager {
                        if let Ok(txt) = String::from_utf8(p.payload.to_vec()) {
                            println!("[kernel] received notification ack MQTT: {}", txt);
                            match serde_json::from_str::<NotificationAckRequest>(&txt) {
                                Ok(req) => {
                                    match notif_mgr.acknowledge(&req.notification_id) {
                                        Ok(()) => println!("[kernel] notification {} acknowledged via MQTT", req.notification_id),
                                        Err(e) => eprintln!("[kernel] failed to acknowledge notification {}: {}", req.notification_id, e),
                                    }
                                }
                                Err(e) => eprintln!("[kernel] notification ack JSON invalide: {txt}, error: {}", e),
                            }
                        }
                    }
                }
                }
                Ok(_) => {}
                Err(e) => {
                    eprintln!("[kernel] MQTT erreur: {:?}", e);
                    tokio::time::sleep(std::time::Duration::from_secs(2)).await;
                }
            }
        }
    });
}
