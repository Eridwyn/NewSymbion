use super::AppState;
use axum::extract::{Path, State};
use axum::http::StatusCode;
use axum::Json;
use serde::Deserialize;
use time::format_description::well_known::Rfc3339;
use utoipa::ToSchema;

// ====== AGENTS ENDPOINTS ======

/// Serializable view of an agent for API responses.
#[derive(serde::Serialize, ToSchema)]
pub(crate) struct AgentView {
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
    version: Option<String>,
    uptime_seconds: Option<u64>,
    cpu_percent: Option<f32>,
    memory_percent: Option<f32>,
}

/// Request body for sending a shell command to an agent.
#[derive(Deserialize, ToSchema)]
pub(crate) struct AgentCommandRequest {
    command: String,
    #[schema(value_type = Object)]
    parameters: Option<serde_json::Value>,
}

/// Request body for sending a tracked command to an agent.
#[derive(Deserialize, ToSchema)]
pub(crate) struct AgentCommandTrackingRequest {
    command_type: String,
    #[schema(value_type = Object)]
    parameters: serde_json::Value,
}

/// Convert an internal Agent model into an AgentView for API serialization.
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
        version: agent.version.clone(),
        uptime_seconds: agent.status.system.as_ref().map(|s| s.uptime_seconds),
        cpu_percent: agent.status.system.as_ref().map(|s| s.cpu.percent),
        memory_percent: agent.status.system.as_ref().map(|s| s.memory.percent_used),
    }
}

/// GET /v1/agents -- List all registered agents with their current status.
#[utoipa::path(
    get,
    path = "/agents",
    tag = "Agents",
    security(("bearer_auth" = [])),
    responses(
        (status = 200, description = "List of all registered agents", body = Vec<AgentView>)
    )
)]
pub(super) async fn list_agents_endpoint(State(app): State<AppState>) -> Json<Vec<AgentView>> {
    let agents = app.agents.list_agents().await;
    let list: Vec<AgentView> = agents.values().map(agent_to_view).collect();
    Json(list)
}

/// GET /v1/agents/latest-version -- Return the highest version reported by connected agents.
pub(super) async fn agents_latest_version(State(app): State<AppState>) -> Json<serde_json::Value> {
    let agents = app.agents.list_agents().await;
    let latest = agents.values()
        .filter(|a| a.deleted_at.is_none())
        .filter_map(|a| a.version.as_deref())
        .max_by(|a, b| version_cmp(a, b));
    Json(serde_json::json!({
        "version": latest
    }))
}

/// Compare two semver-like version strings (e.g. "1.2.7" vs "1.2.10").
fn version_cmp(a: &str, b: &str) -> std::cmp::Ordering {
    let parse = |s: &str| -> Vec<u64> {
        s.split('.').filter_map(|p| p.parse().ok()).collect()
    };
    parse(a).cmp(&parse(b))
}

/// GET /v1/agents/{id} -- Return full details of a single agent by ID.
#[utoipa::path(
    get,
    path = "/agents/{id}",
    tag = "Agents",
    security(("bearer_auth" = [])),
    params(
        ("id" = String, Path, description = "Agent ID")
    ),
    responses(
        (status = 200, description = "Agent details"),
        (status = 404, description = "Agent not found")
    )
)]
pub(super) async fn get_agent_endpoint(
    State(app): State<AppState>,
    Path(id): Path<String>,
) -> Result<Json<crate::agents::Agent>, StatusCode> {
    match app.agents.get_agent(&id).await {
        Some(agent) => Ok(Json(agent)),
        None => Err(StatusCode::NOT_FOUND),
    }
}

/// DELETE /v1/agents/{id} -- Soft-delete an agent (purged after 7 days).
#[utoipa::path(
    delete,
    path = "/v1/agents/{id}",
    tag = "Agents",
    security(("bearer_auth" = [])),
    params(
        ("id" = String, Path, description = "Agent ID"),
        ("X-CSRF-Token" = String, Header, description = "CSRF nonce")
    ),
    responses(
        (status = 204, description = "Agent soft-deleted"),
        (status = 404, description = "Agent not found or already deleted"),
        (status = 500, description = "Internal server error")
    )
)]
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

/// DELETE /v1/environment/sensors/{sensor_id} -- Soft-delete a sensor (purged after 7 days).
#[utoipa::path(
    delete,
    path = "/environment/sensors/{sensor_id}",
    tag = "Agents",
    security(("bearer_auth" = [])),
    params(
        ("sensor_id" = String, Path, description = "Sensor ID"),
        ("X-CSRF-Token" = String, Header, description = "CSRF nonce")
    ),
    responses(
        (status = 204, description = "Sensor soft-deleted"),
        (status = 500, description = "Internal server error")
    )
)]
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

/// POST /v1/agents/{id}/shutdown -- Send a shutdown command to the specified agent.
#[utoipa::path(
    post,
    path = "/agents/{id}/shutdown",
    tag = "Agents",
    security(("bearer_auth" = [])),
    params(
        ("id" = String, Path, description = "Agent ID"),
        ("X-CSRF-Token" = String, Header, description = "CSRF nonce")
    ),
    responses(
        (status = 200, description = "Shutdown command sent"),
        (status = 500, description = "Failed to send shutdown command")
    )
)]
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

