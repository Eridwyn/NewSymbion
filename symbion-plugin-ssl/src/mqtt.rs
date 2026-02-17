//! MQTT publisher for SSL plugin

use anyhow::{Context, Result};
use rumqttc::{AsyncClient, Event, MqttOptions, Packet, QoS};
use serde::Serialize;
use std::time::Duration;
use tokio::sync::mpsc;

use crate::config::{AlertConfig, MqttConfig};
use crate::ssl::CertificateStatus;

/// MQTT topic prefixes
const TOPIC_PREFIX: &str = "symbion/ssl";
const PLUGIN_TOPIC: &str = "symbion/plugins/ssl";
const FEATURES_TOPIC: &str = "symbion/features";

/// MQTT publisher for SSL status
pub struct MqttPublisher {
    client: AsyncClient,
    _event_tx: mpsc::Sender<()>, // Keep eventloop alive
}

/// Domain status for MQTT publication
#[derive(Debug, Clone, Serialize)]
pub struct DomainStatus {
    pub domain_id: String,
    pub hostname: String,
    pub port: u16,
    pub online: bool,
    pub ssl_valid: bool,
    pub days_remaining: Option<i64>,
    pub expiry_date: Option<String>,
    pub issuer: Option<String>,
    pub status_level: String, // "ok", "warning", "critical", "error"
    pub error: Option<String>,
    pub checked_at: String,
}

/// Feature update for Intelligence v2
#[derive(Debug, Serialize)]
pub struct FeatureUpdate {
    pub source: String,
    pub signal_type: String,
    pub feature_id: String,
    pub value: serde_json::Value,
    pub timestamp: String,
    pub ttl_seconds: u32,
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
        options.set_clean_session(false);

        let (client, mut eventloop) = AsyncClient::new(options, 100);

        // Spawn event loop handler
        let (tx, mut rx) = mpsc::channel::<()>(1);
        tokio::spawn(async move {
            loop {
                tokio::select! {
                    event = eventloop.poll() => {
                        match event {
                            Ok(Event::Incoming(Packet::ConnAck(_))) => {
                                println!("[mqtt] Connected to broker");
                            }
                            Ok(Event::Incoming(Packet::PubAck(_))) => {}
                            Ok(_) => {}
                            Err(e) => {
                                eprintln!("[mqtt] Connection error: {:?}", e);
                                tokio::time::sleep(Duration::from_secs(5)).await;
                            }
                        }
                    }
                    _ = rx.recv() => {
                        break;
                    }
                }
            }
        });

        // Wait for connection
        tokio::time::sleep(Duration::from_millis(500)).await;

