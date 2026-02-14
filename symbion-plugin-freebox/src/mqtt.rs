//! MQTT publisher for Freebox data
//!
//! Publishes device presence, connection status, and downloads to Symbion MQTT.

use anyhow::Result;
use rumqttc::{AsyncClient, Event, MqttOptions, Packet, QoS};
use serde::Serialize;
use std::collections::HashMap;
use std::time::Duration;
use tokio::sync::mpsc;
use tracing::{debug, error, info};

use crate::config::MqttConfig;
use crate::freebox::{ConnectionStatus, DownloadsSummary, LanDevice};

/// MQTT publisher handle
pub struct MqttPublisher {
    client: AsyncClient,
    topic_prefix: String,
}

/// Presence status for a tracked device
#[derive(Debug, Clone, Serialize)]
pub struct PresenceStatus {
    pub device_id: String,
    pub friendly_name: String,
    pub present: bool,
    pub last_seen: i64,
    pub ip_address: Option<String>,
    pub device_type: String,
}

/// Network summary published periodically
#[derive(Debug, Clone, Serialize)]
pub struct NetworkSummary {
    pub total_devices: usize,
    pub active_devices: usize,
    pub tracked_present: usize,
    pub tracked_absent: usize,
}

impl MqttPublisher {
    /// Connect to MQTT broker
    pub async fn connect(config: &MqttConfig) -> Result<Self> {
        let mut options = MqttOptions::new(
            &config.client_id,
            &config.host,
            config.port,
        );
        options.set_keep_alive(Duration::from_secs(30));
        options.set_clean_session(true);
        // Increase max packet size for device list (can be >30KB)
        options.set_max_packet_size(1024 * 1024, 1024 * 1024);

        let (client, mut eventloop) = AsyncClient::new(options, 200);

        // Spawn event loop handler
        let (tx, mut rx) = mpsc::channel::<()>(1);
        tokio::spawn(async move {
            loop {
                tokio::select! {
                    event = eventloop.poll() => {
                        match event {
                            Ok(Event::Incoming(Packet::ConnAck(_))) => {
                                info!("MQTT connected to broker");
                            }
                            Ok(Event::Incoming(Packet::PingResp)) => {
                                debug!("MQTT ping response");
                            }
                            Ok(_) => {}
                            Err(e) => {
                                error!("MQTT error: {}", e);
                                tokio::time::sleep(Duration::from_secs(5)).await;
                            }
                        }
                    }
                    _ = rx.recv() => {
                        info!("MQTT event loop shutting down");
                        break;
                    }
                }
            }
        });

        // Store shutdown handle (we don't use it but keep connection alive)
        std::mem::forget(tx);

        Ok(Self {
            client,
            topic_prefix: config.topic_prefix.clone(),
        })
    }

    /// Publish a JSON message to a topic
    async fn publish<T: Serialize + ?Sized>(&self, topic: &str, payload: &T) -> Result<()> {
        let full_topic = format!("{}/{}", self.topic_prefix, topic);
        let json = serde_json::to_string(payload)?;

        self.client
            .publish(&full_topic, QoS::AtLeastOnce, false, json.as_bytes())
            .await?;

        debug!("Published to {}", full_topic);
        Ok(())
    }

    // ========================================================================
    // Presence Publishing
    // ========================================================================

    /// Publish presence status for a tracked device
    pub async fn publish_presence(&self, status: &PresenceStatus) -> Result<()> {
        // Publish to device-specific topic
        let topic = format!("presence/{}", status.device_id);
        self.publish(&topic, status).await?;

        // Also publish simple boolean for automations
        let bool_topic = format!("presence/{}/state", status.device_id);
        let state = if status.present { "home" } else { "away" };
        self.client
            .publish(
                &format!("{}/{}", self.topic_prefix, bool_topic),
                QoS::AtLeastOnce,
                true, // retained
                state.as_bytes(),
            )
            .await?;

        Ok(())
    }

