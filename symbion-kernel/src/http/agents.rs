use super::AppState;
use axum::extract::{Path, State};
use axum::http::StatusCode;
use axum::Json;
use serde::Deserialize;
use time::format_description::well_known::Rfc3339;

// ====== AGENTS ENDPOINTS ======

#[derive(serde::Serialize)]
pub(super) struct AgentView {
    agent_id: String,
    hostname: String,
    os: String,
    architecture: String,
    capabilities: Vec<String>,
    primary_mac: String,
    primary_ip: String,
    status: String,
    last_seen: String,
    registration_time: String,
    uptime_seconds: Option<u64>,
    cpu_percent: Option<f32>,
    memory_percent: Option<f32>,
}

#[derive(Deserialize)]
pub(super) struct AgentCommandRequest {
    command: String,
    parameters: Option<serde_json::Value>,
}

#[derive(Deserialize)]
pub(super) struct AgentCommandTrackingRequest {
    command_type: String,
    parameters: serde_json::Value,
}

pub(super) fn agent_to_view(agent: &crate::agents::Agent) -> AgentView {
    // Prefer IPv4 over IPv6 for display (IPv6 are too long for UI)
    let primary_ip = agent.network.interfaces
        .iter()
        .find(|i| !i.ip.contains(':'))  // IPv4 doesn't contain ':'
        .map(|i| i.ip.clone())
        .or_else(|| agent.network.interfaces.first().map(|i| i.ip.clone()))  // Fallback to any IP
        .unwrap_or_else(|| "unknown".to_string());

    AgentView {
        agent_id: agent.agent_id.clone(),
        hostname: agent.hostname.clone(),
        os: agent.os.clone(),
        architecture: agent.architecture.clone(),
        capabilities: agent.capabilities.clone(),
        primary_mac: agent.network.primary_mac.clone(),
        primary_ip,
        status: agent.status.status.clone(),
        last_seen: agent.last_seen.format(&Rfc3339).unwrap_or_default(),
        registration_time: agent.registration_time.format(&Rfc3339).unwrap_or_default(),
        uptime_seconds: agent.status.system.as_ref().map(|s| s.uptime_seconds),
        cpu_percent: agent.status.system.as_ref().map(|s| s.cpu.percent),
        memory_percent: agent.status.system.as_ref().map(|s| s.memory.percent_used),
    }
}

// GET /agents - Liste des agents
pub(super) async fn list_agents_endpoint(State(app): State<AppState>) -> Json<Vec<AgentView>> {
    let agents = app.agents.list_agents().await;
    let list: Vec<AgentView> = agents.values().map(agent_to_view).collect();
    Json(list)
}

// GET /agents/{id} - Détail d'un agent
pub(super) async fn get_agent_endpoint(
    State(app): State<AppState>,
    Path(id): Path<String>,
) -> Result<Json<crate::agents::Agent>, StatusCode> {
    match app.agents.get_agent(&id).await {
        Some(agent) => Ok(Json(agent)),
        None => Err(StatusCode::NOT_FOUND),
    }
}

// DELETE /v1/agents/{id} - Suppression agent (soft delete, purge après 7 jours)
pub(super) async fn delete_agent_endpoint(
    State(app): State<AppState>,
    Path(id): Path<String>,
) -> Result<StatusCode, (StatusCode, Json<serde_json::Value>)> {
    match app.agents.soft_delete_agent(&id).await {
        Ok(true) => {
            println!("[http] agent {} soft-deleted (will be purged in 7 days)", id);
            Ok(StatusCode::NO_CONTENT)
        }
        Ok(false) => {
            Err((StatusCode::NOT_FOUND, Json(serde_json::json!({
                "error": "Agent not found or already deleted"
            }))))
        }
        Err(e) => {
            eprintln!("[http] failed to delete agent {}: {}", id, e);
            Err((StatusCode::INTERNAL_SERVER_ERROR, Json(serde_json::json!({
                "error": format!("Failed to delete agent: {}", e)
            }))))
        }
    }
}

