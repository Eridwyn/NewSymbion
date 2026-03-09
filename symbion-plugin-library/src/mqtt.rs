use anyhow::{Context, Result};
use rumqttc::{AsyncClient, Event, MqttOptions, Packet, QoS};
use std::time::Duration;
use tokio::sync::mpsc;

const PLUGIN_TOPIC: &str = "symbion/plugins/library";

pub struct MqttPublisher {
    client: AsyncClient,
    _keepalive: mpsc::Sender<()>,
}

impl MqttPublisher {
    pub async fn connect(host: &str, port: u16, client_id: &str) -> Result<Self> {
        let mut options = MqttOptions::new(client_id, host, port);
        options.set_keep_alive(Duration::from_secs(30));
        options.set_clean_session(false);

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

    pub async fn publish_features(&self, nodes: i64, sections: i64, pending: i64) -> Result<()> {
        let payload = serde_json::json!({
            "source": "library",
            "features": {
                "library.nodes.count": nodes,
                "library.sections.count": sections,
                "library.pending_links.count": pending
            },
            "timestamp": chrono::Utc::now().to_rfc3339()
        });
        self.client
            .publish("symbion/features/update", QoS::AtLeastOnce, false, payload.to_string())
            .await
            .context("Failed to publish features")?;
        Ok(())
    }
}
