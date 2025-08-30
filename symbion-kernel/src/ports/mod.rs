/**
 * DATA PORTS v1 - Interface de persistence unifiée pour Symbion
 * 
 * RÔLE :
 * Ce module définit l'architecture standardisée pour que tous les plugins 
 * puissent stocker/lire leurs données de manière cohérente.
 * 
 * FONCTIONNEMENT :
 * - PortRegistry = catalogue central de tous les ports disponibles (memo, journal, finance...)
 * - DataPort trait = interface commune (read/write/delete) que chaque port implémente
 * - PortData = format standardisé des données (timestamp + JSON + metadata)
 * - PortQuery = langage de requête unifié (filtres, pagination, tri)
 * 
 * UTILITÉ POUR SYMBION :
 * ✅ Interface standardisée : même API pour notes, finance, journal...
 * ✅ API unifiée : /ports/[domain] pour tous les domaines métier
 * ✅ Extensibilité : framework prêt pour nouveaux plugins 
 * ✅ Découverte : GET /ports liste tous les domaines disponibles
 * 
 * EXEMPLE D'USAGE FUTUR :
 * ```rust
 * // Plugin finance écrit une transaction
 * let data = PortData { amount: 42.0, type: "expense" };
 * ports.get("finance")?.write(&data)?;
 * 
 * // Plugin dashboard lit les dépenses récentes
 * let query = PortQuery { filters: {"type": "expense"} };
 * let expenses = ports.get("finance")?.read(&query)?;
 * ```
 */

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use time::OffsetDateTime;

/// Erreurs possibles lors des opérations sur les Data Ports
#[derive(Debug, thiserror::Error)]
pub enum PortError {
    #[error("Port not found: {0}")]
    NotFound(String),
    #[error("Invalid query: {0}")]
    InvalidQuery(String),
    #[error("Serialization error: {0}")]
    Serialization(#[from] serde_json::Error),
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
    #[error("Permission denied")]
    PermissionDenied,
}

/// Structure de requête standardisée pour interroger un port
/// Permet filtrage, pagination et tri sur tous les ports de manière cohérente
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PortQuery {
    /// Filtres à appliquer (ex: {"urgent": true, "context": "cravate"})
    pub filters: HashMap<String, serde_json::Value>,
    /// Nombre max de résultats (pagination)
    pub limit: Option<usize>,
    /// Décalage pour pagination (skip N premiers résultats)
    pub offset: Option<usize>,
    /// Champ de tri (ex: "timestamp", "priority")
    pub order_by: Option<String>,
}

/// Format standardisé des données stockées dans tous les ports
/// Structure commune : ID unique + timestamp + données JSON + métadonnées
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PortData {
    /// Identifiant unique de l'enregistrement
    pub id: String,
    /// Horodatage de création/modification
    pub timestamp: OffsetDateTime,
    /// Contenu principal au format JSON (flexible selon le port)
    pub data: serde_json::Value,
    /// Métadonnées additionnelles (tags, source, permissions...)
    pub metadata: HashMap<String, String>,
}

/// Registre central qui maintient la liste de tous les Data Ports disponibles
/// C'est le "catalogue" que consulte le kernel pour trouver memo, journal, finance, etc.
pub struct PortRegistry {
    /// Map nom_port -> implémentation du port
    ports: HashMap<String, Box<dyn DataPort + Send + Sync>>,
}

/// Interface commune que TOUS les Data Ports doivent implémenter
/// Garantit que memo, journal, finance, etc. exposent les mêmes opérations de base
pub trait DataPort {
    /// Lecture de données depuis le port avec requête optionnelle
    /// Ex: lire tous les memos urgents du contexte "cravate"
    fn read(&self, query: &PortQuery) -> Result<Vec<PortData>, PortError>;
    
    /// Écriture de nouvelles données vers le port
    /// Retourne l'ID généré pour la donnée créée
    fn write(&self, data: &PortData) -> Result<String, PortError>; 
    
    /// Suppression d'un enregistrement par son ID (optionnel selon le port)
    fn delete(&self, _id: &str) -> Result<(), PortError> {
        Err(PortError::InvalidQuery("Delete not supported".into()))
    }
    
    /// Métadonnées du port : nom, version, schéma, capacités
    /// Permet au système de découvrir dynamiquement les ports disponibles
    fn info(&self) -> PortInfo;
}

/// Informations descriptives d'un Data Port
/// Expose les capacités et le schéma pour la découverte automatique
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PortInfo {
    /// Nom du port (ex: "memo", "journal", "finance")
    pub name: String,
    /// Version de l'interface (ex: "v1")
    pub version: String,
    /// Description human-readable du port
    pub description: String,
    /// Schéma JSON des données acceptées par ce port
    pub schema: serde_json::Value,
    /// Liste des opérations supportées
    pub capabilities: Vec<String>, // ["read", "write", "delete", "query"]
}


impl PortRegistry {
    /// Crée un nouveau registre vide de Data Ports
    pub fn new() -> Self {
        Self {
            ports: HashMap::new(),
        }
    }
    
    /// Enregistre un nouveau port dans le système
    /// Ex: registry.register("finance", FinancePort::new());
    pub fn register<T: DataPort + Send + Sync + 'static>(&mut self, name: &str, port: T) {
        self.ports.insert(name.to_string(), Box::new(port));
    }
    
    /// Récupère un port par son nom pour effectuer des opérations
    /// Ex: let memo_port = registry.get("memo")?;
    pub fn get(&self, name: &str) -> Option<&Box<dyn DataPort + Send + Sync>> {
        self.ports.get(name)
    }
    
    /// Liste les noms de tous les ports disponibles
    /// Utile pour l'API /ports qui expose la liste
    pub fn list_ports(&self) -> Vec<String> {
        self.ports.keys().cloned().collect()
    }
    
    /// Récupère les informations détaillées de tous les ports
    /// Permet la découverte automatique des capacités du système
    pub fn list_port_info(&self) -> Vec<PortInfo> {
        self.ports.values().map(|p| p.info()).collect()
    }
}

impl Default for PortQuery {
    /// Configuration par défaut des requêtes : 100 résultats max, triés par timestamp
    fn default() -> Self {
        Self {
            filters: HashMap::new(),
            limit: Some(100),
            offset: None,
            order_by: Some("timestamp".to_string()),
        }
    }
}

// NOTE: Les ports spécifiques sont maintenant implémentés comme plugins distribués
// (ex: notes via symbion-plugin-notes, finance via symbion-plugin-finance, etc.)

/// Helper pour initialiser le registre des ports (vide maintenant) 
/// Les ports sont maintenant implémentés comme plugins distribués via MQTT
pub fn create_default_ports(_data_dir: &str) -> Result<PortRegistry, PortError> {
    let registry = PortRegistry::new();
    eprintln!("[ports] initialized empty port registry (ports are now plugins)");
    Ok(registry)
}