// DELETE /v1/environment/sensors/{sensor_id} - Suppression capteur (soft delete, purge après 7 jours)
pub(super) async fn delete_sensor_endpoint(
    State(app): State<AppState>,
    Path(sensor_id): Path<String>,
) -> Result<StatusCode, (StatusCode, Json<serde_json::Value>)> {
    match app.sensors.unregister_sensor(&sensor_id) {
        Ok(()) => {
            println!("[http] sensor {} soft-deleted (will be purged in 7 days)", sensor_id);
            Ok(StatusCode::NO_CONTENT)
        }
        Err(e) => {
            eprintln!("[http] failed to delete sensor {}: {}", sensor_id, e);
            Err((StatusCode::INTERNAL_SERVER_ERROR, Json(serde_json::json!({
                "error": format!("Failed to delete sensor: {}", e)
            }))))
        }
    }
}

// POST /agents/{id}/shutdown - Extinction système
pub(super) async fn agent_shutdown_endpoint(
    State(app): State<AppState>,
    Path(id): Path<String>,
) -> Result<Json<serde_json::Value>, StatusCode> {
    match app.agents.send_command(&id, "shutdown", None).await {
        Ok(command_id) => Ok(Json(serde_json::json!({
            "success": true,
            "command_id": command_id,
            "message": "Shutdown command sent"
        }))),
        Err(e) => {
            eprintln!("[http] failed to send shutdown command to agent {}: {}", id, e);
            Err(StatusCode::INTERNAL_SERVER_ERROR)
        }
    }
}

// POST /agents/{id}/reboot - Redémarrage système
pub(super) async fn agent_reboot_endpoint(
    State(app): State<AppState>,
    Path(id): Path<String>,
) -> Result<Json<serde_json::Value>, StatusCode> {
    match app.agents.send_command(&id, "reboot", None).await {
        Ok(command_id) => Ok(Json(serde_json::json!({
            "success": true,
            "command_id": command_id,
            "message": "Reboot command sent"
        }))),
        Err(e) => {
            eprintln!("[http] failed to send reboot command to agent {}: {}", id, e);
            Err(StatusCode::INTERNAL_SERVER_ERROR)
        }
    }
}

// POST /agents/{id}/hibernate - Mise en veille
pub(super) async fn agent_hibernate_endpoint(
    State(app): State<AppState>,
    Path(id): Path<String>,
) -> Result<Json<serde_json::Value>, StatusCode> {
    match app.agents.send_command(&id, "hibernate", None).await {
        Ok(command_id) => Ok(Json(serde_json::json!({
            "success": true,
            "command_id": command_id,
            "message": "Hibernate command sent"
        }))),
        Err(e) => {
            eprintln!("[http] failed to send hibernate command to agent {}: {}", id, e);
            Err(StatusCode::INTERNAL_SERVER_ERROR)
        }
    }
}

// POST /v1/agents/{id}/reconnect - Demande de reconnexion agent
pub(super) async fn agent_reconnect_endpoint(
    State(app): State<AppState>,
    Path(id): Path<String>,
) -> Result<Json<serde_json::Value>, StatusCode> {
    // Vérifier que l'agent existe
    match app.agents.get_agent(&id).await {
        Some(_) => {
            // Envoyer commande reconnect via MQTT
            match app.agents.send_command(&id, "reconnect", None).await {
                Ok(command_id) => Ok(Json(serde_json::json!({
                    "success": true,
                    "command_id": command_id,
                    "message": "Reconnect command sent to agent via kernel"
                }))),
                Err(e) => {
                    eprintln!("[http] failed to send reconnect command to agent {}: {}", id, e);
                    Err(StatusCode::INTERNAL_SERVER_ERROR)
                }
            }
        },
        None => Err(StatusCode::NOT_FOUND),
    }
}

