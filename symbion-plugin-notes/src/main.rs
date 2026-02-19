/**
 * SYMBION PLUGIN NOTES - Service distribué de gestion des notes
 *
 * RÔLE :
 * Plugin autonome qui gère les notes/mémos/rappels.
 * Conforme au Plugin Contract v1.0.
 *
 * FONCTIONNEMENT :
 * - Stockage JSON local (./notes.json)
 * - Actions via HTTP POST /actions (ACK synchrone)
 * - Events via MQTT symbion/plugins/notes/events
 * - Health via MQTT heartbeat + HTTP /health
 *
 * CONTRACT v1.0 :
 * - Actions: create_note, update_note, delete_note, list_notes
 * - Events: note_created, note_updated, note_deleted
 * - Manifest publié sur symbion/plugins/notes/manifest
 * - Heartbeat sur symbion/plugins/notes/health (30s)
 */

use rumqttc::{AsyncClient, Event, Incoming, MqttOptions, QoS};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;
use time::OffsetDateTime;
use time::format_description::well_known::Rfc3339;
use tokio::time::Duration;
use uuid::Uuid;
use parking_lot::Mutex;
use std::sync::Arc;
use axum::{
    extract::State,
    http::StatusCode,
    response::Json,
    routing::{get, post},
    Router,
};
use symbion_plugin_common::PluginHttpServer;
use tokio::signal::unix::{signal, SignalKind};
use tokio::sync::broadcast;

// ============================================================================
// CONTRACT v1.0 STRUCTURES
// ============================================================================

const SPEC_VERSION: &str = "1.0";
const PLUGIN_ID: &str = "notes";

/// Action request from Kernel (HTTP POST /actions)
#[derive(Debug, Clone, Deserialize)]
pub struct ActionRequest {
    pub spec_version: String,
    pub action_id: Uuid,
    pub action_type: String,
    pub payload: serde_json::Value,
    #[serde(default)]
    pub metadata: serde_json::Value,
}

/// Action response to Kernel (HTTP response = ACK)
#[derive(Debug, Clone, Serialize)]
pub struct ActionResponse {
    pub spec_version: String,
    pub action_id: Uuid,
    pub status: ActionStatus,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub result: Option<serde_json::Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<ActionError>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub execution_time_ms: Option<u64>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum ActionStatus {
    Success,
    Error,
    Rejected,
}

#[derive(Debug, Clone, Serialize)]
pub struct ActionError {
    pub code: String,
    pub message: String,
    pub retryable: bool,
}

/// Event message to Kernel (MQTT)
#[derive(Debug, Clone, Serialize)]
pub struct EventMessage {
    pub spec_version: String,
    pub event_type: String,
    pub plugin_id: String,
    pub payload: serde_json::Value,
    pub timestamp: String,
}

impl EventMessage {
    pub fn new(event_type: &str, payload: serde_json::Value) -> Self {
        Self {
            spec_version: SPEC_VERSION.to_string(),
            event_type: event_type.to_string(),
            plugin_id: PLUGIN_ID.to_string(),
            payload,
            timestamp: OffsetDateTime::now_utc()
                .format(&Rfc3339)
                .unwrap_or_else(|_| "unknown".to_string()),
        }
    }
}

/// Health status for MQTT heartbeat
#[derive(Debug, Clone, Serialize)]
pub struct HealthStatus {
    pub spec_version: String,
    pub plugin_id: String,
    pub status: String,
    pub uptime_seconds: u64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub last_action_at: Option<String>,
}

// ============================================================================
// MQTT TOPICS (Contract v1.0)
// ============================================================================

mod topics {
    pub const MANIFEST: &str = "symbion/plugins/notes/manifest";
    pub const EVENTS: &str = "symbion/plugins/notes/events";
    pub const HEALTH: &str = "symbion/plugins/notes/health";
    // Legacy topic for backward compatibility during migration
    pub const LEGACY_COMMAND: &str = "symbion/notes/command@v1";
}

/// Structure des données de note (identique au kernel)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NoteContent {
    /// Texte principal de la note
    pub content: String,
    /// Priorité/urgence (false par défaut)
    pub urgent: Option<bool>,
    /// Contexte Symbion (cravate, intime, neutre)
    pub context: Option<String>,
    /// Tags libres pour classification
    pub tags: Option<Vec<String>>,
    /// Statut de la note (pending, done, archived)
    pub status: Option<String>,
}

