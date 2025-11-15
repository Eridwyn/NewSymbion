# Architecture Système Symbion

Vue d'ensemble de l'architecture IoT distribuée.

---

## 🏗️ Composants Principaux

### 🧬 symbion-kernel - Cerveau Central

**Rôle**: Centre nerveux de l'écosystème personnel

**Status**: ✅ Hub IoT Opérationnel + Monitoring Automatique
- Memory: 23.6MB
- MQTT: Connected
- Agents: 2+ actifs
- Surveillance: Cron 15 min + Alertes email Gmail

**Fonctionnalités**:
- ✅ **Event Bus MQTT** - Communication temps réel inter-appareils
- ✅ **Agent Registry** - Découverte et gestion appareils connectés
- ✅ **Plugin Orchestration** - Modules de vie (cuisine, santé, finance)
- ✅ **Context Engine** - Apprentissage habitudes et détection situations
- ✅ **Decision Engine** - Évaluation intelligente et garde-fous sécurité (trust scoring, validation multi-niveaux, audit trail)
- ✅ **API REST** - Interface pour contrôles et automatisations (85+ endpoints)
- ✅ **Health Monitoring** - Surveillance automatique + alertes proactives

### 🤖 symbion-agent-host - Assistants Domestiques

**Rôle**: Capteurs et actionneurs de l'environnement

**Status**: ✅ 2 Agents Actifs Multi-Environnement
- PC-Salon (Linux): Monitoring domestique + contrôle appareils
- PC-Bureau (Windows): Mode productivité + assistance professionnelle

**Fonctionnalités**:
- ✅ **Détection présence** - Activité système pour savoir si présent
- ✅ **Contrôle appareils** - Extinction/réveil machines selon contexte
- ✅ **Télémétrie environnementale** - Température, consommation, état matériel
- ✅ **Auto-découverte réseau** - Identification automatique appareils connectés
- ✅ **Adaptation contextuelle** - Comportement selon lieu (maison/bureau)

### 📱 pwa-dashboard - Interface Adaptative

**Rôle**: Miroir de l'écosystème qui s'adapte aux moments de la journée

**Status**: ✅ Interface Domestique Fonctionnelle
- URL: http://localhost:3000
- Widgets: Contrôles maison + Monitoring + Notes markdown + Santé système
- Mobile: Navigation fixe en bas + interface tactile optimisée

**Fonctionnalités**:
- ✅ **Widgets contextuels** - Interface change selon matin/soir/présence
- ✅ **Contrôles domestiques** - Gérer appareils connectés en un clic
- ✅ **Notes intelligentes** - Markdown rendering + expand/collapse + tags auto
- ✅ **PWA responsive** - Accessible tablette cuisine, smartphone, desktop
- ✅ **Navigation mobile fixe** - Tabs Contrôle/Système/Données ancrés en bas
- ✅ **Setup automatique certificat** - Téléchargement et vérification CA

### 📝 symbion-plugin-notes - Mémoire Externe

**Rôle**: Extension de la mémoire qui apprend les patterns

**Status**: ✅ Journal Contextuel Actif (2+ notes stockées)

**Fonctionnalités**:
- ✅ **Tags contextuels automatiques** - Selon SSID, heure, activité
- ✅ **Stockage distribué** - Notes accessibles sur tous appareils
- ✅ **Apprentissage habitudes** - Suggestions basées sur historique

---

## ⚡ Modes Contextuels Intelligents

### 👔 Symbion Cravate (Mode Professionnel)

**Détection**: SSID bureau + horaires 9h-18h + applications pro

**Fonctions IoT**:
- Focus mode avec notifications filtrées
- Préparation automatique notes clients/meetings
- Rappels pauses ergonomiques
- Optimisation éclairage/température bureau

### 🏡 Symbion Intime (Mode Domestique)

**Détection**: SSID domicile + soirée/weekend + apps loisir

**Fonctions IoT**:
- Suggestions repas selon frigo et restes
- Ambiance adaptive (éclairage selon humeur/météo)
- Contrôles entertainment et confort
- Coordination activités familiales

### 🌱 Symbion Neutre (Mode Surveillance)

**Toujours actif**: Maintenance et apprentissage continu

**Fonctions IoT**:
- Monitoring santé appareils domestiques
- Sauvegardes automatiques données personnelles
- Détection patterns comportementaux
- Optimisation énergétique silencieuse

---

## 📦 Structure des Modules

