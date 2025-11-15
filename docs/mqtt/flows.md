# MQTT Message Flows - Symbion

> 🔄 Patterns de communication et séquences de messages

## 🎯 Patterns de Communication

Symbion utilise **4 patterns principaux** de communication MQTT :

| Pattern | Description | Use Case | Topics |
|---------|-------------|----------|--------|
| **Fire & Forget** | Publish sans attendre réponse | Heartbeats, événements | `heartbeat@v1`, `event@v1` |
| **Request-Response** | Publish + attente réponse | Commandes agents | `command@v1` → `response@v1` |
| **Pub/Sub Broadcast** | Notification multicast | Dashboard updates | `dashboard/update@v1` |
| **Plugin RPC** | Kernel ↔ Plugin via MQTT | CRUD notes | `notes/request@v1` ↔ `notes/response@v1` |

---

## 🤖 Flow 1: Agent Lifecycle (Registration + Heartbeats)

### Séquence Complète

```
┌────────────┐                  ┌────────────┐                  ┌────────────┐
│   Agent    │                  │   Broker   │                  │   Kernel   │
└─────┬──────┘                  └─────┬──────┘                  └─────┬──────┘
      │                               │                               │
      │  1. Connexion MQTT            │                               │
      ├──────────────────────────────►│                               │
      │                               │                               │
      │  2. Subscribe command@v1      │                               │
      ├──────────────────────────────►│                               │
      │                               │                               │
      │  3. Publish registration@v1   │                               │
      ├──────────────────────────────►│                               │
      │                               │                               │
      │                               │  4. Forward registration      │
      │                               ├──────────────────────────────►│
      │                               │                               │
      │                               │                               │  5. Update registry
      │                               │                               │     add agent to pool
      │                               │                               │
      │                               │  6. Ack (QoS 1)               │
      │◄──────────────────────────────┤                               │
      │                               │                               │
      │  7. Loop: Heartbeat (30s)     │                               │
      ├──────────────────────────────►│                               │
      │                               │                               │
      │                               │  8. Forward heartbeat         │
      │                               ├──────────────────────────────►│
      │                               │                               │
      │                               │                               │  9. Update metrics
      │                               │                               │     mark agent online
      │                               │                               │
      │                               │ 10. Publish dashboard/update  │
      │                               │◄──────────────────────────────┤
      │                               │                               │
      │                               │ 11. Forward to PWA            │
      │                               ├─────────────────┐             │
      │                               │                 │             │
```

### Étapes Détaillées

**1. Démarrage Agent**
```rust
// symbion-agent-host/src/main.rs:253-302
let mut mqttoptions = MqttOptions::new(agent_id, "127.0.0.1", 1883);
let (client, mut eventloop) = AsyncClient::new(mqttoptions, 10);

// Subscribe commandes
client.subscribe("symbion/agents/command@v1", QoS::AtLeastOnce).await?;
```

**2. Registration**
```rust
// symbion-agent-host/src/main.rs:304-329
let registration = AgentRegistration {
    agent_id: agent_id.clone(),
    hostname: get_hostname(),
    platform: detect_platform(),
    network: detect_network(),
    capabilities: vec!["presence_detection", "energy_monitoring"],
    timestamp: Utc::now().timestamp(),
};

client.publish(
    "symbion/agents/registration@v1",
    QoS::AtLeastOnce,
    false,
    serde_json::to_vec(&registration)?
).await?;
```

**3. Reception Kernel**
```rust
// symbion-kernel/src/mqtt.rs:70
"symbion/agents/registration@v1" => {
    let reg: AgentRegistration = serde_json::from_slice(&payload)?;
    app.agent_registry.register(reg).await;
    println!("[agents] Agent {} registered", reg.agent_id);
}
```

**4. Heartbeat Loop**
```rust
// symbion-agent-host/src/main.rs:331-360
let mut interval = tokio::time::interval(Duration::from_secs(30));

loop {
    interval.tick().await;

    let heartbeat = AgentHeartbeat {
        agent_id: agent_id.clone(),
        timestamp: Utc::now().timestamp(),
        status: "online",
        metrics: collect_metrics(),
        processes: get_top_processes(),
        network: get_network_info(),
    };

    client.publish(
        "symbion/agents/heartbeat@v1",
        QoS::AtLeastOnce,
        false,
        serde_json::to_vec(&heartbeat)?
    ).await?;
}
```

