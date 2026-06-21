use anyhow::{bail, Result};
use serde::Serialize;
use std::collections::HashMap;
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::net::TcpStream;

use crate::config::NutConfig;

#[derive(Debug, Clone, Serialize)]
pub struct UpsStatus {
    pub status: String,
    pub battery_charge: f64,
    pub battery_runtime_seconds: u64,
    pub load_percent: f64,
    pub model: String,
    pub manufacturer: String,
    pub battery_charge_low: f64,
    pub output_voltage: f64,
}

impl UpsStatus {
    pub fn on_battery(&self) -> bool {
        self.status.contains("OB")
    }

    pub fn battery_low(&self) -> bool {
        self.battery_charge <= self.battery_charge_low
    }
}

pub struct NutClient;

impl NutClient {
    pub async fn query(config: &NutConfig) -> Result<UpsStatus> {
        let addr = format!("{}:{}", config.host, config.port);
        let stream = TcpStream::connect(&addr).await?;
        let (reader, mut writer) = stream.into_split();
        let mut reader = BufReader::new(reader);

        // Authenticate if credentials provided
        if let (Some(user), Some(pass)) = (&config.username, &config.password) {
            send_cmd(&mut writer, &mut reader, &format!("USERNAME {}\n", user)).await?;
            send_cmd(&mut writer, &mut reader, &format!("PASSWORD {}\n", pass)).await?;
        }

        // LIST VAR ups
        writer.write_all(format!("LIST VAR {}\n", config.ups_name).as_bytes()).await?;

        let mut vars: HashMap<String, String> = HashMap::new();
        let mut line = String::new();

        // Read until END LIST VAR
        loop {
            line.clear();
            reader.read_line(&mut line).await?;
            let l = line.trim();
            if l.starts_with("BEGIN LIST VAR") {
                continue;
            }
            if l.starts_with("END LIST VAR") {
                break;
            }
            if l.starts_with("ERR") {
                bail!("NUT error: {}", l);
            }
            // VAR ups battery.charge "100"
            if let Some(rest) = l.strip_prefix(&format!("VAR {} ", config.ups_name)) {
                if let Some(idx) = rest.find(' ') {
                    let key = &rest[..idx];
                    let val = rest[idx + 1..].trim_matches('"');
                    vars.insert(key.to_string(), val.to_string());
                }
            }
        }

        writer.write_all(b"LOGOUT\n").await.ok();

        let get = |k: &str| -> String { vars.get(k).cloned().unwrap_or_default() };
        let get_f64 = |k: &str| -> f64 { get(k).parse().unwrap_or(0.0) };
        let get_u64 = |k: &str| -> u64 { get(k).parse().unwrap_or(0) };

        Ok(UpsStatus {
            status: get("ups.status"),
            battery_charge: get_f64("battery.charge"),
            battery_runtime_seconds: get_u64("battery.runtime"),
            load_percent: get_f64("ups.load"),
            model: get("ups.model").trim().to_string(),
            manufacturer: get("ups.mfr").trim().to_string(),
            battery_charge_low: get_f64("battery.charge.low"),
            output_voltage: get_f64("output.voltage"),
        })
    }
}

async fn send_cmd(
    writer: &mut tokio::net::tcp::OwnedWriteHalf,
    reader: &mut BufReader<tokio::net::tcp::OwnedReadHalf>,
    cmd: &str,
) -> Result<()> {
    writer.write_all(cmd.as_bytes()).await?;
    let mut line = String::new();
    reader.read_line(&mut line).await?;
    if line.trim().starts_with("ERR") {
        bail!("NUT auth error: {}", line.trim());
    }
    Ok(())
}
