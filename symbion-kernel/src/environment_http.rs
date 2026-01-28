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
/// Note: DELETE /sensors/{sensor_id} is handled in http.rs with CSRF protection
pub fn build_environment_routes(state: AppState) -> Router {
    Router::new()
        .route("/sensors", get(list_sensors))
        .route("/sensors/{sensor_id}", get(get_sensor))
        .route("/sensors/{sensor_id}/history", get(get_sensor_history))
        .route("/{room_id}", get(get_room_environment))
        .route("/{room_id}/history", get(get_room_history))
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

/// GET /v1/environment/:room_id
/// Get current environment state for a specific room (e.g., "chambre", "salon")
/// Returns the most recent reading from sensors in that room
async fn get_room_environment(
    State(app): State<AppState>,
    Path(room_id): Path<String>,
) -> Result<Json<RoomEnvironmentState>, StatusCode> {
    let environment = app
        .sensors
        .get_environment_by_room(&room_id)
        .ok_or(StatusCode::NOT_FOUND)?;

    Ok(Json(environment))
}

/// GET /v1/environment/:room_id/history?hours=24
/// Get historical environment readings for a room, filtered by hours
async fn get_room_history(
    State(app): State<AppState>,
    Path(room_id): Path<String>,
    Query(params): Query<HistoryQuery>,
) -> Result<Json<Vec<crate::environment::EnvReading>>, StatusCode> {
    let environment = app
        .sensors
        .get_environment_by_room(&room_id)
        .ok_or(StatusCode::NOT_FOUND)?;

    // Filter history by hours parameter
    let filtered_history = environment.get_history(params.hours);

    Ok(Json(filtered_history))
}