/// Structure complète d'une note avec métadonnées
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Note {
    /// ID unique de la note
    pub id: String,
    /// Timestamp de création
    pub timestamp: OffsetDateTime,
    /// Données de la note
    pub data: NoteContent,
    /// Métadonnées additionnelles
    pub metadata: HashMap<String, serde_json::Value>,
}

/// Commandes MQTT pour les opérations sur les notes
#[derive(Debug, Deserialize)]
#[serde(tag = "action")]
pub enum NoteCommand {
    #[serde(rename = "create")]
    Create { 
        request_id: String,
        note: NoteContent 
    },
    #[serde(rename = "list")]
    List { 
        request_id: String,
        filters: Option<HashMap<String, serde_json::Value>>
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
        note: NoteContent 
    },
}

/// Réponses MQTT pour les résultats d'opérations
#[derive(Debug, Serialize)]
#[serde(tag = "type")]
pub enum NoteResponse {
    #[serde(rename = "success")]
    Success {
        request_id: String,
        action: String,
        data: serde_json::Value,
    },
    #[serde(rename = "error")]
    Error {
        request_id: String,
        action: String,
        error: String,
    },
    /// Item de note individuel pour streaming (pagination)
    #[serde(rename = "note_item")]
    NoteItem {
        request_id: String,
        note: Note,
    },
    /// Marqueur de fin de stream pour list
    #[serde(rename = "list_end")]
    ListEnd {
        request_id: String,
        total_count: usize,
    },
}

/// Gestionnaire de stockage des notes (similaire au port memo)
#[derive(Debug, Clone)]
pub struct NotesStorage {
    /// Cache mémoire des notes
    notes: Arc<Mutex<Vec<Note>>>,
    /// Chemin du fichier de stockage
    storage_path: PathBuf,
}

impl NotesStorage {
    /// Crée un nouveau gestionnaire de notes
    pub fn new<P: Into<PathBuf>>(storage_path: P) -> Result<Self, Box<dyn std::error::Error>> {
        let path = storage_path.into();
        let mut storage = NotesStorage {
            notes: Arc::new(Mutex::new(Vec::new())),
            storage_path: path,
        };
        
        // Charger les notes existantes du disque
        storage.load_from_disk()?;
        
        eprintln!("[notes] storage initialized at {:?}", storage.storage_path);
        Ok(storage)
    }
    
    /// Charge les notes depuis le fichier JSON
    fn load_from_disk(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        if !self.storage_path.exists() {
            // Créer fichier vide si inexistant
            fs::write(&self.storage_path, "[]")?;
            eprintln!("[notes] created empty storage file");
            return Ok(());
        }

        let content = fs::read_to_string(&self.storage_path)?;
        let loaded_notes: Vec<Note> = serde_json::from_str(&content)?;

        *self.notes.lock() = loaded_notes;
        eprintln!("[notes] loaded {} notes from disk", self.notes.lock().len());
        Ok(())
    }

    /// Recharge les notes depuis le disque (pour sync après modifications externes)
    pub fn reload_from_disk(&self) -> Result<(), Box<dyn std::error::Error>> {
        if !self.storage_path.exists() {
            return Ok(());
        }

        let content = fs::read_to_string(&self.storage_path)?;
        let loaded_notes: Vec<Note> = serde_json::from_str(&content)?;

        *self.notes.lock() = loaded_notes;
        eprintln!("[notes] reloaded {} notes from disk", self.notes.lock().len());
        Ok(())
    }
    
    /// Sauvegarde les notes sur disque
    fn save_to_disk(&self) -> Result<(), Box<dyn std::error::Error>> {
        let notes = self.notes.lock();
        let content = serde_json::to_string_pretty(&*notes)?;
        fs::write(&self.storage_path, content)?;
        Ok(())
    }
    
    /// Crée une nouvelle note
    pub fn create_note(&self, mut content: NoteContent) -> Result<Note, Box<dyn std::error::Error>> {
        // Normalize tags to lowercase for case-insensitive comparison
        if let Some(tags) = content.tags.as_mut() {
            *tags = tags.iter().map(|t| t.to_lowercase()).collect();
        }

        let note = Note {
            id: Uuid::new_v4().to_string(),
            timestamp: OffsetDateTime::now_utc(),
            data: content,
            metadata: HashMap::new(),
        };

        self.notes.lock().push(note.clone());
        self.save_to_disk()?;

        eprintln!("[notes] created note {}", note.id);
        Ok(note)
    }
    
