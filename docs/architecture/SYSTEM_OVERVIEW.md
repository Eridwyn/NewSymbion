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
- ✅ **API REST** - Interface pour contrôles et automatisations (~100 endpoints)
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

**Note** : Le kernel contient 36 fichiers Rust au total (21 modules racine + 14 dans decision/ + 1 dans ports/).

```
symbion-kernel/src/
├── main.rs              # Entry point, setup server
├── http.rs              # API REST (~100 routes, TLS)
├── auth.rs              # JWT, MFA, WebAuthn, sessions
├── csrf.rs              # CSRF protection tokens
├── mqtt.rs              # MQTT client, pub/sub handlers
├── agents.rs            # Agent registry, discovery (single file)
├── context.rs           # Context detection engine (single file)
├── contracts.rs         # MQTT schema validation (single file)
├── state.rs             # Shared application state
├── config.rs            # Configuration management
├── models.rs            # Data models
├── health.rs            # Health check endpoint
├── mfa.rs               # Multi-factor authentication
├── webauthn.rs          # Passkey biometric auth
├── device_trust.rs      # Trusted device management
├── notes_bridge.rs      # Notes plugin bridge
├── notes_ws.rs          # WebSocket notes streaming
├── plugins.rs           # Plugin orchestration
├── wol.rs               # Wake-on-LAN functionality
├── dashboard_events.rs  # Real-time PWA updates
├── decision_http.rs     # Decision engine HTTP endpoints
├── decision/            # Decision Engine (14 modules)
│   ├── mod.rs           # Module exports
│   ├── engine.rs        # Decision evaluation core
│   ├── trust.rs         # Trust score calculation (332 LOC)
│   ├── guards.rs        # Pre-decision validation
│   ├── idempotence.rs   # Command deduplication (231 LOC)
│   ├── validation.rs    # User approval workflow
│   ├── audit.rs         # Audit trail logging
│   ├── metrics.rs       # Decision metrics tracking
│   ├── config.rs        # Decision config management
│   ├── agent_status.rs  # Agent health monitoring
│   ├── override.rs      # Manual override handling
│   ├── persistence.rs   # State persistence
│   ├── clock.rs         # Time management & testing
│   └── types.rs         # Type definitions
└── ports/
    └── mod.rs           # Plugin port management
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
├── index.html            # Entry point, shell
├── main.js               # Application bootstrap
├── components/
│   ├── dashboard-app.js  # Main layout, routing
│   ├── notes-page.js     # Notes interface & filters
│   ├── boot-terminal.js  # Animated boot sequence
│   ├── passkey-manager.js# WebAuthn passkey management
│   ├── user-settings-page.js # User preferences
│   └── organic-loader.js # Bioluminescent loading (243 LOC)
├── widgets/
│   ├── notes-widget.js          # Markdown notes manager
│   ├── agents-network-widget.js # Agent visualization
│   ├── agent-control-widget.js  # Individual agent controls
│   ├── system-health-widget.js  # Health gauges
│   ├── hosts-widget.js          # Host/device management
│   ├── plugins-widget.js        # Plugin status
│   ├── context-widget.js        # Context mode display
│   ├── context-stats-widget.js  # Context statistics
│   ├── context-settings-widget.js # Context configuration
│   └── widget-registry.js       # Widget registration
├── services/
│   ├── mqtt-service.js         # MQTT WebSocket client
│   ├── api-service.js          # HTTP API wrapper
│   ├── auth-service.js         # Login, JWT, MFA
│   ├── agents-service.js       # Agent management
│   ├── context-service.js      # Context engine client
│   ├── csrf-service.js         # CSRF token handling
│   ├── decision-service.js     # Decision engine client
│   └── notes-stream-service.js # Notes WebSocket streaming
├── utils/
│   ├── notes-filters.js  # Note filtering logic
│   └── notes-scoring.js  # Relevance scoring
└── styles/
    └── shared-styles.js  # Design system tokens
```

---

## 💻 Tech Stack & Versions

### Backend (Kernel + Agent)
- **Rust**: 1.89.0 (2025)
- **Cargo**: 1.89.0
- **Frameworks**:
  - `axum` 0.7.x - HTTP server + routing
  - `tokio` 1.x - Async runtime
  - `rumqttc` 0.24.x - MQTT client
  - `tower` + `tower-http` - Middleware (CORS, rate limiting)
  - `serde` + `serde_json` - Serialization
  - `jsonwebtoken` 9.x - JWT authentication
  - `bcrypt` 0.15.x - Password hashing (cost factor 12)
  - `webauthn-rs` 0.5.x - Passkey biometric auth
  - `time` 0.3.x - Timezone handling (IANA Europe/Zurich)

### Frontend (PWA)
- **Lit**: 3.3.1 - Web Components framework
- **Vite**: 5.4.19 - Build tool + dev server
- **MQTT.js**: 5.x - MQTT WebSocket client
- **Marked.js**: 14.x - Markdown rendering (notes widget)

### Infrastructure
- **Mosquitto**: 2.0.18 - MQTT Broker
- **Systemd**: Service management + auto-restart
- **Let's Encrypt**: Production TLS certificates
- **mkcert**: Development CA (local certificates)

### Runtime Requirements
- **Kernel**: Linux/Windows, Rust 1.75+, 20MB RAM idle
- **Agent**: Linux/Windows/macOS, Rust 1.75+, 5MB RAM idle
- **PWA**: Modern browser (Chrome 90+, Firefox 88+, Safari 15+)
- **MQTT**: Mosquitto 2.0+, port 1883 (TCP) + 9001 (WSS)

---

## 🚀 Technologies IoT Intégrées

### 📡 Bus de Communication

- **MQTT**: Événements temps réel entre appareils (19 topics actifs: 13 documentés + 6 nouveaux dashboard/*)
- **REST API**: Contrôles synchrones et intégrations externes (~100 endpoints)
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
- **Production** : Let's Encrypt (auto-renewal via certbot)
- **Développement** : mkcert (CA local auto-signé)
- **Stockage** : `symbion-kernel/certs/` (cert + key)
- **Format** : PEM (Privacy Enhanced Mail)

**Protocoles & Cipher Suites** :
- **TLS Versions** : TLS 1.3 uniquement (TLS 1.2 et inférieur désactivés)
- **Cipher Suites** (TLS 1.3):
  - `TLS_AES_256_GCM_SHA384` (préféré)
  - `TLS_AES_128_GCM_SHA256`
  - `TLS_CHACHA20_POLY1305_SHA256`
- **HSTS**: `Strict-Transport-Security: max-age=31536000; includeSubDomains`
- **HTTP → HTTPS**: Redirection automatique port 8080 → 8443 (status 301)
- **Certificate Pinning**: Non implémenté (complexité vs bénéfice domestique)
- **OCSP Stapling**: Activé via Let's Encrypt

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
