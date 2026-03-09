pub mod config;
pub mod database;
pub mod models;
pub mod mqtt;
pub mod routes;

use std::sync::Arc;

pub struct PluginState {
    pub db: Arc<database::Database>,
    pub mqtt: Arc<mqtt::MqttPublisher>,
    pub config: config::Config,
    pub started_at: std::time::Instant,
}