    /// Liste les notes avec filtrage optionnel
    pub fn list_notes(&self, filters: Option<HashMap<String, serde_json::Value>>) -> Vec<Note> {
        // Reload depuis le disque pour avoir les dernières modifications
        if let Err(e) = self.reload_from_disk() {
            eprintln!("[notes] failed to reload from disk: {}", e);
        }

        let notes = self.notes.lock();

        if let Some(filters) = filters {
            notes.iter()
                .filter(|note| self.matches_filters(note, &filters))
                .cloned()
                .collect()
        } else {
            notes.clone()
        }
    }
    
    /// Supprime une note par ID
    pub fn delete_note(&self, id: &str) -> Result<bool, Box<dyn std::error::Error>> {
        let mut notes = self.notes.lock();
        let initial_len = notes.len();
        notes.retain(|note| note.id != id);
        
        if notes.len() < initial_len {
            drop(notes); // Libérer le verrou avant save_to_disk
            self.save_to_disk()?;
            eprintln!("[notes] deleted note {}", id);
            Ok(true)
        } else {
            Ok(false)
        }
    }
    
    /// Met à jour une note existante
    pub fn update_note(&self, id: &str, mut new_content: NoteContent) -> Result<Option<Note>, Box<dyn std::error::Error>> {
        // Normalize tags to lowercase for case-insensitive comparison
        if let Some(tags) = new_content.tags.as_mut() {
            *tags = tags.iter().map(|t| t.to_lowercase()).collect();
        }

        let mut notes = self.notes.lock();

        if let Some(note) = notes.iter_mut().find(|note| note.id == id) {
            note.data = new_content;
            // Garder timestamp original mais pouvoir ajouter last_modified
            note.metadata.insert("last_modified".to_string(),
                serde_json::to_value(OffsetDateTime::now_utc())?);

            let updated_note = note.clone();
            drop(notes); // Libérer le verrou

            self.save_to_disk()?;
            eprintln!("[notes] updated note {}", id);
            Ok(Some(updated_note))
        } else {
            Ok(None)
        }
    }
    
    /// Vérifie si une note correspond aux filtres
    fn matches_filters(&self, note: &Note, filters: &HashMap<String, serde_json::Value>) -> bool {
        for (key, value) in filters {
            match key.as_str() {
                "urgent" => {
                    if let Some(urgent) = &note.data.urgent {
                        if let Ok(filter_urgent) = serde_json::from_value::<bool>(value.clone()) {
                            if *urgent != filter_urgent {
                                return false;
                            }
                        }
                    }
                }
                "context" => {
                    if let Some(context) = &note.data.context {
                        if let Ok(filter_context) = serde_json::from_value::<String>(value.clone()) {
                            if *context != filter_context {
                                return false;
                            }
                        }
                    }
                }
                "tags" => {
                    if let Some(tags) = &note.data.tags {
                        if let Ok(filter_tags) = serde_json::from_value::<Vec<String>>(value.clone()) {
                            // Normalize filter tags to lowercase for case-insensitive comparison
                            let filter_tags_lower: Vec<String> = filter_tags.iter().map(|t| t.to_lowercase()).collect();
                            // Vérifie que tous les tags du filtre sont présents (case-insensitive)
                            if !filter_tags_lower.iter().all(|tag| tags.contains(tag)) {
                                return false;
                            }
                        }
                    }
                }
                _ => {
                    // Filtres non supportés ignorés
                }
            }
        }
        true
    }
}

// ===== HTTP Handlers =====
// Note: All legacy handlers (*_http) replaced by _v2 versions using AppState with MQTT client

/// Health check endpoint (Contract v1.0 format)
async fn health_check() -> Json<serde_json::Value> {
    use std::sync::OnceLock;
    static START_TIME: OnceLock<std::time::Instant> = OnceLock::new();
    let start = START_TIME.get_or_init(std::time::Instant::now);
    let uptime_secs = start.elapsed().as_secs();

    Json(serde_json::json!({
        "spec_version": SPEC_VERSION,
        "plugin_id": PLUGIN_ID,
        "status": "healthy",
        "uptime_seconds": uptime_secs
    }))
}

// ============================================================================
// CONTRACT v1.0 ACTION HANDLER
// ============================================================================

/// App state for routes needing MQTT client
#[derive(Clone)]
pub struct AppState {
    storage: Arc<NotesStorage>,
    mqtt_client: AsyncClient,
}