    /// Publish presence for multiple devices
    pub async fn publish_presence_batch(
        &self,
        devices: &HashMap<String, LanDevice>,
        tracked_configs: &HashMap<String, crate::config::DeviceConfig>,
    ) -> Result<()> {
        let mut present_count = 0;
        let mut absent_count = 0;

        for (name, config) in tracked_configs {
            // Lookup by freebox_name, not the config key
            let (present, device) = if let Some(dev) = devices.get(&config.freebox_name) {
                (dev.active && dev.reachable, Some(dev))
            } else {
                (false, None)
            };

            if present {
                present_count += 1;
            } else {
                absent_count += 1;
            }

            let status = PresenceStatus {
                device_id: name.clone(),
                friendly_name: config.friendly_name.clone().unwrap_or_else(|| name.clone()),
                present,
                last_seen: device.map(|d| d.last_time_reachable).unwrap_or(0),
                ip_address: device.and_then(|d| {
                    d.l3connectivities
                        .iter()
                        .find(|c| c.af == "ipv4" && c.active)
                        .map(|c| c.addr.clone())
                }),
                device_type: config.device_type.clone(),
            };

            self.publish_presence(&status).await?;
        }

        // Publish summary
        let summary = serde_json::json!({
            "present": present_count,
            "absent": absent_count,
            "anyone_home": present_count > 0,
        });
        self.publish("presence/summary", &summary).await?;

        Ok(())
    }

    // ========================================================================
    // Devices Publishing
    // ========================================================================

    /// Publish full device list
    pub async fn publish_devices(&self, devices: &[LanDevice]) -> Result<()> {
        let summary = NetworkSummary {
            total_devices: devices.len(),
            active_devices: devices.iter().filter(|d| d.active).count(),
            tracked_present: 0, // Will be updated by presence check
            tracked_absent: 0,
        };

        self.publish("devices/summary", &summary).await?;
        self.publish("devices/list", devices).await?;

        Ok(())
    }

    // ========================================================================
    // Connection Status Publishing
    // ========================================================================

    /// Publish internet connection status
    pub async fn publish_connection(&self, status: &ConnectionStatus) -> Result<()> {
        self.publish("connection/status", status).await?;

        // Publish simplified metrics for dashboards
        let metrics = serde_json::json!({
            "state": status.state,
            "type": status.connection_type,
            "download_mbps": status.bandwidth_down as f64 / 1_000_000.0,
            "upload_mbps": status.bandwidth_up as f64 / 1_000_000.0,
            "current_download_kbps": status.rate_down as f64 / 1_000.0,
            "current_upload_kbps": status.rate_up as f64 / 1_000.0,
            "ipv4": status.ipv4,
            "ipv6": status.ipv6,
        });
        self.publish("connection/metrics", &metrics).await?;

        Ok(())
    }

    // ========================================================================
    // Downloads Publishing
    // ========================================================================

    /// Publish downloads status
    pub async fn publish_downloads(&self, summary: &DownloadsSummary) -> Result<()> {
        // Publish summary
        let metrics = serde_json::json!({
            "total": summary.total,
            "active": summary.active,
            "download_rate_kbps": summary.rx_rate as f64 / 1_000.0,
            "upload_rate_kbps": summary.tx_rate as f64 / 1_000.0,
        });
        self.publish("downloads/summary", &metrics).await?;

        // Publish active downloads list
        let active: Vec<_> = summary.downloads
            .iter()
            .filter(|d| d.status == "downloading" || d.status == "seeding")
            .collect();

        if !active.is_empty() {
            self.publish("downloads/active", &active).await?;
        }

        Ok(())
    }

    // ========================================================================
    // Health Publishing
    // ========================================================================

    /// Publish plugin health status
    pub async fn publish_health(&self, healthy: bool, message: &str) -> Result<()> {
        let status = serde_json::json!({
            "healthy": healthy,
            "message": message,
            "timestamp": chrono::Utc::now().to_rfc3339(),
        });
        self.publish("health", &status).await?;
        Ok(())
    }

    /// Publish plugin manifest for discovery
    pub async fn publish_manifest(&self, manifest: &str) -> Result<()> {
        // Publish to standard plugin manifest topic (retained)
        self.client
            .publish(
                "symbion/plugins/freebox/manifest",
                QoS::AtLeastOnce,
                true, // retained
                manifest.as_bytes(),
            )
            .await?;
        Ok(())
    }
}
