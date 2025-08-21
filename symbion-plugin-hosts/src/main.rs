use rumqttc::{AsyncClient, Event, Incoming, MqttOptions, QoS};
use serde::{Deserialize, Serialize};
use std::time::SystemTime;
use sysinfo::System;
use local_ip_address::local_ip;
use gethostname::gethostname;
use tokio::time::{sleep, Duration};

use anyhow::{Context, Result};

// ===================== Config env =====================
// SYMBION_WOL_BACKENDS="raw,udp,cmd" (ordre d'essai)
// SYMBION_WOL_IFACE="wlo1"          (pour raw)
// SYMBION_WOL_BIND_IP="192.168.1.10" (pour UDP bind)
// SYMBION_WOL_CMD="/usr/sbin/etherwake -i wlo1 {mac}" (pour cmd)
// SYMBION_WOL_UDP_TARGET="192.168.1.255" (broadcast ou unicast IP)
// SYMBION_WOL_UDP_PORTS="9,7"       (ports UDP à arroser)
const DEFAULT_BACKENDS: &str = "raw,udp,cmd";
const DEFAULT_UDP_PORTS: &str = "9,7";

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
    broadcast: String, // pour compat, peut servir comme cible UDP
}

fn parse_mac(s: &str) -> Result<[u8;6]> {
    let mut out = [0u8;6];
    let parts: Vec<&str> = s.split(':').collect();
    if parts.len() != 6 { anyhow::bail!("MAC invalide: {s}"); }
    for (i,p) in parts.iter().enumerate() {
        out[i] = u8::from_str_radix(p, 16)
            .with_context(|| format!("Octet MAC invalide '{p}' dans {s}"))?;
    }
    Ok(out)
}

fn build_magic(mac: [u8;6]) -> Vec<u8> {
    let mut pkt = Vec::with_capacity(6 + 16 * 6);
    pkt.extend_from_slice(&[0xFF; 6]);
    for _ in 0..16 { pkt.extend_from_slice(&mac); }
    pkt
}

// ===================== Backend: UDP (portatif) =====================
#[cfg(feature = "udp")]
fn wol_udp(mac: [u8;6], target: Option<&str>, bind_ip: Option<&str>, ports: &[u16]) -> Result<()> {
    use std::net::UdpSocket;

    let pkt = build_magic(mac);
    // Bind sur IP locale si fournie, sinon 0.0.0.0
    let bind = bind_ip.unwrap_or("0.0.0.0");
    let sock = UdpSocket::bind(format!("{bind}:0"))?;
    sock.set_broadcast(true)?;

    // Cibles: 1) target explicite (env/commande) 2) 255.255.255.255 3) si broadcast param MQTT non vide
    let mut targets = Vec::<String>::new();
    if let Some(t) = target { targets.push(t.to_string()); }
    targets.push("255.255.255.255".into());

    for _ in 0..3 {
        for t in &targets {
            for &p in ports {
                let _ = sock.send_to(&pkt, format!("{t}:{p}"));
            }
        }
        std::thread::sleep(std::time::Duration::from_millis(50));
    }
    Ok(())
}

// ===================== Backend: RAW L2 (optionnel) =====================
#[cfg(feature = "rawwol")]
mod rawwol {
    use super::{build_magic, Result};
    use anyhow::anyhow;
    use std::{fs, mem};
    use nix::sys::socket::{socket, sendto, AddressFamily, SockFlag, SockType, MsgFlags, SockaddrStorage};
    use nix::libc;

    const ETH_P_WAKE_ON_LAN: u16 = 0x0842;

    fn read_iface_mac(iface: &str) -> std::io::Result<[u8;6]> {
        let s = fs::read_to_string(format!("/sys/class/net/{}/address", iface))?.trim().to_string();
        let mut mac = [0u8;6];
        for (i, oct) in s.split(':').enumerate() {
            mac[i] = u8::from_str_radix(oct, 16).map_err(|_| std::io::Error::new(std::io::ErrorKind::Other, "MAC iface invalide"))?;
        }
        Ok(mac)
    }

