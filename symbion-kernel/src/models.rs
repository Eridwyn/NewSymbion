/**
 * MODÈLES DE DONNÉES - Structures centrales du kernel Symbion
 * 
 * RÔLE : Définit les structures de données partagées entre tous les modules.
 * Types principaux : HeartbeatIn (MQTT), HostState (monitoring), HostsMap (collection).
 * 
 * UTILITÉ : Cohérence des données, sérialisation JSON/YAML, typage fort.
 */

use serde::{Deserialize};
use std::collections::HashMap;
use time::OffsetDateTime;

#[derive(Debug, Clone)]
pub struct HostState {
    pub host_id: String,
    pub last_seen: OffsetDateTime,
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
