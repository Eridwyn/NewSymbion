# Système de Plugins Symbion

**Architecture** : Microservices autonomes avec communication MQTT + HTTP Unix sockets
**Version** : 1.1.7
**Date** : 27 Novembre 2025

---

## Vue d'Ensemble

Le système de plugins Symbion est conçu pour l'**extensibilité sans interruption de service**. Les plugins sont des **processus séparés autonomes** qui communiquent avec le kernel via deux canaux :

1. **MQTT** : Communication événementielle temps réel (pub/sub)
2. **Unix Sockets** : API HTTP reverse proxy pour requêtes authentifiées

**Objectif** : Permettre l'ajout de fonctionnalités (organes) sans modifier ni redémarrer le kernel principal.

---

## Architecture

### Hiérarchie

```
symbion-kernel (PID 1)
    ├── Plugin Manager (PluginManager)
    │   ├── Lifecycle management (start/stop/restart)
    │   ├── Health monitoring (30s interval)
    │   └── Circuit breaker (auto-recovery)
    │
    ├── Plugin Proxy (PluginRegistry)
    │   ├── Service Discovery (auto-registration)
    │   ├── HTTP reverse proxy (Unix sockets)
    │   └── Route mapping (/v1/plugin-api/{plugin}/<path>)
    │
    └── MQTT Broker (Mosquitto)
        └── Event Bus (symbion/# topics)

Plugins (processus séparés)
    ├── symbion-plugin-notes (PID 1026841)
    ├── symbion-plugin-sensors (PID 818662)
    └── symbion-plugin-notifications (PID 818660)
```

### Communication Dual Channel

```
┌──────────────────────────────────────────────────────┐
│                   SYMBION KERNEL                     │
│  ┌────────────────┐          ┌──────────────────┐   │
│  │  Plugin Proxy  │          │ Plugin Manager   │   │
│  │  (HTTP Router) │          │ (Lifecycle)      │   │
│  └────────┬───────┘          └────────┬─────────┘   │
│           │                            │             │
└───────────┼────────────────────────────┼─────────────┘
            │ Unix Socket HTTP           │ Process spawn/kill
            │                            │
            ▼                            ▼
┌──────────────────────────────────────────────────────┐
│                 PLUGIN (processus séparé)            │
│  ┌────────────────┐          ┌──────────────────┐   │
│  │  HTTP Server   │          │  MQTT Client     │   │
│  │  (Unix socket) │          │  (pub/sub)       │   │
│  └────────┬───────┘          └────────┬─────────┘   │
│           │                            │             │
│           │                            │             │
│  ┌────────┴────────────────────────────┴──────────┐  │
│  │           BUSINESS LOGIC (Notes, Sensors...)   │  │
│  └────────────────────────────────────────────────┘  │
└──────────────────────────────────────────────────────┘
            ▲                            │
            │ HTTP/TLS                   │ MQTT pub/sub
            │ (via proxy)                │
            │                            ▼
        ┌───────┐                  ┌──────────────┐
        │  PWA  │                  │  MQTT Broker │
        │  API  │                  │  (Mosquitto) │
        └───────┘                  └──────────────┘
```

---

## 1. Plugin Manager (Lifecycle)

**Fichier** : `symbion-kernel/src/plugins.rs` (970 lignes)

### Rôle

Gestionnaire du cycle de vie complet des plugins :
- Chargement/déchargement dynamique (hot reload)
- Monitoring santé avec circuit breaker
- Auto-restart en cas de crash
- Gestion dépendances et priorités de démarrage

### Architecture

