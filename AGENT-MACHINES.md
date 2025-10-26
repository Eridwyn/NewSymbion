# 🤖 Types de Machines pour Agents Symbion

Guide complet des machines supportées pour le déploiement d'agents.

## 🎯 Principe Général

**Un agent Symbion = 1 machine à monitorer/contrôler**

Chaque machine que tu veux intégrer dans ton écosystème Symbion doit avoir son propre agent installé localement.

---

## 💻 Types de Machines Supportées

### 1️⃣ **PC de Bureau / Workstation**

#### **Windows (Recommandé)**

**Cas d'usage** :
- ✅ PC bureau personnel (productivité, gaming)
- ✅ Workstation professionnelle (développement, design)
- ✅ PC multimédia salon (media center)

**Capacités de l'agent** :
- ✅ Monitoring CPU/RAM/Disque temps réel
- ✅ Détection activité utilisateur (processus actifs)
- ✅ Contrôles système (shutdown, hibernate, reboot)
- ✅ Détection contexte (SSID WiFi, horaires, apps)
- ✅ Wake-on-LAN (réveil à distance)

**Configuration minimale** :
- RAM : 4GB+ (agent utilise ~30MB)
- CPU : Tout processeur moderne
- OS : Windows 10/11
- Réseau : Connexion LAN ou WiFi stable

**Installation** :
```powershell
.\scripts\Deploy-SymbionAgent.ps1 -KernelHost "192.168.1.14"
.\scripts\Install-SymbionAgentService.ps1 -KernelHost "192.168.1.14"
```

**Exemple concret** :
```
PC-Bureau-DESKTOP-3BT760L (Windows 11)
├─ RAM: 16GB (agent utilise 25MB)
├─ CPU: i7-10700K
├─ Rôle: Station de travail bureau
├─ Contexte: Mode Cravate (9h-18h)
└─ Fonctions:
   ✅ Monitoring temps productivité
   ✅ Extinction automatique après inactivité
   ✅ Wake via smartphone avant arrivée
   ✅ Détection apps professionnelles
```

#### **Linux (Desktop)**

**Cas d'usage** :
- ✅ PC développement Linux
- ✅ Workstation scientifique/technique
- ✅ Media center (Kodi, Plex)

**Capacités identiques Windows** + :
- ✅ Accès logs système avancés
- ✅ Monitoring détaillé services systemd
- ✅ Contrôle containers Docker (si installé)

**Configuration minimale** :
- RAM : 2GB+ (agent ultra-léger)
- OS : Ubuntu 20.04+, Debian 11+, Fedora 35+
- systemd pour auto-start

**Installation** :
```bash
sudo ./scripts/deploy-agent.sh agent-v1.0.0 192.168.1.14
```

**Exemple concret** :
```
linux-dev-station (Ubuntu 22.04)
├─ RAM: 32GB (agent utilise 20MB)
├─ CPU: AMD Ryzen 9
├─ Rôle: Station dev full-stack
└─ Fonctions:
   ✅ Monitoring compilation Rust/Docker
   ✅ Alertes si disque >90% (builds)
   ✅ Auto-extinction après 2h inactivité
   ✅ Wake pour backups automatiques
```

---

### 2️⃣ **Serveurs / NAS**

#### **Serveur Linux Dédié**

**Cas d'usage** :
- ✅ Serveur domestique (home server)
- ✅ NAS (Synology, TrueNAS, custom)
- ✅ Serveur média (Plex, Jellyfin)
- ✅ Serveur auto-hébergement (Nextcloud, Bitwarden)

**Capacités de l'agent** :
- ✅ Monitoring uptime/health 24/7
- ✅ Alertes usage disque/RAM critique
- ✅ Monitoring services Docker/Systemd
- ✅ Backup automatique déclenché selon horaires
- ✅ Wake-on-LAN pour économie énergie

**Configuration minimale** :
- RAM : 1GB+ (agent très léger)
- OS : Toute distro Linux serveur
- Réseau : LAN Gigabit recommandé

**Installation** :
```bash
sudo ./scripts/deploy-agent.sh agent-v1.0.0 192.168.1.100
```

**Exemple concret** :
```
nas-home-server (Debian 12)
├─ RAM: 8GB (agent utilise 18MB)
├─ CPU: Intel N5105
├─ Stockage: 4x4TB RAID
├─ Rôle: NAS + Docker services
└─ Fonctions:
   ✅ Monitoring 24/7 health disques
   ✅ Alertes RAID dégradé
   ✅ Backup auto bases de données
   ✅ Extinction nocturne (2h-6h) si pas d'activité
   ✅ Wake automatique 6h matin
```

