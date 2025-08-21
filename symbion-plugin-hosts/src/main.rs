use rumqttc::{AsyncClient, Event, Incoming, MqttOptions, QoS};
use serde::{Deserialize, Serialize};
use std::net::UdpSocket;
use std::time::SystemTime;
use sysinfo::{System};
use local_ip_address::local_ip;
use gethostname::gethostname;
use tokio::time::{sleep, Duration};

#[derive(Serialize, Deserialize, Debug)]
struct HeartbeatV2 {
    host_id: String,
    ts: String,
    metrics: Metrics,
    net: NetInfo,
}
#[derive(Serialize, Deserialize, Debug)]
struct Metrics { cpu: f32, ram: f32 }
#[derive(Serialize, Deserialize, Debug)]
struct NetInfo { ip: String }

#[derive(Deserialize, Debug)]
struct WakeV1 {
    host_id: String,
    mac: String,
    broadcast: String,
}

fn magic_packet(mac: &str) -> anyhow::Result<Vec<u8>> {
    // MAC au format "AA:BB:CC:DD:EE:FF"
    let bytes: Result<Vec<u8>, _> = mac.split(':').map(|x| u8::from_str_radix(x, 16)).collect();
    let bytes = bytes?;
    if bytes.len() != 6 {
        anyhow::bail!("MAC invalide");
    }
    let mut pkt = vec![0xFF; 6];
    for _ in 0..16 {
        pkt.extend_from_slice(&bytes);
    }
    Ok(pkt)
}

fn send_wol(mac: &str, broadcast: &str) -> anyhow::Result<()> {
    use std::net::IpAddr;
    let pkt = magic_packet(mac)?;

    // Détecte si "broadcast" est une IP unicast (type 192.168.1.42)
    let maybe_ip = broadcast.parse::<IpAddr>().ok();
    let looks_unicast = maybe_ip.map(|ip| !ip.is_multicast() && ip != IpAddr::from([255,255,255,255])).unwrap_or(false);

    let targets = {
        let mut t = vec![
            format!("{}:9", broadcast),
            format!("{}:7", broadcast),
            "255.255.255.255:9".to_string(),
            "255.255.255.255:7".to_string(),
        ];
        if looks_unicast {
            t.push(format!("{}:9", broadcast)); // unicast port 9
        }
        t
    };

    let sock = UdpSocket::bind("0.0.0.0:0")?;
    sock.set_broadcast(true)?;

    // Envoie plusieurs fois (certains NIC sont sourds)
    for _ in 0..3 {
        for t in &targets {
            let _ = sock.send_to(&pkt, t);
        }
        std::thread::sleep(std::time::Duration::from_millis(50));
    }
    Ok(())
}


#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // Identité machine
    let host_id = gethostname().to_string_lossy().to_string();
    let ip = local_ip().map(|i| i.to_string()).unwrap_or_else(|_| "0.0.0.0".into());

    // MQTT async
    let mut opts = MqttOptions::new("symbion-plugin-hosts", "localhost", 1883);
    opts.set_keep_alive(Duration::from_secs(30));
    let (client, mut eventloop) = AsyncClient::new(opts, 10);

    // S'abonner aux ordres de réveil
    client.subscribe("symbion/hosts/wake@v1", QoS::AtLeastOnce).await?;

    // Boucle d'événements MQTT (réception)
    let mut client_for_loop = client.clone();
    tokio::spawn(async move {
        loop {
            match eventloop.poll().await {
                Ok(Event::Incoming(Incoming::Publish(p))) => {
                    if p.topic == "symbion/hosts/wake@v1" {
                        if let Ok(cmd) = serde_json::from_slice::<WakeV1>(&p.payload) {
                            if let Err(e) = send_wol(&cmd.mac, &cmd.broadcast) {
                                eprintln!("[hosts] WOL échec: {:?}", e);
                            }
                        }
                    }
                }
                Ok(_) => {}
                Err(e) => {
                    eprintln!("[hosts] MQTT loop erreur: {:?}", e);
                    break;
                }
            }
        }
    });

    // Boucle heartbeat (envoi)
    let mut sys = System::new(); // API récente: pas besoin des traits *Ext
    loop {
        // Rafraîchis CPU et mémoire
        sys.refresh_cpu_usage();
        sys.refresh_memory();

        // CPU: pourcentage global (0..100). On renvoie 0..1.
        let cpu_ratio = sys.global_cpu_info().cpu_usage() / 100.0;

        // RAM: utilisé / total (kio). On renvoie un ratio 0..1.
        let total = sys.total_memory() as f32;
        let used = sys.used_memory() as f32;
        let ram_ratio = if total > 0.0 { used / total } else { 0.0 };

        let hb = HeartbeatV2 {
            host_id: host_id.clone(),
            ts: humantime::format_rfc3339(SystemTime::now()).to_string(),
            metrics: Metrics { cpu: cpu_ratio, ram: ram_ratio },
            net: NetInfo { ip: ip.clone() },
        };

        let payload = serde_json::to_vec(&hb)?;
        if let Err(e) = client_for_loop
            .publish("symbion/hosts/heartbeat@v2", QoS::AtLeastOnce, false, payload)
            .await
        {
            eprintln!("[hosts] publish heartbeat erreur: {:?}", e);
        }

        sleep(Duration::from_secs(10)).await;
    }
}
