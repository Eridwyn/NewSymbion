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

pub fn spawn_mqtt_listener(states: Shared<HostsMap>, config: Shared<HostsConfig>, notes_bridge: Option<SharedNotesBridge>) {
    task::spawn(async move {
        let cfg = config.lock().clone();
        let mqtt_cfg = cfg.mqtt.unwrap_or_else(|| crate::config::MqttConf { 
            host: "localhost".into(), 
            port: 1883 
        });
        
        let mut opts = MqttOptions::new("symbion-kernel-listener", &mqtt_cfg.host, mqtt_cfg.port);
        opts.set_keep_alive(std::time::Duration::from_secs(15));
        let (client, mut eventloop) = AsyncClient::new(opts, 10);
        
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

        loop {
            match eventloop.poll().await {
                Ok(Event::Incoming(rumqttc::Incoming::Publish(p))) if p.topic == "symbion/hosts/heartbeat@v2" => {
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
                }
                Ok(Event::Incoming(rumqttc::Incoming::Publish(p))) if p.topic == "symbion/notes/response@v1" => {
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
