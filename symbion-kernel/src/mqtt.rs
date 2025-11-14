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
use rumqttc::{AsyncClient, Event, MqttOptions, QoS};
use time::OffsetDateTime;
use tokio::task;

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

pub fn spawn_mqtt_listener(states: Shared<HostsMap>, config: Shared<HostsConfig>, notes_bridge: Option<SharedNotesBridge>, agents: Option<SharedAgentRegistry>, health_tracker: Option<crate::health::HealthTracker>, dashboard_events: Option<crate::dashboard_events::DashboardEventPublisher>) {
    task::spawn(async move {
        let cfg = config.lock().clone();
        let mqtt_cfg = cfg.mqtt.unwrap_or_else(|| crate::config::MqttConf { 
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
                                Err(_) => eprintln!("[kernel] notes response JSON invalide: {txt}"),
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
