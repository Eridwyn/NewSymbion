/**
 * NOTES BRIDGE - Pont API REST ↔ Plugin Notes via MQTT
 * 
 * RÔLE :
 * Maintient la compatibilité de l'API REST `/ports/memo` en redirigeant
 * les requêtes vers le plugin notes via MQTT. Migration transparente.
 * 
 * FONCTIONNEMENT :
 * - Reçoit requêtes HTTP sur `/ports/memo`
 * - Traduit en commandes MQTT vers le plugin
 * - Attend les réponses MQTT du plugin  
 * - Retourne les résultats en JSON HTTP
 * 
 * UTILITÉ DANS SYMBION :
 * 🎯 Migration transparente : API identique pour l'utilisateur
 * 🎯 Découplage : Kernel ne gère plus les notes directement
 * 🎯 Evolution : Plugin peut évoluer sans casser l'API
 * 🎯 Fallback : Peut détecter si plugin indisponible
 */

use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    Json,
};
use rumqttc::{AsyncClient, QoS};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::oneshot;
use tokio::time::{timeout, Duration};
use uuid::Uuid;
use parking_lot::Mutex;

/// Structure pour les requêtes de création/modification de notes
#[derive(Debug, Deserialize, Serialize)]
pub struct CreateNoteRequest {
    pub content: String,
    pub urgent: Option<bool>,
    pub context: Option<String>, 
    pub tags: Option<Vec<String>>,
    pub status: Option<String>,
}

/// Commandes MQTT envoyées au plugin (identique au plugin)
#[derive(Debug, Serialize)]
#[serde(tag = "action")]
pub enum NoteCommand {
    #[serde(rename = "create")]
    Create { 
        request_id: String,
        note: CreateNoteRequest 
    },
    #[serde(rename = "list")]
    List { 
        request_id: String,
        filters: Option<HashMap<String, Value>>
    },
    #[serde(rename = "delete")]
    Delete { 
        request_id: String,
        id: String 
    },
    #[serde(rename = "update")]
    Update { 
        request_id: String,
        id: String,
        note: CreateNoteRequest 
    },
}

/// Réponses MQTT du plugin (identique au plugin)
#[derive(Debug, Deserialize)]
#[serde(tag = "type")]
pub enum NoteResponse {
    #[serde(rename = "success")]
    Success {
        request_id: String,
        action: String,
        data: Value,
    },
    #[serde(rename = "error")]
    Error {
        request_id: String,
        action: String,
        error: String,
    },
}

/// Gestionnaire des requêtes en attente de réponse
pub struct NotesBridge {
    /// Client MQTT pour communication avec le plugin
    mqtt_client: AsyncClient,
    /// Map des requêtes en attente : request_id -> sender pour réponse
    pending_requests: Arc<Mutex<HashMap<String, oneshot::Sender<NoteResponse>>>>,
}

impl NotesBridge {
    /// Crée un nouveau bridge notes
    pub fn new(mqtt_client: AsyncClient) -> Self {
        Self {
            mqtt_client,
            pending_requests: Arc::new(Mutex::new(HashMap::new())),
        }
    }
    
    /// Traite une réponse MQTT du plugin
    pub fn handle_response(&self, response: NoteResponse) {
        let mut pending = self.pending_requests.lock();
        
        let request_id = match &response {
            NoteResponse::Success { request_id, .. } => request_id.clone(),
            NoteResponse::Error { request_id, .. } => request_id.clone(),
        };
        
        if let Some(sender) = pending.remove(&request_id) {
            if sender.send(response).is_err() {
                eprintln!("[notes-bridge] failed to send response for request {}", request_id);
            }
        } else {
            eprintln!("[notes-bridge] received response for unknown request {}", request_id);
        }
    }
    
    /// Envoie une commande au plugin et attend la réponse
    async fn send_command(&self, command: NoteCommand) -> Result<NoteResponse, StatusCode> {
        let request_id = match &command {
            NoteCommand::Create { request_id, .. } => request_id.clone(),
            NoteCommand::List { request_id, .. } => request_id.clone(),
            NoteCommand::Delete { request_id, .. } => request_id.clone(),
            NoteCommand::Update { request_id, .. } => request_id.clone(),
        };
        
        // Créer le canal pour la réponse
        let (tx, rx) = oneshot::channel();
        self.pending_requests.lock().insert(request_id.clone(), tx);
        
        // Sérialiser et envoyer la commande
        let payload = serde_json::to_string(&command)
            .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
        
        self.mqtt_client
            .publish("symbion/notes/command@v1", QoS::AtLeastOnce, false, payload)
            .await
            .map_err(|_| StatusCode::SERVICE_UNAVAILABLE)?;
        
        // Attendre la réponse avec timeout
        match timeout(Duration::from_secs(5), rx).await {
            Ok(Ok(response)) => Ok(response),
            Ok(Err(_)) => {
                // Canal fermé
                self.pending_requests.lock().remove(&request_id);
                Err(StatusCode::INTERNAL_SERVER_ERROR)
            }
            Err(_) => {
                // Timeout
                self.pending_requests.lock().remove(&request_id);
                Err(StatusCode::GATEWAY_TIMEOUT)
            }
        }
    }
}

