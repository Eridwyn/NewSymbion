use rumqttc::{AsyncClient, QoS};
use serde_json::json;
use time::OffsetDateTime;
use time::format_description::well_known::Rfc3339;

const PLUGIN_ID: &str = "telegram";
const SPEC_VERSION: &str = "1.0";

/// Emit an event to the kernel via MQTT
pub async fn emit_event(client: &AsyncClient, event_type: &str, payload: serde_json::Value) {
    let timestamp = OffsetDateTime::now_utc()
        .format(&Rfc3339)
        .unwrap_or_else(|_| "unknown".into());

    let event = json!({
        "spec_version": SPEC_VERSION,
        "event_type": event_type,
        "plugin_id": PLUGIN_ID,
        "payload": payload,
        "timestamp": timestamp,
    });

    let topic = format!("symbion/plugins/{}/events", PLUGIN_ID);
    let payload_bytes = serde_json::to_vec(&event).unwrap_or_default();

    if let Err(e) = client.publish(&topic, QoS::AtLeastOnce, false, payload_bytes).await {
        eprintln!("[telegram] Failed to emit event {}: {}", event_type, e);
    }
}

/// Publish health heartbeat
pub async fn publish_health(client: &AsyncClient, uptime_secs: u64, status: &str) {
    let timestamp = OffsetDateTime::now_utc()
        .format(&Rfc3339)
        .unwrap_or_else(|_| "unknown".into());

    let health = json!({
        "spec_version": SPEC_VERSION,
        "plugin_id": PLUGIN_ID,
        "status": status,
        "uptime_seconds": uptime_secs,
        "timestamp": timestamp,
    });

    let topic = format!("symbion/plugins/{}/health", PLUGIN_ID);
    let payload = serde_json::to_vec(&health).unwrap_or_default();

    if let Err(e) = client.publish(&topic, QoS::AtLeastOnce, false, payload).await {
        eprintln!("[telegram] Failed to publish health: {}", e);
    }
}

/// Publish plugin manifest (retained)
pub async fn publish_manifest(client: &AsyncClient, socket_path: &str) {
    let manifest = json!({
        "spec_version": SPEC_VERSION,
        "plugin_id": PLUGIN_ID,
        "name": "Symbion Telegram",
        "version": "1.0.0",
        "description": "Bridge Telegram-Claude Code avec integration Symbion",
        "capabilities": [
            {
                "action_type": "send_message",
                "description": "Envoyer un message a un utilisateur Telegram",
                "impact_level": "low",
                "parameters": {
                    "chat_id": { "type": "integer", "required": true },
                    "text": { "type": "string", "required": true },
                    "parse_mode": { "type": "string", "required": false }
                }
            },
            {
                "action_type": "send_notification",
                "description": "Envoyer une notification a tous les utilisateurs autorises",
                "impact_level": "low",
                "parameters": {
                    "text": { "type": "string", "required": true },
                    "level": { "type": "string", "required": false }
                }
            }
        ],
        "events": [
            { "event_type": "message_received", "description": "Message utilisateur recu" },
            { "event_type": "command_received", "description": "Commande Symbion recue" },
            { "event_type": "claude_response_completed", "description": "Reponse Claude terminee" }
        ],
        "health_endpoint": "/health",
        "socket_path": socket_path,
    });

    let topic = format!("symbion/plugins/{}/manifest", PLUGIN_ID);
    let payload = serde_json::to_vec(&manifest).unwrap_or_default();

    if let Err(e) = client.publish(&topic, QoS::AtLeastOnce, true, payload).await {
        eprintln!("[telegram] Failed to publish manifest: {}", e);
    } else {
        println!("[telegram] Manifest published");
    }
}
