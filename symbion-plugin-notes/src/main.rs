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
use tokio::time::{sleep, Duration};
use uuid::Uuid;
use parking_lot::Mutex;
use std::sync::Arc;

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
}

/// Gestionnaire de stockage des notes (similaire au port memo)
#[derive(Debug)]
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
    
    /// Sauvegarde les notes sur disque
    fn save_to_disk(&self) -> Result<(), Box<dyn std::error::Error>> {
        let notes = self.notes.lock();
        let content = serde_json::to_string_pretty(&*notes)?;
        fs::write(&self.storage_path, content)?;
        Ok(())
    }
    
    /// Crée une nouvelle note
    pub fn create_note(&self, content: NoteContent) -> Result<Note, Box<dyn std::error::Error>> {
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
    pub fn update_note(&self, id: &str, new_content: NoteContent) -> Result<Option<Note>, Box<dyn std::error::Error>> {
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
                            // Vérifie que tous les tags du filtre sont présents
                            if !filter_tags.iter().all(|tag| tags.contains(tag)) {
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

/// Point d'entrée principal du plugin
#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    eprintln!("[notes] symbion plugin notes starting...");
    
    // Initialisation du stockage
    let storage = NotesStorage::new("./notes.json")?;
    let storage = Arc::new(storage);
    
    // Configuration MQTT
    let mut mqttopts = MqttOptions::new("symbion-plugin-notes", "localhost", 1883);
    mqttopts.set_keep_alive(Duration::from_secs(30));
    
    let (client, mut eventloop) = AsyncClient::new(mqttopts, 10);
    
    // S'abonner aux topics de commandes
    client.subscribe("symbion/notes/command@v1", QoS::AtLeastOnce).await?;
    
    eprintln!("[notes] connected to MQTT, listening for commands...");
    
    // Boucle principale de traitement des messages
    loop {
        match eventloop.poll().await {
            Ok(Event::Incoming(Incoming::Publish(publish))) => {
                if publish.topic == "symbion/notes/command@v1" {
                    handle_command(&client, &storage, &publish.payload).await;
                }
            }
            Ok(_) => {
                // Autres événements MQTT ignorés
            }
            Err(e) => {
                eprintln!("[notes] MQTT error: {:?}", e);
                sleep(Duration::from_secs(1)).await;
            }
        }
    }
}

/// Traite une commande MQTT reçue
async fn handle_command(
    client: &AsyncClient,
    storage: &NotesStorage,
    payload: &[u8],
) {
    let command_result: Result<NoteCommand, _> = serde_json::from_slice(payload);
    
    let response = match command_result {
        Ok(command) => process_command(storage, command).await,
        Err(e) => NoteResponse::Error {
            request_id: "unknown".to_string(),
            action: "parse".to_string(),
            error: format!("Invalid command JSON: {}", e),
        },
    };
    
    // Publier la réponse
    if let Ok(response_json) = serde_json::to_string(&response) {
        if let Err(e) = client
            .publish("symbion/notes/response@v1", QoS::AtLeastOnce, false, response_json)
            .await
        {
            eprintln!("[notes] failed to publish response: {:?}", e);
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
        
        NoteCommand::List { request_id, filters } => {
            let notes = storage.list_notes(filters);
            NoteResponse::Success {
                request_id,
                action: "list".to_string(),
                data: serde_json::to_value(notes).unwrap_or_default(),
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