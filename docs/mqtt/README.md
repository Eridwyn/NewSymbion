# MQTT Architecture - Symbion

> 🔌 Communication temps réel event-driven entre Kernel, Agents et Plugins

## 🎯 Vue d'Ensemble

Symbion utilise **MQTT (Message Queuing Telemetry Transport)** pour la communication inter-composants.

**Pourquoi MQTT ?**
- ✅ **Pub/Sub asynchrone** : découplage composants
- ✅ **Temps réel** : latence minimale (< 100ms)
- ✅ **Léger** : overhead minimal (idéal IoT)
- ✅ **QoS garanties** : livraison fiable messages
- ✅ **Versioning topics** : évolution schémas sans breaking changes

## 🏗️ Architecture Globale

```
┌────────────────────────────────────────────────────────┐
│             MQTT Broker (Mosquitto)                    │
│             Port: 1883 (TCP)                           │
│             Port: 9001 (WebSocket Secure)              │
└────────────────────────────────────────────────────────┘
                    ↑↓ Pub/Sub
    ┌───────────────┴───────────────┬──────────────────┐
    │                               │                  │
┌───▼──────┐              ┌─────────▼────┐   ┌────────▼──────┐
│  Kernel  │              │    Agents    │   │   Plugins     │
│          │              │   (N hosts)  │   │   (Notes)     │
│ - Pub    │              │              │   │               │
│   commands│             │ - Pub        │   │ - Pub         │
│   requests│             │   registration│  │   responses   │
│          │              │   heartbeat  │   │   events      │
│ - Sub    │              │   response   │   │               │
│   registration│         │              │   │ - Sub         │
│   heartbeat│            │ - Sub        │   │   requests    │
│   response│             │   command    │   │               │
└──────────┘              └──────────────┘   └───────────────┘
```

## 📊 Statistiques

- **Topics actifs** : 13
- **QoS** : 1 (At least once)
- **Versioning** : `@v1` suffix
- **Payload** : JSON (validation via contracts)
- **Latence moyenne** : < 50ms (réseau local)

## 🗂️ Documentation Détaillée

- **[Topics](./topics.md)** - Référence complète des 13 topics
- **[Contracts](./contracts.md)** - Schémas JSON et validation
- **[Flows](./flows.md)** - Patterns de communication

## 🔌 Broker MQTT (Mosquitto)

### Configuration Actuelle

**Fichier** : `/etc/mosquitto/conf.d/websocket.conf`

```conf
# Listener MQTT natif (agents, kernel)
listener 1883 0.0.0.0
protocol mqtt
allow_anonymous true

# Listener WebSocket Secure (PWA dashboard)
listener 9001 0.0.0.0
protocol websockets
allow_anonymous true

# TLS pour WebSocket
cafile /etc/mosquitto/certs/symbion-ca.crt
certfile /etc/mosquitto/certs/cert-mkcert.pem
keyfile /etc/mosquitto/certs/key-mkcert.pem
```

### Lancement Broker

```bash
# Installation
sudo apt install mosquitto mosquitto-clients

# Démarrage service
sudo systemctl start mosquitto
sudo systemctl enable mosquitto

# Vérification
mosquitto_sub -h localhost -t '#' -v  # Écoute tous topics
```

### Test Connexion

```bash
# Terminal 1 : Subscriber
mosquitto_sub -h localhost -t 'symbion/test' -v

# Terminal 2 : Publisher
mosquitto_pub -h localhost -t 'symbion/test' -m 'Hello MQTT'

# Output Terminal 1 :
# symbion/test Hello MQTT
```

## 🔐 Sécurité MQTT

### Authentification (Roadmap)

**Actuellement** : `allow_anonymous true` (développement)

**Production (TODO)** :
```conf
allow_anonymous false
password_file /etc/mosquitto/passwd

# Génération utilisateurs
mosquitto_passwd -c /etc/mosquitto/passwd symbion-kernel
mosquitto_passwd /etc/mosquitto/passwd symbion-agent
```

