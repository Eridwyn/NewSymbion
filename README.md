# 🧬 NewSymbion - Système Distribué Modulaire

**Cerveau personnel extensible** avec architecture distribuée moderne. Kernel central en Rust + plugins MQTT + PWA Dashboard temps réel.

> 🎉 **Phase A terminée** : Spine complet avec DevKit et PWA Dashboard fonctionnels !

## 🏗️ Architecture

### Composants principaux
- **🧬 symbion-kernel** : Serveur central (API REST + Event Bus MQTT + Plugin Manager)
- **📝 symbion-plugin-notes** : Système de notes distribuées avec CRUD complet
- **💻 symbion-plugin-hosts** : Agent de monitoring système (heartbeats, CPU, RAM, Wake-on-LAN)
- **🛠️ devkit/** : Suite de développement avec scaffolding et tests automatisés
- **📱 pwa-dashboard/** : Interface web temps réel (Lit + Vite + PWA)

### Stack technologique
- **Backend** : Rust + Tokio + rumqttc (MQTT) + axum (REST)
- **Communication** : MQTT pour événements + REST API sécurisée
- **Frontend** : Lit + Vite + PWA avec WebSockets temps réel
- **Contracts** : Système de versioning JSON pour validation des événements
- **DevExp** : Hot reload plugins + circuit breaker + health monitoring

## 🚀 Démarrage rapide

### Prérequis
- **Rust stable** + cargo
- **Mosquitto** (broker MQTT local)  
- **Node.js** + npm (pour le PWA dashboard)
- Linux/WSL recommandé

### 1. 🧬 Kernel central
```bash
git clone https://github.com/Eridwyn/NewSymbion
cd NewSymbion

# Configuration via .env (recommandé)
cp .env.example .env
# Éditez .env avec votre clé API

# OU export direct
export SYMBION_API_KEY="s3cr3t-42"

# Lancement kernel
cd symbion-kernel && cargo run
# ✅ Kernel started on http://0.0.0.0:8080
# ✅ Plugin Manager actif avec 2 plugins disponibles
```

### 2. 📝 Plugin Notes (recommandé)
```bash
# Terminal séparé
cd symbion-plugin-notes && cargo run
# ✅ Notes plugin connected via MQTT
# ✅ API /ports/memo disponible
```

### 3. 💻 Plugin Hosts (optionnel)
```bash  
# Terminal séparé
cd symbion-plugin-hosts && cargo run
# ✅ Host monitoring + heartbeats actifs
# ✅ Wake-on-LAN disponible
```

### 4. 📱 PWA Dashboard
```bash
# Terminal séparé  
cd pwa-dashboard

# Configuration (optionnel - clé API automatique en dev)
cp .env.example .env

# Lancement
npm install && npm run dev
# ✅ Dashboard disponible sur http://localhost:3000
# ✅ Interface temps réel avec widgets dynamiques
```

### 5. ✅ Vérification
```bash
# Health check
curl http://localhost:8080/health

# Monitoring infrastructure
curl -H "x-api-key: s3cr3t-42" http://localhost:8080/system/health

# Test notes distribuées
curl -H "x-api-key: s3cr3t-42" http://localhost:8080/ports/memo
curl -H "x-api-key: s3cr3t-42" -X POST -H "Content-Type: application/json" \
  -d '{"content": "Premier memo!", "urgent": true}' http://localhost:8080/ports/memo

# Plugin management
curl -H "x-api-key: s3cr3t-42" http://localhost:8080/plugins
```

## 🔌 API Endpoints (15 endpoints disponibles)

### 📊 Infrastructure & Monitoring
- `GET /health` - Health check simple (pas d'auth requise)
- `GET /system/health` - Métriques complètes (uptime, mémoire, MQTT, plugins)
- `GET /hosts` - Liste des hosts avec heartbeats temps réel  
- `GET /hosts/{id}` - Détails spécifiques d'un host

### 🔧 Plugin Management (hot reload)
- `GET /plugins` - Liste des plugins avec statuts
- `POST /plugins/{name}/start` - Démarrer un plugin à chaud
- `POST /plugins/{name}/stop` - Arrêter un plugin à chaud  
- `POST /plugins/{name}/restart` - Redémarrer un plugin

### 🗂️ Data Ports (architecture extensible)
- `GET /ports` - Ports disponibles (framework pour futurs plugins)
- `GET /ports/memo` - Lire notes avec filtres (urgent, context, tags)
- `POST /ports/memo` - Créer note avec métadonnées
- `PUT /ports/memo/{id}` - Modifier note existante
- `DELETE /ports/memo/{id}` - Supprimer note

### ⚡ Actions système  
- `POST /wake?host_id=X` - Wake-on-LAN magic packets

### 📜 Discovery & Contracts
- `GET /contracts` - Contrats MQTT disponibles (5 MQTT + 5 HTTP)
- `GET /contracts/{name}` - Détail d'un contrat avec validation

> 🔐 **Sécurité** : Tous les endpoints (sauf `/health`) nécessitent `x-api-key: s3cr3t-42`

## 🛠️ Développement

### Workspace Rust
```bash
# Build complet
cargo build --workspace

# Tests et qualité
cargo test --workspace
cargo clippy --workspace  
cargo fmt --workspace
```

### 🚀 DevKit avancé
```bash
# Générer un nouveau plugin
python3 devkit/scaffold-plugin.py mon-plugin --contracts heartbeat@v2

# Tests contractuels automatisés
python3 devkit/contract-tester.py --duration 30

# Tests DevKit
cd devkit && cargo test
```

### 📱 Frontend PWA
```bash
cd pwa-dashboard

# Mode développement avec proxy API  
npm run dev

# Build et preview production
npm run build && npm run serve
```

## 🎯 État du projet

### ✅ Phase A - Spine & DevKit (TERMINÉE)
- **🧬 Kernel 0.1** : Event Bus MQTT + Contract Registry + Plugin Manager
- **🔐 Sécurité** : API key + logs + protection endpoints  
- **📈 Monitoring** : Infrastructure health + métriques temps réel
- **🔌 Plugin System** : Hot reload + circuit breaker + health checks
- **🛠️ DevKit** : Scaffolding + tests automatisés + mocks/stubs
- **📱 PWA Dashboard** : Interface temps réel avec widgets dynamiques

### 🎯 Phase B - Noyau Utile (en cours)
Voir [ROADMAP.md](ROADMAP.md) pour les prochaines fonctionnalités :
- **📖 Journal Auto** unifié  
- **🧭 Context Engine** avec règles avancées
- **💰 Finance v1** avec budgets et import CSV