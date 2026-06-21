//! MQTT publisher for Synology/UPS plugin
//!
//! Contrat Symbion (aligné sur le plugin ssl) :
//! - Features → `symbion/features/update` avec l'enveloppe `FeatureUpdate`
//!   (c'est le SEUL topic que le kernel ingère, cf. symbion-kernel/src/mqtt.rs:568).
//! - État complet UPS → `symbion/synology/ups` (retained, pour la PWA).
//! - Manifest plugin → `symbion/plugins/synology/manifest` (retained).
//! - Health plugin → `symbion/plugins/synology/status` (retained).

use anyhow::{Context, Result};
use rumqttc::{AsyncClient, Event, MqttOptions, Packet, QoS};
use serde::Serialize;
use serde_json::json;
use tokio::sync::mpsc;
use tokio::time::Duration;
use tracing::{info, warn};

use crate::config::MqttConfig;
use crate::nut::UpsStatus;

const PLUGIN_TOPIC: &str = "symbion/plugins/synology";
const UPS_TOPIC: &str = "symbion/synology/ups";
const FEATURES_TOPIC: &str = "symbion/features/update";
const FEATURE_SOURCE: &str = "plugin.synology";
const FEATURE_TTL_SECONDS: u32 = 120; // UPS polled every ~30s

/// Enveloppe attendue par le kernel sur `symbion/features/update`.
/// Le kernel ne lit que feature_id/value/source/ttl_seconds ; les autres
/// champs sont fournis pour rester homogène avec les plugins existants.
#[derive(Debug, Serialize)]
struct FeatureUpdate {
    source: String,
    signal_type: String,
    feature_id: String,
    value: serde_json::Value,
    timestamp: String,
    ttl_seconds: u32,
}

pub struct MqttPublisher {
    client: AsyncClient,
    _event_tx: mpsc::Sender<()>, // garde l'eventloop en vie
}

impl MqttPublisher {
    pub async fn new(config: &MqttConfig) -> Result<Self> {
        let mut opts = MqttOptions::new(&config.client_id, &config.host, config.port);
        opts.set_keep_alive(Duration::from_secs(30));
        opts.set_clean_session(false);

        let (client, mut eventloop) = AsyncClient::new(opts, 100);

        let (tx, mut rx) = mpsc::channel::<()>(1);
        tokio::spawn(async move {
            loop {
                tokio::select! {
                    event = eventloop.poll() => {
                        match event {
                            Ok(Event::Incoming(Packet::ConnAck(_))) => {
                                info!("[mqtt] Connected to broker");
                            }
                            Ok(_) => {}
                            Err(e) => {
                                warn!("[mqtt] Connection error: {:?}", e);
                                tokio::time::sleep(Duration::from_secs(5)).await;
                            }
                        }
                    }
                    _ = rx.recv() => break,
                }
            }
        });

        // Laisse le ConnAck arriver avant les premiers publish retained.
        tokio::time::sleep(Duration::from_millis(500)).await;

        Ok(Self { client, _event_tx: tx })
    }

    /// Manifest plugin (retained).
    pub async fn publish_manifest(&self, manifest: &str) -> Result<()> {
        let topic = format!("{}/manifest", PLUGIN_TOPIC);
        self.client
            .publish(&topic, QoS::AtLeastOnce, true, manifest)
            .await
            .context("Failed to publish manifest")?;
        Ok(())
    }

    /// Health plugin (retained).
    pub async fn publish_health(&self, healthy: bool, message: &str) -> Result<()> {
        let topic = format!("{}/status", PLUGIN_TOPIC);
        let payload = json!({
            "plugin_id": "synology",
            "healthy": healthy,
            "message": message,
            "timestamp": chrono::Utc::now().to_rfc3339(),
        });
        self.client
            .publish(&topic, QoS::AtLeastOnce, true, payload.to_string())
            .await
            .context("Failed to publish health")?;
        Ok(())
    }

    /// Publie l'état UPS : features (pour automations) + payload complet (pour PWA).
    pub async fn publish_ups(&self, status: &UpsStatus) -> Result<()> {
        let on_battery = status.on_battery();
        let battery_low = status.battery_low();
        let now = chrono::Utc::now().to_rfc3339();

        // Features → automations Intelligence v2
        self.publish_feature("synology.ups.on_battery", "ups.on_battery", json!(on_battery), &now).await?;
        self.publish_feature("synology.ups.battery_low", "ups.battery_low", json!(battery_low), &now).await?;
        self.publish_feature("synology.ups.battery_charge", "ups.battery_charge", json!(status.battery_charge), &now).await?;
        self.publish_feature("synology.ups.runtime_seconds", "ups.runtime", json!(status.battery_runtime_seconds), &now).await?;
        self.publish_feature("synology.ups.load", "ups.load", json!(status.load_percent), &now).await?;
        self.publish_feature("synology.ups.status", "ups.status", json!(status.status), &now).await?;

        // Payload complet (retained) pour la PWA
        let payload = json!({
            "status": status.status,
            "on_battery": on_battery,
            "battery_low": battery_low,
            "battery_charge": status.battery_charge,
            "battery_runtime_seconds": status.battery_runtime_seconds,
            "load_percent": status.load_percent,
            "output_voltage": status.output_voltage,
            "model": status.model,
            "manufacturer": status.manufacturer,
            "timestamp": now,
        });
        self.client
            .publish(UPS_TOPIC, QoS::AtLeastOnce, true, payload.to_string())
            .await
            .context("Failed to publish UPS status")?;

        if on_battery {
            info!(
                "UPS on battery! charge={}% runtime={}s low={}",
                status.battery_charge, status.battery_runtime_seconds, battery_low
            );
        }

        Ok(())
    }

    async fn publish_feature(
        &self,
        feature_id: &str,
        signal_type: &str,
        value: serde_json::Value,
        timestamp: &str,
    ) -> Result<()> {
        let feature = FeatureUpdate {
            source: FEATURE_SOURCE.to_string(),
            signal_type: signal_type.to_string(),
            feature_id: feature_id.to_string(),
            value,
            timestamp: timestamp.to_string(),
            ttl_seconds: FEATURE_TTL_SECONDS,
        };
        self.client
            .publish(FEATURES_TOPIC, QoS::AtLeastOnce, false, serde_json::to_string(&feature)?)
            .await
            .with_context(|| format!("Failed to publish feature {}", feature_id))?;
        Ok(())
    }
}