### TLS/SSL (Actuellement WebSocket Only)

**WebSocket Secure (WSS)** : Port 9001 avec TLS

**Recommandation production** :
```conf
# Activer TLS sur port natif MQTT
listener 8883 0.0.0.0
protocol mqtt
cafile /etc/mosquitto/certs/symbion-ca.crt
certfile /etc/mosquitto/certs/cert-mkcert.pem
keyfile /etc/mosquitto/certs/key-mkcert.pem
require_certificate false
```

### ACL (Access Control Lists)

**Fichier** : `/etc/mosquitto/acl.conf` (à créer)

```conf
# Kernel : pub/sub tous topics
user symbion-kernel
topic readwrite symbion/#

# Agents : pub registration/heartbeat/response, sub command
user symbion-agent
topic write symbion/agents/registration@v1
topic write symbion/agents/heartbeat@v1
topic write symbion/agents/response@v1
topic read symbion/agents/command@v1

# Plugins : pub/sub sur ports spécifiques
user symbion-plugin-notes
topic readwrite symbion/notes/#
```

## 🧪 QoS (Quality of Service)

Symbion utilise **QoS 1 (At least once)** pour tous topics.

| QoS | Garantie | Overhead | Use Case Symbion |
|-----|----------|----------|------------------|
| 0 | Fire and forget | Minimal | ❌ Pas utilisé (perte messages inacceptable) |
| 1 | **At least once** | Modéré | ✅ **Utilisé partout** (balance fiabilité/perf) |
| 2 | Exactly once | Élevé | ❌ Overhead inutile (idempotence messages) |

**Implémentation** :
```rust
// symbion-kernel/src/mqtt.rs
use rumqttc::QoS;

client.subscribe("symbion/agents/heartbeat@v1", QoS::AtLeastOnce).await?;

client.publish(
    "symbion/agents/command@v1",
    QoS::AtLeastOnce,
    false,  // retain
    payload
).await?;
```

## 📜 Versioning Topics

Tous les topics incluent un **suffixe de version** : `@v1`

**Avantages** :
- ✅ Migration progressive (v1 et v2 coexistent)
- ✅ Backward compatibility
- ✅ A/B testing nouvelles features

**Exemple évolution** :
```
# Ancien schema
symbion/agents/heartbeat@v1
{
  "agent_id": "eridwyn-Salon",
  "timestamp": 1699887200
}

# Nouveau schema (ajout métriques)
symbion/agents/heartbeat@v2
{
  "agent_id": "eridwyn-Salon",
  "timestamp": 1699887200,
  "metrics": {
    "cpu": 23.5,
    "memory": 50.0
  }
}

# Kernel souscrit aux deux pendant migration
client.subscribe("symbion/agents/heartbeat@v1", QoS::AtLeastOnce);
client.subscribe("symbion/agents/heartbeat@v2", QoS::AtLeastOnce);
```

## 🔄 Retained Messages

**Symbion n'utilise PAS de retained messages** (`retain: false`).

**Raison** : État système géré par Kernel (registry agents), pas par broker.

**Exception possible** : Configuration globale (future)
```rust
// Exemple : publish config avec retain
client.publish(
    "symbion/config/context@v1",
    QoS::AtLeastOnce,
    true,  // retain : nouveaux subscribers reçoivent immédiatement
    serde_json::to_vec(&config)?
).await?;
```

## 📏 Message Size Limits

### Limites Mosquitto Par Défaut

**Mosquitto** impose une limite de **268 MB** (256 MiB) par message par défaut.

```conf
# /etc/mosquitto/mosquitto.conf (limite par défaut)
message_size_limit 268435456  # 256 MiB en octets
```

**Symbion Usage Typique** :
- **Heartbeat** : ~200 octets (agent_id, metrics, timestamp)
- **Telemetry** : ~500 octets (processus top 5)
- **Command** : ~100 octets (shutdown, reboot)
- **Notes** : Variable (100 octets - 10 KB par note)

### Problème : Large Payloads

