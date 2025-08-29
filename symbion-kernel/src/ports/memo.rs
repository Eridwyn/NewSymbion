/**
 * PORT MEMO v1 - Stockage des mémos et rappels Symbion
 * 
 * RÔLE :
 * Ce port gère la persistence des mémos utilisateur : notes, rappels, 
 * tâches rapides, idées. C'est le "carnet de notes" digital de Symbion.
 * 
 * FONCTIONNEMENT :
 * - Stockage en fichier JSON (pour l'instant, SQLite plus tard)
 * - Chaque memo = contenu + priorité + contexte + metadata
 * - Support des requêtes : filtrage par urgent, contexte, date
 * - Auto-génération d'IDs uniques (UUID)
 * 
 * UTILITÉ DANS SYMBION :
 * 🎯 Phase B - Plugin Memo/Rappels : stockage des notes utilisateur
 * 🎯 Phase E - Context Engine : mémos contextuels (cravate, maison, etc.)
 * 🎯 PWA - Interface mobile : ajouter/lire mémos en temps réel
 * 
 * DONNÉES EXEMPLE :
 * ```json
 * {
 *   "content": "Appeler le dentiste pour RDV",
 *   "urgent": true,
 *   "context": "cravate",  // contexte Symbion
 *   "tags": ["santé", "rdv"]
 * }
 * ```
 */

use super::{DataPort, PortData, PortError, PortInfo, PortQuery};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;
use time::OffsetDateTime;
use uuid::Uuid;

/// Structure spécifique aux données du port memo
/// Contient les champs métier propres aux mémos/rappels
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemoContent {
    /// Texte principal du memo
    pub content: String,
    /// Priorité/urgence (false par défaut)
    pub urgent: Option<bool>,
    /// Contexte Symbion (cravate, intime, neutre)
    pub context: Option<String>,
    /// Tags libres pour classification
    pub tags: Option<Vec<String>>,
    /// Statut du memo (pending, done, archived)
    pub status: Option<String>,
}

/// Implémentation du Data Port pour les mémos
/// Gère le stockage persistant en JSON (évoluera vers SQLite)
pub struct MemoPort {
    /// Chemin du fichier de stockage JSON
    storage_path: PathBuf,
    /// Cache en mémoire des mémos (pour perf)
    cache: parking_lot::Mutex<Vec<PortData>>,
}

impl MemoPort {
    /// Crée un nouveau port memo avec le fichier de stockage spécifié
    pub fn new<P: Into<PathBuf>>(storage_path: P) -> Result<Self, PortError> {
        let path = storage_path.into();
        let port = Self {
            storage_path: path.clone(),
            cache: parking_lot::Mutex::new(Vec::new()),
        };
        
        // Charge les données existantes au démarrage
        port.load_from_disk()?;
        eprintln!("[memo] port initialized at {:?}", path);
        Ok(port)
    }
    
    /// Charge les mémos depuis le fichier JSON vers le cache mémoire
    fn load_from_disk(&self) -> Result<(), PortError> {
        if !self.storage_path.exists() {
            // Fichier n'existe pas encore, on crée un tableau vide
            fs::write(&self.storage_path, "[]")?;
        }
        
        let content = fs::read_to_string(&self.storage_path)?;
        let memos: Vec<PortData> = serde_json::from_str(&content)?;
        
        *self.cache.lock() = memos;
        Ok(())
    }
    
    /// Sauvegarde le cache mémoire vers le fichier JSON
    fn save_to_disk(&self) -> Result<(), PortError> {
        let cache = self.cache.lock();
        let json = serde_json::to_string_pretty(&*cache)?;
        fs::write(&self.storage_path, json)?;
        Ok(())
    }
    
    /// Valide qu'une donnée respecte le schéma memo
    fn validate_memo_data(data: &serde_json::Value) -> Result<MemoContent, PortError> {
        serde_json::from_value(data.clone())
            .map_err(|e| PortError::InvalidQuery(format!("Invalid memo format: {}", e)))
    }
    