```rust
pub struct PluginManager {
    plugins: HashMap<String, PluginInstance>,
    plugins_dir: PathBuf,               // ./plugins/
    global_env: HashMap<String, String>, // MQTT config
}

pub struct PluginInstance {
    manifest: PluginManifest,          // Metadata (name, version, binary)
    process: Option<Child>,            // Child process handle
    status: PluginStatus,              // Running, Stopped, Failed, SafeMode
    restart_count: u32,                // Circuit breaker counter
    circuit_state: CircuitState,       // Normal, Degraded, CircuitOpen
    last_working_manifest: Option<PluginManifest>, // Rollback
}

pub enum PluginStatus {
    Stopped,
    WaitingDependencies,
    Starting,
    Running,
    Stopping,
    Killed,
    Failed(String),
    SafeMode,
}

pub enum CircuitState {
    Normal,         // 0-2 échecs
    Degraded,       // 3-5 échecs (wait 60s)
    CircuitOpen,    // 6+ échecs (wait 5min)
}
```

### Manifest Plugin

Fichier JSON décrivant le plugin : `./plugins/{plugin_name}.json`

```json
{
  "name": "notes",
  "version": "1.0.0",
  "binary": "./target/release/symbion-plugin-notes",
  "description": "External memory notes plugin",
  "contracts": ["notes@v1"],
  "auto_start": true,
  "restart_on_failure": true,
  "startup_timeout_seconds": 30,
  "shutdown_timeout_seconds": 10,
  "env": {
    "CUSTOM_VAR": "value"
  },
  "depends_on": [],
  "start_priority": 50
}
```

### Lifecycle Hooks

1. **Démarrage** : `plugin.start(global_env)`
   - Spawn processus child avec `Command::new()`
   - Logs → `/tmp/plugin-{name}.log`
   - Working directory : `.` (racine projet)
   - Environment : `SYMBION_PLUGIN_NAME`, `SYMBION_PLUGIN_INSTANCE_ID`, `SYMBION_MQTT_HOST`, `SYMBION_MQTT_PORT`

2. **Arrêt** : `plugin.stop(intentional)`
   - Phase 1 : SIGTERM (graceful)
   - Phase 2 : Wait avec timeout (default 10s)
   - Phase 3 : SIGKILL si timeout

3. **Health Check** : Toutes les 30s
   - Vérifie si processus actif via `process.try_wait()`
   - Si crash + `restart_on_failure=true` → auto-restart
   - Circuit breaker : limite les redémarrages en boucle

4. **Circuit Breaker**
   - Normal (0-2 échecs) : restart immédiat
   - Degraded (3-5 échecs) : wait 60s entre tentatives
   - CircuitOpen (6+ échecs) : wait 5min, passe en SafeMode

5. **Rollback** : Si plugin crash après update
   - Restaure le `last_working_manifest`
   - Redémarre avec l'ancienne version du binaire

### APIs Manager

```rust
// API interne (utilisée par le kernel)
impl PluginManager {
    pub fn new(plugins_dir: &Path) -> Self
    pub async fn discover_plugins() -> Result<Vec<String>>
    pub fn start_plugin(name: &str) -> Result<()>
    pub fn stop_plugin(name: &str) -> Result<()>
    pub fn restart_plugin(name: &str) -> Result<()>
    pub fn health_check_all() -> ()
    pub fn auto_start_plugins() -> ()
    pub fn list_plugins() -> Vec<PluginInfo>
    pub fn reset_plugin_circuit(name: &str) -> Result<()>
    pub fn force_plugin_rollback(name: &str) -> Result<()>
}
```

---

## 2. Plugin Proxy (Service Discovery + Reverse Proxy)

**Fichier** : `symbion-kernel/src/plugin_proxy.rs` (366 lignes)

### Rôle

Reverse proxy HTTP pour router les requêtes API authentifiées (HTTPS/JWT) vers les Unix sockets des plugins.

**Avantages** :
- Plugins n'ont pas besoin de gérer TLS/JWT/CORS/CSRF
- Kernel centralise l'authentification et le rate limiting
- Plugins exposent juste une API HTTP simple sur Unix socket

### Architecture