        Ok(Self {
            client,
            _event_tx: tx,
        })
    }

    /// Publish plugin manifest (retained)
    pub async fn publish_manifest(&self, manifest: &str) -> Result<()> {
        let topic = format!("{}/manifest", PLUGIN_TOPIC);
        self.client
            .publish(&topic, QoS::AtLeastOnce, true, manifest)
            .await
            .context("Failed to publish manifest")?;
        Ok(())
    }

    /// Publish plugin health status
    pub async fn publish_health(&self, healthy: bool, message: &str) -> Result<()> {
        let topic = format!("{}/status", PLUGIN_TOPIC);
        let payload = serde_json::json!({
            "plugin_id": "ssl",
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

    /// Publish certificate status for a domain
    pub async fn publish_certificate(
        &self,
        domain_id: &str,
        status: &CertificateStatus,
        alerts: &AlertConfig,
    ) -> Result<()> {
        // Determine status level
        let status_level = match status.days_remaining {
            Some(days) if days < 0 => "expired",
            Some(days) if days <= alerts.critical_days => "critical",
            Some(days) if days <= alerts.warning_days => "warning",
            Some(_) => "ok",
            None => "error",
        };

        let domain_status = DomainStatus {
            domain_id: domain_id.to_string(),
            hostname: status.hostname.clone(),
            port: status.port,
            online: true, // If we got here, it's online
            ssl_valid: status.valid,
            days_remaining: status.days_remaining,
            expiry_date: status.expiry_date.map(|d| d.format("%Y-%m-%d").to_string()),
            issuer: status.issuer.clone(),
            status_level: status_level.to_string(),
            error: status.error.clone(),
            checked_at: status.checked_at.to_rfc3339(),
        };

        // Publish to domain topic
        let topic = format!("{}/{}", TOPIC_PREFIX, domain_id);
        let payload = serde_json::to_string(&domain_status)?;
        self.client
            .publish(&topic, QoS::AtLeastOnce, true, payload)
            .await
            .context("Failed to publish domain status")?;

        // Publish features for Intelligence v2
        self.publish_features(domain_id, &domain_status).await?;

        Ok(())
    }

    /// Publish online status for a domain
    pub async fn publish_online(&self, domain_id: &str, hostname: &str, online: bool) -> Result<()> {
        let topic = format!("{}/{}/online", TOPIC_PREFIX, domain_id);
        let payload = serde_json::json!({
            "domain_id": domain_id,
            "hostname": hostname,
            "online": online,
            "timestamp": chrono::Utc::now().to_rfc3339()
        });

        self.client
            .publish(&topic, QoS::AtLeastOnce, false, payload.to_string())
            .await
            .context("Failed to publish online status")?;

        // Publish feature
        let feature = FeatureUpdate {
            source: "plugin.ssl".to_string(),
            signal_type: "domain.online".to_string(),
            feature_id: format!("ssl.{}.online", domain_id),
            value: serde_json::Value::Bool(online),
            timestamp: chrono::Utc::now().to_rfc3339(),
            ttl_seconds: 120, // 2 minutes TTL
        };

        let feature_topic = format!("{}/update", FEATURES_TOPIC);
        self.client
            .publish(&feature_topic, QoS::AtLeastOnce, false, serde_json::to_string(&feature)?)
            .await
            .context("Failed to publish online feature")?;

        Ok(())
    }

    /// Publish features for Intelligence v2
    async fn publish_features(&self, domain_id: &str, status: &DomainStatus) -> Result<()> {
        let now = chrono::Utc::now().to_rfc3339();
        let ttl = 7200; // 2 hours TTL for SSL features

        // Feature: ssl.{domain}.valid (bool)
        let valid_feature = FeatureUpdate {
            source: "plugin.ssl".to_string(),
            signal_type: "ssl.valid".to_string(),
            feature_id: format!("ssl.{}.valid", domain_id),
            value: serde_json::Value::Bool(status.ssl_valid),
            timestamp: now.clone(),
            ttl_seconds: ttl,
        };

        // Feature: ssl.{domain}.days_remaining (int)
        let days_feature = FeatureUpdate {
            source: "plugin.ssl".to_string(),
            signal_type: "ssl.days_remaining".to_string(),
            feature_id: format!("ssl.{}.days_remaining", domain_id),
            value: status.days_remaining
                .map(|d| serde_json::Value::Number(d.into()))
                .unwrap_or(serde_json::Value::Null),
            timestamp: now.clone(),
            ttl_seconds: ttl,
        };

        // Feature: ssl.{domain}.status (string: ok/warning/critical/error)
        let status_feature = FeatureUpdate {
            source: "plugin.ssl".to_string(),
            signal_type: "ssl.status".to_string(),
            feature_id: format!("ssl.{}.status", domain_id),
            value: serde_json::Value::String(status.status_level.clone()),
            timestamp: now.clone(),
            ttl_seconds: ttl,
        };

        let topic = format!("{}/update", FEATURES_TOPIC);

        for feature in [valid_feature, days_feature, status_feature] {
            self.client
                .publish(&topic, QoS::AtLeastOnce, false, serde_json::to_string(&feature)?)
                .await
                .context("Failed to publish feature")?;
        }

        Ok(())
    }

    /// Publish fingerprint change event for automation triggers
    pub async fn publish_fingerprint_change(
        &self,
        domain_id: &str,
        hostname: &str,
        old_fingerprint: &str,
        new_fingerprint: &str,
    ) -> Result<()> {
        let topic = format!("{}/{}/fingerprint-change", TOPIC_PREFIX, domain_id);
        let payload = serde_json::json!({
            "domain_id": domain_id,
            "hostname": hostname,
            "old_fingerprint": old_fingerprint,
            "new_fingerprint": new_fingerprint,
            "timestamp": chrono::Utc::now().to_rfc3339()
        });

        self.client
            .publish(&topic, QoS::AtLeastOnce, false, payload.to_string())
            .await
            .context("Failed to publish fingerprint change")?;

        // Publish feature for automation triggers
        let feature = FeatureUpdate {
            source: "plugin.ssl".to_string(),
            signal_type: "ssl.fingerprint_changed".to_string(),
            feature_id: format!("ssl.{}.fingerprint_changed", domain_id),
            value: serde_json::Value::Bool(true),
            timestamp: chrono::Utc::now().to_rfc3339(),
            ttl_seconds: 3600, // 1 hour TTL
        };

        let feature_topic = format!("{}/update", FEATURES_TOPIC);
        self.client
            .publish(&feature_topic, QoS::AtLeastOnce, false, serde_json::to_string(&feature)?)
            .await
            .context("Failed to publish fingerprint change feature")?;

        Ok(())
    }

    /// Publish summary of all domains
    pub async fn publish_summary(&self, domains: &[DomainStatus]) -> Result<()> {
        let total = domains.len();
        let valid = domains.iter().filter(|d| d.ssl_valid).count();
        let expiring_soon = domains.iter().filter(|d| {
            d.days_remaining.map(|days| days <= 30).unwrap_or(false)
        }).count();

        let summary = serde_json::json!({
            "total_domains": total,
            "valid_certificates": valid,
            "expiring_soon": expiring_soon,
            "domains": domains,
            "timestamp": chrono::Utc::now().to_rfc3339()
        });

        let topic = format!("{}/summary", TOPIC_PREFIX);
        self.client
            .publish(&topic, QoS::AtLeastOnce, true, summary.to_string())
            .await
            .context("Failed to publish summary")?;

        Ok(())
    }
}