    /// Applique les filtres de requête sur un memo
    fn matches_filters(memo: &PortData, filters: &HashMap<String, serde_json::Value>) -> bool {
        // Parse le contenu memo pour appliquer les filtres
        if let Ok(memo_content) = serde_json::from_value::<MemoContent>(memo.data.clone()) {
            for (key, value) in filters {
                match key.as_str() {
                    "urgent" => {
                        let memo_urgent = memo_content.urgent.unwrap_or(false);
                        if value.as_bool() != Some(memo_urgent) {
                            return false;
                        }
                    }
                    "context" => {
                        if let Some(context) = &memo_content.context {
                            if value.as_str() != Some(context) {
                                return false;
                            }
                        } else if !value.is_null() {
                            return false;
                        }
                    }
                    "status" => {
                        let status = memo_content.status.as_deref().unwrap_or("pending");
                        if value.as_str() != Some(status) {
                            return false;
                        }
                    }
                    "content" => {
                        // Recherche textuelle dans le contenu
                        if let Some(search) = value.as_str() {
                            if !memo_content.content.to_lowercase().contains(&search.to_lowercase()) {
                                return false;
                            }
                        }
                    }
                    _ => {} // Filtre non reconnu, on l'ignore
                }
            }
        }
        true
    }
}

impl DataPort for MemoPort {
    /// Lecture des mémos avec filtrage optionnel
    /// Supporte: urgent=true, context="cravate", status="pending", content="dentiste"
    fn read(&self, query: &PortQuery) -> Result<Vec<PortData>, PortError> {
        let cache = self.cache.lock();
        let mut results: Vec<PortData> = cache
            .iter()
            .filter(|memo| Self::matches_filters(memo, &query.filters))
            .cloned()
            .collect();
        
        // Tri par timestamp décroissant par défaut
        results.sort_by(|a, b| b.timestamp.cmp(&a.timestamp));
        
        // Pagination
        if let Some(offset) = query.offset {
            if offset < results.len() {
                results = results.into_iter().skip(offset).collect();
            } else {
                results.clear();
            }
        }
        
        if let Some(limit) = query.limit {
            results.truncate(limit);
        }
        
        Ok(results)
    }
    
    /// Écriture d'un nouveau memo
    /// Génère automatiquement un UUID et timestamp
    fn write(&self, data: &PortData) -> Result<String, PortError> {
        // Validation du format memo
        Self::validate_memo_data(&data.data)?;
        
        // Génération d'un nouvel ID si pas fourni
        let id = if data.id.is_empty() {
            Uuid::new_v4().to_string()
        } else {
            data.id.clone()
        };
        
        // Création du memo avec timestamp actuel
        let memo = PortData {
            id: id.clone(),
            timestamp: OffsetDateTime::now_utc(),
            data: data.data.clone(),
            metadata: data.metadata.clone(),
        };
        
        // Ajout au cache et sauvegarde
        {
            let mut cache = self.cache.lock();
            cache.push(memo);
        }
        
        self.save_to_disk()?;
        eprintln!("[memo] written memo {}", id);
        Ok(id)
    }
    
    /// Suppression d'un memo par ID
    fn delete(&self, id: &str) -> Result<(), PortError> {
        let mut cache = self.cache.lock();
        let initial_len = cache.len();
        cache.retain(|memo| memo.id != id);
        
        if cache.len() == initial_len {
            return Err(PortError::NotFound(format!("Memo {} not found", id)));
        }
        
        drop(cache); // Libère le lock avant save
        self.save_to_disk()?;
        eprintln!("[memo] deleted memo {}", id);
        Ok(())
    }
    
    /// Métadonnées du port memo
    fn info(&self) -> PortInfo {
        PortInfo {
            name: "memo".to_string(),
            version: "v1".to_string(),
            description: "Port de stockage des mémos et rappels utilisateur".to_string(),
            schema: serde_json::json!({
                "type": "object",
                "required": ["content"],
                "properties": {
                    "content": {"type": "string", "description": "Texte du memo"},
                    "urgent": {"type": "boolean", "description": "Priorité urgente"},
                    "context": {"type": "string", "enum": ["cravate", "intime", "neutre"]},
                    "tags": {"type": "array", "items": {"type": "string"}},
                    "status": {"type": "string", "enum": ["pending", "done", "archived"]}
                }
            }),
            capabilities: vec!["read".to_string(), "write".to_string(), "delete".to_string(), "query".to_string()],
        }
    }
}