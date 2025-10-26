# 🏗️ Architecture Distribuée Symbion

Guide complet du modèle **1 Kernel Central + N Agents Distribués**

## 📐 Modèle d'Architecture

### Principe Fondamental

Symbion utilise une **architecture hub-and-spoke** :

```
                    ┌─────────────────────────┐
                    │  🧠 KERNEL CENTRAL      │
                    │  (1 seul serveur)       │
                    │                         │
                    │  - MQTT Broker          │
                    │  - Agent Registry       │
                    │  - Context Engine       │
                    │  - Plugin System        │
                    │  - REST API             │
                    │  - Dashboard Hosting    │
                    └───────────┬─────────────┘
                                │
                    MQTT (port 1883) + HTTPS (port 8443)
                                │
        ┌───────────────────────┼───────────────────────┐
        │                       │                       │
        │                       │                       │
┌───────▼─────────┐    ┌───────▼─────────┐    ┌───────▼─────────┐
│ 🤖 AGENT 1      │    │ 🤖 AGENT 2      │    │ 🤖 AGENT 3      │
│ PC Bureau       │    │ Serveur Salon   │    │ Smartphone      │
│ (Windows)       │    │ (Linux)         │    │ (Android)       │
│                 │    │                 │    │                 │
│ - Monitoring    │    │ - Monitoring    │    │ - Dashboard PWA │
│ - CPU/RAM       │    │ - CPU/RAM       │    │ - Contrôles     │
│ - Contrôles     │    │ - Contrôles     │    │ - Notifications │
│ - Context envoi │    │ - Context envoi │    │                 │
└─────────────────┘    └─────────────────┘    └─────────────────┘
```

### Rôles et Responsabilités

#### 🧠 **Kernel Central** (1 instance unique)

**Plateforme recommandée** : Linux (Ubuntu/Debian) sur serveur domestique ou Raspberry Pi

**Responsabilités** :
- ✅ **MQTT Event Bus** : Coordination temps réel entre tous les agents
- ✅ **Agent Registry** : Enregistrement et monitoring de tous les agents
- ✅ **Context Engine** : Analyse intelligente des modes (Cravate/Intime/Neutre)
- ✅ **Plugin Orchestration** : Modules de vie (notes, santé, cuisine, finance)
- ✅ **REST API** : Interface unifiée pour tous les agents et interfaces
- ✅ **Dashboard Hosting** : Héberge le PWA accessible depuis tous devices
- ✅ **Health Tracking** : Surveillance santé globale de l'écosystème

**Installation** :
```bash
# Sur serveur Linux principal
sudo ./scripts/deploy-kernel.sh kernel-v1.1.0
cd systemd && sudo ./install-services.sh
```

#### 🤖 **Agents Distribués** (N instances, 1 par machine/device)

**Plateformes supportées** : Linux, Windows, Android (expérimental)

**Responsabilités** :
- ✅ **Monitoring local** : CPU, RAM, processus, réseau de la machine hôte
- ✅ **Heartbeat MQTT** : Envoi régulier de son état au kernel (toutes les 30s)
- ✅ **Context reporting** : Détection activité locale (SSID, horaires, apps)
- ✅ **Contrôles système** : Exécution commandes shutdown/hibernate/wake
- ✅ **Découverte réseau** : Scan devices locaux pour intégration domotique

**Installation** :

**Linux (autre machine)** :
```bash
# Sur chaque machine Linux additionnelle
sudo ./scripts/deploy-agent.sh agent-v1.0.0 192.168.1.14
```

**Windows** :
```powershell
# Sur chaque PC Windows
.\scripts\Deploy-SymbionAgent.ps1 -Version "agent-v1.0.0" -KernelHost "192.168.1.14"
.\scripts\Install-SymbionAgentService.ps1 -KernelHost "192.168.1.14"
```

---

## 🌐 Scénarios d'Usage Réels

### Scénario 1 : Configuration Domestique Typique

```
🏠 Maison Familiale
├─ 🧠 Kernel: Raspberry Pi 4 (Salon, toujours allumé)
│  └─ IP: 192.168.1.100
├─ 🤖 Agent 1: PC Bureau Windows (Chambre)
│  └─ Monitoring productivité + mode Cravate
├─ 🤖 Agent 2: Serveur Linux (Garage NAS)
│  └─ Monitoring infrastructure + stockage
├─ 📱 Interface: Smartphone Android (PWA)
│  └─ Dashboard contrôles domestiques
└─ 📱 Interface: Tablette Cuisine (PWA)
   └─ Suggestions repas + notes courses
```

