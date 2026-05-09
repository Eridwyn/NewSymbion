# MQTT Topics - Référence Complète

> 📡 Documentation exhaustive des topics MQTT de l'écosystème Symbion (18 core + 44 plugins = 62 total)

## 🗂️ Classification des Topics

| Catégorie | Topics | Direction | Description |
|-----------|--------|-----------|-------------|
| **Agent Lifecycle** | 4 | Agents → Kernel | Enregistrement, heartbeat (v1+v2), état |
| **Agent Control** | 2 | Kernel → Agents | Commandes, wake-on-LAN |
| **Environment Sensors (F1)** | 2 | Sensors → Kernel | Enregistrement ESP32, telemetry env |
| **Plugin Communication** | 2 | Bidirectionnel | Requêtes/réponses notes |
| **Dashboard Updates** | 6 | Kernel → PWA | Événements temps réel |
| **System Events** | 3 | Multicast | Health, notifications globales + ack |
| **Plugin System** | 5 | Bidirectionnel | Manifest, health, events, status |
| **SSL Plugin** | 4 | Plugin → Kernel | Certificats, domaines, fingerprints |
| **Library Plugin** | 2 | Plugin → Kernel | Nodes events, pending links |
| **Freebox Plugin** | 10 | Plugin → Kernel | Présence, devices, connection, downloads |
| **Telegram Plugin** | 2 | Plugin → Kernel | Events, health |
| **Features/Intelligence** | 2 | Bidirectionnel | Feature updates, notifications |

**Total** : 62 topics (18 core + 44 plugins) + 1 legacy (symbion/context/mode sans version)

---

## 🤖 Agent Lifecycle Topics

### `symbion/agents/registration@v1`

**Direction** : Agents → Kernel
**QoS** : 1 (At least once)
**Fréquence** : Au démarrage agent

**Description** : Enregistrement initial agent auprès du Kernel

**Payload** :
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
    "energy_monitoring",
    "smart_scheduling",
    "wake_on_lan"
  ],
  "timestamp": 1699887200
}
```

**Fichier source** :
- **Publisher** : `symbion-agent-host/src/main.rs:304-329`
- **Subscriber** : `symbion-kernel/src/mqtt.rs:70`

**Traitement Kernel** :
```rust
// symbion-kernel/src/mqtt.rs
match topic.as_str() {
    "symbion/agents/registration@v1" => {
        let reg: AgentRegistration = serde_json::from_slice(&payload)?;
        app.agent_registry.register(reg).await;
        println!("[agents] New agent registered: {}", reg.agent_id);
    }
}
```

**Voir Aussi** :
- [`symbion/agents/heartbeat@v1`](#symbionagentsheartbeatv1) - Heartbeat périodique agent
- [`symbion/agents/command@v1`](#symbionagentscommandv1) - Commandes envoyées aux agents
- [Contracts](./contracts.md#agentregistration) - Schéma validation registration

---

### `symbion/agents/heartbeat@v1`

**Direction** : Agents → Kernel
**QoS** : 1 (At least once)
**Fréquence** : Toutes les 30 secondes

**Description** : Signal de vie agent avec métriques système temps réel

**Payload** :
```json
{
  "agent_id": "eridwyn-Salon",
  "timestamp": 1699887200,
  "status": "online",
  "metrics": {
    "cpu_usage": 23.5,
    "memory": {
      "total_mb": 16384,
      "used_mb": 8192,
      "percent": 50.0
    },
    "disk": {
      "total_gb": 512,
      "used_gb": 256,
      "percent": 50.0
    },
    "uptime_seconds": 266400,
    "temperature": {
      "cpu": 45.0,
      "gpu": null
    }
  },
  "processes": [
    {
      "name": "firefox",
      "cpu": 15.2,
      "memory_mb": 1024,
      "pid": 12345
    }
  ],
  "network": {
    "ssid": "HomeNetwork",
    "signal_strength": -45
  }
}
```

**Fichier source** :
- **Publisher** : `symbion-agent-host/src/main.rs:331-360`
- **Subscriber** : `symbion-kernel/src/mqtt.rs:73`

**Traitement Kernel** :
```rust
// symbion-kernel/src/mqtt.rs:152-158
"symbion/agents/heartbeat@v1" => {
    let heartbeat: AgentHeartbeat = serde_json::from_slice(&payload)?;

    // Mise à jour registry
    app.agent_registry.update_heartbeat(heartbeat).await;

    // Notification dashboard temps réel
    client.publish(
        "symbion/dashboard/update@v1",
        QoS::AtLeastOnce,
        false,
        serde_json::to_vec(&DashboardUpdate {
            event_type: "agent_heartbeat",
            agent_id: heartbeat.agent_id,
            metrics: heartbeat.metrics,
        })?
    ).await?;
}
```

**Note Migration** : ⚠️ Un topic legacy `symbion/hosts/heartbeat@v2` existe également (utilisé en parallèle de @v1). Voir section [Legacy Topics](#legacy-topics) pour migration prévue.

**Voir Aussi** :
- [`symbion/agents/registration@v1`](#symbionagentsregistrationv1) - Enregistrement initial agent
- [`symbion/dashboard/agents@v1`](#symbiondashboardagentsv1) - Updates dashboard temps réel
- [Agent Discovery Workflow](../architecture/SYSTEM_OVERVIEW.md#agent-discovery-workflow) - Process complet
- [Message Size Limits](./README.md#message-size-limits) - Limites payload métriques

---

### `symbion/agents/response@v1`

**Direction** : Agents → Kernel
**QoS** : 1 (At least once)
**Fréquence** : En réponse à commande

**Description** : Résultat exécution commande envoyée par Kernel

**Payload** :
```json
{
  "agent_id": "eridwyn-Salon",
  "command_id": "cmd-123",
  "success": true,
  "output": "● bluetooth.service - Bluetooth service\n   Loaded: loaded...",
  "exit_code": 0,
  "timestamp": 1699887200,
  "duration_ms": 342
}
```

**Payload (erreur)** :
```json
{
  "agent_id": "eridwyn-Bureau",
  "command_id": "cmd-456",
  "success": false,
  "output": "",
  "error": "Command not allowed: rm -rf /",
  "exit_code": 1,
  "timestamp": 1699887300
}
```

**Fichier source** :
- **Publisher** : `symbion-agent-host/src/main.rs:362-462`
- **Subscriber** : `symbion-kernel/src/mqtt.rs:76`

**Traitement Kernel** :
```rust
// symbion-kernel/src/agents.rs:486-518
"symbion/agents/response@v1" => {
    let response: CommandResponse = serde_json::from_slice(&payload)?;

    // Trouver future en attente de réponse
    let mut pending = app.pending_commands.lock().await;
    if let Some(tx) = pending.remove(&response.command_id) {
        tx.send(response).ok();
    }
}
```

**Voir Aussi** :
- [`symbion/agents/command@v1`](#symbionagentscommandv1) - Commande initiale envoyée
- [API Remote Commands](../api/endpoints.md#post-agentsagent_idcommand) - Endpoint HTTP trigger
- [Contracts](./contracts.md#commandresponse) - Schéma validation response

---

## 🎛️ Agent Control Topics

### `symbion/agents/command@v1`

**Direction** : Kernel → Agents
**QoS** : 1 (At least once)
**Fréquence** : À la demande (via API)

**Description** : Commandes de contrôle envoyées aux agents

**Payload** :
```json
{
  "command_id": "cmd-123",
  "agent_id": "eridwyn-Salon",
  "command": "systemctl status bluetooth",
  "timeout_seconds": 30,
  "timestamp": 1699887200
}
```

**Commandes supportées** (whitelist) :
- `shutdown` - Extinction machine
- `reboot` - Redémarrage
- `hibernate` - Mise en hibernation
- `systemctl <service>` - Gestion services
- `sensors` - Capteurs température
- `df -h` - Espace disque
- `free -h` - Mémoire disponible
- `uptime` - Uptime système

**Fichier source** :
- **Publisher** : `symbion-kernel/src/agents.rs:418`
- **Subscriber** : `symbion-agent-host/src/main.rs:183-195`

**Sécurité** :
```rust
// symbion-agent-host/src/main.rs:796
const ALLOWED_COMMANDS: &[&str] = &[
    "systemctl", "shutdown", "reboot", "hibernate",
    "sensors", "df", "free", "uptime",
];

