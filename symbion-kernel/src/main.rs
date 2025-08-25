use axum::{extract::State, routing::get, Json, Router};
use parking_lot::Mutex;
use rumqttc::{AsyncClient, Event, MqttOptions, QoS};
use serde::{Deserialize, Serialize};
use std::{collections::HashMap, net::SocketAddr, sync::Arc};
use time::OffsetDateTime;
use tokio::{net::TcpListener, task};

// --------- Types d'état partagés ---------
#[derive(Debug, Serialize, Deserialize, Clone)]
struct HostState {
    host_id: String,
    last_seen: String,
    cpu: Option<f32>,
    ram: Option<f32>,
    ip: Option<String>,
}

#[derive(Debug, Deserialize)]
struct HeartbeatIn {
    host_id: String,
    ts: String,
    metrics: Metrics,
    net: NetInfo,
}
#[derive(Debug, Deserialize)]
struct Metrics { cpu: f32, ram: f32 }
#[derive(Debug, Deserialize)]
struct NetInfo { ip: String }

type HostsMap = HashMap<String, HostState>;
type Shared<T> = Arc<Mutex<T>>;

#[tokio::main]
async fn main() {
    // 1) État partagé
    let states: Shared<HostsMap> = Arc::new(Mutex::new(HashMap::new()));

    // 2) MQTT (DANS main, pas dehors)
    let mut opts = MqttOptions::new("symbion-kernel", "localhost", 1883);
    opts.set_keep_alive(std::time::Duration::from_secs(15));
    let (client, mut eventloop) = AsyncClient::new(opts, 10);
    client
        .subscribe("symbion/hosts/heartbeat@v2", QoS::AtLeastOnce)
        .await
        .unwrap();

    // 3) Task qui consomme le bus et remplit la map
    let states_for_mqtt = states.clone();
    task::spawn(async move {
        loop {
            match eventloop.poll().await {
                Ok(Event::Incoming(rumqttc::Incoming::Publish(p))) => {
                    if p.topic == "symbion/hosts/heartbeat@v2" {
                        if let Ok(txt) = String::from_utf8(p.payload.to_vec()) {
                            if let Ok(hb) = serde_json::from_str::<HeartbeatIn>(&txt) {
                                let now = OffsetDateTime::now_utc()
                                    .format(&time::format_description::well_known::Rfc3339)
                                    .unwrap();
                                let st = HostState {
                                    host_id: hb.host_id,
                                    last_seen: now,
                                    cpu: Some(hb.metrics.cpu),
                                    ram: Some(hb.metrics.ram),
                                    ip: Some(hb.net.ip),
                                };
                                states_for_mqtt.lock().insert(st.host_id.clone(), st);
                            } else {
                                eprintln!("[kernel] heartbeat JSON invalide: {txt}");
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

    // 4) API HTTP
    let app = Router::new()
        .route("/health", get(|| async { "ok" }))
        .route("/hosts", get(get_hosts))
        .with_state(states);

    // 5) Serve
    let addr = SocketAddr::from(([0, 0, 0, 0], 8080));
    println!("[kernel] listening on http://{addr}");
    let listener = TcpListener::bind(addr).await.unwrap();
    axum::serve(listener, app).await.unwrap();
}

// Handlers
async fn get_hosts(State(states): State<Shared<HostsMap>>) -> Json<Vec<HostState>> {
    Json(states.lock().values().cloned().collect())
}
