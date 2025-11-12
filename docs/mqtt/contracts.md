# MQTT Contracts - Schémas et Validation

> 📋 Registry de contrats JSON pour validation messages MQTT

## 🎯 Principe des Contracts

Les **contracts** définissent les **schémas JSON** attendus pour chaque topic MQTT.

**Objectifs** :
- ✅ **Validation automatique** : rejet messages malformés
- ✅ **Documentation vivante** : schémas = documentation API
- ✅ **Versioning** : évolution schémas sans breaking changes
- ✅ **Type safety** : garantie structure messages

## 🗂️ Contract Registry

**Fichier** : `symbion-kernel/src/contracts.rs` (à créer - recommandation)

```rust
use serde::{Deserialize, Serialize};
use serde_json::Value;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Contract {
    pub topic: String,
    pub version: u32,
    pub schema: Value,  // JSON Schema
    pub examples: Vec<Value>,
}

pub struct ContractRegistry {
    contracts: HashMap<String, Contract>,
}

impl ContractRegistry {
    pub fn validate(&self, topic: &str, payload: &Value) -> Result<(), String> {
        let contract = self.contracts.get(topic)
            .ok_or_else(|| format!("No contract for topic: {}", topic))?;

        // Validation via JSON Schema
        let compiled_schema = JSONSchema::compile(&contract.schema)?;
        if !compiled_schema.is_valid(payload) {
            return Err("Payload does not match schema".to_string());
        }

        Ok(())
    }
}
```

---

## 📄 Contracts par Topic

### `symbion/agents/registration@v1`

**Schema JSON** :
```json
{
  "$schema": "http://json-schema.org/draft-07/schema#",
  "type": "object",
  "required": ["agent_id", "hostname", "platform", "timestamp"],
  "properties": {
    "agent_id": {
      "type": "string",
      "minLength": 1,
      "maxLength": 128,
      "pattern": "^[a-zA-Z0-9_-]+$"
    },
    "hostname": {
      "type": "string",
      "minLength": 1,
      "maxLength": 255
    },
    "platform": {
      "type": "object",
      "required": ["os", "arch"],
      "properties": {
        "os": {
          "type": "string",
          "enum": ["linux", "windows", "macos"]
        },
        "arch": {
          "type": "string",
          "enum": ["x86_64", "aarch64", "armv7"]
        },
        "kernel": {
          "type": "string"
        }
      }
    },
    "network": {
      "type": "object",
      "properties": {
        "ssid": { "type": "string" },
        "local_ip": {
          "type": "string",
          "format": "ipv4"
        },
        "mac_address": {
          "type": "string",
          "pattern": "^([0-9A-Fa-f]{2}[:-]){5}([0-9A-Fa-f]{2})$"
        }
      }
    },
    "capabilities": {
      "type": "array",
      "items": {
        "type": "string",
        "enum": [
          "presence_detection",
          "energy_monitoring",
          "smart_scheduling",
          "wake_on_lan",
          "context_learning"
        ]
      }
    },
    "timestamp": {
      "type": "integer",
      "minimum": 0
    }
  }
}
```

**Exemple valide** :
```json
{
  "agent_id": "eridwyn-Salon",
  "hostname": "eridwyn-Salon",
  "platform": {
    "os": "linux",
    "arch": "x86_64",
    "kernel": "6.14.0-33-generic"
  },
  "network": {
    "ssid": "HomeNetwork",
    "local_ip": "192.168.1.14",
    "mac_address": "00:1A:2B:3C:4D:5E"
  },
  "capabilities": [
    "presence_detection",
    "energy_monitoring"
  ],
  "timestamp": 1699887200
}
```

**Exemple invalide** (agent_id trop long) :
```json
{
  "agent_id": "this-is-a-very-long-agent-id-that-exceeds-128-characters-limit-and-should-be-rejected-by-validation-this-is-a-very-long-agent-id-that-exceeds-128-characters-limit",
  "hostname": "test",
  "platform": { "os": "linux", "arch": "x86_64" },
  "timestamp": 1699887200
}
```

---

### `symbion/agents/heartbeat@v1`