    pub fn wol_raw(iface: &str, mac: [u8;6]) -> Result<()> {
        let magic = build_magic(mac);
        let src_mac = read_iface_mac(iface).map_err(|e| anyhow!("MAC de l'interface {iface} introuvable: {e}"))?;

        // trame Ethernet
        let mut frame = Vec::with_capacity(14 + magic.len());
        frame.extend_from_slice(&[0xFF;6]);                         // dest broadcast
        frame.extend_from_slice(&src_mac);                          // src
        frame.extend_from_slice(&ETH_P_WAKE_ON_LAN.to_be_bytes());  // EtherType
        frame.extend_from_slice(&magic);                            // payload

        let fd = socket(AddressFamily::Packet, SockType::Raw, SockFlag::empty(), None)
            .map_err(|e| anyhow!("socket(AF_PACKET,RAW) a échoué: {e}"))?;

        let ifindex = nix::net::if_::if_nametoindex(iface)
            .map_err(|_| anyhow!("Interface introuvable: {iface}"))? as i32;

        let mut addr: libc::sockaddr_ll = unsafe { mem::zeroed() };
        addr.sll_family   = libc::AF_PACKET as u16;
        addr.sll_protocol = (ETH_P_WAKE_ON_LAN).to_be();
        addr.sll_ifindex  = ifindex;
        addr.sll_halen    = 6;
        addr.sll_addr[..6].copy_from_slice(&[0xFF;6]);

        let raw_ptr = &addr as *const libc::sockaddr_ll as *const libc::sockaddr;
        let storage = unsafe {
            SockaddrStorage::from_raw(raw_ptr, Some(mem::size_of::<libc::sockaddr_ll>() as u32))
        }.ok_or_else(|| anyhow!("SockaddrStorage::from_raw a échoué"))?;

        for _ in 0..3 {
            let _ = sendto(&fd, &frame, &storage, MsgFlags::empty());
            std::thread::sleep(std::time::Duration::from_millis(50));
        }
        Ok(())
    }
}

// ===================== Backend: CMD (optionnel dernier recours) =====================
fn wol_cmd(mac: &str, template: &str) -> Result<()> {
    use std::process::Command;
    let cmdline = template.replace("{mac}", mac);
    let parts: Vec<&str> = cmdline.split_whitespace().collect();
    let (bin, args) = parts.split_first().ok_or_else(|| anyhow::anyhow!("SYMBION_WOL_CMD vide"))?;
    let status = Command::new(bin).args(args).status()?;
    if !status.success() { anyhow::bail!("Commande WOL a échoué: {cmdline}"); }
    Ok(())
}