```rust
pub struct PluginRegistry {
    plugins: Arc<RwLock<HashMap<String, PluginInfo>>>,
}

pub struct PluginInfo {
    name: String,
    socket_path: PathBuf,              // /tmp/symbion-plugin-{name}.sock
    routes: Vec<String>,               // ["/notes", "/notes/:id"]
    version: Option<String>,
    registered_at: DateTime<Utc>,
}
```

### Service Discovery Flow

1. **Plugin démarre** et crée son Unix socket : `/tmp/symbion-plugin-{name}.sock`
2. **Plugin enregistre ses routes** via POST `/v1/plugins/register`
   ```json
   {
     "name": "notes",
     "socket_path": "/tmp/symbion-plugin-notes.sock",
     "routes": ["/notes", "/notes/:id"],
     "version": "1.0.0",
     "description": "Notes management"
   }
   ```
3. **Kernel enregistre** les routes dans le registry
4. **Routing automatique** : `/v1/plugin-api/{plugin}/{route}` → Unix socket

### Routing Flow

```
Client Request (HTTPS/JWT)
  ↓
Kernel (TLS termination + JWT auth)
  ↓
Plugin Proxy Router
  ↓
Route Matching (/v1/plugin-api/notes/123 → /notes/123)
  ↓
Unix Socket Connect (/tmp/symbion-plugin-notes.sock)
  ↓
HTTP/1.1 Request (sans TLS, sans JWT)
  ↓
Plugin HTTP Server
  ↓
Response
  ↓
Kernel (forward response)
  ↓
Client (HTTPS response)
```

### Path Rewriting

```
Client →  /v1/plugin-api/notes/notes/123
           ↓ (strip /v1)
           /plugin-api/notes/notes/123
           ↓ (strip /plugin-api/{plugin})
Plugin ← /notes/123
```

### APIs Proxy

```rust
// HTTP Endpoints (kernel)
POST   /v1/plugins/register     // Plugin self-registration
GET    /v1/plugins              // List all registered plugins
ANY    /v1/plugin-api/{plugin}/<path>  // Proxy to plugin

// Internal (PluginRegistry)
impl PluginRegistry {
    pub async fn register_plugin(registration: PluginRegistration) -> Result<()>
    pub async fn unregister_plugin(plugin_name: &str) -> Result<()>
    pub async fn list_plugins() -> Vec<PluginInfo>
    pub async fn find_socket(path: &str) -> Option<PathBuf>
    fn route_matches(pattern: &str, path: &str) -> bool  // Support :param
}
```

---

## 3. Plugins Actifs

### 3.1 Plugin Notes

**Binary** : `./plugins/symbion-plugin-notes`
**PID** : 1026841
**Socket** : `/tmp/symbion-plugin-notes.sock`
**MQTT Topics** : `symbion/notes/#`

**Fonctionnalité** : Mémoire externe intelligente avec tags contextuels

**Routes HTTP** :
- `GET    /notes` - Liste toutes les notes (streaming via MQTT)
- `POST   /notes` - Créer une nouvelle note
- `GET    /notes/:id` - Récupérer une note spécifique
- `PUT    /notes/:id` - Modifier une note
- `DELETE /notes/:id` - Supprimer une note

**Contrats MQTT** :
- `symbion/notes/list@v1` - Request/Response pour lister notes
- `symbion/notes/add@v1` - Ajouter une note
- `symbion/notes/update@v1` - Modifier une note
- `symbion/notes/delete@v1` - Supprimer une note

**Stockage** : `./notes.json` (JSON file)

### 3.2 Plugin Sensors

**Binary** : `./target/release/symbion-plugin-sensors`
**PID** : 818662 (+ 2 instances orphelines 741503, 793602)
**Socket** : `/tmp/symbion-plugin-sensors.sock`
**MQTT Topics** : `symbion/sensors/#`, `symbion/environment/#`

**Fonctionnalité** : Capteurs environnementaux distribués (F1 - ESP32 sensors)