**Scénario** : Récupération 100+ notes via MQTT (> 1 MB total)

**Erreur** :
```
Error: MQTT message too large (1.5 MB > 256 MB limit)
```

**Solutions** :

#### Option 1 : Augmenter Limite Broker (Non Recommandé)

```bash
# /etc/mosquitto/mosquitto.conf
message_size_limit 0  # Illimité (dangereux, DoS possible)

# Ou limite plus haute
message_size_limit 10485760  # 10 MB

sudo systemctl restart mosquitto
```

**Inconvénients** :
- ❌ Risque saturation mémoire broker
- ❌ Latence réseau élevée pour gros messages
- ❌ Timeouts clients MQTT

#### Option 2 : Pagination/Streaming (✅ Recommandé - Symbion)

**Implémentation** : Envoyer 1 message par item + marqueur fin

```rust
// symbion-kernel/src/notes_ws.rs (exemple notes streaming)

// Publier chaque note individuellement
for note in notes {
    client.publish(
        "symbion/notes/stream@v1",
        QoS::AtLeastOnce,
        false,
        serde_json::to_vec(&note)?
    ).await?;

    tokio::time::sleep(Duration::from_millis(10)).await;
}

// Marqueur fin de stream
client.publish(
    "symbion/notes/stream@v1",
    QoS::AtLeastOnce,
    false,
    serde_json::to_vec(&StreamEnd { total: notes.len() })?
).await?;
```

**Client (PWA)** :
```javascript
let receivedNotes = [];

client.on('message', (topic, payload) => {
    const message = JSON.parse(payload);

    if (message.type === 'note') {
        receivedNotes.push(message);
    } else if (message.type === 'stream_end') {
        console.log(`Received ${receivedNotes.length}/${message.total} notes`);
        renderNotes(receivedNotes);
    }
});
```

**Avantages** :
- ✅ Pas de limite taille totale
- ✅ Progressive rendering (UI responsive)
- ✅ Resilience : échec 1 message n'affecte pas les autres
- ✅ Scalable pour 1000+ items

#### Option 3 : Compression (Pour Payloads Répétitifs)

```rust
use flate2::write::GzEncoder;
use flate2::Compression;

// Compresser avant publish
let compressed = {
    let mut encoder = GzEncoder::new(Vec::new(), Compression::default());
    encoder.write_all(serde_json::to_string(&large_payload)?.as_bytes())?;
    encoder.finish()?
};

client.publish(
    "symbion/data/compressed@v1",
    QoS::AtLeastOnce,
    false,
    compressed
).await?;
```

**Gains** : 60-80% réduction pour JSON répétitif (métriques, logs)

**Inconvénient** : Overhead CPU compression/décompression

### Best Practices Symbion

1. **Keep messages small** : < 10 KB par message
2. **Use pagination** : > 100 items → stream 1-par-1
3. **Avoid binary data** : Utiliser HTTP pour fichiers/images
4. **Monitor payload sizes** : Log warnings si > 50 KB
5. **Set client buffer** : `rumqttc` cap à 200 messages (vs 10 par défaut)

**Configuration Client** :
```rust
// symbion-kernel/src/mqtt.rs
let mut mqttoptions = MqttOptions::new("symbion-kernel", "127.0.0.1", 1883);
mqttoptions.set_keep_alive(Duration::from_secs(30));

// Buffer 200 messages (vs 10 par défaut)
// Permet gérer bursts de streaming
let (client, mut eventloop) = AsyncClient::new(mqttoptions, 200);
```

**Référence** : Voir `symbion-kernel/src/notes_ws.rs:45-78` pour implémentation streaming notes.

## 📊 Monitoring MQTT

### Métriques Broker

```bash
# Clients connectés
mosquitto_sub -h localhost -t '$SYS/broker/clients/connected' -C 1

# Messages reçus/envoyés
mosquitto_sub -h localhost -t '$SYS/broker/messages/received' -C 1
mosquitto_sub -h localhost -t '$SYS/broker/messages/sent' -C 1

# Subscriptions actives
mosquitto_sub -h localhost -t '$SYS/broker/subscriptions/count' -C 1
```

