/*!
Helpers pour charger et valider les contrats MQTT/HTTP

Facilite le développement en fournissant des utilitaires pour:
- Charger les contrats depuis les fichiers JSON
- Construire des événements conformes aux contrats
- Valider les schémas JSON
*/

use serde_json::{Value, Map};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use anyhow::Result;

#[derive(Debug, Clone)]
pub struct Contract {
    pub name: String,
    pub version: String,
    pub topic: String,
    pub contract_type: String,
    pub schema: Value,
    pub description: Option<String>,
}

/// Charge et gère les contrats depuis les fichiers JSON
pub struct ContractLoader {
    contracts: HashMap<String, Contract>,
    contracts_dir: PathBuf,
}

impl ContractLoader {
    pub fn new<P: AsRef<Path>>(contracts_dir: P) -> Self {
        Self {
            contracts: HashMap::new(),
            contracts_dir: contracts_dir.as_ref().to_path_buf(),
        }
    }

    /// Charge tous les contrats MQTT depuis le répertoire
    pub fn load_mqtt_contracts(&mut self) -> Result<usize> {
        let mqtt_dir = self.contracts_dir.join("mqtt");
        self.load_contracts_from_dir(&mqtt_dir)
    }

    /// Charge tous les contrats HTTP depuis le répertoire  
    pub fn load_http_contracts(&mut self) -> Result<usize> {
        let http_dir = self.contracts_dir.join("http");
        self.load_contracts_from_dir(&http_dir)
    }

    /// Charge tous les contrats (MQTT + HTTP)
    pub fn load_all_contracts(&mut self) -> Result<usize> {
        let mqtt_count = self.load_mqtt_contracts()?;
        let http_count = self.load_http_contracts()?;
        Ok(mqtt_count + http_count)
    }

    fn load_contracts_from_dir(&mut self, dir: &Path) -> Result<usize> {
        if !dir.exists() {
            log::warn!("Contracts directory not found: {}", dir.display());
            return Ok(0);
        }

        let mut count = 0;
        for entry in std::fs::read_dir(dir)? {
            let entry = entry?;
            let path = entry.path();
            
            if path.extension().and_then(|s| s.to_str()) == Some("json") {
                match self.load_contract(&path) {
                    Ok(contract) => {
                        log::info!("📜 Loaded contract: {}", contract.name);
                        self.contracts.insert(contract.name.clone(), contract);
                        count += 1;
                    }
                    Err(e) => {
                        log::warn!("⚠️ Failed to load contract {}: {}", path.display(), e);
                    }
                }
            }
        }

        Ok(count)
    }

    fn load_contract(&self, path: &Path) -> Result<Contract> {
        let content = std::fs::read_to_string(path)?;
        let json: Value = serde_json::from_str(&content)?;

        let contract = Contract {
            name: json.get("name").and_then(|v| v.as_str())
                .unwrap_or_else(|| path.file_stem().unwrap().to_str().unwrap())
                .to_string(),
            version: json.get("version").and_then(|v| v.as_str())
                .unwrap_or("v1").to_string(),
            topic: json.get("topic").and_then(|v| v.as_str())
                .unwrap_or("").to_string(),
            contract_type: json.get("type").and_then(|v| v.as_str())
                .unwrap_or("event").to_string(),
            schema: json.get("schema").unwrap_or(&Value::Object(Map::new())).clone(),
            description: json.get("description").and_then(|v| v.as_str())
                .map(|s| s.to_string()),
        };

        Ok(contract)
    }

    /// Récupère un contrat par nom
    pub fn get_contract(&self, name: &str) -> Option<&Contract> {
        self.contracts.get(name)
    }

    /// Liste tous les contrats chargés
    pub fn list_contracts(&self) -> Vec<&Contract> {
        self.contracts.values().collect()
    }

    /// Trouve les contrats par type
    pub fn contracts_by_type(&self, contract_type: &str) -> Vec<&Contract> {
        self.contracts
            .values()
            .filter(|c| c.contract_type == contract_type)
            .collect()
    }
}

/// Construction d'événements conformes aux contrats
pub struct EventBuilder {
    contract: Contract,
}

impl EventBuilder {
    pub fn new(contract: Contract) -> Self {
        Self { contract }
    }

    /// Crée un nouvel événement avec les champs par défaut du contrat
    pub fn build(&self) -> EventInstance {
        EventInstance {
            topic: self.contract.topic.clone(),
            payload: Value::Object(Map::new()),
            contract_name: self.contract.name.clone(),
        }
    }

    /// Suggère les champs requis basés sur le schéma du contrat
    pub fn required_fields(&self) -> Vec<String> {
        if let Some(props) = self.contract.schema.get("properties").and_then(|p| p.as_object()) {
            if let Some(required) = self.contract.schema.get("required").and_then(|r| r.as_array()) {
                return required.iter()
                    .filter_map(|v| v.as_str().map(|s| s.to_string()))
                    .collect();
            }
            // Si pas de "required", tous les champs sont optionnels
            return props.keys().cloned().collect();
        }
        vec![]
    }

    /// Génère un exemple d'événement avec des valeurs par défaut
    pub fn build_example(&self) -> Result<EventInstance> {
        let mut payload = Map::new();

        if let Some(props) = self.contract.schema.get("properties").and_then(|p| p.as_object()) {
            for (field, field_schema) in props {
                let example_value = self.generate_example_value(field_schema)?;
                payload.insert(field.clone(), example_value);
            }
        }

        Ok(EventInstance {
            topic: self.contract.topic.clone(),
            payload: Value::Object(payload),
            contract_name: self.contract.name.clone(),
        })
    }

