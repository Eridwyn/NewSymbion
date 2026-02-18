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
 * - kernel.health@v1 : métriques infrastructure kernel
 * - agents.registration@v1 : agents s'annoncent au kernel  
 * - agents.heartbeat@v1 : télémétrie agents (système, processus, services)
 * - agents.command@v1 : kernel → agent (shutdown, reboot, kill_process, run_command)
 * - agents.response@v1 : agent → kernel (résultats commandes + erreurs)
 * - notes.command@v1 : commandes vers plugin notes (create/list/update/delete)
 * - notes.response@v1 : réponses du plugin notes (success/error)
 * 
 * EXEMPLE CONTRAT JSON :
 * ```json
 * {
 *   "topic": "symbion/agents/heartbeat@v1",
 *   "schema": {
 *     "agent_id": "string",
 *     "status": "online",
 *     "system": {"cpu": "object", "memory": "object", "disk": "array"}
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
    /// Vérifie le parsing JSON et les champs obligatoires du schéma
    #[allow(dead_code)]
    pub fn validate_message(&self, topic: &str, payload: &str) -> Result<(), String> {
        let contract_name = extract_contract_name(topic);

        let contract = self.contracts.get(&contract_name)
            .ok_or_else(|| format!("Contrat '{}' inconnu", contract_name))?;

        let value: serde_json::Value = serde_json::from_str(payload)
            .map_err(|e| format!("JSON invalide: {}", e))?;

        // Validate required top-level fields from schema
        if let Some(required) = contract.schema.get("required").and_then(|r| r.as_array()) {
            for field in required {
                if let Some(field_name) = field.as_str() {
                    if value.get(field_name).is_none() {
                        return Err(format!(
                            "Champ obligatoire '{}' manquant (contrat {})",
                            field_name, contract_name
                        ));
                    }
                }
            }
        }

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
/// Transformation : "symbion/agents/command@v1" -> "agents.command@v1"
/// Transformation : "symbion/hosts/heartbeat@v2" -> "hosts.heartbeat@v2" 
fn extract_contract_name(topic: &str) -> String {
    let parts: Vec<&str> = topic.split('/').collect();
    if parts.len() >= 3 && parts[0] == "symbion" {
        // Nouveau format: symbion/{namespace}/{event}@{version} -> {namespace}.{event}@{version}
        format!("{}.{}", parts[1], parts[2])
    } else {
        // Fallback pour topics non-standards
        topic.split('/').last().unwrap_or(topic).to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_extract_contract_name() {
        assert_eq!(extract_contract_name("symbion/agents/heartbeat@v1"), "agents.heartbeat@v1");
        assert_eq!(extract_contract_name("symbion/agents/command@v1"), "agents.command@v1");
        assert_eq!(extract_contract_name("symbion/notes/command@v1"), "notes.command@v1");
        assert_eq!(extract_contract_name("symbion/kernel/health@v1"), "kernel.health@v1");
        assert_eq!(extract_contract_name("heartbeat@v2"), "heartbeat@v2");
        assert_eq!(extract_contract_name("symbion/memo/created@v1"), "memo.created@v1");
    }
}