/// Bridge state partagé dans Axum
pub type SharedNotesBridge = Arc<NotesBridge>;

// ============ ENDPOINTS API REST ============

/// GET /ports/memo - Liste les notes (compatible avec l'ancien port)
pub async fn list_notes_endpoint(
    State(bridge): State<SharedNotesBridge>,
    Query(params): Query<HashMap<String, String>>,
) -> Result<Json<Value>, StatusCode> {
    let request_id = Uuid::new_v4().to_string();
    
    // Convertir les paramètres de requête en filtres
    let mut filters = HashMap::new();
    if let Some(urgent) = params.get("urgent") {
        if let Ok(urgent_bool) = urgent.parse::<bool>() {
            filters.insert("urgent".to_string(), Value::Bool(urgent_bool));
        }
    }
    if let Some(context) = params.get("context") {
        filters.insert("context".to_string(), Value::String(context.clone()));
    }
    if let Some(tags) = params.get("tags") {
        let tag_list: Vec<String> = tags.split(',').map(|s| s.trim().to_string()).collect();
        filters.insert("tags".to_string(), Value::Array(
            tag_list.into_iter().map(Value::String).collect()
        ));
    }
    
    let command = NoteCommand::List {
        request_id,
        filters: if filters.is_empty() { None } else { Some(filters) },
    };
    
    match bridge.send_command(command).await? {
        NoteResponse::Success { data, .. } => Ok(Json(data)),
        NoteResponse::Error { error, .. } => {
            eprintln!("[notes-bridge] list error: {}", error);
            Err(StatusCode::INTERNAL_SERVER_ERROR)
        }
    }
}

/// POST /ports/memo - Crée une note (compatible avec l'ancien port)
pub async fn create_note_endpoint(
    State(bridge): State<SharedNotesBridge>,
    Json(note_data): Json<CreateNoteRequest>,
) -> Result<Json<Value>, StatusCode> {
    let request_id = Uuid::new_v4().to_string();
    
    let command = NoteCommand::Create {
        request_id,
        note: note_data,
    };
    
    match bridge.send_command(command).await? {
        NoteResponse::Success { data, .. } => Ok(Json(data)),
        NoteResponse::Error { error, .. } => {
            eprintln!("[notes-bridge] create error: {}", error);
            Err(StatusCode::INTERNAL_SERVER_ERROR)
        }
    }
}

/// DELETE /ports/memo/{id} - Supprime une note (compatible avec l'ancien port)
pub async fn delete_note_endpoint(
    State(bridge): State<SharedNotesBridge>,
    Path(id): Path<String>,
) -> Result<Json<Value>, StatusCode> {
    let request_id = Uuid::new_v4().to_string();
    
    let command = NoteCommand::Delete {
        request_id,
        id,
    };
    
    match bridge.send_command(command).await? {
        NoteResponse::Success { data, .. } => Ok(Json(data)),
        NoteResponse::Error { error, .. } => {
            if error == "Note not found" {
                Err(StatusCode::NOT_FOUND)
            } else {
                eprintln!("[notes-bridge] delete error: {}", error);
                Err(StatusCode::INTERNAL_SERVER_ERROR)
            }
        }
    }
}

/// PUT /ports/memo/{id} - Met à jour une note
pub async fn update_note_endpoint(
    State(bridge): State<SharedNotesBridge>,
    Path(id): Path<String>,
    Json(note_data): Json<CreateNoteRequest>,
) -> Result<Json<Value>, StatusCode> {
    let request_id = Uuid::new_v4().to_string();
    
    let command = NoteCommand::Update {
        request_id,
        id,
        note: note_data,
    };
    
    match bridge.send_command(command).await? {
        NoteResponse::Success { data, .. } => Ok(Json(data)),
        NoteResponse::Error { error, .. } => {
            if error == "Note not found" {
                Err(StatusCode::NOT_FOUND)
            } else {
                eprintln!("[notes-bridge] update error: {}", error);
                Err(StatusCode::INTERNAL_SERVER_ERROR)
            }
        }
    }
}