#### **Raspberry Pi / ARM**

**Cas d'usage** :
- ✅ Raspberry Pi 3/4/5 (home automation hub)
- ✅ Orange Pi, Rock Pi (alternatives)
- ✅ Serveur léger always-on

**Capacités de l'agent** :
- ✅ Monitoring température SoC
- ✅ Contrôle GPIO (capteurs, relais)
- ✅ Intégration domotique (Zigbee, Z-Wave)
- ✅ Consommation électrique très basse

**Configuration minimale** :
- RAM : 1GB+ (Raspberry Pi 3B minimum)
- OS : Raspberry Pi OS, Ubuntu ARM
- Stockage : SD Card 16GB+ (ou SSD recommandé)

**Installation** :
```bash
# Cross-compilation depuis PC Linux
cargo build --release --target aarch64-unknown-linux-gnu -p symbion-agent-host

# Ou compilation directe sur Raspberry Pi
sudo ./scripts/deploy-agent.sh agent-v1.0.0 192.168.1.50
```

**Exemple concret** :
```
raspberry-pi-salon (Raspberry Pi 4 - 4GB)
├─ RAM: 4GB (agent utilise 22MB)
├─ CPU: ARM Cortex-A72
├─ Rôle: Hub domotique + media
└─ Fonctions:
   ✅ Contrôle éclairage Philips Hue via GPIO
   ✅ Monitoring température salon (capteur DHT22)
   ✅ Serveur MQTT local redondant
   ✅ Détection présence via Bluetooth
   ✅ Kodi media center
```

---

### 3️⃣ **Machines Virtuelles / Cloud**

#### **VPS Cloud (Linux)**

**Cas d'usage** :
- ✅ Agent monitoring infrastructure cloud
- ✅ Serveur web/API distant
- ✅ CI/CD runner monitoring
- ⚠️ **NON recommandé pour kernel central** (latence MQTT)

**Capacités de l'agent** :
- ✅ Monitoring uptime/latence
- ✅ Alertes CPU/RAM/Bandwidth
- ✅ Health check services web
- ⚠️ Pas de contrôles shutdown (VPS géré par provider)

**Configuration minimale** :
- RAM : 512MB+ (agent très léger)
- CPU : 1 vCore
- Réseau : Connexion stable vers kernel (VPN recommandé si public)

**Installation** :
```bash
# Via SSH sur VPS
sudo ./scripts/deploy-agent.sh agent-v1.0.0 kernel.votre-domaine.local
```

**Exemple concret** :
```
vps-web-prod (Hetzner Cloud)
├─ RAM: 2GB
├─ CPU: 1 vCore Intel
├─ Rôle: Site web production
└─ Fonctions:
   ✅ Monitoring uptime Nginx/PostgreSQL
   ✅ Alertes si RAM >80% (memory leak)
   ✅ Health checks automatiques
   ⚠️ Pas de shutdown (VPS payant)
```

#### **VM locale (Proxmox, VMware, VirtualBox)**

**Cas d'usage** :
- ✅ Lab développement/test
- ✅ Homelab multi-VM
- ✅ Environnements isolés

**Capacités identiques** aux machines physiques selon OS invité

**Installation** :
Selon OS de la VM (Linux ou Windows)

---

### 4️⃣ **Laptops / Portables**

#### **Laptop Windows/Linux/macOS**

**Cas d'usage** :
- ✅ Laptop personnel nomade
- ✅ Laptop professionnel télétravail
- ✅ MacBook développement

**Capacités de l'agent** :
- ✅ Monitoring batterie (si supporté)
- ✅ Détection WiFi/SSID (géolocalisation contextuelle)
- ✅ Mode nomade (déconnexion kernel OK)
- ✅ Synchronisation notes offline → online

**Configuration minimale** :
- RAM : 4GB+
- OS : Windows 10+, Linux, macOS 11+

**Particularités** :
- ⚠️ Connexion intermittente au kernel (WiFi variable)
- ✅ Agent résilient : reconnexion automatique MQTT
- ✅ Mode dégradé offline fonctionnel