**5. Dashboard Update**
```rust
// symbion-kernel/src/mqtt.rs:152-158
"symbion/agents/heartbeat@v1" => {
    let heartbeat: AgentHeartbeat = serde_json::from_slice(&payload)?;

    app.agent_registry.update_heartbeat(heartbeat.clone()).await;

    // Notifier dashboard temps réel (utiliser dashboard/agents@v1 pour updates agents)
    client.publish(
        "symbion/dashboard/agents@v1",
        QoS::AtLeastOnce,
        false,
        serde_json::to_vec(&AgentUpdate {
            agent_id: heartbeat.agent_id,
            status: "online",
            metrics: heartbeat.metrics,
        })?
    ).await?;
}
```

---

## 🎛️ Flow 2: Agent Command Execution (Request-Response)

### Séquence Complète

```
┌────────────┐     ┌────────────┐     ┌────────────┐     ┌────────────┐
│  PWA/API   │     │   Kernel   │     │   Broker   │     │   Agent    │
└─────┬──────┘     └─────┬──────┘     └─────┬──────┘     └─────┬──────┘
      │                  │                  │                  │
      │ 1. POST /agents/{id}/shutdown      │                  │
      ├─────────────────►│                  │                  │
      │                  │                  │                  │
      │                  │  2. Generate cmd_id                 │
      │                  │     Create pending_command          │
      │                  │                  │                  │
      │                  │  3. Publish command@v1             │
      │                  ├─────────────────►│                  │
      │                  │                  │                  │
      │                  │                  │  4. Forward       │
      │                  │                  ├─────────────────►│
      │                  │                  │                  │
      │                  │                  │                  │  5. Validate whitelist
      │                  │                  │                  │     Execute: shutdown
      │                  │                  │                  │
      │                  │                  │  6. Publish response@v1
      │                  │                  │◄─────────────────┤
      │                  │                  │                  │
      │                  │  7. Forward       │                  │
      │                  │◄─────────────────┤                  │
      │                  │                  │                  │
      │                  │  8. Match cmd_id  │                  │
      │                  │     Resolve future│                  │
      │                  │                  │                  │
      │  9. 200 OK       │                  │                  │
      │◄─────────────────┤                  │                  │
      │  {"success": true, "output": "..."}│                  │
      │                  │                  │                  │
```

### Étapes Détaillées

**1. API Request**
```bash
CSRF_TOKEN=$(curl -s https://localhost:8443/csrf-token \
  -H "Authorization: Bearer $JWT" | jq -r '.token')

curl -X POST https://localhost:8443/agents/eridwyn-Salon/shutdown \
  -H "Authorization: Bearer $JWT" \
  -H "X-CSRF-Token: $CSRF_TOKEN"
```

**2. Kernel: Publish Command**
```rust
// symbion-kernel/src/agents.rs:418
pub async fn send_command(
    &self,
    agent_id: &str,
    command: &str
) -> Result<CommandResponse, String> {
    let command_id = format!("cmd-{}", uuid::Uuid::new_v4().simple());

    // Créer future pour attendre réponse
    let (tx, rx) = oneshot::channel();
    self.pending_commands.lock().await.insert(command_id.clone(), tx);

    // Publier commande
    let payload = CommandPayload {
        command_id: command_id.clone(),
        agent_id: agent_id.to_string(),
        command: command.to_string(),
        timeout_seconds: 30,
        timestamp: Utc::now().timestamp(),
    };

    self.mqtt_client.publish(
        "symbion/agents/command@v1",
        QoS::AtLeastOnce,
        false,
        serde_json::to_vec(&payload)?
    ).await?;

    // Attendre réponse (timeout 30s)
    tokio::time::timeout(Duration::from_secs(30), rx)
        .await
        .map_err(|_| "Command timeout".to_string())?
        .map_err(|_| "Channel closed".to_string())
}
```