### Kernel (Rust - `symbion-kernel/src/`)
```
symbion-kernel/src/
├── main.rs              # Entry point, setup server
├── http.rs              # API REST (85+ routes, TLS)
├── auth.rs              # JWT, MFA, WebAuthn, sessions
├── csrf.rs              # CSRF protection tokens
├── mqtt.rs              # MQTT client, pub/sub handlers
├── agents/
│   ├── mod.rs           # Agent registry, discovery
│   └── commands.rs      # Remote commands (shutdown, processes)
├── context/
│   ├── mod.rs           # Context detection engine
│   ├── hysteresis.rs    # State stabilization
│   └── patterns.rs      # Behavioral pattern learning
├── decision/
│   ├── engine.rs        # Decision evaluation core
│   ├── trust.rs         # Trust score calculation (332 LOC)
│   ├── guards.rs        # Pre-decision validation
│   ├── idempotence.rs   # Command deduplication (264 LOC)
│   ├── audit.rs         # Audit trail logging
│   └── validation.rs    # User approval workflow
├── contracts/           # MQTT schema validation
├── dashboard_events.rs  # Real-time PWA updates
├── webauthn_manager.rs  # Passkey biometric auth
└── notes_ws.rs          # WebSocket notes streaming
```

### Agent (Rust - `symbion-agent-host/src/`)
```
symbion-agent-host/src/
├── main.rs              # Entry point, MQTT connect
├── telemetry.rs         # System metrics (CPU, RAM, disk, processes)
├── commands.rs          # Command execution (shutdown, reboot, kill)
├── presence.rs          # Activity detection (keyboard, mouse)
└── discovery.rs         # Network scanning, auto-registration
```

### PWA (Lit Web Components - `pwa-dashboard/src/`)
```
pwa-dashboard/src/
├── index.html           # Entry point, shell
├── components/
│   ├── app-shell.js     # Main layout, routing
│   ├── control-page.js  # Agent controls, commands
│   ├── system-page.js   # Health metrics, monitoring
│   ├── data-page.js     # Notes, patterns, history
│   └── widgets/
│       ├── notes-widget.js        # Markdown notes manager
│       ├── agents-network.js      # Agent visualization
│       ├── organic-loader.js      # Bioluminescent loading (810 LOC)
│       └── system-health-widget.js# Health gauges
├── services/
│   ├── mqtt-service.js  # MQTT WebSocket client
│   ├── api-service.js   # HTTP API wrapper
│   └── auth-service.js  # Login, JWT, MFA
└── styles/
    └── shared-styles.js # Design system tokens
```

---

## 🚀 Technologies IoT Intégrées

### 📡 Bus de Communication

- **MQTT**: Événements temps réel entre appareils (actif)
- **REST API**: Contrôles synchrones et intégrations externes (90+ endpoints)
- **WebSocket PWA**: Interface temps réel responsive
- **Contracts Registry**: Validation et versioning événements IoT

### 🧠 Intelligence Distribuée

- **Context Engine**: Détection SSID + horaires + patterns activité
- **Pattern Learning**: ML basique pour habitudes comportementales
- **Rule Engine**: Automatisations configurables (si-alors-action)
- **Semantic Tagging**: NLP basique pour catégorisation automatique

### 🔐 Sécurité Domestique