**Flux de communication** :
1. Agent Windows détecte activité utilisateur → Heartbeat MQTT → Kernel
2. Kernel analyse contexte global → Mode Cravate activé
3. Dashboard PWA reçoit mise à jour contexte via WebSocket
4. Utilisateur envoie commande "Shutdown PC Bureau" depuis smartphone
5. Kernel route commande via MQTT → Agent Windows exécute

### Scénario 2 : Bureau Multi-Sites

```
🏢 Infrastructure Professionnelle
├─ 🧠 Kernel: VPS Cloud Linux (toujours accessible)
│  └─ IP publique: symbion.entreprise.com
├─ 🤖 Agent 1: PC Bureau Maison (Windows)
│  └─ Télétravail avec monitoring temps
├─ 🤖 Agent 2: PC Bureau Entreprise (Windows)
│  └─ Présentiel avec détection SSID entreprise
├─ 🤖 Agent 3: Laptop Déplacements (Linux)
│  └─ Mode nomade avec contexte géolocalisé
└─ 📱 Interface: Smartphone (PWA)
   └─ Suivi productivité + notes + temps
```

### Scénario 3 : Smart Home Avancé

```
🏡 Maison Connectée
├─ 🧠 Kernel: Serveur Linux (Placard technique)
│  └─ IP: 192.168.1.10
├─ 🤖 Agent 1: Raspberry Pi (Salon)
│  └─ Contrôle TV + multimédia + capteurs température
├─ 🤖 Agent 2: Raspberry Pi (Cuisine)
│  └─ Tablette murale + contrôles électroménager
├─ 🤖 Agent 3: PC Windows (Bureau)
│  └─ Monitoring workstation + éclairage intelligent
├─ 🤖 Agent 4: Android (Termux expérimental)
│  └─ Smartphone mobile avec location tracking
└─ 📱 Interfaces: Multiple PWA sur tous devices
```

---

## 🔄 Communication Inter-Composants

### MQTT Topics Structure

```
symbion/
├─ agents/
│  ├─ heartbeat@v1           → Agents envoient leur état
│  ├─ register@v1            → Nouveau agent s'annonce
│  ├─ commands/              → Kernel envoie commandes
│  │  ├─ {agent_id}/shutdown
│  │  ├─ {agent_id}/hibernate
│  │  └─ {agent_id}/wake
│  └─ telemetry/             → Métriques détaillées
│     └─ {agent_id}/metrics
├─ context/
│  ├─ mode_change@v1         → Changement Cravate/Intime/Neutre
│  └─ rules_triggered@v1     → Règles automation déclenchées
└─ plugins/
   ├─ notes/created@v1       → Nouvelle note créée
   └─ health/alert@v1        → Alerte santé système
```

### REST API Endpoints (Kernel)

```
Kernel Central (https://192.168.1.14:8443)

Agent Management:
  GET  /agents                → Liste tous agents enregistrés
  GET  /agents/{id}           → Détails agent spécifique
  POST /agents/{id}/shutdown  → Commande shutdown agent
  POST /agents/{id}/hibernate → Commande hibernate agent
  POST /wake?host_id={id}     → Wake-on-LAN agent

Context Engine:
  GET  /context/current       → Mode contexte actuel
  POST /context/override      → Forcer mode manuellement

Plugins:
  GET  /ports/memo            → Notes du plugin memo
  POST /ports/memo            → Créer nouvelle note

System:
  GET  /health                → Santé kernel
  GET  /system/health         → Santé globale écosystème
```

---

## 📦 Déploiement Multi-Machines

### Étape 1 : Installer le Kernel Central (1 fois)

**Sur votre serveur principal (Linux recommandé)** :

```bash
# 1. Cloner repo
git clone https://github.com/USER/NewSymbion.git
cd NewSymbion

# 2. Compiler kernel
cargo build --release -p symbion-kernel

# 3. Installer service systemd
cd systemd
sudo ./install-services.sh
# → Répondre "N" pour ne PAS installer l'agent local (sauf si désiré)

# 4. Vérifier kernel actif
curl -k https://localhost:8443/health
# Doit retourner: "ok"

# 5. Noter l'IP du serveur kernel
ip addr show | grep "inet "
# Ex: 192.168.1.14
```

### Étape 2 : Déployer Agents sur Autres Machines

#### **Agent Linux (PC/Serveur additionnel)** :