**Installation** :
```bash
# Linux/macOS
sudo ./scripts/deploy-agent.sh agent-v1.0.0 kernel-home.local

# Windows
.\scripts\Deploy-SymbionAgent.ps1 -KernelHost "kernel-home.local"
```

**Exemple concret** :
```
laptop-thinkpad-nomad (Linux Mint)
├─ RAM: 16GB
├─ CPU: i5-1135G7
├─ Rôle: Dev nomade + télétravail
└─ Fonctions:
   ✅ Détection SSID (bureau vs café vs maison)
   ✅ Context auto (Cravate au bureau, Intime à maison)
   ✅ Sync notes offline → kernel quand connecté
   ✅ Alertes batterie <20%
   ✅ Wake-on-LAN impossible (mobile)
```

---

### 5️⃣ **Smartphones / Tablettes (Expérimental)**

#### **Android (via Termux)**

**Cas d'usage** :
- 🔶 Smartphone comme agent mobile (expérimental)
- 🔶 Tablette fixe cuisine/salon (monitoring)
- ⚠️ **Recommandé seulement comme interface PWA**

**Capacités limitées** :
- 🔶 Monitoring CPU/RAM (limité sans root)
- ❌ Contrôles système (shutdown impossible)
- 🔶 Détection géolocalisation/WiFi (avec permissions)
- ⚠️ Termux doit rester foreground (notification)

**Configuration minimale** :
- RAM : 2GB+
- Android : 9.0+
- Termux depuis F-Droid (PAS Google Play)

**Installation** :
```bash
# Dans Termux
pkg install rust openssl git
git clone https://github.com/USER/NewSymbion.git
cd NewSymbion
cargo build --release -p symbion-agent-host
SYMBION_MQTT_BROKER="192.168.1.14:1883" ./target/release/symbion-agent-host
```

**Limitations** :
- ⚠️ Android tue processus background agressivement
- ⚠️ Batterie drainée si agent foreground permanent
- ⚠️ Pas de contrôles système sans root

**Recommandation** :
👉 **Utilisez Android comme interface (PWA Dashboard) plutôt que comme agent**

```
✅ Smartphone Android → Dashboard PWA installé
   - Contrôler tous agents depuis mobile
   - Recevoir notifications
   - Créer notes contextuelles
   - Monitoring temps réel

❌ Smartphone Android → Agent Termux
   - Complexe à maintenir
   - Batterie drainée
   - Fonctionnalités limitées
```

---

## 📊 Tableau Récapitulatif

| Type Machine | OS | RAM Min | Agent | Contrôles | Cas d'Usage Principal |
|--------------|----|---------|---------|-----------|-----------------------|
| **PC Bureau** | Windows | 4GB | ✅ Complet | ✅ Shutdown/Wake | Productivité, Gaming |
| **PC Desktop** | Linux | 2GB | ✅ Complet | ✅ Shutdown/Wake | Dev, Media Center |
| **Serveur** | Linux | 1GB | ✅ Complet | ✅ Shutdown/Wake | NAS, Services 24/7 |
| **Raspberry Pi** | Linux ARM | 1GB | ✅ Complet | ✅ Shutdown/GPIO | Hub domotique |
| **Laptop** | Win/Linux/macOS | 4GB | ✅ Complet | ⚠️ Nomade | Télétravail, Mobile |
| **VPS Cloud** | Linux | 512MB | ✅ Monitoring | ⚠️ Limité | Infra cloud |
| **VM locale** | Selon OS | Variable | ✅ Complet | ✅ Selon host | Lab, Tests |
| **Smartphone** | Android Termux | 2GB | 🔶 Expérimental | ❌ Très limité | ⚠️ NON recommandé |

**Légende** :
- ✅ Support complet
- 🔶 Support partiel/expérimental
- ⚠️ Limité ou déconseillé
- ❌ Non supporté

---

## 🏠 Exemples de Configurations Domestiques

### Configuration 1 : Maison Simple

```
Écosystème Symbion Domestique

🧠 Kernel (1x) : Raspberry Pi 4 (Salon, always-on)
🤖 Agents (2x) :
   - PC Bureau Windows (Chambre)
   - PC Linux Media Center (Salon)
📱 Interfaces (3x) :
   - Smartphone Android (PWA)
   - Tablette Cuisine (PWA)
   - Laptop nomade (PWA)

Total machines agents : 2
```

### Configuration 2 : Maison Connectée Avancée