- **HTTPS/TLS Encryption**: Kernel HTTPS port 8443 (Let's Encrypt prod)
- **JWT Authentication**: Tokens JWT + bcrypt (cost factor 12)
- **Rate Limiting**: Protection API brute-force (5 req/sec par IP)
- **API Key Authentication**: Clé API secrète inter-services
- **Network Isolation**: Séparation appareils IoT du réseau principal
- **Device Authentication**: Certificats pour appareils de confiance
- **Command Validation**: Whitelist actions autorisées par contexte
- **Audit Trail**: Traçabilité complète automatisations domestiques

### 🌐 Architecture Réseau

**Ports Réseau** :
- **8080** (HTTP) : Redirection automatique → 8443 (HTTPS)
- **8443** (HTTPS) : API REST + WebSocket (TLS 1.3)
- **1883** (TCP) : MQTT Broker (Mosquitto, local only)
- **9001** (WSS) : MQTT WebSocket Secure (PWA → Broker)

**Flux TLS** :
```
┌─────────────┐         ┌──────────────┐         ┌─────────────┐
│ PWA/Agent   │  HTTPS  │   Kernel     │  MQTT   │  Mosquitto  │
│ (Client)    ├────────►│  (Server)    ├────────►│   Broker    │
│             │  :8443  │   TLS 1.3    │  :1883  │   (Local)   │
└─────────────┘         └──────────────┘         └─────────────┘
     │                                                   │
     │                    MQTT WSS :9001                 │
     └───────────────────────────────────────────────────┘
```

**Certificats TLS** :
- **Production** : Let's Encrypt (auto-renewal)
- **Développement** : mkcert (CA local)
- **Stockage** : `symbion-kernel/certs/` (cert + key)
- **Protocoles** : TLS 1.3 uniquement, HSTS headers activés

**Configuration** :
- Variables d'environnement :
  - `SYMBION_TLS_CERT_PATH` : Chemin certificat
  - `SYMBION_TLS_KEY_PATH` : Chemin clé privée
  - `SYMBION_MQTT_BROKER` : Adresse broker (défaut `127.0.0.1:1883`)
- Systemd service : `/etc/systemd/system/symbion-kernel.service`
- Healthcheck : `GET https://localhost:8443/health`

---

## 🔍 Agent Discovery Workflow

**Processus de Découverte Automatique** :

```
1. Agent Boot
   ↓
2. Network Scan (mDNS/ARP)
   │  → Discover Kernel IP
   ↓
3. MQTT Connect
   │  → broker: kernel_ip:1883
   ↓
4. Publish Registration
   │  topic: symbion/agents/register@v1
   │  payload: {
   │    agent_id: "hostname-uuid",
   │    capabilities: ["shutdown", "processes", "presence"],
   │    os: "Linux",
   │    version: "0.3.0"
   │  }
   ↓
5. Kernel Updates Registry
   │  → Add agent to online list
   │  → Store capabilities
   ↓
6. Subscribe Agent Topics
   │  → symbion/agents/{id}/command@v1
   │  → symbion/agents/{id}/shutdown@v1
   │  → symbion/agents/wake@v1
   ↓
7. Start Heartbeat Loop (30s)
   │  topic: symbion/agents/heartbeat@v1
   │  payload: {
   │    agent_id,
   │    metrics: { cpu, memory, disk },
   │    uptime_seconds,
   │    presence_detected: bool
   │  }
   ↓
8. Kernel Monitors Health
   │  → If no heartbeat > 90s: mark offline
   │  → Publish dashboard/agents@v1 update
   │  → Alert user if critical agent down
```

**Modes de Découverte** :

- **Auto-Discovery** : Agent scanne réseau local (mDNS, ARP)
- **Manual Config** : `SYMBION_KERNEL_HOST` environment variable
- **Zero-Config** : Multicast DNS résolution `symbion-kernel.local`

**Gestion Offline** :

- **Heartbeat Timeout** : 90 secondes → Status "offline"
- **Reconnexion** : Agent retry every 30s with exponential backoff
- **State Persistence** : Kernel stocke last_seen timestamp
- **Notifications** : Dashboard alerte si agent critique down > 5 min

---

## 🎯 Expérience Utilisateur

### 🌅 Matin
1. Symbion détecte réveil via activité système/réseau
2. Prépare automatiquement agenda + météo sur dashboard
3. Suggère petit-déjeuner selon frigo + préférences apprises
4. Active mode productivité si jour de travail détecté

### 🏢 Bureau
1. Détection SSID professionnel → Mode Cravate activé
2. Notifications personnelles filtrées automatiquement
3. Préparation notes clients/réunions selon planning
4. Rappels pauses ergonomiques + optimisation environnement

### 🏠 Retour Maison
1. Géolocalisation/SSID → Mode Intime activé
2. Suggestions menu selon restes + envies + objectifs santé
3. Ambiance adaptive (éclairage/température) selon humeur/météo
4. Interface tablette cuisine avec contrôles domestiques

### 🌙 Nuit
1. Sauvegarde automatique journée (notes + finances + santé)
2. Préparation lendemain selon agenda + habitudes apprises
3. Optimisation environnement sommeil (température, silence)
4. Mode surveillance énergétique nocturne

---

## 📚 Modules de Vie (Roadmap)

### 🍳 Module Cuisine (Phase C)
- Frigo connecté + inventaire automatique
- Suggestions repas IA selon restes + santé + goûts
- Électroménager programmable
- Assistant culinaire avec recettes adaptatives

### 💰 Module Finance (Phase D)
- Synchronisation banques + catégorisation automatique
- Budget intelligent avec alertes + optimisations
- Épargne automatique selon revenus + objectifs
- Conseil investissements basé sur profil

### 💪 Module Santé (Phase E)
- Coaching adaptatif selon forme + météo + planning
- Nutrition optimisée selon objectifs + préférences
- Sommeil intelligent + optimisation environnement
- Métriques holistiques (activité/humeur/productivité)

### 🤝 Module Famille (Phase F)
- Multi-utilisateurs avec profils personnalisés
- Coordination activités + planning partagé
- Communication contextuelle + médiation automatique
- Listes collaboratives intelligentes

---

## 🛠️ Installation

### 1. Hub Central (Une machine fixe)
```bash
cd NewSymbion/symbion-kernel
SYMBION_API_KEY="your-key" cargo run
```

### 2. Agents Domestiques (Un par pièce/contexte)
```bash
cargo run --release -p symbion-agent-host
```

### 3. Interface (Tablette/Mobile)
```bash
cd pwa-dashboard && npm run dev
# Dashboard: http://localhost:3000
```

---

Voir aussi:
- [PHILOSOPHY.md](../PHILOSOPHY.md) - Principes architecturaux
- [QUICK_REFERENCE.md](../QUICK_REFERENCE.md) - API et commandes
- [CODE_STANDARDS.md](../CODE_STANDARDS.md) - Normes de développement