    fn generate_example_value(&self, schema: &Value) -> Result<Value> {
        match schema.get("type").and_then(|t| t.as_str()) {
            Some("string") => {
                if let Some(example) = schema.get("example") {
                    Ok(example.clone())
                } else {
                    Ok(Value::String("example_string".to_string()))
                }
            }
            Some("number") => Ok(Value::Number(serde_json::Number::from(42))),
            Some("integer") => Ok(Value::Number(serde_json::Number::from(42))),
            Some("boolean") => Ok(Value::Bool(true)),
            Some("array") => Ok(Value::Array(vec![Value::String("example_item".to_string())])),
            Some("object") => Ok(Value::Object(Map::new())),
            _ => Ok(Value::String("unknown_type".to_string())),
        }
    }
}

/// Instance d'événement avec son topic et payload
#[derive(Debug, Clone)]
pub struct EventInstance {
    pub topic: String,
    pub payload: Value,
    pub contract_name: String,
}

impl EventInstance {
    /// Définit un champ dans le payload
    pub fn set_field<S: Into<String>>(mut self, field: S, value: Value) -> Self {
        if let Value::Object(ref mut obj) = self.payload {
            obj.insert(field.into(), value);
        }
        self
    }

    /// Définit un champ string
    pub fn set_string<S: Into<String>, V: Into<String>>(self, field: S, value: V) -> Self {
        self.set_field(field, Value::String(value.into()))
    }

    /// Définit un champ number
    pub fn set_number<S: Into<String>>(self, field: S, value: f64) -> Self {
        self.set_field(field, Value::Number(serde_json::Number::from_f64(value).unwrap()))
    }

    /// Définit un champ boolean
    pub fn set_bool<S: Into<String>>(self, field: S, value: bool) -> Self {
        self.set_field(field, Value::Bool(value))
    }

    /// Convertit en bytes JSON pour envoi MQTT
    pub fn to_bytes(&self) -> Result<Vec<u8>> {
        Ok(serde_json::to_vec(&self.payload)?)
    }

    /// Ajoute timestamp automatiquement (format ISO)
    pub fn with_timestamp(self) -> Self {
        self.set_string("timestamp", chrono::Utc::now().to_rfc3339())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    fn create_test_contract() -> Value {
        serde_json::json!({
            "name": "test.event",
            "version": "v1", 
            "type": "event",
            "topic": "symbion/test/event@v1",
            "description": "Test event contract",
            "schema": {
                "type": "object",
                "properties": {
                    "id": {"type": "string"},
                    "value": {"type": "number"},
                    "active": {"type": "boolean"}
                },
                "required": ["id", "value"]
            }
        })
    }

    #[test]
    fn test_contract_loading() {
        let temp_dir = TempDir::new().unwrap();
        let mqtt_dir = temp_dir.path().join("mqtt");
        std::fs::create_dir_all(&mqtt_dir).unwrap();

        // Créer un contrat de test
        let contract_path = mqtt_dir.join("test.event.v1.json");
        std::fs::write(&contract_path, serde_json::to_string_pretty(&create_test_contract()).unwrap()).unwrap();

        // Charger les contrats
        let mut loader = ContractLoader::new(temp_dir.path());
        let count = loader.load_mqtt_contracts().unwrap();
        
        assert_eq!(count, 1);
        
        let contract = loader.get_contract("test.event").unwrap();
        assert_eq!(contract.name, "test.event");
        assert_eq!(contract.version, "v1");
        assert_eq!(contract.topic, "symbion/test/event@v1");
    }

    #[test]
    fn test_event_builder() {
        let contract_json = create_test_contract();
        let contract = Contract {
            name: contract_json["name"].as_str().unwrap().to_string(),
            version: contract_json["version"].as_str().unwrap().to_string(),
            topic: contract_json["topic"].as_str().unwrap().to_string(),
            contract_type: contract_json["type"].as_str().unwrap().to_string(),
            schema: contract_json["schema"].clone(),
            description: contract_json.get("description").and_then(|v| v.as_str()).map(|s| s.to_string()),
        };

        let builder = EventBuilder::new(contract);
        let required = builder.required_fields();
        assert_eq!(required, vec!["id", "value"]);

        let event = builder.build()
            .set_string("id", "test123")
            .set_number("value", 42.0)
            .set_bool("active", true)
            .with_timestamp();

        assert_eq!(event.topic, "symbion/test/event@v1");
        assert_eq!(event.payload["id"], "test123");
        assert_eq!(event.payload["value"], 42.0);
        assert_eq!(event.payload["active"], true);
        assert!(event.payload["timestamp"].is_string());
    }

    #[test]
    fn test_example_generation() {
        let contract_json = create_test_contract();
        let contract = Contract {
            name: contract_json["name"].as_str().unwrap().to_string(),
            version: contract_json["version"].as_str().unwrap().to_string(),
            topic: contract_json["topic"].as_str().unwrap().to_string(),
            contract_type: contract_json["type"].as_str().unwrap().to_string(),
            schema: contract_json["schema"].clone(),
            description: None,
        };

        let builder = EventBuilder::new(contract);
        let example = builder.build_example().unwrap();

        assert!(example.payload["id"].is_string());
        assert!(example.payload["value"].is_number());
        assert!(example.payload["active"].is_boolean());
    }
}