//! Command processing and response dispatch

use anyhow::{Result, Context};
use chrono::Utc;
use tracing::{info, error, debug, warn};

use crate::messages::*;
use crate::mqtt_client;

use super::Agent;

impl Agent {
    /// Process incoming command from MQTT
    pub(crate) async fn process_command(&mut self, cmd: ReceivedCommand) -> Result<()> {
        let start_time = std::time::Instant::now();

        let incoming: IncomingCommand = serde_json::from_str(&cmd.payload)
            .context("Failed to parse incoming command")?;

        // Only process commands for this agent
        if incoming.agent_id != self.system_info.agent_id {
            debug!("Ignoring command {} for agent {}", incoming.command_id, incoming.agent_id);
            return Ok(());
        }

        info!("Executing command: {} ({})", incoming.command_type, incoming.command_id);
        self.log("INFO", &format!("Command: {} ({})", incoming.command_type, incoming.command_id)).await;

        // Dispatch via registry (with special-case for reconnect)
        let result = if incoming.command_type == "reconnect" {
            crate::execution::handler::CommandResult::success(serde_json::json!({
                "message": "Reconnect acknowledged — agent will re-register on next heartbeat"
            }))
        } else {
            match self.command_registry.execute(&incoming.command_type, incoming.parameters.as_ref()).await {
                Some(result) => result,
                None => {
                    warn!(
                        command_type = %incoming.command_type,
                        command_id = %incoming.command_id,
                        "[agent] REJECTED unknown command type — forensic audit"
                    );
                    crate::execution::handler::CommandResult::error(
                        "UNKNOWN_COMMAND",
                        format!("Unknown command type: {}", incoming.command_type),
                    )
                }
            }
        };

        // Update last command info
        self.last_command = Some(CommandInfo {
            command_id: incoming.command_id.clone(),
            command_type: incoming.command_type.clone(),
            status: result.status.clone(),
            timestamp: Utc::now(),
        });

        // Build and send response
        let response = CommandResponse {
            command_id: incoming.command_id,
            agent_id: self.system_info.agent_id.clone(),
            status: result.status,
            output: result.data,
            error: result.error,
            execution_time_ms: start_time.elapsed().as_millis(),
            timestamp: Utc::now(),
        };

        self.send_response(response).await
    }

    /// Send command response, truncating large outputs for MQTT transport
    pub(crate) async fn send_response(&self, mut response: CommandResponse) -> Result<()> {
        super::truncate_output(&mut response);
        mqtt_client::publish_json(&self.mqtt_client, mqtt_client::TOPIC_RESPONSE, &response).await
    }

    /// Best-effort error response when command JSON fails to parse.
    /// Extracts command_id from raw JSON so the kernel can update PendingCommand status.
    pub(crate) async fn send_error_response_from_raw(&self, raw_json: &str, error_msg: &str) {
        // Try to extract command_id and agent_id from the raw JSON
        let parsed: Result<serde_json::Value, _> = serde_json::from_str(raw_json);
        let (command_id, agent_id) = match parsed {
            Ok(val) => {
                let cid = val.get("command_id").and_then(|v| v.as_str()).unwrap_or("unknown");
                let aid = val.get("agent_id").and_then(|v| v.as_str()).unwrap_or("unknown");
                (cid.to_string(), aid.to_string())
            }
            Err(_) => return, // Can't even parse as JSON — nothing we can do
        };

        // Only respond if this command is for us
        if agent_id != self.system_info.agent_id {
            return;
        }

        let response = CommandResponse {
            command_id,
            agent_id,
            status: "error".to_string(),
            output: None,
            error: Some(ErrorInfo {
                code: "PARSE_ERROR".to_string(),
                message: error_msg.to_string(),
            }),
            execution_time_ms: 0,
            timestamp: Utc::now(),
        };

        if let Err(e) = mqtt_client::publish_json(&self.mqtt_client, mqtt_client::TOPIC_RESPONSE, &response).await {
            error!("Failed to send error response: {}", e);
        }
    }
}
