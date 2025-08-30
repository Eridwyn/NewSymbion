# Claude Context - NewSymbion

## Architecture Overview
- **Rust workspace** avec 3 composants: symbion-kernel (serveur) + symbion-plugin-hosts (agent) + symbion-plugin-notes (notes distribuées)
- **Communication**: MQTT pour télémétrie + IPC plugins, REST API pour contrôles
- **Fonctionnalités**: Monitoring infrastructure + hôtes + Wake-on-LAN + Plugin Manager + Notes distribuées
- **Contract Registry**: Système de versioning des events avec validation JSON
- **Plugin System**: Hot loading/unloading + circuit breaker + health monitoring

## 🎉 Phase A.1 - TERMINÉE COMPLÈTEMENT ✅

### ✅ Sécurité (Corrigé)
- **API key obligatoire** - routes protégées sauf `/health` 
- **Logs sécurité** ajoutés pour accès non autorisés
- **Versions rumqttc alignées** kernel=0.24.0, plugin=0.24.0
- **Config MQTT** depuis YAML au lieu de hardcodée

### ✅ Contract Registry (Opérationnel)
- **Chargement auto** des contrats depuis `contracts/mqtt/`
- **5 contrats MQTT** : heartbeat@v2, wake@v1, health@v1, notes.command@v1, notes.response@v1
- **5 contrats HTTP** : api.system.health, api.hosts, api.ports.memo, api.contracts, api.wake
- **API REST** `/contracts` et `/contracts/{name}` 
- **Validation JSON** basique (TODO: JSON Schema complet)

### ✅ Monitoring Infrastructure (Production Ready)
- **Auto-publication** kernel health toutes les 30s sur MQTT
- **Métriques temps réel** : uptime, mémoire, état MQTT, contrats chargés, hosts trackés
- **API `/system/health`** pour monitoring infrastructure
- **Tracking reconnexions** MQTT automatique

### ✅ Plugin Manager (Production Ready) 
- **Hot loading/unloading** : plugins démarrés/arrêtés à chaud
- **Circuit breaker** : protection contre les plugins défaillants
- **Health monitoring** : surveillance continue des plugins 
- **Rollback automatique** : retour à l'état stable si échec
- **API REST plugins** : `/plugins`, `/plugins/{name}/start|stop|restart`

### ✅ Notes System Migration (TERMINÉ ✅)
- **Migration transparente** : API `/ports/memo` 100% compatible
- **Plugin distribué** : symbion-plugin-notes via MQTT
- **Bridge API** : fallback automatique port→plugin
- **Stockage JSON** : persistance dans ./notes.json (plugin)
- **Contrats MQTT** : notes.command@v1 + notes.response@v1

### ✅ Documentation Professionnelle
- **Tous les modules commentés** au niveau de ports/mod.rs et ports/memo.rs
- **En-têtes détaillés** : rôle, fonctionnement, utilité dans Symbion
- **Exemples concrets** : JSON, YAML, usage patterns
- **Vision d'ensemble** : comment chaque module s'intègre

## Architecture Actuelle

```
symbion-kernel/
├── src/
│   ├── contracts.rs           # Contract Registry MQTT + validation
│   ├── health.rs              # Infrastructure monitoring temps réel
│   ├── http.rs                # API REST sécurisée (15 endpoints)
│   ├── mqtt.rs                # Event Bus MQTT + heartbeats
│   ├── config.rs              # Configuration YAML centralisée
│   ├── models.rs              # Structures de données partagées
│   ├── wol.rs                 # Wake-on-LAN via magic packets
│   ├── state.rs               # Gestion d'état thread-safe
│   ├── main.rs                # Orchestration générale
│   ├── plugins.rs             # Plugin Manager + lifecycle management
│   ├── notes_bridge.rs        # API Bridge memo → plugin MQTT
│   └── ports/
│       ├── mod.rs             # Data Ports architecture + PortRegistry
│       └── memo.rs            # Port mémos/rappels (fallback)
├── data/
│   └── memo.json              # Stockage fallback des mémos
symbion-plugin-notes/
├── src/
│   └── main.rs                # Plugin notes distribué via MQTT
└── notes.json                 # Stockage principal des notes
contracts/
├── mqtt/                      # Contrats événements MQTT
│   ├── hosts.heartbeat.v2.json
│   ├── hosts.wake.v1.json
│   ├── kernel.health.v1.json
│   ├── notes.command.v1.json   # Commandes vers plugin notes
│   └── notes.response.v1.json  # Réponses du plugin notes
└── http/                      # Contrats API REST
    ├── api.system.health.v1.json
    ├── api.hosts.v1.json
    ├── api.ports.memo.v1.json
    ├── api.contracts.v1.json
    └── api.wake.v1.json
plugins/                       # Manifestes plugins
├── symbion-plugin-hosts.json
└── symbion-plugin-notes.json
```

