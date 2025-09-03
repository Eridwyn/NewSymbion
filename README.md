# 🧬 NewSymbion - Système Distribué Multi-OS v1.0.2

**Infrastructure de contrôle réseau local moderne** avec architecture distribuée. Kernel central + agents multi-OS + PWA Dashboard temps réel.

> 🎉 **Phase A terminée** : Spine complet + DevKit + PWA Dashboard  
> 🚀 **Phase B.5 en cours** : Agents LAN multi-OS avec contrôle système complet

## 🏗️ Architecture v1.0.2

### Composants principaux
- **🧬 symbion-kernel** : Serveur central (API REST + Event Bus MQTT + Plugin Manager + Agent Registry)
- **🤖 symbion-agent-host** : Agent multi-OS léger (monitoring + contrôle système + auto-update)
- **📝 symbion-plugin-notes** : Système de notes distribuées avec CRUD complet
- **🛠️ devkit/** : Suite de développement avec scaffolding et tests automatisés
- **📱 pwa-dashboard/** : Interface web temps réel avec widgets agents (Lit + Vite + PWA)

### Stack technologique
- **Backend** : Rust + Tokio + rumqttc (MQTT) + axum (REST) + cross-compilation
- **Communication** : MQTT pour événements agents + REST API sécurisée
- **Frontend** : Lit + Vite + PWA avec agents widgets et contrôles temps réel
- **Contracts** : Système de versioning JSON (9 MQTT + 5 HTTP)
- **Multi-OS** : Linux/Windows/Android avec capacités spécifiques par plateforme

## 🚀 Démarrage rapide

### Prérequis
- **Rust stable** + cargo (cross-compilation targets optionnels)
- **Mosquitto** (broker MQTT local)  
- **Node.js** + npm (pour le PWA dashboard)
- Linux/WSL/Windows/Android supportés

### 1. 🧬 Kernel central
```bash
git clone https://github.com/Eridwyn/NewSymbion
cd NewSymbion

# Configuration via .env (recommandé)
cp .env.example .env
# Éditez .env avec votre clé API

# Lancement kernel
cd symbion-kernel && cargo run
# ✅ Kernel started on http://0.0.0.0:8080
# ✅ Plugin Manager + Agent Registry actifs
```

### 2. 🤖 Agent multi-OS (recommandé)
```bash
# Build agent (local)
cargo build --release -p symbion-agent-host

# OU télécharger release GitHub
curl -L https://github.com/eridwyn/NewSymbion/releases/download/v1.0.2/symbion-agent-host-linux-x64 -o symbion-agent-host
chmod +x symbion-agent-host

# Premier lancement - wizard interactif
./symbion-agent-host
# ✅ Interactive CLI wizard pour configuration complète
# ✅ Auto-découverte MAC/IP + registration MQTT
# ✅ Système de mise à jour automatique activé
```

### 3. 📝 Plugin Notes (optionnel)
```bash
# Terminal séparé
cd symbion-plugin-notes && cargo run
# ✅ Notes plugin connected via MQTT
# ✅ API /ports/memo disponible
```

### 4. 📱 PWA Dashboard
```bash
cd pwa-dashboard

# Configuration (optionnel - clé API automatique en dev)
cp .env.example .env

# Lancement
npm install && npm run dev
# ✅ Dashboard sur http://localhost:3000
# ✅ Widget agents réseau + contrôles système temps réel
```

### 5. ✅ Vérification
```bash
# Health check
curl http://localhost:8080/health

# Agents découverts
curl -H "x-api-key: s3cr3t-42" http://localhost:8080/agents

# Test notes distribuées
curl -H "x-api-key: s3cr3t-42" http://localhost:8080/ports/memo
curl -H "x-api-key: s3cr3t-42" -X POST -H "Content-Type: application/json" \
  -d '{"content": "Premier memo!", "urgent": true}' http://localhost:8080/ports/memo
```

## 🔌 API Endpoints (25+ endpoints disponibles)

### 📊 Infrastructure & Monitoring
- `GET /health` - Health check simple (pas d'auth requise)
- `GET /system/health` - Métriques infrastructure (uptime, mémoire, MQTT, agents)

### 🤖 **Agents Management** (🆕 Phase B.5)
- `GET /agents` - Liste agents avec statuts + capacités multi-OS
- `GET /agents/{id}` - Détails agent + métriques temps réel
- `POST /agents/{id}/shutdown` - Extinction système à distance
- `POST /agents/{id}/reboot` - Redémarrage système à distance  
- `POST /agents/{id}/hibernate` - Mise en veille prolongée
- `GET /agents/{id}/processes` - Liste processus actifs
- `POST /agents/{id}/processes/{pid}/kill` - Tuer processus spécifique
- `POST /agents/{id}/command` - Exécuter commande shell sécurisée
- `GET /agents/{id}/metrics` - CPU, RAM, disque, température temps réel

### 🔧 Plugin Management
- `GET /plugins` - Liste plugins avec statuts
- `POST /plugins/{name}/start` - Démarrer plugin à chaud
- `POST /plugins/{name}/stop` - Arrêter plugin à chaud  
- `POST /plugins/{name}/restart` - Redémarrer plugin

### 🗂️ Notes System (Plugin distribué)
- `GET /ports/memo` - Lire notes avec filtres (urgent, context, tags)
- `POST /ports/memo` - Créer note avec métadonnées
- `PUT /ports/memo/{id}` - Modifier note existante
- `DELETE /ports/memo/{id}` - Supprimer note

### ⚡ Actions système  
- `POST /wake?host_id=X` - Wake-on-LAN magic packets

### 📜 Discovery & Contracts
- `GET /contracts` - Contrats MQTT disponibles (9 MQTT + 5 HTTP)
- `GET /contracts/{name}` - Détail contrat avec validation JSON

> 🔐 **Sécurité** : Tous les endpoints (sauf `/health`) nécessitent `x-api-key: s3cr3t-42`

## 🤖 Agent Multi-OS

### Capacités par plateforme
- **🐧 Linux** : systemctl, /proc/stats, bash commands, thermal sensors
- **🪟 Windows** : PowerShell, WMI, tasklist/taskkill, Performance Counters  
- **🤖 Android** : Termux shell, intents, battery APIs, process control

### Fonctionnalités clés
- **Auto-découverte** : MAC primaire, IP, hostname automatique
- **CLI Wizard** : Configuration interactive complète premier lancement
- **Auto-update** : Mise à jour automatique via GitHub releases
- **Cross-platform** : Binaries Linux/Windows disponibles automatiquement
- **Sécurité** : Stockage credentials via keyring système
- **Télémétrie** : Heartbeats enrichis toutes les 30s

### Déploiement
```bash
# Cross-compilation manuelle
cargo build --release --target x86_64-pc-windows-gnu -p symbion-agent-host
cargo build --release --target x86_64-unknown-linux-gnu -p symbion-agent-host

# OU utiliser releases GitHub automatiques
# Chaque tag v*.* déclenche build multi-plateforme CI/CD
```

## 🛠️ Développement

### Workspace Rust
```bash
# Build complet (kernel + agent + plugins + devkit)
cargo build --workspace

# Tests et qualité
cargo test --workspace
cargo clippy --workspace  
cargo fmt --workspace
```

### 🚀 DevKit avancé
```bash
# Générer nouveau plugin
python3 devkit/scaffold-plugin.py mon-plugin --contracts agents.heartbeat@v1

# Tests contractuels automatisés
python3 devkit/contract-tester.py --duration 30

# Tests DevKit
cd devkit && cargo test
```

### 📱 Frontend PWA avec agents
```bash
cd pwa-dashboard

# Mode développement avec proxy API + agents widgets
npm run dev

# Build production PWA
npm run build && npm run serve
```

## 🎯 État du projet v1.0.2

### ✅ Phase A - Spine & DevKit (TERMINÉE)
- **🧬 Kernel** : Event Bus MQTT + Contract Registry + Plugin Manager + Agent Registry
- **🔐 Sécurité** : API key + logs + protection endpoints  
- **📈 Monitoring** : Infrastructure health + métriques agents temps réel
- **🔌 Plugin System** : Hot reload + circuit breaker + health checks
- **🛠️ DevKit** : Scaffolding + tests automatisés + mocks/stubs
- **📱 PWA Dashboard** : Interface temps réel avec widgets dynamiques

### 🚀 Phase B.5 - Agents LAN v1 (EN COURS)
- **✅ Agent multi-OS** : Linux/Windows/Android avec capacités spécifiques
- **✅ Auto-découverte** : MAC, IP, hostname avec priorité réseau
- **✅ CLI Wizard** : Configuration interactive premier lancement  
- **✅ Auto-update** : Système de mise à jour GitHub releases
- **✅ Contrôle système** : Shutdown, reboot, processus, commandes à distance
- **✅ Télémétrie** : CPU, RAM, disque, température temps réel
- **⏳ PWA Extensions** : Widgets agents network + control modal détaillé

### 🎯 Prochaines phases
- **Phase F** : Agents distribués v2 avec authentification + mobile/VPN
- **Multi-utilisateurs** : Permissions topic-based + sécurité renforcée
- **Plugin Journal** : Auto-journalisation unifiée
- **Context Engine v2** : Détection automatique environnement

## 📊 Événements MQTT

### Agents System (🆕 v1.0.2)
- `symbion/agents/registration@v1` - Agent s'annonce (MAC, OS, hostname, capacités)
- `symbion/agents/command@v1` - Kernel → Agent (shutdown, reboot, kill_process, run_command, get_metrics) 
- `symbion/agents/response@v1` - Agent → Kernel (success/error + données résultat)
- `symbion/agents/heartbeat@v1` - Télémétrie enrichie (processus, disque, température, services)

### Infrastructure
- `symbion/kernel/health@v1` - Health kernel (auto toutes les 30s)
- `symbion/notes/command@v1` - Commandes vers plugin notes
- `symbion/notes/response@v1` - Réponses plugin notes
- `symbion/hosts/wake@v1` - Wake-on-LAN requests

## 🔧 Configuration

### Agent (via CLI wizard)
```toml
# ~/.config/symbion-agent/config.toml (généré automatiquement)
[mqtt]
broker_host = "127.0.0.1"
broker_port = 1883

[elevation]  
store_credentials = false
auto_elevate = false

[update]
auto_update = true
channel = "Stable"
check_interval_hours = 24
github_repo = "eridwyn/NewSymbion"

[agent]
agent_id = "auto"
hostname = "auto" 
version = "1.0.2"
```

### Kernel
```env
# .env
SYMBION_API_KEY=s3cr3t-42
SYMBION_MQTT_HOST=127.0.0.1
SYMBION_MQTT_PORT=1883
```

## 🚀 Releases & Auto-Update

### Système automatique
- **GitHub Actions** : Build cross-platform automatique sur git tags `v*.*`
- **Releases** : Binaries Linux/Windows/macOS générés automatiquement
- **Auto-update** : Agents vérifient et téléchargent nouvelles versions
- **CLI Wizard** : Configuration automatique premier lancement

### Commandes releases
```bash
# Créer nouvelle version
git tag v1.0.3 && git push origin v1.0.3

# GitHub Actions génère automatiquement :
# - symbion-agent-host-linux-x64
# - symbion-agent-host-windows-x64.exe  
# - symbion-agent-host-macos-x64
```

---

> **Symbion v1.0.2** - Infrastructure de contrôle réseau local production-ready  
> Rust workspace + agents multi-OS + PWA Dashboard + auto-update  
> 🔗 [GitHub](https://github.com/eridwyn/NewSymbion) • 📊 [Releases](https://github.com/eridwyn/NewSymbion/releases)