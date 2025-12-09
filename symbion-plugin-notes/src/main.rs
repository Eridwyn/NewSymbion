/**
 * SYMBION PLUGIN NOTES - Service distribué de gestion des notes
 * 
 * RÔLE :
 * Plugin autonome qui gère les notes/mémos/rappels via MQTT.
 * Remplace le port memo intégré du kernel pour une architecture plus modulaire.
 * 
 * FONCTIONNEMENT :
 * - Stockage JSON local (./notes.json)
 * - Écoute MQTT : create, list, delete, update notes
 * - Répond sur MQTT : résultats des opérations
 * 
 * UTILITÉ DANS SYMBION :
 * 🎯 Découplement : Notes séparées du kernel central
 * 🎯 Extensibilité : Plugin peut évoluer indépendamment  
 * 🎯 Distribution : Peut tourner sur machine dédiée
 * 🎯 Résilience : Crash plugin n'affecte pas le kernel
 * 
 * COMMUNICATION MQTT :
 * Écoute: symbion/notes/create@v1, symbion/notes/list@v1
 * Publie: symbion/notes/response@v1
 */

use rumqttc::{AsyncClient, Event, Incoming, MqttOptions, QoS};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;
use time::OffsetDateTime;
use tokio::time::Duration;
use uuid::Uuid;
use parking_lot::Mutex;
use std::sync::Arc;
use axum::{
    extract::State,
    http::StatusCode,
    response::Json,
    routing::get,
    Router,
};
use symbion_plugin_common::{PluginHttpServer, PluginRegistrationBuilder};
use tokio::signal::unix::{signal, SignalKind};
use tokio::sync::broadcast;

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

/// GET /notes - Liste toutes les notes
async fn list_notes_http(
    State(storage): State<Arc<NotesStorage>>,
) -> Json<serde_json::Value> {
    let notes = storage.list_notes(None);
    Json(serde_json::json!({
        "notes": notes,
        "count": notes.len()
    }))
}

/// POST /notes - Crée une nouvelle note
async fn create_note_http(
    State(storage): State<Arc<NotesStorage>>,
    Json(content): Json<NoteContent>,
) -> Result<Json<Note>, (StatusCode, String)> {
    match storage.create_note(content) {
        Ok(note) => Ok(Json(note)),
        Err(e) => Err((StatusCode::INTERNAL_SERVER_ERROR, e.to_string())),
    }
}

/// DELETE /notes/:id - Supprime une note
async fn delete_note_http(
    State(storage): State<Arc<NotesStorage>>,
    axum::extract::Path(id): axum::extract::Path<String>,
) -> Result<Json<serde_json::Value>, (StatusCode, String)> {
    match storage.delete_note(&id) {
        Ok(true) => Ok(Json(serde_json::json!({"deleted": true, "id": id}))),
        Ok(false) => Err((StatusCode::NOT_FOUND, "Note not found".to_string())),
        Err(e) => Err((StatusCode::INTERNAL_SERVER_ERROR, e.to_string())),
    }
}

/// PUT /notes/:id - Met à jour une note
async fn update_note_http(
    State(storage): State<Arc<NotesStorage>>,
    axum::extract::Path(id): axum::extract::Path<String>,
    Json(content): Json<NoteContent>,
) -> Result<Json<Note>, (StatusCode, String)> {
    match storage.update_note(&id, content) {
        Ok(Some(note)) => Ok(Json(note)),
        Ok(None) => Err((StatusCode::NOT_FOUND, "Note not found".to_string())),
        Err(e) => Err((StatusCode::INTERNAL_SERVER_ERROR, e.to_string())),
    }
}

/// Health check endpoint
async fn health_check() -> Json<serde_json::Value> {
    use std::sync::OnceLock;
    static START_TIME: OnceLock<std::time::Instant> = OnceLock::new();
    let start = START_TIME.get_or_init(std::time::Instant::now);
    let uptime_secs = start.elapsed().as_secs();

    Json(serde_json::json!({
        "status": "healthy",
        "plugin": "notes",
        "version": "0.1.0",
        "uptime_seconds": uptime_secs
    }))
}

/// Construit le router HTTP pour le plugin
fn build_router(storage: Arc<NotesStorage>) -> Router {
    Router::new()
        .route("/health", get(health_check))
        .route("/notes", get(list_notes_http).post(create_note_http))
        .route("/notes/:id", axum::routing::delete(delete_note_http).put(update_note_http))
        .with_state(storage)
}

/// Point d'entrée principal du plugin
#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    eprintln!("[notes] symbion plugin notes starting...");

    // Initialisation du stockage
    let storage = NotesStorage::new("./notes.json")?;
    let storage = Arc::new(storage);

    // Unix socket path (systemd RuntimeDirectory)
    let socket_path = "/run/symbion-plugins/notes.sock";

    // Cleanup old socket at startup (triple safety net)
    if std::path::Path::new(socket_path).exists() {
        eprintln!("[notes] cleaning up old socket at startup");
        let _ = std::fs::remove_file(socket_path);
    }

    // Create shutdown channel for graceful termination
    let (shutdown_tx, mut shutdown_rx) = broadcast::channel::<()>(1);

    // Construire le router HTTP
    let app = build_router(storage.clone());

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

    // Service Discovery: Auto-registration avec le kernel
    let socket_path_clone = socket_path.to_string();
    tokio::spawn(async move {
        tokio::time::sleep(Duration::from_millis(500)).await;

        match PluginRegistrationBuilder::new("notes", &socket_path_clone)
            .route("/notes")
            .route("/notes/:id")
            .route("/health")
            .version("1.0.0")
            .description("Notes plugin with MQTT streaming and HTTP API")
            .register()
            .await
        {
            Ok(_) => eprintln!("[notes] ✅ Registered with kernel via Service Discovery"),
            Err(e) => eprintln!("[notes] ❌ Failed to register with kernel: {}", e),
        }
    });

    // Configuration MQTT
    let mut mqttopts = MqttOptions::new("symbion-plugin-notes", "localhost", 1883);
    mqttopts.set_keep_alive(Duration::from_secs(30));
    mqttopts.set_clean_session(true); // Nettoie la session à la déconnexion (évite collision client ID)
    mqttopts.set_max_packet_size(1024 * 1024, 1024 * 1024); // 1 MB max pour les gros payloads (notes)

    // Buffer de 200 messages pour supporter le streaming de notes sans deadlock
    let (client, mut eventloop) = AsyncClient::new(mqttopts, 200);

    // S'abonner aux topics de commandes
    client.subscribe("symbion/notes/command@v1", QoS::AtLeastOnce).await?;

    eprintln!("[notes] connected to MQTT, listening for commands...");

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
            let _ = std::fs::remove_file(&socket_path_for_cleanup);
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
            // Process MQTT events
            event = eventloop.poll() => {
                match event {
                    Ok(Event::Incoming(Incoming::Publish(publish))) => {
                        if publish.topic == "symbion/notes/command@v1" {
                            // IMPORTANT: Spawner dans une task séparée pour ne PAS bloquer l'eventloop
                            // Sinon deadlock quand handle_command fait client.publish().await
                            let client_clone = client.clone();
                            let storage_clone = storage.clone();
                            let payload = publish.payload.to_vec();
                            tokio::spawn(async move {
                                handle_command(&client_clone, &storage_clone, &payload).await;
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

/// Traite une commande MQTT reçue
async fn handle_command(
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