fn validate_command(cmd: &str) -> bool {
    let first_word = cmd.split_whitespace().next().unwrap_or("");
    ALLOWED_COMMANDS.contains(&first_word)
}
```

**Voir Aussi** :
- [`symbion/agents/response@v1`](#symbionagentsresponsev1) - Réponse agent à la commande
- [API Remote Commands](../api/endpoints.md#post-agentsagent_idcommand) - Endpoint HTTP pour envoyer commandes
- [Security](../api/security.md) - Command validation & whitelisting

---

### `symbion/agents/wake@v1` 🔄 PLANNED

**Status** : Non implémenté (planifié pour Phase 5)
**Direction** : Kernel → Broadcast
**QoS** : 1 (At least once)
**Fréquence** : À la demande (via API `/wake`)

**Description** : Wake-on-LAN pour réveil machine à distance

**Payload prévu** :
```json
{
  "target_agent_id": "eridwyn-Bureau",
  "mac_address": "00:1A:2B:3C:4D:5E",
  "broadcast_ip": "192.168.1.255",
  "timestamp": 1699887200
}
```

**Implémentation prévue** :
```rust
// Génération magic packet (6x 0xFF + 16x MAC address)
let mut packet = vec![0xFF; 6];
for _ in 0..16 {
    packet.extend_from_slice(&mac_bytes);
}

