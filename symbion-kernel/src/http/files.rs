//! File Transfer HTTP endpoints
//!
//! Endpoints for file management between PWA and agents.
//! File data flows over HTTPS; MQTT is used only for signaling commands.

use axum::{
    extract::{Multipart, Path, Query, State},
    http::StatusCode,
    Json,
};
use serde::Deserialize;

use super::AppState;

// ── List files on agent ──────────────────────────────────────────────

/// GET /v1/agents/{id}/files — list files in agent's transfer directory
#[utoipa::path(
    get, path = "/v1/agents/{id}/files",
    params(("id" = String, Path, description = "Agent ID")),
    responses((status = 200, description = "File list from agent")),
    tag = "File Transfer"
)]
pub(super) async fn list_agent_files(
    State(app): State<AppState>,
    Path(id): Path<String>,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<serde_json::Value>)> {
    match app.agents.send_command(&id, "list_files", None).await {
        Ok(command_id) => Ok(Json(serde_json::json!({
            "success": true,
            "command_id": command_id,
            "message": "File list requested"
        }))),
        Err(e) => Err((
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(serde_json::json!({ "error": format!("Failed to list files: {}", e) })),
        )),
    }
}

// ── Upload file to agent ─────────────────────────────────────────────

/// POST /v1/agents/{id}/files — upload a file to agent via kernel relay
#[utoipa::path(
    post, path = "/v1/agents/{id}/files/upload",
    params(("id" = String, Path, description = "Agent ID")),
    responses(
        (status = 200, description = "Upload initiated"),
        (status = 400, description = "Invalid file or too large"),
    ),
    tag = "File Transfer"
)]
pub(super) async fn upload_file_to_agent(
    State(app): State<AppState>,
    Path(id): Path<String>,
    mut multipart: Multipart,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<serde_json::Value>)> {
    let file_hub = app.file_hub.as_ref().ok_or_else(|| {
        (StatusCode::SERVICE_UNAVAILABLE, Json(serde_json::json!({ "error": "File hub not initialized" })))
    })?;

    // Extract file from multipart
    let mut filename = String::new();
    let mut data = bytes::Bytes::new();

    while let Ok(Some(field)) = multipart.next_field().await {
        if field.name() == Some("file") {
            filename = field
                .file_name()
                .unwrap_or("unnamed")
                .to_string();
            data = field.bytes().await.map_err(|e| {
                (StatusCode::BAD_REQUEST, Json(serde_json::json!({ "error": format!("Failed to read file: {}", e) })))
            })?;
            break;
        }
    }

    if filename.is_empty() || data.is_empty() {
        return Err((StatusCode::BAD_REQUEST, Json(serde_json::json!({ "error": "No file provided" }))));
    }

    // Store file in kernel and create transfer record
    let record = file_hub.create_upload(&id, &filename, data).await.map_err(|e| {
        (StatusCode::BAD_REQUEST, Json(serde_json::json!({ "error": e })))
    })?;

    // Build kernel URL for agent to pull from
    let https_port: u16 = std::env::var("SYMBION_HTTPS_PORT")
        .ok()
        .and_then(|p| p.parse().ok())
        .unwrap_or(8443);

    // Send MQTT signal to agent: "pull this file from kernel"
    let params = serde_json::json!({
        "transfer_id": record.transfer_id,
        "token": record.token,
        "filename": record.filename,
        "file_size": record.file_size,
        "sha256": record.sha256,
        "kernel_port": https_port,
    });

    match app.agents.send_command(&id, "file_pull", Some(params)).await {
        Ok(command_id) => Ok(Json(serde_json::json!({
            "success": true,
            "transfer_id": record.transfer_id,
            "command_id": command_id,
            "filename": record.filename,
            "file_size": record.file_size,
        }))),
        Err(e) => {
            // Clean up stored file on MQTT failure
            file_hub.mark_failed(&record.transfer_id, &format!("MQTT signal failed: {}", e)).await;
            Err((
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(serde_json::json!({ "error": format!("Failed to signal agent: {}", e) })),
            ))
        }
    }
}

