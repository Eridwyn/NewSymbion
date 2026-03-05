//! Heartbeat and registration publishing

use anyhow::{Result, Context};
use chrono::Utc;
use tracing::{info, warn, debug};

use crate::capabilities;
use crate::messages::*;
use crate::metrics;
use crate::mqtt_client;

use super::Agent;

impl Agent {
    /// Send a final heartbeat with status=offline before shutting down
    pub(crate) async fn send_offline_heartbeat(&self) {
        let heartbeat = HeartbeatMessage {
            agent_id: self.system_info.agent_id.clone(),
            status: "offline".to_string(),
            system: match metrics::SystemMetrics::collect().await {
                Ok(m) => m,
                Err(_) => return,
            },
            processes: None,
            services: None,
            last_command: self.last_command.clone(),
            watchdog: Some(self.watchdog.report().await),
            plugin_data: None,
            timestamp: Utc::now(),
        };

        if let Err(e) = mqtt_client::publish_json(
            &self.mqtt_client, mqtt_client::TOPIC_HEARTBEAT, &heartbeat
        ).await {
            warn!("Failed to send offline heartbeat: {}", e);
        } else {
            info!("Offline heartbeat sent");
        }
    }

    /// Register agent with kernel
    pub(crate) async fn register(&self) -> Result<()> {
        let capabilities = self.get_capabilities().await;

        let registration = RegistrationMessage {
            agent_id: self.system_info.agent_id.clone(),
            hostname: self.system_info.hostname.clone(),
            os: self.system_info.os.clone(),
            architecture: self.system_info.architecture.clone(),
            capabilities,
            network: self.system_info.network.clone(),
            version: env!("CARGO_PKG_VERSION").to_string(),
            timestamp: Utc::now(),
        };

        mqtt_client::publish_json(&self.mqtt_client, mqtt_client::TOPIC_REGISTRATION, &registration).await?;
        info!("Agent registered successfully");
        Ok(())
    }

    /// Send heartbeat with system metrics. Returns collected metrics for reuse.
    pub(crate) async fn send_heartbeat(&self) -> Result<metrics::SystemMetrics> {
        let system_metrics = metrics::SystemMetrics::collect().await
            .context("Failed to collect system metrics")?;
        let process_info = metrics::ProcessInfo::collect().await.ok();
        let services = metrics::ServiceStatus::collect_critical().await.ok();

        let watchdog_report = self.watchdog.report().await;

        // Tick plugins and collect data
        let plugin_data = {
            let registry = self.plugin_registry.lock().await;
            let data = registry.tick_all().await;
            if data.is_empty() { None } else { Some(data) }
        };

        let heartbeat = HeartbeatMessage {
            agent_id: self.system_info.agent_id.clone(),
            status: "online".to_string(),
            system: system_metrics.clone(),
            processes: process_info,
            services,
            last_command: self.last_command.clone(),
            watchdog: Some(watchdog_report),
            plugin_data,
            timestamp: Utc::now(),
        };

        mqtt_client::publish_json(&self.mqtt_client, mqtt_client::TOPIC_HEARTBEAT, &heartbeat).await?;
        debug!("Heartbeat sent");

        // Flush buffered logs to kernel
        self.log_collector.flush().await;

        Ok(system_metrics)
    }

    /// Get agent capabilities
    pub(crate) async fn get_capabilities(&self) -> Vec<String> {
        capabilities::CapabilityDetector::get_available_capabilities().await
    }
}
