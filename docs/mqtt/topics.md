# MQTT Topics - Référence Complète

> 📡 Documentation exhaustive des 13 topics MQTT de l'écosystème Symbion

## 🗂️ Classification des Topics

| Catégorie | Topics | Direction | Description |
|-----------|--------|-----------|-------------|
| **Agent Lifecycle** | 3 | Agents → Kernel | Enregistrement, heartbeat, état |
| **Agent Control** | 2 | Kernel → Agents | Commandes, wake-on-LAN |
| **Plugin Communication** | 4 | Bidirectionnel | Requêtes/réponses plugins |
| **Dashboard Updates** | 2 | Kernel → PWA | Événements temps réel |
| **System Events** | 2 | Multicast | Notifications globales |

**Total** : 13 topics actifs

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

---

### `symbion/agents/wake@v1`

**Direction** : Kernel → Broadcast
**QoS** : 1 (At least once)
**Fréquence** : À la demande (via API `/wake`)

**Description** : Wake-on-LAN pour réveil machine à distance

**Payload** :
```json
{
  "target_agent_id": "eridwyn-Bureau",
  "mac_address": "00:1A:2B:3C:4D:5E",
  "broadcast_ip": "192.168.1.255",
  "timestamp": 1699887200
}
```

**Fichier source** :
- **Publisher** : `symbion-kernel/src/agents.rs` (endpoint `/wake`)
- **Subscriber** : `symbion-agent-host/src/main.rs` (agents sur même réseau)

**Implémentation WOL** :
```rust
// Génération magic packet (6x 0xFF + 16x MAC address)
let mut packet = vec![0xFF; 6];
for _ in 0..16 {
    packet.extend_from_slice(&mac_bytes);
}

// Envoi UDP broadcast
socket.send_to(&packet, "192.168.1.255:9").await?;
```

---

## 🔌 Plugin Communication Topics

### `symbion/notes/request@v1`

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

---

### `symbion/notes/response@v1`

**Direction** : Plugin Notes → Kernel
**QoS** : 1 (At least once)
**Fréquence** : En réponse à requête

**Description** : Résultat opération CRUD

**Payload (SUCCESS)** :
```json
{
  "request_id": "req-123",
  "success": true,
  "data": {
    "id": "note-456",
    "content": "Acheter lait + pain",
    "context": "intime",
    "tags": ["courses"],
    "created_at": 1699887200
  }
}
```

**Payload (ERROR)** :
```json
{
  "request_id": "req-789",
  "success": false,
  "error": "Note not found: note-999"
}
```

**Fichier source** :
- **Publisher** : `symbion-plugin-notes/src/main.rs`
- **Subscriber** : `symbion-kernel/src/mqtt.rs:63`

---

### `symbion/ports/{plugin_name}/request@v1`

**Direction** : Kernel → Plugin
**QoS** : 1 (At least once)
**Fréquence** : À la demande

**Description** : Pattern générique pour communication Kernel → Plugins

**Exemples topics** :
- `symbion/ports/memo/request@v1` (Notes)
- `symbion/ports/kitchen/request@v1` (Cuisine - futur)
- `symbion/ports/finance/request@v1` (Finance - futur)

**Payload générique** :
```json
{
  "request_id": "req-<uuid>",
  "action": "create|read|update|delete|custom",
  "data": { /* payload spécifique plugin */ },
  "timestamp": 1699887200
}
```

---

### `symbion/ports/{plugin_name}/response@v1`

**Direction** : Plugin → Kernel
**QoS** : 1 (At least once)
**Fréquence** : En réponse à requête

**Description** : Pattern générique pour communication Plugin → Kernel

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

### `symbion/dashboard/update@v1`

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

### `symbion/dashboard/notification@v1`

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

### `symbion/system/event@v1`

**Direction** : Multicast (tous composants)
**QoS** : 1 (At least once)
**Fréquence** : Événements système

**Description** : Événements globaux système (broadcast)

**Payload (KERNEL_STARTUP)** :
```json
{
  "event_type": "kernel_startup",
  "version": "0.1.0",
  "mqtt_broker": "127.0.0.1:1883",
  "http_port": 8443,
  "timestamp": 1699887200
}
```

**Payload (KERNEL_SHUTDOWN)** :
```json
{
  "event_type": "kernel_shutdown",
  "reason": "graceful_shutdown",
  "timestamp": 1699887300
}
```

**Payload (CONFIG_CHANGED)** :
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

---

### `symbion/system/health@v1`

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

## 📖 Récapitulatif Topics

| Topic | Direction | QoS | Fréquence |
|-------|-----------|-----|-----------|
| `symbion/agents/registration@v1` | Agents → Kernel | 1 | Au démarrage |
| `symbion/agents/heartbeat@v1` | Agents → Kernel | 1 | 30s |
| `symbion/agents/response@v1` | Agents → Kernel | 1 | À la demande |
| `symbion/agents/command@v1` | Kernel → Agents | 1 | À la demande |
| `symbion/agents/wake@v1` | Kernel → Broadcast | 1 | À la demande |
| `symbion/notes/request@v1` | Kernel → Plugin | 1 | À la demande |
| `symbion/notes/response@v1` | Plugin → Kernel | 1 | À la demande |
| `symbion/ports/{plugin}/request@v1` | Kernel → Plugin | 1 | À la demande |
| `symbion/ports/{plugin}/response@v1` | Plugin → Kernel | 1 | À la demande |
| `symbion/dashboard/update@v1` | Kernel → PWA | 1 | Temps réel |
| `symbion/dashboard/notification@v1` | Kernel → PWA | 1 | Événements |
| `symbion/system/event@v1` | Multicast | 1 | Événements |
| `symbion/system/health@v1` | Kernel → Tous | 1 | 5 min |

---

**Dernière mise à jour** : 2025-11-12
**Fichiers sources** :
- `symbion-kernel/src/mqtt.rs` (subscriptions Kernel)
- `symbion-agent-host/src/main.rs` (publishers Agents)
- `symbion-kernel/src/agents.rs` (commandes Agents)