**Routes HTTP** :
- `GET /sensors` - Liste tous les capteurs
- `POST /sensors` - Enregistrer un nouveau capteur
- `GET /sensors/:id` - État d'un capteur spécifique
- `GET /environment/:room_id` - Données environnementales d'une pièce

**Contrats MQTT** :
- `symbion/sensors/register@v1` - Enregistrement capteur (ESP32 → Kernel)
- `symbion/sensors/data@v1` - Données capteur (ESP32 → Kernel)
- `symbion/environment/status@v1` - État environnement (température, humidité, CO2)

**Alertes** : Magnus Dew Point formula pour alertes humidité (risque moisissures)

### 3.3 Plugin Notifications

**Binary** : `./plugins/symbion-plugin-notifications`
**PID** : 818660
**Socket** : `/tmp/symbion-plugin-notifications.sock`
**MQTT Topics** : `symbion/notifications/#`

**Fonctionnalité** : Système de notifications multi-canal (MQTT → Telegram + PWA push)

**Routes HTTP** :
- `GET  /notifications` - Liste notifications
- `POST /notifications/send` - Envoyer notification (fallback HTTP)
- `GET  /notifications/:id` - Détails notification
- `POST /notifications/:id/acknowledge` - Marquer comme lue

**Contrats MQTT** :
- `symbion/notifications/send@v1` - Envoyer notification (priorité MQTT)
- `symbion/notifications/status@v1` - État notifications

**Backend** : MQTT (notifications relayées vers Telegram avec boutons interactifs)

---

## 4. Développement de Plugins

### Template Plugin Minimal

```rust
// my-plugin/src/main.rs
use rumqttc::{AsyncClient, MqttOptions, QoS};
use tokio::net::UnixListener;
use axum::{Router, routing::get};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // 1. MQTT Client (events)
    let mut mqttoptions = MqttOptions::new(
        "my-plugin",
        env::var("SYMBION_MQTT_HOST").unwrap_or("localhost".into()),
        env::var("SYMBION_MQTT_PORT")?.parse()?,
    );
    let (mqtt_client, mut event_loop) = AsyncClient::new(mqttoptions, 10);

    mqtt_client.subscribe("symbion/my-plugin/#", QoS::AtLeastOnce).await?;

    // 2. HTTP Server (API)
    let app = Router::new()
        .route("/", get(|| async { "My Plugin API" }));

    let socket_path = "/tmp/symbion-plugin-my-plugin.sock";
    let _ = std::fs::remove_file(socket_path);
    let listener = UnixListener::bind(socket_path)?;

    // 3. Service Discovery
    let client = reqwest::Client::new();
    let registration = serde_json::json!({
        "name": "my-plugin",
        "socket_path": socket_path,
        "routes": ["/", "/status"],
        "version": "1.0.0",
    });

    let kernel_url = env::var("SYMBION_KERNEL_URL")
        .unwrap_or("https://localhost:8443".into());

    client.post(format!("{}/v1/plugins/register", kernel_url))
        .json(&registration)
        .send()
        .await?;

    println!("[my-plugin] Registered with kernel via Service Discovery");

    // 4. Main loop
    tokio::select! {
        _ = axum::serve(listener, app) => {},
        _ = async {
            loop {
                if let Ok(event) = event_loop.poll().await {
                    // Handle MQTT events
                }
            }
        } => {},
    }

    Ok(())
}
```

### Manifest Plugin

```json
{
  "name": "my-plugin",
  "version": "1.0.0",
  "binary": "./target/release/my-plugin",
  "description": "Mon plugin custom",
  "contracts": ["my-plugin@v1"],
  "auto_start": true,
  "restart_on_failure": true,
  "startup_timeout_seconds": 30,
  "shutdown_timeout_seconds": 10,
  "depends_on": [],
  "start_priority": 100
}
```

### Checklist Développement

