//! MQTT publisher for coffee machine events and features

use anyhow::Result;
use rumqttc::{AsyncClient, EventLoop, MqttOptions, QoS};
use tokio::time::Duration;
use tracing::{error, warn};

use crate::MachineStatus;

const PLUGIN_ID: &str = "coffee";

pub struct MqttPublisher {
    client: AsyncClient,
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

    /// Publish machine features for automation engine
    pub async fn publish_features(&self, status: &MachineStatus) {
        let features = serde_json::json!({
            "features": {
                "coffee.online": status.online,
                "coffee.ready": status.mainstate == 2,
                "coffee.brewing": status.brewing,
                "coffee.water_level": status.water_level as i32,
                "coffee.bean_level": status.bean_level as i32,
                "coffee.maintenance": status.maintenance_needed,
                "coffee.descale_status": status.descale_status as i32,
            }
        });

        if let Err(e) = self
            .client
            .publish(
                "symbion/features/update",
                QoS::AtLeastOnce,
                false,
                serde_json::to_vec(&features).unwrap_or_default(),
            )
            .await
        {
            error!("MQTT features publish failed: {}", e);
        }
    }
}