// ===================== Orchestrateur de backends =====================
fn wol_send(mac_str: &str, broadcast_hint: &str) -> Result<()> {
    let backends = std::env::var("SYMBION_WOL_BACKENDS").unwrap_or_else(|_| DEFAULT_BACKENDS.to_string());
    let mut last_err: Option<anyhow::Error> = None;

    for b in backends.split(',').map(|s| s.trim()) {
        match b {
            "raw" => {
                #[cfg(feature = "rawwol")]
                {
                    let iface = std::env::var("SYMBION_WOL_IFACE").unwrap_or_else(|_| "wlo1".into());
                    match parse_mac(mac_str).and_then(|m| rawwol::wol_raw(&iface, m)) {
                        Ok(()) => { eprintln!("[wol] OK via RAW ({iface})"); return Ok(()); }
                        Err(e) => { eprintln!("[wol] RAW échec: {e:?}"); last_err = Some(e); }
                    }
                }
                #[cfg(not(feature = "rawwol"))]
                { eprintln!("[wol] RAW non compilé (feature rawwol absente)"); }
            }
            "udp" => {
                #[cfg(feature = "udp")]
                {
                    let bind_ip = std::env::var("SYMBION_WOL_BIND_IP").ok();
                    let target  = std::env::var("SYMBION_WOL_UDP_TARGET").ok()
                        .or_else(|| if !broadcast_hint.is_empty() { Some(broadcast_hint.to_string()) } else { None });
                    let ports: Vec<u16> = std::env::var("SYMBION_WOL_UDP_PORTS").unwrap_or_else(|_| DEFAULT_UDP_PORTS.into())
                        .split(',').filter_map(|p| p.trim().parse().ok()).collect();
                    match parse_mac(mac_str).and_then(|m| wol_udp(m, target.as_deref(), bind_ip.as_deref(), &ports)) {
                        Ok(()) => { eprintln!("[wol] OK via UDP"); return Ok(()); }
                        Err(e) => { eprintln!("[wol] UDP échec: {e:?}"); last_err = Some(e); }
                    }
                }
            }
            "cmd" => {
                let tpl = std::env::var("SYMBION_WOL_CMD")
                    .unwrap_or_else(|_| "/usr/sbin/etherwake -i wlo1 {mac}".into());
                match wol_cmd(mac_str, &tpl) {
                    Ok(()) => { eprintln!("[wol] OK via CMD ({tpl})"); return Ok(()); }
                    Err(e) => { eprintln!("[wol] CMD échec: {e:?}"); last_err = Some(e); }
                }
            }
            other => eprintln!("[wol] backend inconnu: {other}"),
        }
    }

    Err(last_err.unwrap_or_else(|| anyhow::anyhow!("Aucun backend WOL n'a réussi")))
}

// ===================== MAIN: MQTT + heartbeat =====================
#[tokio::main]
async fn main() -> Result<()> {
    // Identité
    let host_id = gethostname().to_string_lossy().to_string();
    let ip = local_ip().map(|i| i.to_string()).unwrap_or_else(|_| "0.0.0.0".into());
    eprintln!("[hosts] démarrage; IP locale {ip}");

    // MQTT
    let mut opts = MqttOptions::new("symbion-plugin-hosts", "localhost", 1883);
    opts.set_keep_alive(Duration::from_secs(30));
    let (client, mut eventloop) = AsyncClient::new(opts, 10);

    client.subscribe("symbion/hosts/wake@v1", QoS::AtLeastOnce).await?;

    // Réception MQTT
    let client_pub = client.clone();
    tokio::spawn(async move {
        loop {
            match eventloop.poll().await {
                Ok(Event::Incoming(Incoming::Publish(p))) => {
                    if p.topic == "symbion/hosts/wake@v1" {
                        match serde_json::from_slice::<WakeV1>(&p.payload) {
                            Ok(cmd) => {
                                eprintln!("[hosts] ordre WOL → {} (hint: {})", cmd.mac, cmd.broadcast);
                                if let Err(e) = wol_send(&cmd.mac, &cmd.broadcast) {
                                    eprintln!("[hosts] WOL KO: {e:?}");
                                }
                            }
                            Err(e) => eprintln!("[hosts] payload wake invalide: {e:?}"),
                        }
                    }
                }
                Ok(_) => {}
                Err(e) => {
                    eprintln!("[hosts] MQTT erreur: {:?}", e);
                    break;
                }
            }
        }
    });

    // Heartbeat
    let mut sys = System::new();
    loop {
        sys.refresh_cpu_usage();
        sys.refresh_memory();

        let cpu_ratio = sys.global_cpu_info().cpu_usage() / 100.0;
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
        if let Err(e) = client_pub
            .publish("symbion/hosts/heartbeat@v2", QoS::AtLeastOnce, false, payload)
            .await
        {
            eprintln!("[hosts] publish heartbeat erreur: {:?}", e);
        }

        sleep(Duration::from_secs(10)).await;
    }
}