```bash
# Sur chaque machine Linux où vous voulez un agent

# Option A : Script déploiement automatique (recommandé)
curl -sSL https://raw.githubusercontent.com/USER/NewSymbion/master/scripts/deploy-agent.sh | \
  sudo bash -s agent-v1.0.0 192.168.1.14

# Option B : Installation manuelle
# 1. Télécharger binaire depuis releases GitHub
wget https://github.com/USER/NewSymbion/releases/download/agent-v1.0.0/symbion-agent-linux-x64-agent-v1.0.0
sudo mv symbion-agent-linux-x64-agent-v1.0.0 /opt/symbion/symbion-agent-host
sudo chmod +x /opt/symbion/symbion-agent-host

# 2. Créer service systemd
sudo nano /etc/systemd/system/symbion-agent.service

[Unit]
Description=Symbion Agent Host
After=network-online.target

[Service]
Type=simple
Environment="SYMBION_MQTT_BROKER=192.168.1.14:1883"
Environment="RUST_LOG=info"
ExecStart=/opt/symbion/symbion-agent-host
Restart=always
RestartSec=15s

[Install]
WantedBy=multi-user.target

# 3. Activer et démarrer
sudo systemctl daemon-reload
sudo systemctl enable symbion-agent
sudo systemctl start symbion-agent
```

#### **Agent Windows (PC)** :

```powershell
# Sur chaque PC Windows où vous voulez un agent

# Exécuter PowerShell en Administrateur

# 1. Télécharger scripts
Invoke-WebRequest -Uri "https://raw.githubusercontent.com/USER/NewSymbion/master/scripts/Deploy-SymbionAgent.ps1" -OutFile "Deploy-SymbionAgent.ps1"
Invoke-WebRequest -Uri "https://raw.githubusercontent.com/USER/NewSymbion/master/scripts/Install-SymbionAgentService.ps1" -OutFile "Install-SymbionAgentService.ps1"

# 2. Déployer agent (télécharge binaire depuis releases)
.\Deploy-SymbionAgent.ps1 -Version "agent-v1.0.0" -KernelHost "192.168.1.14"

# 3. Installer service Windows
.\Install-SymbionAgentService.ps1 -KernelHost "192.168.1.14"

# 4. Vérifier service actif
Get-Service SymbionAgent
```

#### **Agent Android (Expérimental - Termux)** :

```bash
# Sur smartphone Android avec Termux installé

# 1. Installer Termux depuis F-Droid (PAS Google Play)
# https://f-droid.org/packages/com.termux/

# 2. Installer Rust dans Termux
pkg update && pkg upgrade
pkg install rust openssl git

# 3. Cloner et compiler
git clone https://github.com/USER/NewSymbion.git
cd NewSymbion
cargo build --release -p symbion-agent-host

# 4. Lancer agent (foreground)
SYMBION_MQTT_BROKER="192.168.1.14:1883" \
  ./target/release/symbion-agent-host

# Note: Termux doit rester en foreground avec notification persistante
```

### Étape 3 : Accéder au Dashboard depuis N'importe Quel Device

#### **Desktop (Linux/Windows/macOS)** :

```
1. Ouvrir navigateur (Chrome, Firefox, Edge)
2. Naviguer vers: http://192.168.1.14:3000
3. Accepter certificat auto-signé si HTTPS
4. Dashboard s'affiche avec tous agents visibles
```

#### **Mobile (Android/iOS)** :

```
1. Ouvrir Chrome (Android) ou Safari (iOS)
2. Naviguer vers: http://192.168.1.14:3000
3. Menu → "Ajouter à l'écran d'accueil"
4. PWA s'installe comme app native
5. Ouvrir app depuis l'écran d'accueil
6. Mode offline + notifications activés
```

---

## 🔍 Vérification Post-Déploiement

### Vérifier Kernel Central

```bash
# Health check kernel
curl -k https://192.168.1.14:8443/health

# Liste agents enregistrés
curl -k -H "x-api-key: s3cr3t-42" https://192.168.1.14:8443/agents | jq

# Doit afficher JSON avec tous agents:
# [
#   {
#     "agent_id": "DESKTOP-WINDOWS",
#     "status": { "status": "online", "cpu_usage": 15.2, "memory_usage_mb": 156 }
#   },
#   {
#     "agent_id": "linux-salon",
#     "status": { "status": "online", "cpu_usage": 8.1, "memory_usage_mb": 89 }
#   }
# ]
```

### Vérifier Agent Individuel