```
Écosystème Symbion Smart Home

🧠 Kernel (1x) : Serveur Linux NAS (Placard technique)
🤖 Agents (5x) :
   - PC Bureau Windows (Bureau)
   - PC Linux Gaming (Salon)
   - Raspberry Pi 4 (Cuisine - contrôles domotique)
   - Raspberry Pi 3B (Chambre - capteurs)
   - NAS Synology (Garage - backups)
📱 Interfaces (4x) :
   - Smartphone Android principal (PWA)
   - Smartphone conjoint (PWA)
   - Tablette cuisine (PWA)
   - Laptop nomade (PWA)

Total machines agents : 5
```

### Configuration 3 : Télétravail Multi-Sites

```
Écosystème Symbion Pro

🧠 Kernel (1x) : VPS Cloud Linux (toujours accessible)
🤖 Agents (4x) :
   - PC Bureau Maison Windows (Télétravail)
   - Laptop Windows nomade (Déplacements)
   - PC Bureau Entreprise Windows (Présentiel)
   - Serveur Dev Linux (Cloud secondaire)
📱 Interfaces (2x) :
   - Smartphone Android (PWA)
   - Laptop (PWA intégrée)

Total machines agents : 4
```

---

## ❌ Machines NON Supportées

### Appareils IoT Basiques

**Ne PAS installer agent sur** :
- ❌ Ampoules connectées (Philips Hue, etc.)
- ❌ Prises intelligentes (TP-Link Kasa, etc.)
- ❌ Thermostats (Nest, Ecobee, etc.)
- ❌ Caméras IP (Xiaomi, etc.)
- ❌ Assistants vocaux (Alexa, Google Home)

**Pourquoi** :
- Pas d'OS complet (firmware propriétaire)
- Pas de capacité installer binaires Rust
- Puissance CPU/RAM insuffisante

**Alternative** :
👉 Contrôler ces appareils via un agent installé sur **Raspberry Pi** qui fait le pont :
```
Raspberry Pi (Agent Symbion)
├─ Plugin Philips Hue Bridge
├─ Plugin TP-Link Kasa
├─ Plugin MQTT → Zigbee2MQTT
└─ Contrôles tous devices IoT indirectement
```

### Consoles de Jeux

**Ne PAS installer agent sur** :
- ❌ PlayStation, Xbox, Nintendo Switch
- Raison : OS fermé, pas d'accès développeur

**Alternative** :
👉 Contrôler la prise électrique console via agent Raspberry Pi + prise connectée

---

## 🔧 Contraintes Techniques

### Réseau

**Requis** :
- ✅ Connexion TCP/IP vers kernel (LAN ou VPN)
- ✅ Ports accessibles : 8443 (HTTPS), 1883 (MQTT)
- ✅ DNS ou IP fixe pour kernel

**Débit minimum** :
- Upload agent → kernel : ~1 KB/s (heartbeat 30s)
- Download kernel → agent : ~1 KB/s (commandes)
- Très peu gourmand, fonctionne même sur 3G/4G mobile

### Ressources

**Agent Symbion consommation** :
- RAM : 15-30 MB (ultra-léger)
- CPU : <1% idle, ~5% pic lors monitoring
- Disque : 10MB binaire + ~100KB logs/jour
- Réseau : <1 MB/jour (heartbeats uniquement)

**Impact** :
- ✅ Négligeable sur toute machine moderne
- ✅ Compatible même Raspberry Pi Zero (512MB RAM)

---

## 💡 Recommandations

### Pour Débuter

**Configuration minimale recommandée** :
```
🧠 Kernel : 1x Raspberry Pi 4 (ou serveur Linux existant)
🤖 Agents : 2x machines les plus utilisées
   - Ton PC principal (Windows/Linux)
   - Ton serveur/NAS si tu en as un
📱 Interface : Ton smartphone (PWA)
```

### Pour Scale

**Ajout progressif agents** :
1. Commencer avec 1-2 machines critiques
2. Vérifier stabilité 1 semaine
3. Ajouter machines supplémentaires une par une
4. Tester contrôles (shutdown/wake) individuellement

**Limite théorique** :
- Aucune limite nombre agents
- Testé jusqu'à 50 agents simultanés
- MQTT supporte 1000+ clients facilement

---

**Documentation mise à jour** : 26 Octobre 2025
**Plateformes supportées** : Windows, Linux (x64, ARM), Android (expérimental)
**Recommandation** : PC/Serveurs pour agents, Smartphones pour interfaces PWA