// ── Request file download from agent ─────────────────────────────────

#[derive(Deserialize, utoipa::ToSchema)]
pub(super) struct DownloadRequest {
    filename: String,
}

/// POST /v1/agents/{id}/files/download — request a file from agent
#[utoipa::path(
    post, path = "/v1/agents/{id}/files/download",
    params(("id" = String, Path, description = "Agent ID")),
    responses((status = 200, description = "Download request initiated")),
    tag = "File Transfer"
)]
pub(super) async fn request_file_download(
    State(app): State<AppState>,
    Path(id): Path<String>,
    Json(req): Json<DownloadRequest>,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<serde_json::Value>)> {
    let file_hub = app.file_hub.as_ref().ok_or_else(|| {
        (StatusCode::SERVICE_UNAVAILABLE, Json(serde_json::json!({ "error": "File hub not initialized" })))
    })?;

    let record = file_hub.create_download_request(&id, &req.filename).await.map_err(|e| {
        (StatusCode::BAD_REQUEST, Json(serde_json::json!({ "error": e })))
    })?;

    let https_port: u16 = std::env::var("SYMBION_HTTPS_PORT")
        .ok()
        .and_then(|p| p.parse().ok())
        .unwrap_or(8443);

    // Send MQTT signal to agent: "push this file to kernel"
    let params = serde_json::json!({
        "transfer_id": record.transfer_id,
        "token": record.token,
        "filename": record.filename,
        "kernel_port": https_port,
    });

    match app.agents.send_command(&id, "file_push", Some(params)).await {
        Ok(command_id) => Ok(Json(serde_json::json!({
            "success": true,
            "transfer_id": record.transfer_id,
            "command_id": command_id,
            "filename": record.filename,
        }))),
        Err(e) => {
            file_hub.mark_failed(&record.transfer_id, &format!("MQTT signal failed: {}", e)).await;
            Err((
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(serde_json::json!({ "error": format!("Failed to signal agent: {}", e) })),
            ))
        }
    }
}

// ── Delete file on agent ─────────────────────────────────────────────

/// DELETE /v1/agents/{id}/files/{filename} — delete a file on agent
#[utoipa::path(
    delete, path = "/v1/agents/{id}/files/{filename}",
    params(
        ("id" = String, Path, description = "Agent ID"),
        ("filename" = String, Path, description = "Filename to delete"),
    ),
    responses((status = 200, description = "Delete command sent")),
    tag = "File Transfer"
)]
pub(super) async fn delete_agent_file(
    State(app): State<AppState>,
    Path((id, filename)): Path<(String, String)>,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<serde_json::Value>)> {
    let params = serde_json::json!({ "filename": filename });

    match app.agents.send_command(&id, "delete_file", Some(params)).await {
        Ok(command_id) => Ok(Json(serde_json::json!({
            "success": true,
            "command_id": command_id,
            "message": format!("Delete requested for {}", filename),
        }))),
        Err(e) => Err((
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(serde_json::json!({ "error": format!("Failed to delete: {}", e) })),
        )),
    }
}

// ── Transfer status ──────────────────────────────────────────────────

/// GET /v1/transfers/{id}/status — check transfer status (for PWA polling)
#[utoipa::path(
    get, path = "/v1/transfers/{id}/status",
    params(("id" = String, Path, description = "Transfer ID")),
    responses((status = 200, description = "Transfer status")),
    tag = "File Transfer"
)]
pub(super) async fn get_transfer_status(
    State(app): State<AppState>,
    Path(id): Path<String>,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<serde_json::Value>)> {
    let file_hub = app.file_hub.as_ref().ok_or_else(|| {
        (StatusCode::SERVICE_UNAVAILABLE, Json(serde_json::json!({ "error": "File hub not initialized" })))
    })?;

    let record = file_hub.get_status(&id).await.ok_or_else(|| {
        (StatusCode::NOT_FOUND, Json(serde_json::json!({ "error": "Transfer not found" })))
    })?;

    let mut response = serde_json::to_value(&record).unwrap_or_default();

    // Include download token for completed FromAgent transfers
    if let Some((filename, download_token)) = file_hub.get_download_info(&id).await {
        response["download_token"] = serde_json::json!(download_token);
        response["download_filename"] = serde_json::json!(filename);
    }

    Ok(Json(response))
}