/// POST /v1/agents/{id}/reboot -- Send a reboot command to the specified agent.
#[utoipa::path(
    post,
    path = "/agents/{id}/reboot",
    tag = "Agents",
    security(("bearer_auth" = [])),
    params(
        ("id" = String, Path, description = "Agent ID"),
        ("X-CSRF-Token" = String, Header, description = "CSRF nonce")
    ),
    responses(
        (status = 200, description = "Reboot command sent"),
        (status = 500, description = "Failed to send reboot command")
    )
)]
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

/// POST /v1/agents/{id}/hibernate -- Send a hibernate (sleep) command to the specified agent.
#[utoipa::path(
    post,
    path = "/agents/{id}/hibernate",
    tag = "Agents",
    security(("bearer_auth" = [])),
    params(
        ("id" = String, Path, description = "Agent ID"),
        ("X-CSRF-Token" = String, Header, description = "CSRF nonce")
    ),
    responses(
        (status = 200, description = "Hibernate command sent"),
        (status = 500, description = "Failed to send hibernate command")
    )
)]
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

/// POST /v1/agents/{id}/reconnect -- Send a reconnect command to the specified agent via MQTT.
#[utoipa::path(
    post,
    path = "/v1/agents/{id}/reconnect",
    tag = "Agents",
    security(("bearer_auth" = [])),
    params(
        ("id" = String, Path, description = "Agent ID"),
        ("X-CSRF-Token" = String, Header, description = "CSRF nonce")
    ),
    responses(
        (status = 200, description = "Reconnect command sent"),
        (status = 404, description = "Agent not found"),
        (status = 500, description = "Failed to send reconnect command")
    )
)]
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

/// GET /v1/agents/{id}/processes -- Return the process list for an agent, or request it via MQTT.
#[utoipa::path(
    get,
    path = "/agents/{id}/processes",
    tag = "Agents",
    security(("bearer_auth" = [])),
    params(
        ("id" = String, Path, description = "Agent ID")
    ),
    responses(
        (status = 200, description = "Process list or request acknowledgement"),
        (status = 404, description = "Agent not found"),
        (status = 500, description = "Failed to request processes")
    )
)]
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

/// POST /v1/agents/{id}/processes/{pid}/kill -- Kill a specific process on the agent by PID.
#[utoipa::path(
    post,
    path = "/agents/{id}/processes/{pid}/kill",
    tag = "Agents",
    security(("bearer_auth" = [])),
    params(
        ("id" = String, Path, description = "Agent ID"),
        ("pid" = u32, Path, description = "Process ID to kill"),
        ("X-CSRF-Token" = String, Header, description = "CSRF nonce")
    ),
    responses(
        (status = 200, description = "Kill process command sent"),
        (status = 500, description = "Failed to send kill process command")
    )
)]
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

/// POST /v1/agents/{id}/command -- Execute a shell command on the specified agent.
#[utoipa::path(
    post,
    path = "/agents/{id}/command",
    tag = "Agents",
    security(("bearer_auth" = [])),
    params(
        ("id" = String, Path, description = "Agent ID"),
        ("X-CSRF-Token" = String, Header, description = "CSRF nonce")
    ),
    request_body = AgentCommandRequest,
    responses(
        (status = 200, description = "Command execution requested"),
        (status = 400, description = "Command too long"),
        (status = 500, description = "Failed to send command")
    )
)]
pub(super) async fn agent_command_endpoint(
    State(app): State<AppState>,
    Path(id): Path<String>,
    Json(req): Json<AgentCommandRequest>,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<serde_json::Value>)> {
    // P1: Validate command length (max 1000 chars)
    if req.command.len() > 1000 {
        return Err((StatusCode::BAD_REQUEST, Json(serde_json::json!({
            "error": "Command too long (max 1000 characters)"
        }))));
    }

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
            Err((StatusCode::INTERNAL_SERVER_ERROR, Json(serde_json::json!({
                "error": format!("Failed to send command: {}", e)
            }))))
        }
    }
}

/// GET /v1/agents/{id}/metrics -- Return real-time system metrics for an agent, or request them via MQTT.
#[utoipa::path(
    get,
    path = "/agents/{id}/metrics",
    tag = "Agents",
    security(("bearer_auth" = [])),
    params(
        ("id" = String, Path, description = "Agent ID")
    ),
    responses(
        (status = 200, description = "Agent system metrics or request acknowledgement"),
        (status = 404, description = "Agent not found"),
        (status = 500, description = "Failed to request metrics")
    )
)]
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

/// GET /v1/agents/{id}/commands -- List pending commands for the specified agent.
#[utoipa::path(
    get,
    path = "/agents/{id}/commands",
    tag = "Agents",
    security(("bearer_auth" = [])),
    params(
        ("id" = String, Path, description = "Agent ID")
    ),
    responses(
        (status = 200, description = "Pending commands for the agent")
    )
)]
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