- [ ] Créer crate Rust : `cargo new --bin symbion-plugin-{name}`
- [ ] Dépendances : `tokio`, `rumqttc`, `axum`, `reqwest`, `serde_json`
- [ ] MQTT client avec topics `symbion/{plugin}/#`
- [ ] HTTP server sur Unix socket `/tmp/symbion-plugin-{name}.sock`
- [ ] Service Discovery POST `/v1/plugins/register` au démarrage
- [ ] Manifest JSON dans `./plugins/{name}.json`
- [ ] Logs → `/tmp/plugin-{name}.log` (via PluginManager)
- [ ] Tester avec `cargo run` (manuel) puis via PluginManager

### Best Practices

1. **MQTT First** : Utiliser MQTT pour événements temps réel, HTTP pour requêtes ponctuelles
2. **Stateless HTTP** : API HTTP doit être sans état (pas de sessions)
3. **Graceful Shutdown** : Gérer SIGTERM pour arrêt propre
4. **Error Handling** : Logger les erreurs, ne jamais paniquer
5. **Idempotence** : Supporter les redémarrages (auto-recovery)
6. **Minimal Dependencies** : Limiter les dépendances externes (taille binaire, stabilité)

---

## 5. Commandes Administration

### Via Kernel API (HTTPS/JWT)

```bash
# Lister tous les plugins
curl -H "Authorization: Bearer $TOKEN" https://localhost:8443/v1/plugins

# Enregistrer un plugin (service discovery)
curl -X POST https://localhost:8443/v1/plugins/register \
  -H "Content-Type: application/json" \
  -d '{
    "name": "my-plugin",
    "socket_path": "/tmp/symbion-plugin-my-plugin.sock",
    "routes": ["/", "/status"],
    "version": "1.0.0"
  }'
```

### Via PluginManager (code interne)

```rust
// main.rs
let mut plugin_manager = PluginManager::new("./plugins");
plugin_manager.discover_plugins().await?;
plugin_manager.auto_start_plugins();

// Démarrer un plugin
plugin_manager.start_plugin("notes")?;

// Arrêter un plugin
plugin_manager.stop_plugin("notes")?;

// Redémarrer un plugin
plugin_manager.restart_plugin("notes")?;

// Lister plugins
let plugins = plugin_manager.list_plugins();

// Réinitialiser circuit breaker (après SafeMode)
plugin_manager.reset_plugin_circuit("notes")?;

// Rollback vers version précédente
plugin_manager.force_plugin_rollback("notes")?;
```

### Via Processus (debugging)

```bash
# Lister processus plugins actifs
ps aux | grep symbion-plugin

# Logs d'un plugin
tail -f /tmp/plugin-notes.log

# Tuer un plugin manuellement (pour tests)
pkill -f symbion-plugin-notes

# Démarrer plugin manuellement (sans PluginManager)
SYMBION_MQTT_BROKER='127.0.0.1:1883' ./target/release/symbion-plugin-notes
```

---

## 6. Monitoring & Debugging

### Health Monitoring

**Automatique** : PluginManager check santé toutes les 30s

```rust
// symbion-kernel/src/plugins.rs:943
pub fn spawn_plugin_health_monitor(plugins: Shared<PluginManager>) {
    task::spawn(async move {
        let mut interval = tokio::time::interval(Duration::from_secs(30));
        loop {
            interval.tick().await;
            let mut manager = plugins.lock();
            manager.health_check_all();
        }
    });
}
```

### Logs

- **Kernel** : `/tmp/kernel.log` ou `journalctl -u symbion-kernel`
- **Plugins** : `/tmp/plugin-{name}.log`
- **MQTT** : `mosquitto_sub -t 'symbion/#' -v`

### Debugging Tips