// Envoi UDP broadcast
socket.send_to(&packet, "192.168.1.255:9").await?;
```

**Voir Aussi** :
- [ROADMAP.md](../ROADMAP.md) - Phase 5: Wake-on-LAN implementation

---

## 🌡️ Environment Sensor Topics (F1)

> ✅ **IMPLEMENTED (November 18, 2025)**: F1 Environment Monitoring Organ - ESP32/BME280 sensors
>
> **Source**: `symbion-kernel/src/environment.rs`, `symbion-kernel/src/sensors.rs`, `symbion-kernel/src/mqtt.rs:86-92,208-223`

### `symbion/sensors/registration@v1`

**Direction** : Sensors (ESP32) → Kernel
**QoS** : 1 (At least once)
**Fréquence** : Au démarrage capteur

**Description** : Enregistrement automatique capteur environnemental (ESP32 + BME280)

**Payload** :
```json
{
  "sensor_id": "esp32-chambre-01",
  "sensor_type": "bme280",
  "room_id": "chambre",
  "firmware_version": "v1.0.2"
}
```

**Champs** :
- `sensor_id` : Identifiant unique capteur (format: `esp32-{room}-{num}`)
- `sensor_type` : Type de capteur matériel (`bme280`, `dht22`, `scd30`)
- `room_id` : Pièce surveillée (clé pour RoomEnvironmentState)
- `firmware_version` : Version firmware ESP32 (optionnel)

**Fichier source** :
- **Publisher** : `symbion-plugin-sensors/src/main.rs` (ESP32 firmware)
- **Subscriber** : `symbion-kernel/src/mqtt.rs:86`
- **Handler** : `symbion-kernel/src/sensors.rs:106-135` (SensorRegistry::register)

**Traitement Kernel** :
```rust
// symbion-kernel/src/mqtt.rs:193-206
if p.topic == "symbion/sensors/registration@v1" {
    match serde_json::from_str::<SensorRegistrationMessage>(&txt) {
        Ok(reg) => {
            if let Err(e) = sensor_registry.register(reg) {
                eprintln!("[kernel] sensor registration failed: {}", e);
            } else {
                println!("[kernel] sensor registration handled successfully");
            }
        }
        Err(e) => eprintln!("[kernel] sensor registration JSON invalide: {txt}, error: {}", e),
    }
}
```

**Voir Aussi** :
- [`symbion/sensors/{sensor_id}/env@v1`](#symbionsensorssensor_idenvv1) - Telemetry readings périodiques
- [API Environment Endpoints](../api/endpoints.md#environment-sensors) - HTTP API endpoints
- [Decision Engine Rules](../architecture/SYSTEM_OVERVIEW.md#f1-environment-monitoring) - Automated alerts

---

### `symbion/sensors/{sensor_id}/env@v1`

**Direction** : Sensors (ESP32) → Kernel
**QoS** : 1 (At least once)
**Fréquence** : Toutes les 30 secondes (2 readings/min)

**Description** : Telemetry environnementale temps réel (température, humidité, battery, signal)

**Topic Pattern** : `symbion/sensors/esp32-chambre-01/env@v1`

**Payload** :
```json
{
  "sensor_id": "esp32-chambre-01",
  "temperature_c": 22.5,
  "humidity_pct": 58.2,
  "battery_pct": 87,
  "signal_rssi": -45
}
```

**Champs** :
- `sensor_id` : Identifiant capteur (doit matcher registration)
- `temperature_c` : Température en degrés Celsius
- `humidity_pct` : Humidité relative en pourcentage (0-100)
- `battery_pct` : Niveau batterie (optionnel, 0-100)
- `signal_rssi` : Signal WiFi en dBm (optionnel, ex: -45 = excellent, -70 = faible)

**Capacité Stockage** :
- Buffer circulaire : **20,160 readings** (7 jours @ 2 readings/min)
- Taille mémoire : ~500 KB par room (température + humidité + timestamps)
- Persistence : Sauvegarde périodique (5 min debounce)

**Détection Offline** :
- Timeout : 5 minutes sans message → Status `NA` (Not Available)
- Frontend : Badge "⚠️ Capteur Déconnecté" affiché
- API : `temperature_c: null`, `humidity_pct: null` retournés

**Fichier source** :
- **Publisher** : `symbion-plugin-sensors/src/main.rs` (ESP32 firmware)
- **Subscriber** : `symbion-kernel/src/mqtt.rs:89` (wildcard `symbion/sensors/+/env@v1`)
- **Handler** : `symbion-kernel/src/mqtt.rs:208-223` + `symbion-kernel/src/sensors.rs:137-169`
- **State Management** : `symbion-kernel/src/environment.rs:52-146` (RoomEnvironmentState)
- **Decision Rules** : `symbion-kernel/src/decision/environment.rs:45-171` (EnvironmentRules)

**Traitement Kernel** :
```rust
// symbion-kernel/src/mqtt.rs:208-223
else if p.topic.starts_with("symbion/sensors/") && p.topic.ends_with("/env@v1") {
    // Environment sensor readings (topic pattern: symbion/sensors/{sensor_id}/env@v1)
    if let Some(ref sensor_registry) = sensors {
        match serde_json::from_str::<SensorEnvMessage>(&txt) {
            Ok(msg) => {
                println!("[kernel] received env reading from sensor {}: {}°C, {}%",
                    msg.sensor_id, msg.temperature_c, msg.humidity_pct);
                if let Err(e) = sensor_registry.handle_env_reading(msg) {
                    eprintln!("[kernel] failed to handle env reading: {}", e);
                }
            }
            Err(e) => eprintln!("[kernel] sensor env JSON invalide: {txt}, error: {}", e),
        }
    }
}
```

**Automated Decision Rules** (evaluated on each reading):
- **ALERT_HUMIDITY_CHAMBRE** : Humidity >65% sustained 30 min → Medium impact alert
- **ALERT_HUMIDITY_CRITICAL** : Humidity >75% sustained 10 min → High impact alert (mold risk)
- **ALERT_COLD_NIGHT** : Temperature <16°C during night (22:00-07:00) → Low impact alert

**Voir Aussi** :
- [`symbion/sensors/registration@v1`](#symbionsensorsregistrationv1) - Initial sensor registration
- [API GET /v1/environment/{room_id}](../api/endpoints.md#get-v1environmentroom_id) - Récupérer état room
- [API GET /v1/environment/{room_id}/history](../api/endpoints.md#get-v1environmentroom_idhistory) - Historique telemetry
- [API GET /v1/environment/sensors](../api/endpoints.md#get-v1environmentsensors) - Liste tous sensors
- [Decision Environment Rules](../architecture/SYSTEM_OVERVIEW.md#decision-engine-rules) - Automated alerting logic
- [PWA Environment Widget](../architecture/SYSTEM_OVERVIEW.md#environment-widget) - 7-day Chart.js visualization

---

## 🔌 Plugin Communication Topics

### `symbion/notes/command@v1`

**Direction** : Kernel → Plugin Notes
**QoS** : 1 (At least once)
**Fréquence** : À la demande (via API `/ports/memo`)

**Description** : Requête CRUD sur notes

**Payload (CREATE)** :
```json
{
  "request_id": "req-123",
  "action": "create",
  "data": {
    "content": "Acheter lait + pain",
    "context": "intime",
    "tags": ["courses"]
  },
  "timestamp": 1699887200
}
```

**Payload (READ)** :
```json
{
  "request_id": "req-456",
  "action": "read",
  "filters": {
    "context": "cravate",
    "tags": ["travail"]
  }
}
```

**Payload (UPDATE)** :
```json
{
  "request_id": "req-789",
  "action": "update",
  "note_id": "note-123",
  "data": {
    "content": "Acheter lait + pain + oeufs",
    "tags": ["courses", "urgent"]
  }
}
```

**Payload (DELETE)** :
```json
{
  "request_id": "req-012",
  "action": "delete",
  "note_id": "note-123"
}
```

**Fichier source** :
- **Publisher** : `symbion-kernel/src/http.rs:588-625`
- **Subscriber** : `symbion-plugin-notes/src/main.rs`

**Voir Aussi** :
- [`symbion/notes/response@v1`](#symbionnotesresponsev1) - Réponse plugin à la requête
- [API Notes](../api/endpoints.md#notes) - Endpoints HTTP CRUD
- [Message Size Limits](./README.md#message-size-limits) - Streaming pagination pour large datasets

---

### `symbion/notes/response@v1`

**Direction** : Plugin Notes → Kernel
**QoS** : 1 (At least once)
**Fréquence** : En réponse à requête

**Description** : Résultat opération CRUD avec streaming pour `list` (pagination MQTT)

**Types de Réponse** :

#### **SUCCESS** (create/update/delete)
```json
{
  "type": "success",
  "request_id": "req-123",
  "action": "create",
  "data": {
    "id": "note-456",
    "timestamp": "2025-11-14T20:00:00Z",
    "data": {
      "content": "Acheter lait + pain",
      "context": "intime",
      "tags": ["courses"],
      "urgent": false
    }
  }
}
```

#### **NOTE_ITEM** (list streaming - 1 note par message)
```json
{
  "type": "note_item",
  "request_id": "req-123",
  "note": {
    "id": "note-456",
    "timestamp": "2025-11-14T20:00:00Z",
    "data": {
      "content": "Note example",
      "context": "intime",
      "tags": ["example"]
    },
    "metadata": {}
  }
}
```

#### **LIST_END** (fin du stream list)
```json
{
  "type": "list_end",
  "request_id": "req-123",
  "total_count": 5
}
```

#### **ERROR**
```json
{
  "type": "error",
  "request_id": "req-789",
  "action": "delete",
  "error": "Note not found"
}
```

**Protocole Streaming** :
- Pour `list` : Plugin envoie N messages `note_item` + 1 message `list_end`
- Kernel bridge agrège tous les `note_item` jusqu'à `list_end`
- Contourne limite taille paquet MQTT (10KB par défaut)
- Scalable pour nombre arbitraire de notes

**Fichier source** :
- **Publisher** : `symbion-plugin-notes/src/main.rs:106-118,320-399`
- **Subscriber** : `symbion-kernel/src/notes_bridge.rs:122-151`

**Voir Aussi** :
- [`symbion/notes/command@v1`](#symbionnotescommandv1) - Requête initiale du kernel
- [Message Size Limits](./README.md#message-size-limits) - Détails streaming pagination
- [Communication Flows](./flows.md#plugin-communication) - Workflow complet plugin notes

---

### `symbion/ports/{plugin_name}/request@v1` 🔄 PATTERN

**Status** : Pattern générique (non utilisé directement)
**Direction** : Kernel → Plugin
**QoS** : 1 (At least once)
**Fréquence** : À la demande

**Description** : Pattern générique pour communication Kernel → Plugins. **En pratique, des topics spécifiques sont utilisés** (ex: `symbion/notes/command@v1` au lieu de `symbion/ports/memo/request@v1`).

**Topics spécifiques implémentés** :
- ✅ `symbion/notes/command@v1` (Notes) - Voir section [symbion/notes/command@v1](#symbionnotescommandv1)

**Topics futurs planifiés** :
- 🔄 `symbion/ports/kitchen/request@v1` (Cuisine - Phase 6)
- 🔄 `symbion/ports/finance/request@v1` (Finance - Phase 7)

**Payload générique** :
```json
{
  "request_id": "req-<uuid>",
  "action": "create|read|update|delete|custom",
  "data": { /* payload spécifique plugin */ },
  "timestamp": 1699887200
}
```

**Note** : Ce pattern est documenté pour référence architecturale, mais l'implémentation actuelle préfère des topics nommés explicitement pour chaque plugin.

---

### `symbion/ports/{plugin_name}/response@v1` 🔄 PATTERN

**Status** : Pattern générique (non utilisé directement)
**Direction** : Plugin → Kernel
**QoS** : 1 (At least once)
**Fréquence** : En réponse à requête

**Description** : Pattern générique pour communication Plugin → Kernel. **En pratique, des topics spécifiques sont utilisés** (ex: `symbion/notes/response@v1`).

**Topics spécifiques implémentés** :
- ✅ `symbion/notes/response@v1` (Notes) - Voir section [symbion/notes/response@v1](#symbionnotesresponsev1)

**Payload générique** :
```json
{
  "request_id": "req-<uuid>",
  "success": true|false,
  "data": { /* résultat opération */ },
  "error": "message d'erreur si échec",
  "timestamp": 1699887200
}
```

---

## 📊 Dashboard Updates Topics

> ✅ **IMPLEMENTED (November 15, 2025)**: 6 topics spécifiques remplacent les anciens topics génériques
>
> **Source**: `symbion-kernel/src/dashboard_events.rs:46-82`
>
> ---

### `symbion/dashboard/context@v1`

**Direction** : Kernel → PWA
**QoS** : 1 (At least once)
**Retain** : true (pour que nouveaux clients reçoivent état actuel)
**Fréquence** : En temps réel (changements de mode)

**Description** : Notifications de changement de contexte/mode (Cravate/Intime/Neutre)

**Payload** :
```json
{
  "mode": "cravate",
  "confidence": 0.95,
  "reason": "ssid_work + time_9to5",
  "timestamp": "2025-11-15T10:00:00Z",
  "metadata": {
    "ssid": "BureauWiFi",
    "time_of_day": "morning"
  }
}
```

**Fichier source** :
- **Publisher** : `symbion-kernel/src/dashboard_events.rs:45-47`
- **Context Logic** : `symbion-kernel/src/context.rs:714` (legacy topic `symbion/context/mode` aussi publié)

---

### `symbion/dashboard/agents@v1`

**Direction** : Kernel → PWA
**QoS** : 1 (At least once)
**Fréquence** : En temps réel (heartbeat agents, registration)

**Description** : Mise à jour état des agents connectés

**Payload** :
```json
[
  {
    "agent_id": "eridwyn-Salon",
    "hostname": "eridwyn-Salon",
    "status": "online",
    "last_heartbeat": 1699887200,
    "metrics": {
      "cpu_usage": 23.5,
      "memory_percent": 50.0
    }
  },
  {
    "agent_id": "eridwyn-Bureau",
    "hostname": "eridwyn-Bureau",
    "status": "offline",
    "last_heartbeat": 1699880000
  }
]
```

**Fichier source** :
- **Publisher** : `symbion-kernel/src/dashboard_events.rs:50-52`

---

### `symbion/dashboard/health@v1`

**Direction** : Kernel → PWA
**QoS** : 1 (At least once)
**Fréquence** : Périodique (5 minutes) + événements critiques

**Description** : État de santé global système

**Payload** :
```json
{
  "status": "healthy",
  "components": {
    "mqtt": {
      "status": "healthy",
      "connected": true
    },
    "agents": {
      "status": "healthy",
      "online": 2,
      "offline": 0
    },
    "plugins": {
      "status": "healthy",
      "running": 1
    }
  },
  "uptime_seconds": 86400,
  "timestamp": 1699887200
}
```

**Fichier source** :
- **Publisher** : `symbion-kernel/src/dashboard_events.rs:55-57`

---

### `symbion/dashboard/notes@v1`

**Direction** : Kernel → PWA
**QoS** : 1 (At least once)
**Fréquence** : Création/modification note

**Description** : Événements notes (création, mise à jour, suppression)

**Payload** :
```json
{
  "note_id": "note-456",
  "timestamp": "2025-11-15T14:30:00Z"
}
```

**Fichier source** :
- **Publisher** : `symbion-kernel/src/dashboard_events.rs:60-73`

---

### `symbion/dashboard/stats@v1`

**Direction** : Kernel → PWA
**QoS** : 1 (At least once)
**Fréquence** : Mise à jour statistiques contextuelles

**Description** : Statistiques par mode (temps passé, nombre de sessions)

**Payload** :
```json
[
  {
    "mode": "cravate",
    "total_duration_seconds": 28800,
    "session_count": 12,
    "avg_session_seconds": 2400
  },
  {
    "mode": "intime",
    "total_duration_seconds": 43200,
    "session_count": 18,
    "avg_session_seconds": 2400
  }
]
```

**Fichier source** :
- **Publisher** : `symbion-kernel/src/dashboard_events.rs:76-78`

---

### `symbion/dashboard/pattern@v1`

**Direction** : Kernel → PWA
**QoS** : 1 (At least once)
**Fréquence** : Détection nouveau pattern

**Description** : Notifications de patterns détectés (habitudes, anomalies)

**Payload** :
```json
{
  "pattern_type": "routine",
  "description": "Daily work session detected: 9:00-17:00",
  "confidence": 0.92,
  "occurrences": 15,
  "first_seen": "2025-11-01T09:00:00Z",
  "last_seen": "2025-11-15T09:05:00Z"
}
```

**Fichier source** :
- **Publisher** : `symbion-kernel/src/dashboard_events.rs:81-83`

---

### Topics Obsolètes (Archived)

Les topics suivants ont été remplacés par les 6 topics spécifiques ci-dessus :

### `symbion/dashboard/update@v1` ⚠️ DEPRECATED

**Direction** : Kernel → PWA
**QoS** : 1 (At least once)
**Fréquence** : En temps réel (événements)

**Description** : Notifications temps réel pour mise à jour interface

**Payload (AGENT_HEARTBEAT)** :
```json
{
  "event_type": "agent_heartbeat",
  "agent_id": "eridwyn-Salon",
  "metrics": {
    "cpu_usage": 23.5,
    "memory": {
      "percent": 50.0
    }
  },
  "timestamp": 1699887200
}
```

**Payload (CONTEXT_CHANGE)** :
```json
{
  "event_type": "context_change",
  "old_mode": "intime",
  "new_mode": "cravate",
  "reason": "manual_override",
  "timestamp": 1699887300
}
```

**Payload (AGENT_OFFLINE)** :
```json
{
  "event_type": "agent_offline",
  "agent_id": "eridwyn-Bureau",
  "last_seen": 1699887100,
  "timestamp": 1699887400
}
```

**Fichier source** :
- **Publisher** : `symbion-kernel/src/mqtt.rs:152-158`, `symbion-kernel/src/context.rs`
- **Subscriber** : `pwa-dashboard/src/services/mqtt-service.js`

---

### `symbion/dashboard/notification@v1` ⚠️ DEPRECATED

**Direction** : Kernel → PWA
**QoS** : 1 (At least once)
**Fréquence** : Événements importants

**Description** : Notifications utilisateur (alertes, warnings, infos)

**Payload** :
```json
{
  "level": "info|warning|error",
  "title": "Agent Déconnecté",
  "message": "eridwyn-Bureau n'a pas envoyé de heartbeat depuis 5 minutes",
  "action": {
    "type": "open_url",
    "url": "/agents/eridwyn-Bureau"
  },
  "timestamp": 1699887200,
  "expires_at": 1699890800
}
```

**Niveaux** :
- `info` : Informations générales (agent reconnecté, plugin redémarré)
- `warning` : Avertissements (agent offline, metrics élevées)
- `error` : Erreurs critiques (kernel MQTT disconnected, plugin crashed)

---

## 🔔 System Events Topics

### `symbion/system/event@v1` 🔄 PLANNED

**Status** : Non implémenté (planifié pour Phase 5)
**Direction** : Multicast (tous composants)
**QoS** : 1 (At least once)
**Fréquence** : Événements système

**Description** : Événements globaux système (broadcast) - kernel startup/shutdown, config changes

**Payload prévu (KERNEL_STARTUP)** :
```json
{
  "event_type": "kernel_startup",
  "version": "0.1.0",
  "mqtt_broker": "127.0.0.1:1883",
  "http_port": 8443,
  "timestamp": 1699887200
}
```

**Payload prévu (KERNEL_SHUTDOWN)** :
```json
{
  "event_type": "kernel_shutdown",
  "reason": "graceful_shutdown",
  "timestamp": 1699887300
}
```

**Payload prévu (CONFIG_CHANGED)** :
```json
{
  "event_type": "config_changed",
  "config_key": "context.auto_detection",
  "old_value": true,
  "new_value": false,
  "changed_by": "admin",
  "timestamp": 1699887400
}
```

**Note** : Actuellement, ces événements sont loggés mais non diffusés via MQTT.

---

### `symbion/kernel/health@v1`

**Direction** : Kernel → Tous
**QoS** : 1 (At least once)
**Fréquence** : Toutes les 5 minutes

**Description** : État de santé global système

**Payload** :
```json
{
  "status": "healthy|degraded|critical",
  "components": {
    "mqtt": {
      "status": "healthy",
      "connected": true,
      "messages_processed": 15432
    },
    "agents": {
      "status": "healthy",
      "online": 2,
      "offline": 0
    },
    "plugins": {
      "status": "healthy",
      "running": 1,
      "failed": 0
    }
  },
  "uptime_seconds": 86400,
  "timestamp": 1699887200
}
```

**Status globaux** :
- `healthy` : Tous composants OK
- `degraded` : 1+ composant en warning
- `critical` : 1+ composant en erreur

---

### `symbion/notifications/acknowledge@v1`

**Direction** : PWA → Kernel
**QoS** : 1 (At least once)
**Fréquence** : Événements utilisateur (clic sur toast)

**Description** : Acquittement des notifications toast PWA. Envoyé quand l'utilisateur ferme une notification toast.

**Payload** :
```json
{
  "notification_id": "notif-2026-02-04-123456"
}
```

**Fichier source** :
- **Publisher** : `pwa-dashboard/src/components/toast-notifications.js`
- **Subscriber** : `symbion-kernel/src/mqtt.rs:115`

**Traitement Kernel** :
```rust
// symbion-kernel/src/mqtt.rs
else if p.topic == "symbion/notifications/acknowledge@v1" {
    if let Some(ref notif_mgr) = notifications_manager {
        let req: NotificationAckRequest = serde_json::from_slice(&payload)?;
        notif_mgr.acknowledge(&req.notification_id)?;
        println!("[kernel] notification {} acknowledged via MQTT", req.notification_id);
    }
}
```

**Voir Aussi** :
- [`symbion/dashboard/notification@v1`](#symbiondashboardnotificationv1) - Topic sortant (Kernel → PWA)
- [Notifications Manager](../../symbion-kernel/src/notifications.rs) - Gestion notifications

---

## 🔌 Plugin System Topics

> ✅ **IMPLEMENTED**: Contrat plugin v1.0 — tous les plugins publient manifest/health/events
>
> **Source**: `symbion-kernel/src/plugins/contract.rs`

### `symbion/plugins/{plugin_id}/manifest`

**Direction** : Plugin → Kernel
**QoS** : 1
**Fréquence** : Au démarrage plugin

**Description** : Enregistrement du manifest plugin (nom, version, capabilities, socket path)

**Plugins actifs** : `notes`, `ssl`, `sensors`, `library`, `telegram`, `freebox`, `coffee`

---

### `symbion/plugins/{plugin_id}/health`

**Direction** : Plugin → Kernel
**QoS** : 1
**Fréquence** : Heartbeat périodique (30s)

**Description** : Signal de vie du plugin avec statut santé

---

### `symbion/plugins/{plugin_id}/events`

**Direction** : Plugin → Kernel
**QoS** : 1
**Fréquence** : À la demande

**Description** : Événements métier émis par le plugin

---

### `symbion/features/update`

**Direction** : Plugins → Kernel
**QoS** : 1
**Fréquence** : Mise à jour features Intelligence v2 (1 message par feature)

**Schéma attendu** (`ExternalFeatureUpdate`, cf. `symbion-kernel/src/mqtt.rs:66-72`) :
```json
{
  "source": "plugin.<name>",
  "feature_id": "<scope>.<key>",
  "value": <bool | number | string>,
  "timestamp": "<RFC3339>",
  "ttl_seconds": <u32>
}
```

> ⚠️ Tout autre format (ex: `{"features": {...}}` wrappé) est **rejeté silencieusement** — log `feature update JSON invalide` côté kernel uniquement. Le champ `signal_type` envoyé par certains plugins (SSL) est ignoré par serde (pas dans la struct).

**Utilisé par** : SSL plugin, Library plugin, Coffee plugin

---

## 🔐 SSL Plugin Topics

> **Source**: `symbion-plugin-ssl/src/mqtt.rs:155-289`

| Topic | Direction | Description |
|-------|-----------|-------------|
| `symbion/ssl/{domain_id}` | Plugin → Kernel | Status certificat par domaine |
| `symbion/ssl/{domain_id}/online` | Plugin → Kernel | Domaine en ligne oui/non |
| `symbion/ssl/{domain_id}/fingerprint-change` | Plugin → Kernel | Changement de certificat détecté |
| `symbion/ssl/summary` | Plugin → Kernel | Résumé tous domaines |

---

## 📚 Library Plugin Topics

> **Source**: `symbion-plugin-library/src/mqtt.rs`

| Topic | Direction | Description |
|-------|-----------|-------------|
| `symbion/library/nodes/{event_type}` | Plugin → Kernel | Événements knowledge nodes (create/update/delete) |
| `symbion/library/links/pending` | Plugin → Kernel | Liens en attente de confirmation |
| `symbion/features/update` | Plugin → Kernel | Features Intelligence : `library.nodes.count`, `library.sections.count`, `library.pending_links.count` (TTL 600s) |

---

## ☕ Coffee Plugin Topics

> **Source**: `symbion-plugin-coffee/src/mqtt.rs`

| Topic | Direction | Description |
|-------|-----------|-------------|
| `symbion/coffee/brewing/started` | Plugin → Kernel | Brewing démarré (drink, temperature, cups) |
| `symbion/coffee/brewing/completed` | Plugin → Kernel | Brewing terminé (incrémente `brews_today`) |
| `symbion/coffee/power` | Plugin → Kernel | Changement état alim (on/off) |
| `symbion/coffee/maintenance/alert` | Plugin → Kernel | Alerte maintenance (raison) |
| `symbion/coffee/status` | Plugin → Kernel | Dump complet état machine (toutes les 10s) |
| `symbion/features/update` | Plugin → Kernel | 11 features Intelligence (TTL 120s) : `coffee.online`, `.ready`, `.brewing`, `.brew_progress`, `.water_level`, `.bean_level`, `.maintenance`, `.descale_status`, `.aquaclean_remaining`, `.brews_today`, `.last_brew_minutes_ago` |

---

## 📡 Freebox Plugin Topics

> **Source**: `symbion-plugin-freebox/src/mqtt.rs:120-276`

| Topic | Direction | Description |
|-------|-----------|-------------|
| `symbion/freebox/presence/{device_id}` | Plugin → Kernel | Détail présence appareil |
| `symbion/freebox/presence/{device_id}/state` | Plugin → Kernel | État simple home/away |
| `symbion/freebox/presence/summary` | Plugin → Kernel | Résumé présent/absent |
| `symbion/freebox/devices/summary` | Plugin → Kernel | Résumé appareils réseau |
| `symbion/freebox/devices/list` | Plugin → Kernel | Liste complète appareils |
| `symbion/freebox/connection/status` | Plugin → Kernel | Status connexion internet |
| `symbion/freebox/connection/metrics` | Plugin → Kernel | Métriques bande passante |
| `symbion/freebox/downloads/summary` | Plugin → Kernel | Status téléchargements |
| `symbion/freebox/downloads/active` | Plugin → Kernel | Téléchargements actifs |
| `symbion/freebox/health` | Plugin → Kernel | Santé plugin |

---

## 🤖 Telegram Plugin Topics

> **Source**: `symbion-plugin-telegram/src/events.rs:23-91`

| Topic | Direction | Description |
|-------|-----------|-------------|
| `symbion/plugins/telegram/health` | Plugin → Kernel | Heartbeat plugin |
| `symbion/plugins/telegram/events` | Plugin → Kernel | Événements Telegram (messages, commandes) |

---

## 📖 Récapitulatif Topics

### Topics Actifs (Implémentés)

| Topic | Direction | QoS | Fréquence | Status |
|-------|-----------|-----|-----------|--------|
| `symbion/agents/registration@v1` | Agents → Kernel | 1 | Au démarrage | ✅ |
| `symbion/agents/heartbeat@v1` | Agents → Kernel | 1 | 30s | ✅ |
| `symbion/hosts/heartbeat@v2` | Agents → Kernel | 1 | 30s | ⚠️ Legacy |
| `symbion/agents/response@v1` | Agents → Kernel | 1 | À la demande | ✅ |
| `symbion/agents/command@v1` | Kernel → Agents | 1 | À la demande | ✅ |
| `symbion/sensors/registration@v1` | Sensors → Kernel | 1 | Au démarrage | ✅ F1 |
| `symbion/sensors/{sensor_id}/env@v1` | Sensors → Kernel | 1 | 30s | ✅ F1 |
| `symbion/notes/command@v1` | Kernel → Plugin | 1 | À la demande | ✅ |
| `symbion/notes/response@v1` | Plugin → Kernel | 1 | À la demande | ✅ |
| `symbion/dashboard/context@v1` | Kernel → PWA | 1 (retain) | Temps réel | ✅ |
| `symbion/dashboard/agents@v1` | Kernel → PWA | 1 | Temps réel | ✅ |
| `symbion/dashboard/health@v1` | Kernel → PWA | 1 | 5 min + événements | ✅ |
| `symbion/dashboard/notes@v1` | Kernel → PWA | 1 | Événements | ✅ |
| `symbion/dashboard/stats@v1` | Kernel → PWA | 1 | Mise à jour stats | ✅ |
| `symbion/dashboard/pattern@v1` | Kernel → PWA | 1 | Détection pattern | ⚠️ Non publié (patterns via Intelligence Engine) |
| `symbion/kernel/health@v1` | Kernel → Tous | 1 | 5 min | ✅ |
| `symbion/notifications/acknowledge@v1` | PWA → Kernel | 1 | Événements | ✅ |
| `symbion/context/mode` | Kernel → Tous | 1 (retain) | Temps réel | ⚠️ No version |

**Core** : 18 topics | **Plugins** : 44 topics | **Total** : 62 topics actifs

### Topics Planifiés (Non Implémentés)

| Topic | Direction | QoS | Status |
|-------|-----------|-----|--------|
| `symbion/agents/wake@v1` | Kernel → Broadcast | 1 | 🔄 Phase 5 |
| `symbion/system/event@v1` | Multicast | 1 | 🔄 Phase 5 |
| `symbion/ports/{plugin}/request@v1` | Kernel → Plugin | 1 | 🔄 Pattern (non utilisé) |
| `symbion/ports/{plugin}/response@v1` | Plugin → Kernel | 1 | 🔄 Pattern (non utilisé) |

### Topics Obsolètes (Remplacés)

| Topic | Remplacé par |
|-------|--------------|
| `symbion/dashboard/update@v1` | 6 topics dashboard/* spécifiques |
| `symbion/dashboard/notification@v1` | `symbion/dashboard/health@v1` + `symbion/dashboard/pattern@v1` |

---

## 🔧 Legacy Topics

### `symbion/hosts/heartbeat@v2`

**Status** : ⚠️ Topic legacy utilisé en parallèle de `symbion/agents/heartbeat@v1`

**Source** : `symbion-kernel/src/mqtt.rs:58`

**Migration prévue** : Consolider vers `symbion/agents/heartbeat@v2` (Phase 5)

---

### `symbion/context/mode`

**Status** : ⚠️ Topic sans versioning utilisé pour changements de mode

**Source** : `symbion-kernel/src/context.rs:714`

**Migration prévue** : Migrer vers `symbion/dashboard/context@v1` uniquement (remplacer publish legacy)

**Note** : Ce topic est publié en parallèle de `symbion/dashboard/context@v1` pour rétrocompatibilité

---

## 📚 Documentation Connexe

### MQTT Architecture
- **[README.md](./README.md)** - Vue d'ensemble MQTT (broker, QoS, versioning, monitoring)
  - [Message Size Limits](./README.md#message-size-limits) - Pagination/streaming patterns
  - [QoS Strategy](./README.md#qos-quality-of-service) - At least once explained
  - [Monitoring](./README.md#monitoring-mqtt) - Broker metrics & dashboard

### Communication Patterns
- **[contracts.md](./contracts.md)** - Schémas JSON validation & versioning
- **[flows.md](./flows.md)** - Workflows complets (agent lifecycle, plugin comm, dashboard)

### Architecture & API
- **[SYSTEM_OVERVIEW.md](../architecture/SYSTEM_OVERVIEW.md)** - Architecture globale
  - [Agent Discovery Workflow](../architecture/SYSTEM_OVERVIEW.md#agent-discovery-workflow) - Process complet
  - [Network Architecture](../architecture/SYSTEM_OVERVIEW.md#network-architecture) - Ports & TLS
- **[endpoints.md](../api/endpoints.md)** - API HTTP (trigger MQTT commands)
  - [Remote Commands](../api/endpoints.md#post-agentsagent_idcommand) - POST /agents/:id/command
  - [Notes CRUD](../api/endpoints.md#notes) - Notes endpoints
- **[security.md](../api/security.md)** - Command validation & whitelisting

### Guides Pratiques
- **[TROUBLESHOOTING.md](../TROUBLESHOOTING.md)** - Diagnostic MQTT connection issues
- **[DEPLOYMENT.md](../DEPLOYMENT.md)** - Mosquitto setup production

---

**Dernière mise à jour** : 13 Mars 2026 (Audit documentation complet — ajout 44 topics plugins)
**Fichiers sources** :
- `symbion-kernel/src/mqtt.rs` (subscriptions Kernel + sensor topics)
- `symbion-kernel/src/dashboard_events.rs` (6 topics dashboard)
- `symbion-kernel/src/sensors.rs` (sensor registry + handlers)
- `symbion-kernel/src/environment.rs` (environment state management)
- `symbion-agent-host/src/main.rs` (publishers Agents)
- `symbion-kernel/src/agents.rs` (commandes Agents)
- `symbion-kernel/src/context.rs` (legacy topic context/mode)
- `symbion-plugin-sensors/src/main.rs` (ESP32 firmware - sensor publisher)