/// POST /actions - Contract v1.0 action endpoint (synchronous ACK)
#[axum::debug_handler]
async fn handle_action(
    State(state): State<AppState>,
    Json(request): Json<ActionRequest>,
) -> Json<serde_json::Value> {
    let start = std::time::Instant::now();
    eprintln!("[notes] received action: {} (id: {})", request.action_type, request.action_id);

    let response = match request.action_type.as_str() {
        "create_note" => handle_create_note(&state, &request).await,
        "update_note" => handle_update_note(&state, &request).await,
        "delete_note" => handle_delete_note(&state, &request).await,
        "list_notes" => handle_list_notes(&state, &request).await,
        _ => ActionResponse {
            spec_version: SPEC_VERSION.to_string(),
            action_id: request.action_id,
            status: ActionStatus::Rejected,
            result: None,
            error: Some(ActionError {
                code: "UNKNOWN_ACTION".to_string(),
                message: format!("Unknown action type: {}", request.action_type),
                retryable: false,
            }),
            execution_time_ms: None,
        },
    };

    let mut response = response;
    response.execution_time_ms = Some(start.elapsed().as_millis() as u64);

    eprintln!(
        "[notes] action {} completed: {:?} ({}ms)",
        request.action_id,
        response.status,
        response.execution_time_ms.unwrap_or(0)
    );

    Json(serde_json::to_value(response).unwrap_or_default())
}

async fn handle_create_note(state: &AppState, request: &ActionRequest) -> ActionResponse {
    // Parse payload into NoteContent
    let content: NoteContent = match serde_json::from_value(request.payload.clone()) {
        Ok(c) => c,
        Err(e) => {
            return ActionResponse {
                spec_version: SPEC_VERSION.to_string(),
                action_id: request.action_id,
                status: ActionStatus::Error,
                result: None,
                error: Some(ActionError {
                    code: "INVALID_PAYLOAD".to_string(),
                    message: format!("Failed to parse note content: {}", e),
                    retryable: false,
                }),
                execution_time_ms: None,
            };
        }
    };

    // Execute storage and extract all data BEFORE any await (for Send safety)
    let (response, maybe_event) = {
        match state.storage.create_note(content) {
            Ok(note) => {
                let note_id = note.id.clone();
                let context = note.data.context.clone();
                let event = EventMessage::new("note_created", serde_json::json!({
                    "note_id": &note_id,
                    "context": &context
                }));
                let response = ActionResponse {
                    spec_version: SPEC_VERSION.to_string(),
                    action_id: request.action_id,
                    status: ActionStatus::Success,
                    result: Some(serde_json::json!({
                        "note_id": note_id,
                        "note": note
                    })),
                    error: None,
                    execution_time_ms: None,
                };
                (response, Some(event))
            }
            Err(e) => {
                let response = ActionResponse {
                    spec_version: SPEC_VERSION.to_string(),
                    action_id: request.action_id,
                    status: ActionStatus::Error,
                    result: None,
                    error: Some(ActionError {
                        code: "STORAGE_ERROR".to_string(),
                        message: e.to_string(),
                        retryable: true,
                    }),
                    execution_time_ms: None,
                };
                (response, None)
            }
        }
    };

    // Now safe to await - no non-Send types in scope
    if let Some(event) = maybe_event {
        emit_event(&state.mqtt_client, event).await;
    }

    response
}