**3. Agent: Execute Command**
```rust
// symbion-agent-host/src/main.rs:362-462
"symbion/agents/command@v1" => {
    let cmd: CommandPayload = serde_json::from_slice(&payload)?;

    // Vérifier whitelist
    if !validate_command_whitelist(&cmd.command) {
        let error_response = CommandResponse {
            agent_id: agent_id.clone(),
            command_id: cmd.command_id.clone(),
            success: false,
            output: String::new(),
            error: Some(format!("Command not allowed: {}", cmd.command)),
            exit_code: 1,
            timestamp: Utc::now().timestamp(),
            duration_ms: 0,
        };

        client.publish(
            "symbion/agents/response@v1",
            QoS::AtLeastOnce,
            false,
            serde_json::to_vec(&error_response)?
        ).await?;

        continue;
    }

    // Exécuter commande
    let start = Instant::now();
    let output = tokio::process::Command::new("sh")
        .arg("-c")
        .arg(&cmd.command)
        .output()
        .await?;
    let duration = start.elapsed().as_millis();

    // Sanitize output (suppression ANSI codes)
    let sanitized_output = sanitize_ansi_codes(
        &String::from_utf8_lossy(&output.stdout)
    );

    // Publier réponse
    let response = CommandResponse {
        agent_id: agent_id.clone(),
        command_id: cmd.command_id.clone(),
        success: output.status.success(),
        output: sanitized_output,
        error: if !output.status.success() {
            Some(String::from_utf8_lossy(&output.stderr).to_string())
        } else {
            None
        },
        exit_code: output.status.code().unwrap_or(1),
        timestamp: Utc::now().timestamp(),
        duration_ms: duration as u64,
    };

    client.publish(
        "symbion/agents/response@v1",
        QoS::AtLeastOnce,
        false,
        serde_json::to_vec(&response)?
    ).await?;
}
```

**4. Kernel: Receive Response**
```rust
// symbion-kernel/src/mqtt.rs:76
"symbion/agents/response@v1" => {
    let response: CommandResponse = serde_json::from_slice(&payload)?;

    // Trouver pending command future
    let mut pending = app.pending_commands.lock().await;
    if let Some(tx) = pending.remove(&response.command_id) {
        // Résoudre future avec résultat
        tx.send(response).ok();
    } else {
        eprintln!("[mqtt] Received response for unknown command: {}",
            response.command_id);
    }
}
```

---

## 📝 Flow 3: Plugin RPC (Notes CRUD)

### Séquence Complète

```
┌────────────┐     ┌────────────┐     ┌────────────┐     ┌────────────┐
│    PWA     │     │   Kernel   │     │   Broker   │     │Plugin Notes│
└─────┬──────┘     └─────┬──────┘     └─────┬──────┘     └─────┬──────┘
      │                  │                  │                  │
      │ 1. POST /ports/memo                │                  │
      │    {"content": "Buy milk"}         │                  │
      ├─────────────────►│                  │                  │
      │                  │                  │                  │
      │                  │  2. Generate req_id                 │
      │                  │     Auto-inject context            │
      │                  │                  │                  │
      │                  │  3. Publish notes/request@v1       │
      │                  ├─────────────────►│                  │
      │                  │                  │                  │
      │                  │                  │  4. Forward       │
      │                  │                  ├─────────────────►│
      │                  │                  │                  │
      │                  │                  │                  │  5. Insert note DB
      │                  │                  │                  │     Generate note_id
      │                  │                  │                  │
      │                  │                  │  6. Publish notes/response@v1
      │                  │                  │◄─────────────────┤
      │                  │                  │                  │
      │                  │  7. Forward       │                  │
      │                  │◄─────────────────┤                  │
      │                  │                  │                  │
      │                  │  8. Resolve future│                  │
      │                  │                  │                  │
      │  9. 201 Created  │                  │                  │
      │◄─────────────────┤                  │                  │
      │  {"id": "note-123", "content": "..."│                  │
```

### Étapes Détaillées

**1. API Request**
```bash
CSRF_TOKEN=$(curl -s https://localhost:8443/csrf-token \
  -H "Authorization: Bearer $JWT" | jq -r '.token')

curl -X POST https://localhost:8443/ports/memo \
  -H "Authorization: Bearer $JWT" \
  -H "X-CSRF-Token: $CSRF_TOKEN" \
  -H "Content-Type: application/json" \
  -d '{
    "content": "Acheter lait + pain",
    "tags": ["courses"]
  }'
```

