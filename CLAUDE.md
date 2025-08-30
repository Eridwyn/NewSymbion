# Claude Context - NewSymbion

## Architecture Overview
- **Rust workspace** avec 2 composants: symbion-kernel (serveur) + symbion-plugin-hosts (agent)
- **Communication**: MQTT pour télémétrie, REST API pour contrôles
- **Fonctionnalités**: Monitoring infrastructure + hôtes + Wake-on-LAN + Data Ports
- **Contract Registry**: Système de versioning des events avec validation JSON
- **Data Ports v1**: Interface unifiée de persistence (memo, journal, finance...)

## 🎉 Phase A.1 - TERMINÉE COMPLÈTEMENT ✅

### ✅ Sécurité (Corrigé)
- **API key obligatoire** - routes protégées sauf `/health` 
- **Logs sécurité** ajoutés pour accès non autorisés
- **Versions rumqttc alignées** kernel=0.24.0, plugin=0.24.0
- **Config MQTT** depuis YAML au lieu de hardcodée

### ✅ Contract Registry (Opérationnel)
- **Chargement auto** des contrats depuis `contracts/mqtt/`
- **3 contrats MQTT** : heartbeat@v2, wake@v1, health@v1
- **5 contrats HTTP** : api.system.health, api.hosts, api.ports.memo, api.contracts, api.wake
- **API REST** `/contracts` et `/contracts/{name}` 
- **Validation JSON** basique (TODO: JSON Schema complet)

### ✅ Monitoring Infrastructure (Production Ready)
- **Auto-publication** kernel health toutes les 30s sur MQTT
- **Métriques temps réel** : uptime, mémoire, état MQTT, contrats chargés, hosts trackés
- **API `/system/health`** pour monitoring infrastructure
- **Tracking reconnexions** MQTT automatique

### ✅ Data Ports v1 (Système Unifié)
- **Architecture modulaire** : trait DataPort + PortRegistry
- **Port Memo v1** opérationnel : CRUD mémos avec filtrage (urgent, context, tags)
- **API REST complète** : `/ports`, `/ports/memo` (GET/POST/DELETE)
- **Stockage JSON** : ./data/memo.json avec cache mémoire
- **Schéma documenté** : contrats HTTP + exemples Postman

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
│   ├── http.rs                # API REST sécurisée (12 endpoints)
│   ├── mqtt.rs                # Event Bus MQTT + heartbeats
│   ├── config.rs              # Configuration YAML centralisée
│   ├── models.rs              # Structures de données partagées
│   ├── wol.rs                 # Wake-on-LAN via magic packets
│   ├── state.rs               # Gestion d'état thread-safe
│   ├── main.rs                # Orchestration générale
│   └── ports/
│       ├── mod.rs             # Data Ports architecture + PortRegistry
│       └── memo.rs            # Port mémos/rappels avec JSON storage
├── data/
│   └── memo.json              # Stockage persistent des mémos
contracts/
├── mqtt/                      # Contrats événements MQTT
│   ├── hosts.heartbeat.v2.json
│   ├── hosts.wake.v1.json
│   └── kernel.health.v1.json
└── http/                      # Contrats API REST
    ├── api.system.health.v1.json
    ├── api.hosts.v1.json
    ├── api.ports.memo.v1.json
    ├── api.contracts.v1.json
    └── api.wake.v1.json
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

### 🗂️ Data Ports v1
- `GET /ports` - Liste des ports + schémas
- `GET /ports/memo` - Lire mémos (avec filtres: urgent, context, tags)
- `POST /ports/memo` - Créer memo  
- `DELETE /ports/memo/{id}` - Supprimer memo

## Événements MQTT

- `symbion/hosts/heartbeat@v2` - Télémétrie hosts (CPU, RAM, IP)
- `symbion/hosts/wake@v1` - Demandes Wake-on-LAN  
- `symbion/kernel/health@v1` - Health kernel (auto toutes les 30s)

## État Système

- ✅ **Phase A.1 - Kernel 0.1** : Spine & DevKit TERMINÉ
- ✅ **Sécurité renforcée** : API key + logs + validation
- ✅ **Contract Registry** : MQTT + HTTP séparés  
- ✅ **Data Ports v1** : memo opérationnel + architecture extensible
- ✅ **Control API** : 12 endpoints documentés + testés
- ✅ **Documentation** : niveau professionnel sur tous les fichiers

## Prochaines Étapes

### 🔄 Phase A.2 - Plugin Manager (En cours)
- Chargement/déchargement à chaud de plugins
- Sandbox + healthcheck + lifecycle management  
- Rollback & safe-mode si plugin défaillant

### 🎯 Phase B - Noyau Utile (Hoodie)
- Plugin Hosts opérationnel (publication heartbeats)
- Plugin Memo/Rappels avec règles contextuelles
- Journal Auto unifié

## Commandes Utiles
```bash
# Build workspace
cargo build --workspace

# Run kernel avec API key
SYMBION_API_KEY="s3cr3t-42" cargo run

# Test API complet
curl -H "x-api-key: s3cr3t-42" http://192.168.1.14:8080/system/health
curl -H "x-api-key: s3cr3t-42" http://192.168.1.14:8080/ports
curl -H "x-api-key: s3cr3t-42" -X POST -H "Content-Type: application/json" \
  -d '{"content": "Test memo", "urgent": true}' http://192.168.1.14:8080/ports/memo

# Tests & Linting
cargo test --workspace
cargo clippy --workspace  
cargo fmt --workspace
```