async fn handle_update_note(state: &AppState, request: &ActionRequest) -> ActionResponse {
    let note_id_str = request.payload.get("note_id")
        .and_then(|v| v.as_str())
        .unwrap_or("")
        .to_string();

    if note_id_str.is_empty() {
        return ActionResponse {
            spec_version: SPEC_VERSION.to_string(),
            action_id: request.action_id,
            status: ActionStatus::Error,
            result: None,
            error: Some(ActionError {
                code: "MISSING_NOTE_ID".to_string(),
                message: "note_id is required".to_string(),
                retryable: false,
            }),
            execution_time_ms: None,
        };
    }

    // Build NoteContent from payload (excluding note_id)
    let mut payload = request.payload.clone();
    if let Some(obj) = payload.as_object_mut() {
        obj.remove("note_id");
    }

    let content: NoteContent = match serde_json::from_value(payload) {
        Ok(c) => c,
        Err(e) => {
            return ActionResponse {
                spec_version: SPEC_VERSION.to_string(),
                action_id: request.action_id,
                status: ActionStatus::Error,
                result: None,
                error: Some(ActionError {
                    code: "INVALID_PAYLOAD".to_string(),
                    message: format!("Failed to parse note content: {}", e),
                    retryable: false,
                }),
                execution_time_ms: None,
            };
        }
    };

    // Execute storage and extract all data BEFORE any await
    let (response, maybe_event) = {
        match state.storage.update_note(&note_id_str, content) {
            Ok(Some(note)) => {
                let event = EventMessage::new("note_updated", serde_json::json!({
                    "note_id": &note.id
                }));
                let response = ActionResponse {
                    spec_version: SPEC_VERSION.to_string(),
                    action_id: request.action_id,
                    status: ActionStatus::Success,
                    result: Some(serde_json::json!({ "note": note })),
                    error: None,
                    execution_time_ms: None,
                };
                (response, Some(event))
            }
            Ok(None) => {
                let response = ActionResponse {
                    spec_version: SPEC_VERSION.to_string(),
                    action_id: request.action_id,
                    status: ActionStatus::Error,
                    result: None,
                    error: Some(ActionError {
                        code: "NOT_FOUND".to_string(),
                        message: format!("Note {} not found", note_id_str),
                        retryable: false,
                    }),
                    execution_time_ms: None,
                };
                (response, None)
            }
            Err(e) => {
                let response = ActionResponse {
                    spec_version: SPEC_VERSION.to_string(),
                    action_id: request.action_id,
                    status: ActionStatus::Error,
                    result: None,
                    error: Some(ActionError {
                        code: "STORAGE_ERROR".to_string(),
                        message: e.to_string(),
                        retryable: true,
                    }),
                    execution_time_ms: None,
                };
                (response, None)
            }
        }
    };

    if let Some(event) = maybe_event {
        emit_event(&state.mqtt_client, event).await;
    }

    response
}

async fn handle_delete_note(state: &AppState, request: &ActionRequest) -> ActionResponse {
    let note_id_str = request.payload.get("note_id")
        .and_then(|v| v.as_str())
        .unwrap_or("")
        .to_string();

    if note_id_str.is_empty() {
        return ActionResponse {
            spec_version: SPEC_VERSION.to_string(),
            action_id: request.action_id,
            status: ActionStatus::Error,
            result: None,
            error: Some(ActionError {
                code: "MISSING_NOTE_ID".to_string(),
                message: "note_id is required".to_string(),
                retryable: false,
            }),
            execution_time_ms: None,
        };
    }

    // Execute storage and extract all data BEFORE any await
    let (response, maybe_event) = {
        match state.storage.delete_note(&note_id_str) {
            Ok(true) => {
                let event = EventMessage::new("note_deleted", serde_json::json!({
                    "note_id": &note_id_str
                }));
                let response = ActionResponse {
                    spec_version: SPEC_VERSION.to_string(),
                    action_id: request.action_id,
                    status: ActionStatus::Success,
                    result: Some(serde_json::json!({ "deleted": true, "note_id": &note_id_str })),
                    error: None,
                    execution_time_ms: None,
                };
                (response, Some(event))
            }
            Ok(false) => {
                let response = ActionResponse {
                    spec_version: SPEC_VERSION.to_string(),
                    action_id: request.action_id,
                    status: ActionStatus::Error,
                    result: None,
                    error: Some(ActionError {
                        code: "NOT_FOUND".to_string(),
                        message: format!("Note {} not found", note_id_str),
                        retryable: false,
                    }),
                    execution_time_ms: None,
                };
                (response, None)
            }
            Err(e) => {
                let response = ActionResponse {
                    spec_version: SPEC_VERSION.to_string(),
                    action_id: request.action_id,
                    status: ActionStatus::Error,
                    result: None,
                    error: Some(ActionError {
                        code: "STORAGE_ERROR".to_string(),
                        message: e.to_string(),
                        retryable: true,
                    }),
                    execution_time_ms: None,
                };
                (response, None)
            }
        }
    };

    if let Some(event) = maybe_event {
        emit_event(&state.mqtt_client, event).await;
    }

    response
}

async fn handle_list_notes(state: &AppState, request: &ActionRequest) -> ActionResponse {
    // Convert payload to filters if present
    let filters: Option<HashMap<String, serde_json::Value>> =
        serde_json::from_value(request.payload.clone()).ok();

    let notes = state.storage.list_notes(filters);

    ActionResponse {
        spec_version: SPEC_VERSION.to_string(),
        action_id: request.action_id,
        status: ActionStatus::Success,
        result: Some(serde_json::json!({
            "notes": notes,
            "count": notes.len()
        })),
        error: None,
        execution_time_ms: None,
    }
}

