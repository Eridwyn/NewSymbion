use crate::models::{HeartbeatIn, HostState, HostsMap};
use crate::state::Shared;
use crate::config::HostsConfig;
use rumqttc::{AsyncClient, Event, MqttOptions, QoS};
use time::OffsetDateTime;
use tokio::task;

pub fn spawn_mqtt_listener(states: Shared<HostsMap>, config: Shared<HostsConfig>) {
    task::spawn(async move {
        let cfg = config.lock().clone();
        let mqtt_cfg = cfg.mqtt.unwrap_or_else(|| crate::config::MqttConf { 
            host: "localhost".into(), 
            port: 1883 
        });
        
        let mut opts = MqttOptions::new("symbion-kernel", &mqtt_cfg.host, mqtt_cfg.port);
        opts.set_keep_alive(std::time::Duration::from_secs(15));
        let (client, mut eventloop) = AsyncClient::new(opts, 10);
        if let Err(e) = client.subscribe("symbion/hosts/heartbeat@v2", QoS::AtLeastOnce).await {
            eprintln!("[kernel] subscribe MQTT failed: {e:?}");
            return;
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
                Ok(_) => {}
                Err(e) => {
                    eprintln!("[kernel] MQTT erreur: {:?}", e);
                    tokio::time::sleep(std::time::Duration::from_secs(2)).await;
                }
            }
        }
    });
}