**Linux** :
```bash
# Status service
sudo systemctl status symbion-agent

# Logs temps réel
journalctl -u symbion-agent -f

# Doit afficher:
# [agent] Connecting to MQTT broker: 192.168.1.14:1883
# [agent] Successfully registered to kernel
# [agent] Sending heartbeat...
```

**Windows** :
```powershell
# Status service
Get-Service SymbionAgent

# Logs
Get-Content C:\Symbion\logs\agent-stdout.log -Tail 50 -Wait
```

### Tester Communication Bi-directionnelle

```bash
# Depuis n'importe quelle machine avec accès kernel

# 1. Envoyer commande shutdown à agent Windows
curl -k -X POST -H "x-api-key: s3cr3t-42" \
  https://192.168.1.14:8443/agents/DESKTOP-WINDOWS/shutdown

# 2. Vérifier logs agent pour confirmation
# Agent doit recevoir commande via MQTT et l'exécuter

# 3. Vérifier sur dashboard que agent passe en "offline" après shutdown
```

---

## 🚨 Troubleshooting

### Problème : Agent ne s'enregistre pas au kernel

**Symptômes** :
- Agent démarre mais n'apparaît pas dans `/agents`
- Dashboard ne montre pas l'agent

**Causes possibles** :
1. **MQTT broker inaccessible**
   ```bash
   # Tester connectivité MQTT depuis machine agent
   telnet 192.168.1.14 1883
   # Doit se connecter. Si "Connection refused" → firewall
   ```

2. **Firewall bloque port 1883**
   ```bash
   # Sur serveur kernel, ouvrir port MQTT
   sudo ufw allow 1883/tcp comment 'MQTT broker pour agents'
   ```

3. **Mauvaise config MQTT_BROKER**
   ```bash
   # Vérifier variable d'environnement agent
   # Linux:
   systemctl show symbion-agent | grep SYMBION_MQTT_BROKER

   # Windows:
   C:\Tools\nssm\nssm.exe get SymbionAgent AppEnvironmentExtra
   ```

### Problème : Agent "offline" alors que service actif

**Symptômes** :
- Service agent running
- Dashboard affiche agent comme "offline"

**Causes possibles** :
1. **Timeout trop agressif**
   - Par défaut 2 minutes (ligne symbion-kernel/src/main.rs:145)
   - Augmenter à 5 minutes :
   ```rust
   AgentRegistry::start_agent_monitoring(agents.clone(), 5);
   ```

2. **Heartbeat MQTT bloqué**
   - Vérifier logs agent pour "heartbeat sent"
   - Vérifier logs kernel pour "heartbeat received"

### Problème : Dashboard inaccessible depuis autre machine

**Symptômes** :
- `curl http://192.168.1.14:3000` fonctionne sur serveur
- ERR_CONNECTION_REFUSED depuis autre machine

**Causes** :
1. **Firewall bloque port 3000**
   ```bash
   sudo ufw allow 3000/tcp comment 'Symbion Dashboard'
   ```

2. **Vite dev server listen sur 127.0.0.1 seulement**
   - Vérifier vite.config.js :
   ```javascript
   server: {
     host: '0.0.0.0',  // Doit être 0.0.0.0, PAS 127.0.0.1
     port: 3000
   }
   ```

---

## 📊 Métriques et Monitoring

### Dashboard Temps Réel

Le dashboard PWA affiche en temps réel :
- ✅ Nombre agents online/offline
- ✅ CPU/RAM de chaque agent
- ✅ Dernière activité (heartbeat timestamp)
- ✅ Platform (Linux/Windows/Android)
- ✅ Contexte global (Cravate/Intime/Neutre)

### Logs Centralisés

**Kernel (tous événements écosystème)** :
```bash
journalctl -u symbion-kernel -f
```

**Agent spécifique (Linux)** :
```bash
journalctl -u symbion-agent -f
```

**Agent spécifique (Windows)** :
```powershell
Get-Content C:\Symbion\logs\agent-stdout.log -Tail 50 -Wait
```

### Health Checks Automatiques

Le script `scripts/monitor-symbion.sh` (Linux) vérifie automatiquement :
- ✅ Kernel alive
- ✅ MQTT connected
- ✅ Agents online count
- ✅ Envoie email si problème détecté

---

**Documentation mise à jour** : 26 Octobre 2025
**Architecture** : Hub-and-Spoke (1 Kernel + N Agents)
**Protocoles** : MQTT (events) + HTTPS REST (API) + WebSocket (Dashboard)