## API Endpoints Complète (avec x-api-key)

### 📊 Monitoring & Infrastructure
- `GET /health` - Health check simple (pas de clé requise)
- `GET /system/health` - Métriques infrastructure complètes

### 🖥️ Hosts Management  
- `GET /hosts` - Liste des hosts monitoring
- `GET /hosts/{id}` - Détails d'un host spécifique

### ⚡ Actions
- `POST /wake?host_id=X` - Wake-on-LAN

### 📜 Discovery
- `GET /contracts` - Liste des contrats MQTT disponibles
- `GET /contracts/{name}` - Détail d'un contrat

### 🗂️ Data Ports v1 / Notes System
- `GET /ports` - Liste des ports + schémas
- `GET /ports/memo` - Lire mémos (via plugin notes ou fallback)
- `POST /ports/memo` - Créer memo (via plugin notes ou fallback)
- `PUT /ports/memo/{id}` - Modifier memo (via plugin notes)
- `DELETE /ports/memo/{id}` - Supprimer memo (via plugin notes ou fallback)

### 🔌 Plugin Management
- `GET /plugins` - Liste des plugins avec status
- `POST /plugins/{name}/start` - Démarrer un plugin
- `POST /plugins/{name}/stop` - Arrêter un plugin
- `POST /plugins/{name}/restart` - Redémarrer un plugin

## Événements MQTT

- `symbion/hosts/heartbeat@v2` - Télémétrie hosts (CPU, RAM, IP)
- `symbion/hosts/wake@v1` - Demandes Wake-on-LAN  
- `symbion/kernel/health@v1` - Health kernel (auto toutes les 30s)
- `symbion/notes/command@v1` - Commandes vers plugin notes (create/list/delete/update)
- `symbion/notes/response@v1` - Réponses du plugin notes (success/error)

## État Système

- ✅ **Phase A.1 - Kernel 0.1** : Spine & DevKit TERMINÉ
- ✅ **Phase A.2 - Plugin Manager** : TERMINÉ COMPLÈTEMENT ✅
- ✅ **Sécurité renforcée** : API key + logs + validation
- ✅ **Contract Registry** : MQTT + HTTP séparés avec 5+5 contrats
- ✅ **Plugin System** : Hot loading + circuit breaker + health monitoring
- ✅ **Notes Migration** : Système distribué via plugin MQTT
- ✅ **Control API** : 15 endpoints documentés + testés
- ✅ **Documentation** : niveau professionnel sur tous les fichiers

## Phase Actuelle

### 🎯 Phase B - Noyau Utile (PRÊT à démarrer)
- ✅ Plugin Hosts opérationnel (publication heartbeats)
- ✅ Plugin Notes opérationnel (CRUD distribué via MQTT)
- ⏳ Plugin Journal Auto unifié (prochaine étape)
- ⏳ Règles contextuelles avancées pour notes
- ⏳ Dashboard web temps réel

## Commandes Utiles
```bash
# Build workspace
cargo build --workspace

# Run kernel avec API key
cd symbion-kernel && SYMBION_API_KEY="s3cr3t-42" cargo run

# Test API complet
curl -H "x-api-key: s3cr3t-42" http://localhost:8080/system/health
curl -H "x-api-key: s3cr3t-42" http://localhost:8080/plugins
curl -H "x-api-key: s3cr3t-42" http://localhost:8080/ports/memo
curl -H "x-api-key: s3cr3t-42" -X POST -H "Content-Type: application/json" \
  -d '{"content": "Test memo distribué", "urgent": true, "context": "test"}' http://localhost:8080/ports/memo

# Tests & Linting
cargo test --workspace
cargo clippy --workspace  
cargo fmt --workspace
```