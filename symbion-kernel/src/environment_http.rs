/**
 * SYMBION KERNEL - Environment API Endpoints (F1)
 *
 * RÔLE : Endpoints HTTP pour gestion sensors environnementaux IoT
 *
 * ENDPOINTS :
 * - GET    /v1/environment/sensors              → List all sensors
 * - GET    /v1/environment/sensors/{sensor_id}  → Get sensor info
 * - GET    /v1/environment/sensors/{sensor_id}/history → Get environment history
 * - DELETE /v1/environment/sensors/{sensor_id}  → Unregister sensor
 */

use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    routing::{delete, get},
    Json, Router,
};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

use crate::environment::RoomEnvironmentState;
use crate::http::AppState;
use crate::sensors::Sensor;

/// Response for list sensors endpoint
#[derive(Debug, Serialize)]
pub struct SensorsListResponse {
    pub sensors: Vec<Sensor>,
    pub count: usize,
    pub online_count: usize,
}

/// Response for sensor detail endpoint
#[derive(Debug, Serialize)]
pub struct SensorDetailResponse {
    pub sensor: Sensor,
    pub environment: Option<RoomEnvironmentState>,
}

/// Query parameters for history endpoint
#[derive(Debug, Deserialize)]
pub struct HistoryQuery {
    #[serde(default = "default_hours")]
    pub hours: u32,
}

fn default_hours() -> u32 {
    24
}

/// Build environment routes
pub fn build_environment_routes(state: AppState) -> Router {
    Router::new()
        .route("/sensors", get(list_sensors))
        .route("/sensors/:sensor_id", get(get_sensor))
        .route("/sensors/:sensor_id", delete(unregister_sensor))
        .route("/sensors/:sensor_id/history", get(get_sensor_history))
        .with_state(state)
}

/// GET /v1/environment/sensors
/// List all registered sensors
async fn list_sensors(
    State(app): State<AppState>,
) -> Result<Json<SensorsListResponse>, StatusCode> {
    let sensors = app.sensors.list_sensors();
    let count = sensors.len();
    let online_count = app.sensors.online_sensor_count();

    Ok(Json(SensorsListResponse {
        sensors,
        count,
        online_count,
    }))
}

/// GET /v1/environment/sensors/:sensor_id
/// Get sensor details with current environment state
async fn get_sensor(
    State(app): State<AppState>,
    Path(sensor_id): Path<String>,
) -> Result<Json<SensorDetailResponse>, StatusCode> {
    let sensor = app
        .sensors
        .get_sensor(&sensor_id)
        .ok_or(StatusCode::NOT_FOUND)?;

    let environment = app.sensors.get_environment(&sensor_id);

    Ok(Json(SensorDetailResponse {
        sensor,
        environment,
    }))
}

/// GET /v1/environment/sensors/:sensor_id/history?hours=24
/// Get environment reading history for sensor
async fn get_sensor_history(
    State(app): State<AppState>,
    Path(sensor_id): Path<String>,
    Query(params): Query<HistoryQuery>,
) -> Result<Json<RoomEnvironmentState>, StatusCode> {
    let environment = app
        .sensors
        .get_environment(&sensor_id)
        .ok_or(StatusCode::NOT_FOUND)?;

    // Note: RoomEnvironmentState already contains full history
    // The hours parameter would require filtering in the future
    // For now, return full state (client can filter by timestamp)

    Ok(Json(environment))
}

/// DELETE /v1/environment/sensors/:sensor_id
/// Unregister a sensor (manual removal)
async fn unregister_sensor(
    State(app): State<AppState>,
    Path(sensor_id): Path<String>,
) -> Result<StatusCode, StatusCode> {
    app.sensors
        .unregister_sensor(&sensor_id)
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    Ok(StatusCode::NO_CONTENT)
}