// GET /agents/{id}/processes - Liste des processus
pub(super) async fn agent_processes_endpoint(
    State(app): State<AppState>,
    Path(id): Path<String>,
) -> Result<Json<serde_json::Value>, StatusCode> {
    match app.agents.get_agent(&id).await {
        Some(agent) => {
            if let Some(processes) = &agent.status.processes {
                Ok(Json(serde_json::to_value(processes).unwrap()))
            } else {
                // Demander les processus via MQTT
                match app.agents.send_command(&id, "list_processes", None).await {
                    Ok(command_id) => Ok(Json(serde_json::json!({
                        "success": true,
                        "command_id": command_id,
                        "message": "Process list requested, check agent status for results"
                    }))),
                    Err(e) => {
                        eprintln!("[http] failed to request processes from agent {}: {}", id, e);
                        Err(StatusCode::INTERNAL_SERVER_ERROR)
                    }
                }
            }
        }
        None => Err(StatusCode::NOT_FOUND),
    }
}

// POST /agents/{id}/processes/{pid}/kill - Tuer un processus
pub(super) async fn agent_kill_process_endpoint(
    State(app): State<AppState>,
    Path((id, pid)): Path<(String, u32)>,
) -> Result<Json<serde_json::Value>, StatusCode> {
    let params = serde_json::json!({ "pid": pid });

    match app.agents.send_command(&id, "kill_process", Some(params)).await {
        Ok(command_id) => Ok(Json(serde_json::json!({
            "success": true,
            "command_id": command_id,
            "message": format!("Kill process {} command sent", pid)
        }))),
        Err(e) => {
            eprintln!("[http] failed to send kill process command to agent {}: {}", id, e);
            Err(StatusCode::INTERNAL_SERVER_ERROR)
        }
    }
}

// POST /agents/{id}/command - Exécuter une commande shell
pub(super) async fn agent_command_endpoint(
    State(app): State<AppState>,
    Path(id): Path<String>,
    Json(req): Json<AgentCommandRequest>,
) -> Result<Json<serde_json::Value>, StatusCode> {
    let params = serde_json::json!({
        "command": req.command,
        "parameters": req.parameters
    });

    match app.agents.send_command(&id, "run_command", Some(params)).await {
        Ok(command_id) => Ok(Json(serde_json::json!({
            "success": true,
            "command_id": command_id,
            "message": "Command execution requested"
        }))),
        Err(e) => {
            eprintln!("[http] failed to send command to agent {}: {}", id, e);
            Err(StatusCode::INTERNAL_SERVER_ERROR)
        }
    }
}

// GET /agents/{id}/metrics - Métriques système temps réel
pub(super) async fn agent_metrics_endpoint(
    State(app): State<AppState>,
    Path(id): Path<String>,
) -> Result<Json<serde_json::Value>, StatusCode> {
    match app.agents.get_agent(&id).await {
        Some(agent) => {
            if let Some(system) = &agent.status.system {
                Ok(Json(serde_json::to_value(system).unwrap()))
            } else {
                // Demander les métriques via MQTT
                match app.agents.send_command(&id, "get_metrics", None).await {
                    Ok(command_id) => Ok(Json(serde_json::json!({
                        "success": true,
                        "command_id": command_id,
                        "message": "Metrics requested, check agent status for results"
                    }))),
                    Err(e) => {
                        eprintln!("[http] failed to request metrics from agent {}: {}", id, e);
                        Err(StatusCode::INTERNAL_SERVER_ERROR)
                    }
                }
            }
        }
        None => Err(StatusCode::NOT_FOUND),
    }
}

// GET /agents/{id}/commands - Liste des commandes en cours pour un agent
pub(super) async fn agent_commands_endpoint(
    State(app): State<AppState>,
    Path(id): Path<String>,
) -> Result<Json<serde_json::Value>, StatusCode> {
    let commands = app.agents.get_agent_pending_commands(&id).await;
    Ok(Json(serde_json::json!({
        "agent_id": id,
        "pending_commands": commands
    })))
}