**2. Kernel: Auto-inject Context**
```rust
// symbion-kernel/src/http.rs:588-625
async fn create_note(
    State(app): State<Arc<AppState>>,
    Extension(user): Extension<User>,
    Json(payload): Json<CreateNoteRequest>,
) -> Result<Json<NoteResponse>, (StatusCode, String)> {
    // Auto-injection contexte actuel
    let context = payload.context.or_else(|| {
        app.context_engine.get_state()
            .map(|state| format!("{:?}", state.mode).to_lowercase())
    });

    let request_id = format!("req-{}", uuid::Uuid::new_v4().simple());
    let (tx, rx) = oneshot::channel();
    app.pending_notes_requests.lock().await.insert(request_id.clone(), tx);

    let request = NotesRequest {
        request_id: request_id.clone(),
        action: "create",
        data: Some(json!({
            "content": payload.content,
            "context": context,
            "tags": payload.tags,
        })),
        timestamp: Utc::now().timestamp(),
    };

    app.mqtt_client.publish(
        "symbion/notes/request@v1",
        QoS::AtLeastOnce,
        false,
        serde_json::to_vec(&request)?
    ).await?;

    // Attendre réponse plugin
    let response = tokio::time::timeout(Duration::from_secs(5), rx)
        .await
        .map_err(|_| (StatusCode::GATEWAY_TIMEOUT, "Plugin timeout".to_string()))?
        .map_err(|_| (StatusCode::INTERNAL_SERVER_ERROR, "Channel error".to_string()))?;

    if response.success {
        Ok(Json(response.data))
    } else {
        Err((StatusCode::INTERNAL_SERVER_ERROR, response.error))
    }
}
```

**3. Plugin Notes: Process Request**
```rust
// symbion-plugin-notes/src/main.rs
"symbion/notes/request@v1" => {
    let req: NotesRequest = serde_json::from_slice(&payload)?;

    let result = match req.action.as_str() {
        "create" => {
            let note = Note {
                id: format!("note-{}", uuid::Uuid::new_v4().simple()),
                content: req.data["content"].as_str().unwrap().to_string(),
                context: req.data["context"].as_str().map(String::from),
                tags: req.data["tags"]
                    .as_array()
                    .map(|arr| arr.iter().filter_map(|v| v.as_str().map(String::from)).collect())
                    .unwrap_or_default(),
                created_at: Utc::now().timestamp(),
                updated_at: Utc::now().timestamp(),
            };

            // Insert DB
            db.insert(&note).await?;

            NotesResponse {
                request_id: req.request_id.clone(),
                success: true,
                data: Some(serde_json::to_value(&note)?),
                error: None,
            }
        },
        "read" => { /* ... */ },
        "update" => { /* ... */ },
        "delete" => { /* ... */ },
        _ => {
            NotesResponse {
                request_id: req.request_id.clone(),
                success: false,
                data: None,
                error: Some(format!("Unknown action: {}", req.action)),
            }
        }
    };

    client.publish(
        "symbion/notes/response@v1",
        QoS::AtLeastOnce,
        false,
        serde_json::to_vec(&result)?
    ).await?;
}
```

**4. Kernel: Receive Response**
```rust
// symbion-kernel/src/mqtt.rs:63
"symbion/notes/response@v1" => {
    let response: NotesResponse = serde_json::from_slice(&payload)?;

    let mut pending = app.pending_notes_requests.lock().await;
    if let Some(tx) = pending.remove(&response.request_id) {
        tx.send(response).ok();
    }
}
```

---

## 📊 Flow 4: Dashboard Real-Time Updates

### Séquence Complète

```
┌────────────┐     ┌────────────┐     ┌────────────┐     ┌────────────┐
│   Agent    │     │   Kernel   │     │   Broker   │     │    PWA     │
└─────┬──────┘     └─────┬──────┘     └─────┬──────┘     └─────┬──────┘
      │                  │                  │                  │
      │                  │                  │  1. Subscribe dashboard/update@v1
      │                  │                  │◄─────────────────┤
      │                  │                  │                  │
      │  2. Heartbeat    │                  │                  │
      ├─────────────────►│                  │                  │
      │                  │                  │                  │
      │                  │  3. Process      │                  │
      │                  │     Update registry                │
      │                  │                  │                  │
      │                  │  4. Publish dashboard/update@v1    │
      │                  ├─────────────────►│                  │
      │                  │                  │                  │
      │                  │                  │  5. Forward       │
      │                  │                  ├─────────────────►│
      │                  │                  │                  │
      │                  │                  │                  │  6. Update UI
      │                  │                  │                  │     (widget metrics)
      │                  │                  │                  │
```

### Implémentation PWA