1. **Plugin ne démarre pas** :
   - Vérifier manifest : `cat ./plugins/{name}.json`
   - Vérifier binaire existe : `ls -lh ./target/release/symbion-plugin-{name}`
   - Vérifier logs : `tail -50 /tmp/plugin-{name}.log`

2. **Plugin crash en boucle** :
   - Circuit breaker activé → SafeMode après 6 échecs
   - Réinitialiser circuit : `plugin_manager.reset_plugin_circuit("{name}")?`
   - Rollback version précédente : `plugin_manager.force_plugin_rollback("{name}")?`

3. **Route HTTP non trouvée (404)** :
   - Vérifier enregistrement : `curl https://localhost:8443/v1/plugins`
   - Vérifier socket existe : `ls -lh /tmp/symbion-plugin-{name}.sock`
   - Vérifier chemin route : `/v1/plugin-api/{plugin}/{route}` (ex: `/v1/plugin-api/notes/notes`)

4. **MQTT messages non reçus** :
   - Vérifier connexion plugin : `mosquitto_sub -t 'symbion/+/status@v1' -v`
   - Vérifier topic pattern : `symbion/{plugin}/#`
   - Vérifier QoS (AtLeastOnce recommandé)

---

## 7. Roadmap Plugins

### Organes Actuels (Production)

- ✅ **Notes** (F0) : Mémoire externe
- ✅ **Sensors** (F1) : ESP32 capteurs environnementaux
- ✅ **Notifications** (F4) : Telegram + PWA push

### Organes Prochains (Q1 2026)

- 🔄 **Digital Hygiene** (F2) : PC activity tracking, burnout prevention
- 🔄 **Intentions Log** (F3) : Decision memory and analytics
- 🔄 **Light Actuator** (F5) : Smart lights control (Tuya LAN API)

### Organes Futurs

- 🔮 **Energy Monitor** : Suivi consommation électrique (Shelly 3EM)
- 🔮 **Calendar** : Gestion agenda intelligent
- 🔮 **Media Center** : Contrôle Jellyfin/Plex
- 🔮 **Security** : Caméras + détection intrusion
- 🔮 **Voice Assistant** : Interface vocale locale (Whisper + Piper)

---

## 8. Migration depuis Data Ports

**Ancien système (DEPRECATED)** :
- Modules compilés dans le kernel (`symbion-kernel/src/ports/`)
- Couplage fort, redémarrage kernel obligatoire
- Pas de sandboxing, crash = crash kernel
- Difficile à tester en isolation

**Nouveau système (ACTUEL)** :
- Processus séparés autonomes
- Hot reload sans redémarrer kernel
- Sandboxing via isolation processus
- Testable indépendamment
- Auto-recovery avec circuit breaker
- Service Discovery dynamique

**Code nettoyé** (27 Nov 2025) :
- ❌ Supprimé : `symbion-kernel/src/ports/` (~240 lignes)
- ❌ Supprimé : Initialization Data Ports dans `main.rs`
- ✅ Gardé : Bridge MQTT pour notes (compatibilité PWA)

---

## Références

- **Code Source** :
  - Plugin Manager : `symbion-kernel/src/plugins.rs`
  - Plugin Proxy : `symbion-kernel/src/plugin_proxy.rs`
  - Plugin Notes : `symbion-plugin-notes/src/main.rs`
  - Plugin Sensors : `symbion-plugin-sensors/src/main.rs`
  - Plugin Notifications : `symbion-plugin-notifications/src/main.rs`

- **Documentation** :
  - Architecture : `docs/architecture/SYSTEM_OVERVIEW.md`
  - API Endpoints : `docs/api/endpoints.md`
  - MQTT Topics : `docs/mqtt/topics.md`

- **Scripts** :
  - Monitoring : `./scripts/monitor-symbion.sh`
  - Docs Lookup : `./scripts/docs-lookup.sh`

---

**Version** : 1.0.0
**Dernière mise à jour** : 27 Novembre 2025
**Auteur** : Claude Code + eridwyn