// POST /agents/{id}/commands - Nouvelle API avec tracking pour exécuter des commandes
pub(super) async fn agent_commands_post_endpoint(
    State(app): State<AppState>,
    Path(id): Path<String>,
    Json(req): Json<AgentCommandTrackingRequest>,
) -> Result<Json<serde_json::Value>, StatusCode> {
    // Extract command from parameters for shell_command type
    if req.command_type == "shell_command" {
        if let Some(command) = req.parameters.get("command") {
            if let Some(_command_str) = command.as_str() {
                match app.agents.send_command(&id, "run_command", Some(req.parameters)).await {
                    Ok(command_id) => Ok(Json(serde_json::json!({
                        "success": true,
                        "command_id": command_id,
                        "message": "Command execution requested with tracking"
                    }))),
                    Err(e) => {
                        eprintln!("[http] failed to send tracked command to agent {}: {}", id, e);
                        Err(StatusCode::INTERNAL_SERVER_ERROR)
                    }
                }
            } else {
                Err(StatusCode::BAD_REQUEST)
            }
        } else {
            Err(StatusCode::BAD_REQUEST)
        }
    } else {
        // Handle other command types in the future
        match app.agents.send_command(&id, &req.command_type, Some(req.parameters)).await {
            Ok(command_id) => Ok(Json(serde_json::json!({
                "success": true,
                "command_id": command_id,
                "message": "Command execution requested with tracking"
            }))),
            Err(e) => {
                eprintln!("[http] failed to send tracked command to agent {}: {}", id, e);
                Err(StatusCode::INTERNAL_SERVER_ERROR)
            }
        }
    }
}

// POST /commands/{command_id}/cancel - Annule une commande
pub(super) async fn cancel_command_endpoint(
    State(app): State<AppState>,
    Path(command_id): Path<String>,
) -> Result<Json<serde_json::Value>, StatusCode> {
    match app.agents.cancel_command(&command_id).await {
        Ok(cancelled) => {
            if cancelled {
                Ok(Json(serde_json::json!({
                    "success": true,
                    "command_id": command_id,
                    "message": "Command cancelled successfully"
                })))
            } else {
                Ok(Json(serde_json::json!({
                    "success": false,
                    "command_id": command_id,
                    "message": "Command cannot be cancelled (already completed or failed)"
                })))
            }
        }
        Err(e) => {
            eprintln!("[http] failed to cancel command {}: {}", command_id, e);
            Err(StatusCode::NOT_FOUND)
        }
    }
}

// GET /commands/{command_id}/status - Statut d'une commande
pub(super) async fn command_status_endpoint(
    State(app): State<AppState>,
    Path(command_id): Path<String>,
) -> Result<Json<serde_json::Value>, StatusCode> {
    match app.agents.get_command_status(&command_id).await {
        Some(command) => Ok(Json(serde_json::json!({
            "command_id": command_id,
            "status": command.status,
            "output": command.output,
            "error": command.error
        }))),
        None => Err(StatusCode::NOT_FOUND),
    }
}

// =============== PLUGIN SYSTEMCTL ENDPOINTS ===============

/// POST /v1/plugins/:name/start - Start plugin via sudo systemctl start (async, returns immediately)
pub(super) async fn start_plugin_systemctl(
    Path(name): Path<String>,
    State(_state): State<AppState>,
) -> Result<Json<serde_json::Value>, StatusCode> {
    // Validate plugin name (alphanumeric + hyphens only)
    if !name.chars().all(|c| c.is_alphanumeric() || c == '-') {
        return Err(StatusCode::BAD_REQUEST);
    }

    let service_name = format!("symbion-plugin-{}", name);
    let service_name_clone = service_name.clone();

    // Spawn systemctl in background (don't wait for completion to avoid timeout)
    tokio::spawn(async move {
        let result = tokio::process::Command::new("sudo")
            .args(&["systemctl", "start", &service_name_clone])
            .output()
            .await;

        match result {
            Ok(output) if output.status.success() => {
                eprintln!("[kernel] plugin {} start command succeeded", service_name_clone);
            }
            Ok(output) => {
                let stderr = String::from_utf8_lossy(&output.stderr);
                eprintln!("[kernel] failed to start plugin {}: {}", service_name_clone, stderr);
            }
            Err(e) => {
                eprintln!("[kernel] failed to execute systemctl start {}: {}", service_name_clone, e);
            }
        }
    });

    // Return immediately (systemctl runs in background)
    Ok(Json(serde_json::json!({
        "status": "accepted",
        "message": format!("Start command sent for plugin {}", name),
        "service": service_name
    })))
}