```javascript
// pwa-dashboard/src/services/mqtt-service.js
import mqtt from 'mqtt';

class MQTTService {
    constructor() {
        this.client = mqtt.connect('wss://symbion.local:9001', {
            clientId: 'pwa-' + Math.random().toString(16).substr(2, 8),
            clean: true,
            reconnectPeriod: 1000,
        });

        this.client.on('connect', () => {
            console.log('[mqtt] Connected');
            // Subscribe to current dashboard topics (6 total)
            this.client.subscribe('symbion/dashboard/context@v1', { qos: 1 });
            this.client.subscribe('symbion/dashboard/agents@v1', { qos: 1 });
            this.client.subscribe('symbion/dashboard/health@v1', { qos: 1 });
            this.client.subscribe('symbion/dashboard/notes@v1', { qos: 1 });
            this.client.subscribe('symbion/dashboard/stats@v1', { qos: 1 });
            this.client.subscribe('symbion/dashboard/pattern@v1', { qos: 1 });
        });

        this.client.on('message', (topic, payload) => {
            const message = JSON.parse(payload.toString());

            switch (topic) {
                case 'symbion/dashboard/agents@v1':
                    this.handleAgentsUpdate(message);
                    break;
                case 'symbion/dashboard/health@v1':
                    this.handleHealthUpdate(message);
                    break;
                case 'symbion/dashboard/context@v1':
                    this.handleContextUpdate(message);
                    break;
            }
        });
    }

    handleDashboardUpdate(message) {
        switch (message.event_type) {
            case 'agent_heartbeat':
                // Mettre à jour widget agents
                window.dispatchEvent(new CustomEvent('agent-update', {
                    detail: {
                        agent_id: message.agent_id,
                        metrics: message.metrics,
                    }
                }));
                break;

            case 'context_change':
                // Changer thème dashboard
                window.dispatchEvent(new CustomEvent('context-changed', {
                    detail: {
                        mode: message.new_mode,
                        reason: message.reason,
                    }
                }));
                break;

            case 'agent_offline':
                // Afficher alerte
                this.showAgentOfflineAlert(message.agent_id);
                break;
        }
    }

    handleNotification(message) {
        // Afficher notification navigateur
        if ('Notification' in window && Notification.permission === 'granted') {
            new Notification(message.title, {
                body: message.message,
                icon: '/symbion-icon.png',
            });
        }

        // Toast UI
        this.showToast(message.level, message.title, message.message);
    }
}

export default new MQTTService();
```

---

## 📊 Latences et Performances

### Latences Mesurées (Réseau Local)

| Flow | Latence p50 | Latence p99 | Overhead |
|------|-------------|-------------|----------|
| **Heartbeat** (Agent → Kernel) | 8ms | 25ms | Minimal |
| **Command** (API → Agent → API) | 45ms | 120ms | Modéré |
| **Plugin RPC** (API → Plugin → API) | 35ms | 80ms | Modéré |
| **Dashboard Update** (Kernel → PWA) | 12ms | 35ms | Minimal |

### Optimisations

**1. Lock Minimization**
```rust
// ❌ MAUVAIS : lock pendant I/O
let registry = app.agent_registry.lock().await;
let agent = registry.get(agent_id)?;
send_command(&agent).await?;  // Lock tenu pendant I/O!

// ✅ BON : clone avant I/O
let agent = {
    let registry = app.agent_registry.lock().await;
    registry.get(agent_id)?.clone()
};  // Lock released
send_command(&agent).await?;
```

**2. QoS Adapté**
- QoS 1 partout : balance fiabilité/performance
- QoS 0 évité : risque perte messages critiques
- QoS 2 évité : overhead inutile (idempotence messages)

**3. Payload Compression (TODO)**
```rust
// Future : compression payloads > 1KB
use flate2::write::GzEncoder;

if payload.len() > 1024 {
    let compressed = compress_gzip(&payload)?;
    client.publish(topic, QoS::AtLeastOnce, false, compressed).await?;
}
```

---

**Dernière mise à jour** : 2025-11-12
**Fichiers sources** :
- `symbion-kernel/src/mqtt.rs` (event loop Kernel)
- `symbion-agent-host/src/main.rs` (event loop Agents)
- `symbion-kernel/src/agents.rs` (command execution)
- `pwa-dashboard/src/services/mqtt-service.js` (PWA client)
