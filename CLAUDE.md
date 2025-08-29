# Claude Context - NewSymbion

## Architecture Overview
- **Rust workspace** avec 2 composants: symbion-kernel (serveur) + symbion-plugin-hosts (agent)
- **Communication**: MQTT pour télémétrie, REST API pour contrôles
- **Fonctionnalités**: Monitoring infrastructure + hôtes + Wake-on-LAN
- **Contract Registry**: Système de versioning des events avec validation JSON

## Phase A.1 - État d'avancement ✅ TERMINÉ

### ✅ Sécurité (Corrigé)
- **API key obligatoire** - routes protégées sauf `/health` 
- **Logs sécurité** ajoutés pour accès non autorisés
- **Versions rumqttc alignées** kernel=0.24.0, plugin=0.24.0
- **Config MQTT** depuis YAML au lieu de hardcodée

### ✅ Contract Registry (Nouveau)
- **Chargement auto** des contrats depuis `contracts/*.json`
- **3 contrats** disponibles: heartbeat@v2, wake@v1, health@v1
- **API REST** `/contracts` et `/contracts/{name}` 
- **Validation JSON** basique (TODO: JSON Schema complet)

### ✅ Monitoring Infrastructure (Nouveau)
- **Auto-publication** kernel health toutes les 30s sur MQTT
- **Métriques temps réel** : uptime, mémoire, état MQTT, contrats chargés
- **API `/system/health`** pour monitoring infrastructure
- **Tracking reconnexions** MQTT automatique

## Architecture Actuelle

```
symbion-kernel/
├── src/
│   ├── contracts.rs    # Contract Registry + validation
│   ├── health.rs       # Infrastructure monitoring 
│   ├── http.rs         # API REST sécurisée
│   ├── mqtt.rs         # Event Bus MQTT
│   ├── config.rs       # Configuration YAML
│   ├── models.rs       # Structures de données
│   ├── wol.rs          # Wake-on-LAN
│   └── main.rs         # Orchestration
contracts/
├── hosts.heartbeat.v2.json   # Contrat télémétrie hosts
├── hosts.wake.v1.json        # Contrat Wake-on-LAN  
└── kernel.health.v1.json     # Contrat monitoring kernel
```

## API Endpoints (avec x-api-key)

- `GET /health` - Health check simple (pas de clé requise)
- `GET /system/health` - Métriques infrastructure complètes  
- `GET /hosts` - Liste des hosts monitoring
- `GET /hosts/{id}` - Détails d'un host
- `POST /wake?host_id=X` - Wake-on-LAN
- `GET /contracts` - Liste des contrats disponibles
- `GET /contracts/{name}` - Détail d'un contrat

## Événements MQTT

- `symbion/hosts/heartbeat@v2` - Télémétrie hosts (CPU, RAM, IP)
- `symbion/hosts/wake@v1` - Demandes Wake-on-LAN  
- `symbion/kernel/health@v1` - Health kernel (auto toutes les 30s)

## Prochaines étapes - Phase A.1 (Fin)

### 🚧 À venir
1. **Data Ports v1** - Interfaces memo, journal pour persistence
2. **Control API étendue** - Routes /plugins, gestion lifecycle

### 🔄 Phase A.2 - Plugin Manager  
- Chargement/déchargement à chaud
- Sandbox + healthcheck plugins
- Rollback & safe-mode

## Commandes Utiles
```bash
# Build
cargo build --workspace

# Run kernel 
SYMBION_API_KEY="s3cr3t-42" cargo run

# Test API avec clé
curl -H "x-api-key: s3cr3t-42" http://localhost:8080/system/health
curl -H "x-api-key: s3cr3t-42" http://localhost:8080/contracts

# Tests & Linting
cargo test --workspace
cargo clippy --workspace  
cargo fmt --workspace
```