/// POST /v1/plugins/:name/stop - Stop plugin via sudo systemctl stop (async, returns immediately)
pub(super) async fn stop_plugin_systemctl(
    Path(name): Path<String>,
    State(_state): State<AppState>,
) -> Result<Json<serde_json::Value>, StatusCode> {
    // Validate plugin name (alphanumeric + hyphens only)
    if !name.chars().all(|c| c.is_alphanumeric() || c == '-') {
        return Err(StatusCode::BAD_REQUEST);
    }

    let service_name = format!("symbion-plugin-{}", name);
    let service_name_clone = service_name.clone();

    // Spawn systemctl in background (don't wait for completion to avoid timeout)
    tokio::spawn(async move {
        let result = tokio::process::Command::new("sudo")
            .args(&["systemctl", "stop", &service_name_clone])
            .output()
            .await;

        match result {
            Ok(output) if output.status.success() => {
                eprintln!("[kernel] plugin {} stop command succeeded", service_name_clone);
            }
            Ok(output) => {
                let stderr = String::from_utf8_lossy(&output.stderr);
                eprintln!("[kernel] failed to stop plugin {}: {}", service_name_clone, stderr);
            }
            Err(e) => {
                eprintln!("[kernel] failed to execute systemctl stop {}: {}", service_name_clone, e);
            }
        }
    });

    // Return immediately (systemctl runs in background)
    Ok(Json(serde_json::json!({
        "status": "accepted",
        "message": format!("Stop command sent for plugin {}", name),
        "service": service_name
    })))
}

/// POST /v1/plugins/:name/restart - Restart plugin via sudo systemctl restart (async, returns immediately)
pub(super) async fn restart_plugin_systemctl(
    Path(name): Path<String>,
    State(_state): State<AppState>,
) -> Result<Json<serde_json::Value>, StatusCode> {
    // Validate plugin name (alphanumeric + hyphens only)
    if !name.chars().all(|c| c.is_alphanumeric() || c == '-') {
        return Err(StatusCode::BAD_REQUEST);
    }

    let service_name = format!("symbion-plugin-{}", name);
    let service_name_clone = service_name.clone();

    // Spawn systemctl in background (don't wait for completion to avoid timeout)
    tokio::spawn(async move {
        let result = tokio::process::Command::new("sudo")
            .args(&["systemctl", "restart", &service_name_clone])
            .output()
            .await;

        match result {
            Ok(output) if output.status.success() => {
                eprintln!("[kernel] plugin {} restart command succeeded", service_name_clone);
            }
            Ok(output) => {
                let stderr = String::from_utf8_lossy(&output.stderr);
                eprintln!("[kernel] failed to restart plugin {}: {}", service_name_clone, stderr);
            }
            Err(e) => {
                eprintln!("[kernel] failed to execute systemctl restart {}: {}", service_name_clone, e);
            }
        }
    });

    // Return immediately (systemctl runs in background)
    Ok(Json(serde_json::json!({
        "status": "accepted",
        "message": format!("Restart command sent for plugin {}", name),
        "service": service_name
    })))
}

/// GET /v1/plugins/:name/status - Get plugin status via systemctl --user is-active
pub(super) async fn get_plugin_systemctl_status(
    Path(name): Path<String>,
    State(_state): State<AppState>,
) -> Result<Json<serde_json::Value>, StatusCode> {
    if !name.chars().all(|c| c.is_alphanumeric() || c == '-') {
        return Err(StatusCode::BAD_REQUEST);
    }

    let service_name = format!("symbion-plugin-{}", name);

    let output = tokio::process::Command::new("systemctl")
        .args(&["--user", "is-active", &service_name])
        .output()
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    let status = String::from_utf8_lossy(&output.stdout).trim().to_string();
    let is_active = output.status.success();

    Ok(Json(serde_json::json!({
        "service": service_name,
        "status": status,
        "is_active": is_active
    })))
}