/// POST /v1/agents/{id}/commands -- Submit a tracked command for execution on the agent.
#[utoipa::path(
    post,
    path = "/agents/{id}/commands",
    tag = "Agents",
    security(("bearer_auth" = [])),
    params(
        ("id" = String, Path, description = "Agent ID"),
        ("X-CSRF-Token" = String, Header, description = "CSRF nonce")
    ),
    request_body = AgentCommandTrackingRequest,
    responses(
        (status = 200, description = "Command execution requested with tracking"),
        (status = 400, description = "Bad request or command too long"),
        (status = 500, description = "Failed to send tracked command")
    )
)]
pub(super) async fn agent_commands_post_endpoint(
    State(app): State<AppState>,
    Path(id): Path<String>,
    Json(req): Json<AgentCommandTrackingRequest>,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<serde_json::Value>)> {
    // Extract command from parameters for shell_command type
    if req.command_type == "shell_command" {
        if let Some(command) = req.parameters.get("command") {
            if let Some(command_str) = command.as_str() {
                // P1: Validate command length (max 1000 chars)
                if command_str.len() > 1000 {
                    return Err((StatusCode::BAD_REQUEST, Json(serde_json::json!({
                        "error": "Command too long (max 1000 characters)"
                    }))));
                }
                match app.agents.send_command(&id, "run_command", Some(req.parameters)).await {
                    Ok(command_id) => Ok(Json(serde_json::json!({
                        "success": true,
                        "command_id": command_id,
                        "message": "Command execution requested with tracking"
                    }))),
                    Err(e) => {
                        eprintln!("[http] failed to send tracked command to agent {}: {}", id, e);
                        Err((StatusCode::INTERNAL_SERVER_ERROR, Json(serde_json::json!({
                            "error": format!("Failed to send command: {}", e)
                        }))))
                    }
                }
            } else {
                Err((StatusCode::BAD_REQUEST, Json(serde_json::json!({
                    "error": "Parameter 'command' must be a string"
                }))))
            }
        } else {
            Err((StatusCode::BAD_REQUEST, Json(serde_json::json!({
                "error": "Missing 'command' in parameters"
            }))))
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
                Err((StatusCode::INTERNAL_SERVER_ERROR, Json(serde_json::json!({
                    "error": format!("Failed to send command: {}", e)
                }))))
            }
        }
    }
}

/// POST /v1/commands/{command_id}/cancel -- Cancel a pending or in-progress command.
#[utoipa::path(
    post,
    path = "/commands/{command_id}/cancel",
    tag = "Agents",
    security(("bearer_auth" = [])),
    params(
        ("command_id" = String, Path, description = "Command ID"),
        ("X-CSRF-Token" = String, Header, description = "CSRF nonce")
    ),
    responses(
        (status = 200, description = "Command cancellation result"),
        (status = 404, description = "Command not found")
    )
)]
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

/// GET /v1/commands/{command_id}/status -- Return the current status and output of a command.
#[utoipa::path(
    get,
    path = "/commands/{command_id}/status",
    tag = "Agents",
    security(("bearer_auth" = [])),
    params(
        ("command_id" = String, Path, description = "Command ID")
    ),
    responses(
        (status = 200, description = "Command status and output"),
        (status = 404, description = "Command not found")
    )
)]
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
#[utoipa::path(
    post,
    path = "/v1/plugins/{name}/start",
    tag = "Agents",
    security(("bearer_auth" = [])),
    params(
        ("name" = String, Path, description = "Plugin name"),
        ("X-CSRF-Token" = String, Header, description = "CSRF nonce")
    ),
    responses(
        (status = 200, description = "Start command accepted"),
        (status = 400, description = "Invalid plugin name")
    )
)]
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
#[utoipa::path(
    post,
    path = "/v1/plugins/{name}/stop",
    tag = "Agents",
    security(("bearer_auth" = [])),
    params(
        ("name" = String, Path, description = "Plugin name"),
        ("X-CSRF-Token" = String, Header, description = "CSRF nonce")
    ),
    responses(
        (status = 200, description = "Stop command accepted"),
        (status = 400, description = "Invalid plugin name")
    )
)]
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
#[utoipa::path(
    post,
    path = "/v1/plugins/{name}/restart",
    tag = "Agents",
    security(("bearer_auth" = [])),
    params(
        ("name" = String, Path, description = "Plugin name"),
        ("X-CSRF-Token" = String, Header, description = "CSRF nonce")
    ),
    responses(
        (status = 200, description = "Restart command accepted"),
        (status = 400, description = "Invalid plugin name")
    )
)]
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
#[utoipa::path(
    get,
    path = "/v1/plugins/{name}/status",
    tag = "Agents",
    security(("bearer_auth" = [])),
    params(
        ("name" = String, Path, description = "Plugin name")
    ),
    responses(
        (status = 200, description = "Plugin systemctl status"),
        (status = 400, description = "Invalid plugin name"),
        (status = 500, description = "Failed to execute systemctl")
    )
)]
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