/// Emit event on MQTT
async fn emit_event(client: &AsyncClient, event: EventMessage) {
    if let Ok(json) = serde_json::to_string(&event) {
        if let Err(e) = client.publish(topics::EVENTS, QoS::AtLeastOnce, false, json).await {
            eprintln!("[notes] failed to emit event {}: {:?}", event.event_type, e);
        } else {
            eprintln!("[notes] emitted event: {}", event.event_type);
        }
    }
}

// Legacy HTTP handlers adapted for AppState
async fn list_notes_http_v2(
    State(state): State<AppState>,
) -> Json<serde_json::Value> {
    let notes = state.storage.list_notes(None);
    Json(serde_json::json!({
        "notes": notes,
        "count": notes.len()
    }))
}

async fn create_note_http_v2(
    State(state): State<AppState>,
    Json(content): Json<NoteContent>,
) -> Result<Json<Note>, (StatusCode, String)> {
    match state.storage.create_note(content) {
        Ok(note) => Ok(Json(note)),
        Err(e) => Err((StatusCode::INTERNAL_SERVER_ERROR, e.to_string())),
    }
}

async fn delete_note_http_v2(
    State(state): State<AppState>,
    axum::extract::Path(id): axum::extract::Path<String>,
) -> Result<Json<serde_json::Value>, (StatusCode, String)> {
    match state.storage.delete_note(&id) {
        Ok(true) => Ok(Json(serde_json::json!({"deleted": true, "id": id}))),
        Ok(false) => Err((StatusCode::NOT_FOUND, "Note not found".to_string())),
        Err(e) => Err((StatusCode::INTERNAL_SERVER_ERROR, e.to_string())),
    }
}

async fn update_note_http_v2(
    State(state): State<AppState>,
    axum::extract::Path(id): axum::extract::Path<String>,
    Json(content): Json<NoteContent>,
) -> Result<Json<Note>, (StatusCode, String)> {
    match state.storage.update_note(&id, content) {
        Ok(Some(note)) => Ok(Json(note)),
        Ok(None) => Err((StatusCode::NOT_FOUND, "Note not found".to_string())),
        Err(e) => Err((StatusCode::INTERNAL_SERVER_ERROR, e.to_string())),
    }
}

/// Construit le router HTTP pour le plugin
fn build_router(state: AppState) -> Router {
    Router::new()
        // Contract v1.0 routes
        .route("/health", get(health_check))
        .route("/actions", post(handle_action))
        // Legacy routes (will be deprecated)
        .route("/notes", get(list_notes_http_v2).post(create_note_http_v2))
        .route("/notes/:id", axum::routing::delete(delete_note_http_v2).put(update_note_http_v2))
        .with_state(state)
}

