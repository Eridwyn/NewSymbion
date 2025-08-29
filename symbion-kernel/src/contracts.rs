use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::Path;
use tokio::fs;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Contract {
    pub topic: String,
    pub schema: serde_json::Value,
}

#[derive(Debug, Clone)]
pub struct ContractRegistry {
    contracts: HashMap<String, Contract>, // "heartbeat@v2" -> Contract
}

impl ContractRegistry {
    pub fn new() -> Self {
        Self {
            contracts: HashMap::new(),
        }
    }

    /// Charge tous les contrats depuis le dossier contracts/
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
                                eprintln!("[registry] loaded contract: {}", contract_name);
                                registry.contracts.insert(contract_name, contract);
                            }
                            Err(e) => eprintln!("[registry] invalid JSON in {:?}: {}", path, e),
                        }
                    }
                    Err(e) => eprintln!("[registry] failed to read {:?}: {}", path, e),
                }
            }
        }
        
        Ok(registry)
    }

    /// Valide qu'un message JSON respecte le contrat
    pub fn validate_message(&self, topic: &str, payload: &str) -> Result<(), String> {
        let contract_name = extract_contract_name(topic);
        
        let _contract = self.contracts.get(&contract_name)
            .ok_or_else(|| format!("Contract '{}' not found", contract_name))?;

        // Pour l'instant, validation basique : juste parse JSON
        // TODO: validation JSON Schema complète
        serde_json::from_str::<serde_json::Value>(payload)
            .map_err(|e| format!("Invalid JSON: {}", e))?;

        Ok(())
    }

    /// Retourne tous les contrats enregistrés
    pub fn list_contracts(&self) -> Vec<String> {
        self.contracts.keys().cloned().collect()
    }

    /// Retourne un contrat spécifique
    pub fn get_contract(&self, contract_name: &str) -> Option<&Contract> {
        self.contracts.get(contract_name)
    }
}

/// Extrait le nom du contrat depuis le topic
/// Ex: "symbion/hosts/heartbeat@v2" -> "heartbeat@v2"
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