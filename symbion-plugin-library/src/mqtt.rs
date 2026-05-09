use anyhow::{Context, Result};
use rumqttc::{AsyncClient, Event, MqttOptions, Packet, QoS};
use serde::Serialize;
use std::time::Duration;
use tokio::sync::mpsc;

const PLUGIN_TOPIC: &str = "symbion/plugins/library";

#[derive(Debug, Serialize)]
struct FeatureUpdate {
    source: String,
    feature_id: String,
    value: serde_json::Value,
    timestamp: String,
    ttl_seconds: u32,
}

pub struct MqttPublisher {
    client: AsyncClient,
    _keepalive: mpsc::Sender<()>,
}

impl MqttPublisher {
    pub async fn connect(host: &str, port: u16, client_id: &str) -> Result<Self> {
        let mut options = MqttOptions::new(client_id, host, port);
        options.set_keep_alive(Duration::from_secs(30));
        options.set_clean_session(true);

        let (client, mut eventloop) = AsyncClient::new(options, 100);

        let (tx, mut rx) = mpsc::channel::<()>(1);
        tokio::spawn(async move {
            loop {
                tokio::select! {
                    event = eventloop.poll() => {
                        match event {
                            Ok(Event::Incoming(Packet::ConnAck(_))) => {
                                tracing::info!("[library-mqtt] connected to broker");
                            }
                            Ok(_) => {}
                            Err(e) => {
                                tracing::warn!("[library-mqtt] connection error: {:?}", e);
                                tokio::time::sleep(Duration::from_secs(5)).await;
                            }
                        }
                    }
                    _ = rx.recv() => break,
                }
            }
        });

        tokio::time::sleep(Duration::from_millis(500)).await;
        Ok(Self { client, _keepalive: tx })
    }

    pub async fn publish_manifest(&self, manifest: &str) -> Result<()> {
        let topic = format!("{}/manifest", PLUGIN_TOPIC);
        self.client
            .publish(&topic, QoS::AtLeastOnce, true, manifest)
            .await
            .context("Failed to publish manifest")?;
        Ok(())
    }

    pub async fn publish_health(&self, healthy: bool, message: &str) -> Result<()> {
        let topic = format!("{}/status", PLUGIN_TOPIC);
        let payload = serde_json::json!({
            "plugin_id": "library",
            "healthy": healthy,
            "message": message,
            "timestamp": chrono::Utc::now().to_rfc3339()
        });
        self.client
            .publish(&topic, QoS::AtLeastOnce, true, payload.to_string())
            .await
            .context("Failed to publish health")?;
        Ok(())
    }

    pub async fn publish_node_event(&self, event_type: &str, node_id: &str, title: &str) -> Result<()> {
        let topic = format!("symbion/library/nodes/{}", event_type);
        let payload = serde_json::json!({
            "node_id": node_id,
            "title": title,
            "event": event_type,
            "timestamp": chrono::Utc::now().to_rfc3339()
        });
        self.client
            .publish(&topic, QoS::AtLeastOnce, false, payload.to_string())
            .await
            .context("Failed to publish node event")?;
        Ok(())
    }

    pub async fn publish_pending_links(&self, count: usize) -> Result<()> {
        let topic = "symbion/library/links/pending".to_string();
        let payload = serde_json::json!({
            "new_pending_links": count,
            "timestamp": chrono::Utc::now().to_rfc3339()
        });
        self.client
            .publish(&topic, QoS::AtLeastOnce, false, payload.to_string())
            .await
            .context("Failed to publish pending links")?;
        Ok(())
    }

    /// Publish library features to the kernel FeatureRegistry.
    /// Schema: one MQTT message per feature, matching `ExternalFeatureUpdate`
    /// expected by symbion-kernel/src/mqtt.rs:66-72.
    pub async fn publish_features(&self, nodes: i64, sections: i64, pending: i64) -> Result<()> {
        let now = chrono::Utc::now().to_rfc3339();
        let ttl = 600; // library publie peu fréquemment (sur changement)

        let features: Vec<(&str, serde_json::Value)> = vec![
            ("library.nodes.count", nodes.into()),
            ("library.sections.count", sections.into()),
            ("library.pending_links.count", pending.into()),
        ];

        for (id, value) in features {
            let msg = FeatureUpdate {
                source: "plugin.library".to_string(),
                feature_id: id.to_string(),
                value,
                timestamp: now.clone(),
                ttl_seconds: ttl,
            };
            self.client
                .publish(
                    "symbion/features/update",
                    QoS::AtLeastOnce,
                    false,
                    serde_json::to_vec(&msg).context("serialize feature")?,
                )
                .await
                .with_context(|| format!("Failed to publish feature {}", id))?;
        }
        Ok(())
    }
}