/// Point d'entrée principal du plugin
#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    eprintln!("[notes] symbion plugin notes v1.1.0 (Contract v1.0) starting...");

    // Initialisation du stockage
    let storage = NotesStorage::new("./notes.json")?;
    let storage = Arc::new(storage);

    // Unix socket path (systemd RuntimeDirectory)
    let socket_path = "/run/symbion-plugins/notes.sock";

    // Cleanup old socket at startup (triple safety net)
    if std::path::Path::new(socket_path).exists() {
        eprintln!("[notes] cleaning up old socket at startup");
        if let Err(e) = std::fs::remove_file(socket_path) {
            eprintln!("[notes] failed to remove old socket: {}", e);
        }
    }

    // Create shutdown channel for graceful termination
    let (shutdown_tx, mut shutdown_rx) = broadcast::channel::<()>(1);

    // Configuration MQTT (avant le router pour créer AppState)
    let mut mqttopts = MqttOptions::new("symbion-plugin-notes", "localhost", 1883);
    mqttopts.set_keep_alive(Duration::from_secs(30));
    mqttopts.set_clean_session(true);
    mqttopts.set_max_packet_size(1024 * 1024, 1024 * 1024);

    let (client, mut eventloop) = AsyncClient::new(mqttopts, 200);

    // Create AppState with storage and MQTT client
    let app_state = AppState {
        storage: storage.clone(),
        mqtt_client: client.clone(),
    };

    // Construire le router HTTP avec Contract v1.0
    let app = build_router(app_state);

    // Démarrer le serveur HTTP en arrière-plan
    let socket_path_clone = socket_path.to_string();
    tokio::spawn(async move {
        eprintln!("[notes] starting HTTP server on Unix socket: {}", socket_path_clone);
        if let Err(e) = PluginHttpServer::new(&socket_path_clone, app).serve().await {
            eprintln!("[notes] HTTP server error: {:?}", e);
        }
    });

    // Attendre que le socket soit créé
    tokio::time::sleep(Duration::from_millis(100)).await;

    // Contract v1.0: Publish manifest on MQTT at startup
    let manifest = include_str!("../manifest.json");
    if let Err(e) = client.publish(topics::MANIFEST, QoS::AtLeastOnce, true, manifest).await {
        eprintln!("[notes] failed to publish manifest: {:?}", e);
    } else {
        eprintln!("[notes] ✅ manifest published on {}", topics::MANIFEST);
    }

    // Contract v1.0: Heartbeat every 30 seconds
    let heartbeat_client = client.clone();
    tokio::spawn(async move {
        let start = std::time::Instant::now();
        loop {
            tokio::time::sleep(Duration::from_secs(30)).await;

            let health = HealthStatus {
                spec_version: SPEC_VERSION.to_string(),
                plugin_id: PLUGIN_ID.to_string(),
                status: "healthy".to_string(),
                uptime_seconds: start.elapsed().as_secs(),
                last_action_at: None,
            };

            if let Ok(json) = serde_json::to_string(&health) {
                // [P0-6] Use QoS::AtLeastOnce (QoS 1) for health - loss of health msg is critical
                if let Err(e) = heartbeat_client.publish(topics::HEALTH, QoS::AtLeastOnce, false, json).await {
                    eprintln!("[notes] heartbeat failed: {:?}", e);
                }
            }
        }
    });

    // Legacy: S'abonner aux anciens topics de commandes (backward compatibility)
    client.subscribe(topics::LEGACY_COMMAND, QoS::AtLeastOnce).await?;

    eprintln!("[notes] connected to MQTT, Contract v1.0 active");

    // Signal handlers for graceful shutdown (SIGTERM from systemd, SIGINT from Ctrl+C)
    let socket_path_for_cleanup = socket_path.to_string();
    let shutdown_tx_clone = shutdown_tx.clone();
    tokio::spawn(async move {
        let mut sigterm = signal(SignalKind::terminate()).expect("failed to install SIGTERM handler");
        let mut sigint = signal(SignalKind::interrupt()).expect("failed to install SIGINT handler");

        tokio::select! {
            _ = sigterm.recv() => {
                eprintln!("[notes] received SIGTERM, shutting down gracefully...");
            }
            _ = sigint.recv() => {
                eprintln!("[notes] received SIGINT (Ctrl+C), shutting down gracefully...");
            }
        }

        // Cleanup socket
        if std::path::Path::new(&socket_path_for_cleanup).exists() {
            eprintln!("[notes] cleaning up socket: {}", socket_path_for_cleanup);
            if let Err(e) = std::fs::remove_file(&socket_path_for_cleanup) {
                eprintln!("[notes] failed to remove socket on shutdown: {}", e);
            }
        }

        // Signal main loop to exit
        let _ = shutdown_tx_clone.send(());
    });

    // Boucle principale de traitement des messages
    loop {
        tokio::select! {
            // Check for shutdown signal
            _ = shutdown_rx.recv() => {
                eprintln!("[notes] shutdown signal received, exiting main loop");
                break;
            }
            // Process MQTT events (legacy commands only - Contract v1.0 uses HTTP /actions)
            event = eventloop.poll() => {
                match event {
                    Ok(Event::Incoming(Incoming::Publish(publish))) => {
                        if publish.topic == topics::LEGACY_COMMAND {
                            // Legacy MQTT commands (backward compatibility)
                            let client_clone = client.clone();
                            let storage_clone = storage.clone();
                            let payload = publish.payload.to_vec();
                            tokio::spawn(async move {
                                handle_legacy_command(&client_clone, &storage_clone, &payload).await;
                            });
                        }
                    }
                    Ok(_) => {
                        // Autres événements MQTT ignorés
                    }
                    Err(e) => {
                        eprintln!("[notes] MQTT error: {:?}", e);
                        eprintln!("[notes] Fatal error - exiting to allow restart");
                        break;
                    }
                }
            }
        }
    }

    eprintln!("[notes] exited main loop, performing final cleanup");
    Ok(())
}