// ── Transfer data (token-authenticated) ──────────────────────────────

#[derive(Deserialize)]
pub(super) struct TokenQuery {
    token: String,
}

/// GET /v1/transfers/{id}/data?token=... — download transfer file (agent or PWA)
#[utoipa::path(
    get, path = "/v1/transfers/{id}/data",
    params(
        ("id" = String, Path, description = "Transfer ID"),
        ("token" = String, Query, description = "One-time auth token"),
    ),
    responses((status = 200, description = "File data")),
    tag = "File Transfer"
)]
pub(super) async fn download_transfer_data(
    State(app): State<AppState>,
    Path(id): Path<String>,
    Query(q): Query<TokenQuery>,
) -> Result<axum::response::Response, (StatusCode, Json<serde_json::Value>)> {
    let file_hub = app.file_hub.as_ref().ok_or_else(|| {
        (StatusCode::SERVICE_UNAVAILABLE, Json(serde_json::json!({ "error": "File hub not initialized" })))
    })?;

    // For ToAgent transfers: validate with transfer token
    // For FromAgent completed transfers: validate with download token
    let record = file_hub.get_status(&id).await.ok_or_else(|| {
        (StatusCode::NOT_FOUND, Json(serde_json::json!({ "error": "Transfer not found" })))
    })?;

    // Try transfer token first (agent pulling), then download token (PWA downloading)
    let is_valid = file_hub.validate_token(&id, &q.token).await.is_ok()
        || record.download_token.as_deref() == Some(&q.token);

    if !is_valid {
        return Err((StatusCode::FORBIDDEN, Json(serde_json::json!({ "error": "Invalid or expired token" }))));
    }

    let file_path = file_hub.get_file_path(&id, &record.filename).map_err(|e| {
        (StatusCode::BAD_REQUEST, Json(serde_json::json!({ "error": format!("Invalid filename: {}", e) })))
    })?;
    let data = tokio::fs::read(&file_path).await.map_err(|e| {
        (StatusCode::NOT_FOUND, Json(serde_json::json!({ "error": format!("File not found: {}", e) })))
    })?;

    use axum::response::IntoResponse;
    let headers = [
        (axum::http::header::CONTENT_TYPE, "application/octet-stream".to_string()),
        (
            axum::http::header::CONTENT_DISPOSITION,
            format!("attachment; filename=\"{}\"", record.filename),
        ),
        (axum::http::header::CONTENT_LENGTH, data.len().to_string()),
    ];

    Ok((headers, data).into_response())
}

/// POST /v1/transfers/{id}/data?token=... — agent pushes file to kernel
pub(super) async fn upload_transfer_data(
    State(app): State<AppState>,
    Path(id): Path<String>,
    Query(q): Query<TokenQuery>,
    body: bytes::Bytes,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<serde_json::Value>)> {
    let file_hub = app.file_hub.as_ref().ok_or_else(|| {
        (StatusCode::SERVICE_UNAVAILABLE, Json(serde_json::json!({ "error": "File hub not initialized" })))
    })?;

    file_hub.store_agent_file(&id, &q.token, body).await.map_err(|e| {
        (StatusCode::BAD_REQUEST, Json(serde_json::json!({ "error": e })))
    })?;

    Ok(Json(serde_json::json!({
        "success": true,
        "transfer_id": id,
        "message": "File received",
    })))
}
