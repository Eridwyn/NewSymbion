use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct HostState {
    pub host_id: String,
    pub last_seen: String,
    pub cpu: Option<f32>,
    pub ram: Option<f32>,
    pub ip: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct HeartbeatIn {
    pub host_id: String,
    pub ts: String,
    pub metrics: Metrics,
    pub net: NetInfo,
}
#[derive(Debug, Deserialize)]
pub struct Metrics { pub cpu: f32, pub ram: f32 }
#[derive(Debug, Deserialize)]
pub struct NetInfo { pub ip: String }

pub type HostsMap = HashMap<String, HostState>;
