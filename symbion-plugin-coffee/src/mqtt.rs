//! MQTT publisher for coffee machine events and features

use anyhow::Result;
use rumqttc::{AsyncClient, EventLoop, MqttOptions, QoS};
use serde::Serialize;
use tokio::time::Duration;
use tracing::{error, warn};

use crate::MachineStatus;

const PLUGIN_ID: &str = "coffee";

pub struct MqttPublisher {
    client: AsyncClient,
}

#[derive(Debug, Serialize)]
struct FeatureUpdate {
    source: String,
    feature_id: String,
    value: serde_json::Value,
    timestamp: String,
    ttl_seconds: u32,
}

impl MqttPublisher {
    pub async fn new(host: &str, port: u16) -> Result<Self> {
        let mut opts = MqttOptions::new("symbion-plugin-coffee", host, port);
        opts.set_keep_alive(Duration::from_secs(30));
        opts.set_clean_session(true);

        let (client, eventloop) = AsyncClient::new(opts, 32);

        // Spawn event loop handler
        tokio::spawn(Self::run_eventloop(eventloop));

        Ok(Self { client })
    }

    async fn run_eventloop(mut eventloop: EventLoop) {
        loop {
            match eventloop.poll().await {
                Ok(_) => {}
                Err(e) => {
                    warn!("MQTT event loop error: {} (reconnecting...)", e);
                    tokio::time::sleep(Duration::from_secs(5)).await;
                }
            }
        }
    }

    /// Publish an event to MQTT
    pub async fn publish_event(&self, event_type: &str, payload: &serde_json::Value) {
        let topic = format!("symbion/{}/{}", PLUGIN_ID, event_type);
        let message = serde_json::json!({
            "spec_version": "1.0",
            "event_type": event_type,
            "plugin_id": PLUGIN_ID,
            "payload": payload,
            "timestamp": chrono::Utc::now().to_rfc3339(),
        });

        if let Err(e) = self
            .client
            .publish(&topic, QoS::AtLeastOnce, false, serde_json::to_vec(&message).unwrap_or_default())
            .await
        {
            error!("MQTT publish failed ({}): {}", topic, e);
        }
    }

    /// Publish machine features to the kernel FeatureRegistry.
    /// Schema: one MQTT message per feature, matching `ExternalFeatureUpdate`
    /// expected by symbion-kernel/src/mqtt.rs:66-72.
    pub async fn publish_features(&self, status: &MachineStatus) {
        let now = chrono::Utc::now().to_rfc3339();
        let ttl = 120; // 4× la cadence de publish (30s) — même logique que SSL.online

        // Build feature list — skip last_brew_minutes_ago when no brew yet
        // (sinon le kernel sérialise Value::Null en FeatureValue::String("null"), pas exploitable)
        let mut features: Vec<(&str, serde_json::Value)> = vec![
            ("coffee.online", status.online.into()),
            ("coffee.ready", (status.mainstate == 2).into()),
            ("coffee.brewing", status.brewing.into()),
            ("coffee.brew_progress", (status.brew_progress as i32).into()),
            ("coffee.water_level", (status.water_level as i32).into()),
            ("coffee.bean_level", (status.bean_level as i32).into()),
            ("coffee.maintenance", status.maintenance_needed.into()),
            ("coffee.descale_status", (status.descale_status as i32).into()),
            (
                "coffee.aquaclean_remaining",
                (status.aquaclean_remaining.unwrap_or(0) as i32).into(),
            ),
            ("coffee.brews_today", (status.brew_count_today as i32).into()),
        ];

        if let Some(t) = status.last_brew_at {
            let minutes = (chrono::Utc::now() - t).num_minutes();
            features.push(("coffee.last_brew_minutes_ago", minutes.into()));
        }

        for (id, value) in features {
            let msg = FeatureUpdate {
                source: "plugin.coffee".to_string(),
                feature_id: id.to_string(),
                value,
                timestamp: now.clone(),
                ttl_seconds: ttl,
            };
            let payload = match serde_json::to_vec(&msg) {
                Ok(p) => p,
                Err(e) => {
                    error!("MQTT serialize {}: {}", id, e);
                    continue;
                }
            };
            if let Err(e) = self
                .client
                .publish("symbion/features/update", QoS::AtLeastOnce, false, payload)
                .await
            {
                error!("MQTT publish {}: {}", id, e);
            }
        }
    }
}
