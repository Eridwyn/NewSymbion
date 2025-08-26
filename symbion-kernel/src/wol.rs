use crate::config::HostsConfig;
use axum::http::StatusCode;
use std::net::{Ipv4Addr, SocketAddrV4, UdpSocket};

fn parse_mac(mac: &str) -> Result<[u8; 6], &'static str> {
    let hex: String = mac.chars().filter(|c| c.is_ascii_hexdigit()).collect();
    if hex.len() != 12 { return Err("bad mac len"); }
    let mut out = [0u8; 6];
    for i in 0..6 {
        let byte = u8::from_str_radix(&hex[i*2..i*2+2], 16).map_err(|_| "bad mac hex")?;
        out[i] = byte;
    }
    Ok(out)
}

fn magic_packet(mac: [u8; 6]) -> [u8; 102] {
    let mut pkt = [0u8; 102];
    // 6 x 0xFF
    for i in 0..6 { pkt[i] = 0xFF; }
    // 16 fois l'adresse MAC
    for i in 0..16 {
        let base = 6 + i*6;
        pkt[base..base+6].copy_from_slice(&mac);
    }
    pkt
}

fn parse_broadcast(hint: Option<&str>) -> Ipv4Addr {
    if let Some(s) = hint {
        if let Ok(ip) = s.parse::<Ipv4Addr>() {
            return ip;
        }
    }
    Ipv4Addr::new(255, 255, 255, 255)
}

/// Envoie le magic packet en UDP broadcast (ports 9 et 7).
pub async fn trigger_wol_udp(cfg: &HostsConfig, host_id: &str) -> (StatusCode, &'static str) {
    let Some(host) = cfg.hosts.get(host_id) else {
        return (StatusCode::NOT_FOUND, "unknown host");
    };

    let mac = match parse_mac(&host.mac) {
        Ok(m) => m,
        Err(_) => return (StatusCode::BAD_REQUEST, "invalid mac"),
    };
    let pkt = magic_packet(mac);
    let bcast = parse_broadcast(host.hint.as_deref());

    // socket UDP avec broadcast
    let sock = match UdpSocket::bind(("0.0.0.0", 0)) {
        Ok(s) => s,
        Err(_) => return (StatusCode::BAD_GATEWAY, "bind error"),
    };
    if let Err(_) = sock.set_broadcast(true) {
        return (StatusCode::BAD_GATEWAY, "broadcast off");
    }

    // on tente port 9 puis 7
    let mut ok = false;
    for port in [9u16, 7u16] {
        let addr = SocketAddrV4::new(bcast, port);
        if let Err(e) = sock.send_to(&pkt, addr) {
            eprintln!("[kernel] WOL send error to {}:{} -> {}", bcast, port, e);
        } else {
            ok = true;
        }
    }
    if ok { (StatusCode::OK, "ok") } else { (StatusCode::BAD_GATEWAY, "wol failed") }
}