**Schema JSON** :
```json
{
  "$schema": "http://json-schema.org/draft-07/schema#",
  "type": "object",
  "required": ["agent_id", "timestamp", "status"],
  "properties": {
    "agent_id": {
      "type": "string",
      "minLength": 1,
      "maxLength": 128
    },
    "timestamp": {
      "type": "integer",
      "minimum": 0
    },
    "status": {
      "type": "string",
      "enum": ["online", "idle", "busy"]
    },
    "metrics": {
      "type": "object",
      "properties": {
        "cpu_usage": {
          "type": "number",
          "minimum": 0,
          "maximum": 100
        },
        "memory": {
          "type": "object",
          "required": ["total_mb", "used_mb", "percent"],
          "properties": {
            "total_mb": {
              "type": "integer",
              "minimum": 0
            },
            "used_mb": {
              "type": "integer",
              "minimum": 0
            },
            "percent": {
              "type": "number",
              "minimum": 0,
              "maximum": 100
            }
          }
        },
        "disk": {
          "type": "object",
          "properties": {
            "total_gb": { "type": "integer" },
            "used_gb": { "type": "integer" },
            "percent": {
              "type": "number",
              "minimum": 0,
              "maximum": 100
            }
          }
        },
        "uptime_seconds": {
          "type": "integer",
          "minimum": 0
        },
        "temperature": {
          "type": "object",
          "properties": {
            "cpu": {
              "type": ["number", "null"],
              "minimum": -50,
              "maximum": 150
            },
            "gpu": {
              "type": ["number", "null"]
            }
          }
        }
      }
    },
    "processes": {
      "type": "array",
      "items": {
        "type": "object",
        "required": ["name", "cpu", "memory_mb"],
        "properties": {
          "name": { "type": "string" },
          "cpu": {
            "type": "number",
            "minimum": 0,
            "maximum": 100
          },
          "memory_mb": {
            "type": "integer",
            "minimum": 0
          },
          "pid": {
            "type": "integer",
            "minimum": 0
          }
        }
      }
    },
    "network": {
      "type": "object",
      "properties": {
        "ssid": { "type": "string" },
        "signal_strength": {
          "type": "integer",
          "minimum": -100,
          "maximum": 0
        }
      }
    }
  }
}
```

---

### `symbion/agents/command@v1`

**Schema JSON** :
```json
{
  "$schema": "http://json-schema.org/draft-07/schema#",
  "type": "object",
  "required": ["command_id", "agent_id", "command", "timestamp"],
  "properties": {
    "command_id": {
      "type": "string",
      "pattern": "^cmd-[a-f0-9]+$"
    },
    "agent_id": {
      "type": "string",
      "minLength": 1,
      "maxLength": 128
    },
    "command": {
      "type": "string",
      "minLength": 1,
      "maxLength": 1024
    },
    "timeout_seconds": {
      "type": "integer",
      "minimum": 1,
      "maximum": 300,
      "default": 30
    },
    "timestamp": {
      "type": "integer",
      "minimum": 0
    }
  }
}
```

**Validation supplémentaire (whitelisting)** :
```rust
// Après validation schema JSON, vérifier commande autorisée
const ALLOWED_COMMANDS: &[&str] = &[
    "systemctl", "shutdown", "reboot", "hibernate",
    "sensors", "df", "free", "uptime",
];

fn validate_command_whitelist(command: &str) -> bool {
    let first_word = command.split_whitespace().next().unwrap_or("");
    ALLOWED_COMMANDS.contains(&first_word)
}
```

---

### `symbion/agents/response@v1`

**Schema JSON** :
```json
{
  "$schema": "http://json-schema.org/draft-07/schema#",
  "type": "object",
  "required": ["agent_id", "command_id", "success", "timestamp"],
  "properties": {
    "agent_id": { "type": "string" },
    "command_id": {
      "type": "string",
      "pattern": "^cmd-[a-f0-9]+$"
    },
    "success": { "type": "boolean" },
    "output": {
      "type": "string",
      "maxLength": 50000
    },
    "error": { "type": "string" },
    "exit_code": {
      "type": "integer",
      "minimum": 0,
      "maximum": 255
    },
    "timestamp": {
      "type": "integer",
      "minimum": 0
    },
    "duration_ms": {
      "type": "integer",
      "minimum": 0
    }
  }
}
```

---

### `symbion/notes/request@v1`

**Schema JSON** :
```json
{
  "$schema": "http://json-schema.org/draft-07/schema#",
  "type": "object",
  "required": ["request_id", "action", "timestamp"],
  "properties": {
    "request_id": {
      "type": "string",
      "pattern": "^req-[a-f0-9]+$"
    },
    "action": {
      "type": "string",
      "enum": ["create", "read", "update", "delete"]
    },
    "note_id": {
      "type": "string",
      "pattern": "^note-[a-f0-9]+$"
    },
    "data": {
      "type": "object",
      "properties": {
        "content": {
          "type": "string",
          "minLength": 1,
          "maxLength": 10000
        },
        "context": {
          "type": "string",
          "enum": ["cravate", "intime", "neutre"]
        },
        "tags": {
          "type": "array",
          "items": {
            "type": "string",
            "maxLength": 50
          },
          "maxItems": 10
        }
      }
    },
    "filters": {
      "type": "object",
      "properties": {
        "context": {
          "type": "string",
          "enum": ["cravate", "intime", "neutre"]
        },
        "tags": {
          "type": "array",
          "items": { "type": "string" }
        }
      }
    },
    "timestamp": {
      "type": "integer",
      "minimum": 0
    }
  }
}
```