/// Legacy: Traite une commande MQTT (backward compatibility, deprecated)
async fn handle_legacy_command(
    client: &AsyncClient,
    storage: &NotesStorage,
    payload: &[u8],
) {
    let command_result: Result<NoteCommand, _> = serde_json::from_slice(payload);

    match command_result {
        Ok(NoteCommand::List { request_id, filters }) => {
            // Streaming: envoyer 1 note par message
            let notes = storage.list_notes(filters);
            let total_count = notes.len();

            eprintln!("[notes] streaming {} notes for request {}", total_count, request_id);

            // Publier chaque note individuellement
            for note in notes {
                let response = NoteResponse::NoteItem {
                    request_id: request_id.clone(),
                    note,
                };

                if let Ok(response_json) = serde_json::to_string(&response) {
                    if let Err(e) = client
                        .publish("symbion/notes/response@v1", QoS::AtLeastOnce, false, response_json)
                        .await
                    {
                        eprintln!("[notes] failed to publish note item: {:?}", e);
                        return;
                    }
                }
            }

            // Publier le marqueur de fin
            let end_response = NoteResponse::ListEnd {
                request_id,
                total_count,
            };

            if let Ok(response_json) = serde_json::to_string(&end_response) {
                if let Err(e) = client
                    .publish("symbion/notes/response@v1", QoS::AtLeastOnce, false, response_json)
                    .await
                {
                    eprintln!("[notes] failed to publish list end: {:?}", e);
                }
            }
        }
        Ok(command) => {
            // Autres commandes: réponse unique
            let response = process_command(storage, command).await;

            if let Ok(response_json) = serde_json::to_string(&response) {
                if let Err(e) = client
                    .publish("symbion/notes/response@v1", QoS::AtLeastOnce, false, response_json)
                    .await
                {
                    eprintln!("[notes] failed to publish response: {:?}", e);
                }
            }
        }
        Err(e) => {
            let response = NoteResponse::Error {
                request_id: "unknown".to_string(),
                action: "parse".to_string(),
                error: format!("Invalid command JSON: {}", e),
            };

            if let Ok(response_json) = serde_json::to_string(&response) {
                if let Err(e) = client
                    .publish("symbion/notes/response@v1", QoS::AtLeastOnce, false, response_json)
                    .await
                {
                    eprintln!("[notes] failed to publish error response: {:?}", e);
                }
            }
        }
    }
}

/// Traite une commande et génère une réponse
async fn process_command(
    storage: &NotesStorage,
    command: NoteCommand,
) -> NoteResponse {
    match command {
        NoteCommand::Create { request_id, note } => {
            match storage.create_note(note) {
                Ok(created_note) => NoteResponse::Success {
                    request_id,
                    action: "create".to_string(),
                    data: serde_json::to_value(created_note).unwrap_or_default(),
                },
                Err(e) => NoteResponse::Error {
                    request_id,
                    action: "create".to_string(),
                    error: e.to_string(),
                },
            }
        }

        NoteCommand::List { request_id, .. } => {
            // List est géré en streaming dans handle_command
            NoteResponse::Error {
                request_id,
                action: "list".to_string(),
                error: "List command should be handled in handle_command".to_string(),
            }
        }

        NoteCommand::Delete { request_id, id } => {
            match storage.delete_note(&id) {
                Ok(true) => NoteResponse::Success {
                    request_id,
                    action: "delete".to_string(),
                    data: serde_json::json!({"deleted": true, "id": id}),
                },
                Ok(false) => NoteResponse::Error {
                    request_id,
                    action: "delete".to_string(),
                    error: "Note not found".to_string(),
                },
                Err(e) => NoteResponse::Error {
                    request_id,
                    action: "delete".to_string(),
                    error: e.to_string(),
                },
            }
        }
        
        NoteCommand::Update { request_id, id, note } => {
            match storage.update_note(&id, note) {
                Ok(Some(updated_note)) => NoteResponse::Success {
                    request_id,
                    action: "update".to_string(),
                    data: serde_json::to_value(updated_note).unwrap_or_default(),
                },
                Ok(None) => NoteResponse::Error {
                    request_id,
                    action: "update".to_string(),
                    error: "Note not found".to_string(),
                },
                Err(e) => NoteResponse::Error {
                    request_id,
                    action: "update".to_string(),
                    error: e.to_string(),
                },
            }
        }
    }
}