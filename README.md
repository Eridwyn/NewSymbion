# NewSymbion - Système Distribué Modulaire

Cerveau personnel modulaire avec architecture distribuée. Kernel central en Rust + plugins MQTT.  
API REST pour humains, bus MQTT pour communication inter-plugins.

## Architecture

### 🏗️ Composants
- **symbion-kernel** : Serveur central (API REST + MQTT broker)
- **symbion-plugin-notes** : Plugin de gestion des notes/mémos
- **symbion-plugin-hosts** : Agent de monitoring système (CPU, RAM, réseau)

### 🔄 Communication
- **REST API** : Interface humaine sécurisée (avec API key)
- **MQTT Bus** : Communication asynchrone entre plugins
- **Contracts Registry** : Validation des événements JSON

## Prérequis
- Rust (stable) + cargo  
- Mosquitto (broker MQTT) local
- Linux/WSL recommandé

## Démarrage rapide

### 1. Kernel central
```bash
git clone https://github.com/Eridwyn/NewSymbion
cd NewSymbion/symbion-kernel

# Config
cp kernel.yaml.example kernel.yaml
export SYMBION_API_KEY="s3cr3t-42"

# Lancement
cargo run
# -> listening on 0.0.0.0:8080
```

### 2. Plugin Notes (optionnel)
```bash
cd ../symbion-plugin-notes
cargo run
# -> notes plugin connecté via MQTT
```

### 3. Tests API
```bash
# Health check
curl http://localhost:8080/health

# Monitoring complet
curl -H "x-api-key: s3cr3t-42" http://localhost:8080/system/health

# Notes (via plugin MQTT)
curl -H "x-api-key: s3cr3t-42" http://localhost:8080/ports/memo
curl -H "x-api-key: s3cr3t-42" -X POST -H "Content-Type: application/json" \
  -d '{"content": "Test memo", "urgent": true}' http://localhost:8080/ports/memo
```

## API Endpoints

### 📊 Monitoring
- `GET /health` - Health check simple
- `GET /system/health` - Métriques infrastructure complètes
- `GET /hosts` - Liste des hosts connectés
- `GET /hosts/{id}` - Détails d'un host

### 🗂️ Data Ports (via plugins)
- `GET /ports` - Liste des ports disponibles
- `GET /ports/memo` - Lire notes (filtres: urgent, context, tags)
- `POST /ports/memo` - Créer note
- `DELETE /ports/memo/{id}` - Supprimer note

### ⚡ Actions
- `POST /wake?host_id=X` - Wake-on-LAN

### 📜 Discovery
- `GET /contracts` - Contrats MQTT disponibles
- `GET /contracts/{name}` - Détail d'un contrat

## Développement

```bash
# Build workspace complet
cargo build --workspace

# Tests
cargo test --workspace

# Linting
cargo clippy --workspace
cargo fmt --workspace
```