---

### `symbion/notes/response@v1`

**Schema JSON** :
```json
{
  "$schema": "http://json-schema.org/draft-07/schema#",
  "type": "object",
  "required": ["request_id", "success"],
  "properties": {
    "request_id": {
      "type": "string",
      "pattern": "^req-[a-f0-9]+$"
    },
    "success": { "type": "boolean" },
    "data": {
      "oneOf": [
        {
          "type": "object",
          "properties": {
            "id": { "type": "string" },
            "content": { "type": "string" },
            "context": { "type": "string" },
            "tags": { "type": "array" },
            "created_at": { "type": "integer" },
            "updated_at": { "type": "integer" }
          }
        },
        {
          "type": "array",
          "items": {
            "type": "object"
          }
        }
      ]
    },
    "error": { "type": "string" }
  }
}
```

---

### `symbion/dashboard/update@v1`

**Schema JSON** :
```json
{
  "$schema": "http://json-schema.org/draft-07/schema#",
  "type": "object",
  "required": ["event_type", "timestamp"],
  "properties": {
    "event_type": {
      "type": "string",
      "enum": [
        "agent_heartbeat",
        "agent_offline",
        "context_change",
        "note_created",
        "note_updated",
        "note_deleted"
      ]
    },
    "agent_id": { "type": "string" },
    "old_mode": { "type": "string" },
    "new_mode": { "type": "string" },
    "reason": { "type": "string" },
    "metrics": { "type": "object" },
    "timestamp": {
      "type": "integer",
      "minimum": 0
    }
  }
}
```

---

### `symbion/dashboard/notification@v1`

**Schema JSON** :
```json
{
  "$schema": "http://json-schema.org/draft-07/schema#",
  "type": "object",
  "required": ["level", "title", "message", "timestamp"],
  "properties": {
    "level": {
      "type": "string",
      "enum": ["info", "warning", "error"]
    },
    "title": {
      "type": "string",
      "minLength": 1,
      "maxLength": 100
    },
    "message": {
      "type": "string",
      "minLength": 1,
      "maxLength": 500
    },
    "action": {
      "type": "object",
      "required": ["type"],
      "properties": {
        "type": {
          "type": "string",
          "enum": ["open_url", "trigger_action", "dismiss"]
        },
        "url": {
          "type": "string",
          "format": "uri-reference"
        },
        "action_id": { "type": "string" }
      }
    },
    "timestamp": {
      "type": "integer",
      "minimum": 0
    },
    "expires_at": {
      "type": "integer",
      "minimum": 0
    }
  }
}
```

---

### `symbion/system/health@v1`

**Schema JSON** :
```json
{
  "$schema": "http://json-schema.org/draft-07/schema#",
  "type": "object",
  "required": ["status", "components", "timestamp"],
  "properties": {
    "status": {
      "type": "string",
      "enum": ["healthy", "degraded", "critical"]
    },
    "components": {
      "type": "object",
      "required": ["mqtt", "agents", "plugins"],
      "properties": {
        "mqtt": {
          "type": "object",
          "required": ["status", "connected"],
          "properties": {
            "status": {
              "type": "string",
              "enum": ["healthy", "degraded", "critical"]
            },
            "connected": { "type": "boolean" },
            "messages_processed": {
              "type": "integer",
              "minimum": 0
            }
          }
        },
        "agents": {
          "type": "object",
          "required": ["status", "online", "offline"],
          "properties": {
            "status": { "type": "string" },
            "online": {
              "type": "integer",
              "minimum": 0
            },
            "offline": {
              "type": "integer",
              "minimum": 0
            }
          }
        },
        "plugins": {
          "type": "object",
          "required": ["status", "running", "failed"],
          "properties": {
            "status": { "type": "string" },
            "running": {
              "type": "integer",
              "minimum": 0
            },
            "failed": {
              "type": "integer",
              "minimum": 0
            }
          }
        }
      }
    },
    "uptime_seconds": {
      "type": "integer",
      "minimum": 0
    },
    "timestamp": {
      "type": "integer",
      "minimum": 0
    }
  }
}
```

---

## 🛠️ Implémentation Validation

### Setup Contract Registry

**Fichier** : `symbion-kernel/src/contracts.rs`

