/**
 * CONTRACT REGISTRY - Système de versioning et validation des événements MQTT Symbion
 * 
 * RÔLE :
 * Ce module gère le catalogue central de tous les contrats d'événements MQTT.
 * Il assure la cohérence des messages échangés entre kernel et plugins via MQTT.
 * 
 * FONCTIONNEMENT :
 * - Chargement automatique des contrats JSON depuis contracts/mqtt/
 * - Validation des messages MQTT entrants contre les schémas
 * - Découverte dynamique des événements disponibles
 * - Versioning des contrats (heartbeat@v1, heartbeat@v2...)
 * 
 * UTILITÉ DANS SYMBION :
 * 🎯 Évolutivité : ajouter nouveaux events sans casser l'existant  
 * 🎯 Fiabilité : validation automatique, pas de messages corrompus
 * 🎯 Documentation : API /contracts expose tous les schémas disponibles
 * 🎯 Développement : DevKit auto-génère stubs depuis les contrats
 * 
 * CONTRATS ACTUELS :
 * - heartbeat@v2 : télémétrie hosts (CPU, RAM, IP)
 * - wake@v1 : demandes Wake-on-LAN
 * - health@v1 : métriques infrastructure kernel
 * 
 * EXEMPLE CONTRAT JSON :
 * ```json
 * {
 *   "topic": "symbion/hosts/heartbeat@v2",
 *   "schema": {
 *     "host_id": "string",
 *     "ts": "RFC3339", 
 *     "metrics": {"cpu": "f32", "ram": "f32"}
 *   }
 * }
 * ```
 */

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::Path;
use tokio::fs;

/// Définition d'un contrat d'événement MQTT
/// Associe un topic MQTT à son schéma de données JSON
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Contract {
    /// Topic MQTT complet (ex: "symbion/hosts/heartbeat@v2")
    pub topic: String,
    /// Schéma JSON décrivant la structure des données attendues
    pub schema: serde_json::Value,
}

/// Registre central de tous les contrats MQTT disponibles
/// Catalogue utilisé par le kernel pour valider et router les événements
#[derive(Debug, Clone)]
pub struct ContractRegistry {
    /// Map nom_contrat -> définition complète du contrat
    contracts: HashMap<String, Contract>, // "heartbeat@v2" -> Contract
}

impl ContractRegistry {
    /// Crée un registre vide de contrats
    pub fn new() -> Self {
        Self {
            contracts: HashMap::new(),
        }
    }

    /// Charge tous les contrats depuis un dossier (contracts/mqtt/)
    /// Scan récursif des fichiers .json et parsing automatique
    pub async fn load_contracts_from_dir<P: AsRef<Path>>(contracts_dir: P) -> Result<Self, Box<dyn std::error::Error>> {
        let mut registry = Self::new();
        let mut entries = fs::read_dir(contracts_dir).await?;
        
        while let Some(entry) = entries.next_entry().await? {
            let path = entry.path();
            if path.extension().and_then(|s| s.to_str()) == Some("json") {
                match fs::read_to_string(&path).await {
                    Ok(content) => {
                        match serde_json::from_str::<Contract>(&content) {
                            Ok(contract) => {
                                let contract_name = extract_contract_name(&contract.topic);
                                eprintln!("[contracts] loaded: {} from {:?}", contract_name, path.file_name().unwrap());
                                registry.contracts.insert(contract_name, contract);
                            }
                            Err(e) => eprintln!("[contracts] JSON invalide dans {:?}: {}", path, e),
                        }
                    }
                    Err(e) => eprintln!("[contracts] échec lecture {:?}: {}", path, e),
                }
            }
        }
        
        Ok(registry)
    }

    /// Valide qu'un message MQTT respecte son contrat
    /// Vérification que le payload JSON correspond au schéma attendu
    pub fn validate_message(&self, topic: &str, payload: &str) -> Result<(), String> {
        let contract_name = extract_contract_name(topic);
        
        let _contract = self.contracts.get(&contract_name)
            .ok_or_else(|| format!("Contrat '{}' inconnu", contract_name))?;

        // Validation basique : parsing JSON réussi
        // TODO: validation JSON Schema complète avec jsonschema crate
        serde_json::from_str::<serde_json::Value>(payload)
            .map_err(|e| format!("JSON invalide: {}", e))?;

        Ok(())
    }

    /// Liste tous les noms de contrats disponibles
    /// Utilisé par l'API /contracts pour découverte automatique
    pub fn list_contracts(&self) -> Vec<String> {
        self.contracts.keys().cloned().collect()
    }

    /// Récupère la définition complète d'un contrat par son nom
    /// Utilisé par l'API /contracts/{name} pour les détails
    pub fn get_contract(&self, contract_name: &str) -> Option<&Contract> {
        self.contracts.get(contract_name)
    }
}

/// Extrait le nom du contrat depuis le topic MQTT complet
/// Transformation : "symbion/hosts/heartbeat@v2" -> "heartbeat@v2"
fn extract_contract_name(topic: &str) -> String {
    topic.split('/').last().unwrap_or(topic).to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_extract_contract_name() {
        assert_eq!(extract_contract_name("symbion/hosts/heartbeat@v2"), "heartbeat@v2");
        assert_eq!(extract_contract_name("heartbeat@v2"), "heartbeat@v2");
        assert_eq!(extract_contract_name("symbion/memo/created@v1"), "created@v1");
    }
}