### Dashboard Kernel

**Endpoint HTTP** : `GET /system/status`

```json
{
  "mqtt": {
    "connected": true,
    "broker": "127.0.0.1:1883",
    "subscriptions": 6,
    "messages_processed": 15432,
    "last_message_at": 1699887200
  }
}
```

### Logs MQTT

**Kernel logs** :
```bash
tail -f /tmp/symbion-kernel.log | grep mqtt

# Output
[mqtt] Connected to broker 127.0.0.1:1883
[mqtt] Subscribed to symbion/agents/registration@v1
[mqtt] Message received on symbion/agents/heartbeat@v1
```

**Broker logs** :
```bash
sudo tail -f /var/log/mosquitto/mosquitto.log

# Output
1699887200: New connection from 127.0.0.1:54321
1699887201: New client connected from 127.0.0.1 as symbion-kernel-123
```

## 🛠️ Développement

### Client MQTT (Rust - rumqttc)

**Dépendance** : `Cargo.toml`
```toml
[dependencies]
rumqttc = "0.24"
tokio = { version = "1", features = ["full"] }
serde_json = "1.0"
```

**Exemple connexion** :
```rust
use rumqttc::{MqttOptions, AsyncClient, QoS, Event, Packet};

#[tokio::main]
async fn main() {
    // Configuration client
    let mut mqttoptions = MqttOptions::new("symbion-kernel", "127.0.0.1", 1883);
    mqttoptions.set_keep_alive(Duration::from_secs(30));

    // Connexion
    let (client, mut eventloop) = AsyncClient::new(mqttoptions, 10);

    // Subscription
    client.subscribe("symbion/agents/heartbeat@v1", QoS::AtLeastOnce).await.unwrap();

    // Event loop
    while let Ok(event) = eventloop.poll().await {
        if let Event::Incoming(Packet::Publish(p)) = event {
            let topic = p.topic;
            let payload = String::from_utf8(p.payload.to_vec()).unwrap();
            println!("Received on {}: {}", topic, payload);
        }
    }
}
```

### Client MQTT (JavaScript - PWA)

**Bibliothèque** : `mqtt.js` ou `paho-mqtt`

```javascript
import mqtt from 'mqtt';

// Connexion WebSocket Secure
const client = mqtt.connect('wss://symbion.local:9001', {
  clientId: 'pwa-dashboard-' + Math.random().toString(16).substr(2, 8),
  clean: true,
  reconnectPeriod: 1000,
});

client.on('connect', () => {
  console.log('MQTT connected');
  // Subscribe to current dashboard topics (6 topics)
  client.subscribe('symbion/dashboard/context@v1', { qos: 1 });
  client.subscribe('symbion/dashboard/agents@v1', { qos: 1 });
  client.subscribe('symbion/dashboard/health@v1', { qos: 1 });
});

client.on('message', (topic, payload) => {
  const message = JSON.parse(payload.toString());
  console.log('Received:', topic, message);
});
```

### Test Tools

**MQTT Explorer** (GUI) :
- Download : http://mqtt-explorer.com/
- Connexion : `mqtt://localhost:1883`
- Visualisation temps réel de tous topics

**Mosquitto CLI** :
```bash
# Subscribe à tous topics
mosquitto_sub -h localhost -t 'symbion/#' -v

# Publish test message
mosquitto_pub -h localhost -t 'symbion/test' -m '{"test": true}'
```

## 📚 Références

- **MQTT Specification** : https://mqtt.org/mqtt-specification/
- **Mosquitto Documentation** : https://mosquitto.org/documentation/
- **rumqttc Crate** : https://docs.rs/rumqttc/latest/rumqttc/
- **MQTT.js Library** : https://github.com/mqttjs/MQTT.js

---

**Dernière mise à jour** : 2025-11-12
**Broker** : Mosquitto 2.0.18
**Client Rust** : rumqttc 0.24