```rust
use serde_json::Value;
use jsonschema::{JSONSchema, ValidationError};
use std::collections::HashMap;

pub struct ContractRegistry {
    schemas: HashMap<String, JSONSchema>,
}

impl ContractRegistry {
    pub fn new() -> Self {
        let mut registry = ContractRegistry {
            schemas: HashMap::new(),
        };

        // Charger tous les schemas
        registry.register_all_schemas();
        registry
    }

    fn register_all_schemas(&mut self) {
        // Agent registration
        let registration_schema = serde_json::json!({
            "$schema": "http://json-schema.org/draft-07/schema#",
            "type": "object",
            "required": ["agent_id", "hostname", "platform", "timestamp"],
            // ... (voir schemas ci-dessus)
        });
        self.register("symbion/agents/registration@v1", registration_schema);

        // Heartbeat
        // ...
    }

    fn register(&mut self, topic: &str, schema: Value) {
        let compiled = JSONSchema::compile(&schema)
            .expect(&format!("Failed to compile schema for {}", topic));
        self.schemas.insert(topic.to_string(), compiled);
    }

    pub fn validate(&self, topic: &str, payload: &Value) -> Result<(), Vec<String>> {
        let schema = self.schemas.get(topic)
            .ok_or_else(|| vec![format!("No schema for topic: {}", topic)])?;

        let errors: Vec<String> = schema
            .validate(payload)
            .err()
            .map(|errors| errors.map(|e| e.to_string()).collect())
            .unwrap_or_default();

        if errors.is_empty() {
            Ok(())
        } else {
            Err(errors)
        }
    }
}
```

### Validation dans MQTT Listener

**Fichier** : `symbion-kernel/src/mqtt.rs`

```rust
// Intégrer validation dans event loop
while let Ok(event) = eventloop.poll().await {
    if let Event::Incoming(Packet::Publish(p)) = event {
        let topic = p.topic.clone();
        let payload: Value = serde_json::from_slice(&p.payload)?;

        // Validation contract
        if let Err(errors) = app.contract_registry.validate(&topic, &payload) {
            eprintln!("[mqtt] Invalid payload on {}: {:?}", topic, errors);
            continue;  // Ignorer message invalide
        }

        // Traitement normal
        handle_message(&topic, payload).await;
    }
}
```

---

## 📖 Endpoint Registry

**API HTTP** pour consulter contracts :

### `GET /contracts`

```bash
curl https://localhost:8443/contracts \
  -H "Authorization: Bearer $JWT"
```

**Response** :
```json
{
  "contracts": [
    {
      "topic": "symbion/agents/registration@v1",
      "version": 1,
      "schema": { /* JSON Schema */ },
      "examples": [ /* payloads valides */ ]
    }
  ]
}
```

### `GET /contracts/{topic}`

```bash
curl https://localhost:8443/contracts/symbion%2Fagents%2Fheartbeat%40v1 \
  -H "Authorization: Bearer $JWT"
```

**Response** :
```json
{
  "topic": "symbion/agents/heartbeat@v1",
  "version": 1,
  "schema": { /* JSON Schema complet */ },
  "examples": [
    { /* exemple 1 */ },
    { /* exemple 2 */ }
  ]
}
```

---

## 🧪 Tests Validation

**Fichier** : `symbion-kernel/tests/contracts_test.rs`

```rust
#[test]
fn test_registration_valid() {
    let registry = ContractRegistry::new();
    let payload = serde_json::json!({
        "agent_id": "test-agent",
        "hostname": "test-host",
        "platform": {
            "os": "linux",
            "arch": "x86_64"
        },
        "timestamp": 1699887200
    });

    assert!(registry.validate("symbion/agents/registration@v1", &payload).is_ok());
}

#[test]
fn test_registration_invalid_missing_field() {
    let registry = ContractRegistry::new();
    let payload = serde_json::json!({
        "agent_id": "test-agent",
        // hostname manquant
        "platform": { "os": "linux", "arch": "x86_64" },
        "timestamp": 1699887200
    });

    assert!(registry.validate("symbion/agents/registration@v1", &payload).is_err());
}

#[test]
fn test_heartbeat_metrics_ranges() {
    let registry = ContractRegistry::new();
    let payload = serde_json::json!({
        "agent_id": "test",
        "timestamp": 1699887200,
        "status": "online",
        "metrics": {
            "cpu_usage": 150.0  // INVALIDE: > 100
        }
    });

    assert!(registry.validate("symbion/agents/heartbeat@v1", &payload).is_err());
}
```

---

## 📊 Statistiques Contracts

- **Total topics** : 13
- **Contracts définis** : 10 (topics bidirectionnels)
- **Champs requis** : 3-5 par contract
- **Validations** : Types, ranges, patterns, enums
- **Taille max payload** : 50 KB (command output), 10 KB (notes), 1 KB (autres)

---

**Dernière mise à jour** : 2025-11-12
**Bibliothèque validation** : `jsonschema` crate (Rust)
**Standard** : JSON Schema Draft-07
