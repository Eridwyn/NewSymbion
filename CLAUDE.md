# Claude Context - NewSymbion

## Architecture Overview
- **Rust workspace** avec 3 composants: symbion-kernel (serveur) + symbion-plugin-hosts (agent) + symbion-plugin-notes (notes distribuées)
- **Communication**: MQTT pour télémétrie + IPC plugins, REST API pour contrôles
- **Fonctionnalités**: Monitoring infrastructure + hôtes + Wake-on-LAN + Plugin Manager + Notes distribuées
- **Contract Registry**: Système de versioning des events avec validation JSON
- **Plugin System**: Hot loading/unloading + circuit breaker + health monitoring

## 🎉 Phase A - TERMINÉE COMPLÈTEMENT ✅

### ✅ Phase A.1 - Kernel 0.1 & Plugin Manager

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
- **Plugin exclusif** : symbion-plugin-notes via MQTT uniquement
- **Bridge API** : routage direct vers plugin (pas de fallback)
- **Stockage JSON** : persistance dans ./notes.json (plugin)
- **Contrats MQTT** : notes.command@v1 + notes.response@v1

### ✅ Documentation Professionnelle
- **Tous les modules commentés** au niveau ports/mod.rs, plugins.rs, notes_bridge.rs
- **En-têtes détaillés** : rôle, fonctionnement, utilité dans Symbion
- **Exemples concrets** : JSON, YAML, usage patterns
- **Vision d'ensemble** : comment chaque module s'intègre
- **Architecture clean** : code legacy supprimé, plugins purs

### 🛠️ Phase A.3 - DevKit (TERMINÉ ✅)
- **Scaffolding automatique** : `devkit/scaffold-plugin.py` génère plugins complets
- **Templates Rust** : Cargo.toml + main.rs + manifeste JSON avec setup MQTT
- **Tests contractuels** : `devkit/contract-tester.py` valide conformité MQTT
- **Stubs/Mocks** : Bibliothèque `symbion-devkit` avec MockMqttClient + helpers
- **Contract helpers** : Chargement/validation/génération événements JSON
- **Test harness** : Système complet avec assertions et expectations

### 📱 Phase A.4 - PWA Dashboard (TERMINÉ ✅ + FIXES RÉCENTS)
- **Architecture moderne** : Lit + Vite + PWA avec service workers
- **Dashboard temps réel** : Interface adaptative avec widgets dynamiques
- **Services intégrés** : API REST + MQTT WebSocket pour événements live
- **Widgets système** : Santé, plugins, hosts, notes avec actions intégrées
- **Manifest-driven** : Widgets pilotés par manifestes des plugins
- **Responsive design** : Interface mobile-first avec animations fluides

#### 🔧 FIXES PWA Dashboard (Session du 31/08/2025) :
- **Plugins 0/2 → 2/2** : Fix comparaison status "Running" vs "running" 
- **Mémoire formatage** : Format 13.64 MB au lieu de 13.636719 MB
- **Hosts offline/online** : Détection bidirectionnelle robuste avec vérification 5s + API sync 30s
- **API Status robuste** : 5xx errors ne marquent plus toute l'API offline (différenciation plugin vs kernel)
- **README complet** : Documentation professionnelle avec 15 endpoints + Phase A terminée
- **Commentaires code** : vite.config.js et Cargo.toml documentés

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
│       └── mod.rs             # Data Ports architecture + PortRegistry (vide)
├── data/                      # Répertoire vide (plus de stockage local)
symbion-plugin-notes/
├── src/
│   └── main.rs                # Plugin notes distribué via MQTT
└── notes.json                 # Stockage principal des notes
devkit/                        # 🛠️ DevKit pour développement plugins
├── src/
│   ├── lib.rs                 # Point d'entrée bibliothèque DevKit
│   ├── mqtt_stub.rs           # MockMqttClient + helpers de test
│   ├── contract_helpers.rs    # Chargement/validation contrats JSON
│   └── test_utils.rs          # TestHarness + assertions plugins
├── templates/
│   └── plugin/                # Templates scaffolding nouveaux plugins
│       ├── Cargo.toml.template
│       ├── src/main.rs.template
│       └── plugin.json.template
├── scaffold-plugin.py         # 🔧 Générateur plugins automatique
├── contract-tester.py         # 🧪 Tests contractuels MQTT
└── Cargo.toml                 # Bibliothèque DevKit Rust
pwa-dashboard/                 # 📱 Dashboard PWA temps réel
├── src/
│   ├── main.js                # Point d'entrée PWA
│   ├── components/
│   │   └── dashboard-app.js   # Composant principal interface
│   ├── services/
│   │   ├── api-service.js     # Client API REST Symbion
│   │   └── mqtt-service.js    # Client MQTT WebSocket temps réel
│   └── widgets/
│       ├── widget-registry.js  # Registry widgets dynamiques
│       ├── system-health-widget.js  # Widget métriques kernel
│       ├── plugins-widget.js   # Widget gestion plugins
│       ├── hosts-widget.js     # Widget monitoring hosts
│       └── notes-widget.js     # Widget CRUD notes
├── public/
│   └── index.html             # PWA HTML + service worker
├── package.json               # Dépendances Lit + Vite + PWA
└── vite.config.js             # Config build + dev server + proxy API
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

### 🗂️ Notes System (Plugin distribué)
- `GET /ports/memo` - Lire notes avec filtres (urgent, context, tags)
- `POST /ports/memo` - Créer note avec métadonnées
- `PUT /ports/memo/{id}` - Modifier note existante
- `DELETE /ports/memo/{id}` - Supprimer note

### 🔧 Data Ports Framework
- `GET /ports` - Liste des ports disponibles (architecture extensible pour futurs plugins)

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

- ✅ **Phase A complète** : Spine & DevKit & PWA TERMINÉS
- ✅ **A.1 - Kernel 0.1** : Plugin Manager + sécurité + contracts + monitoring  
- ✅ **A.2 - Notes Migration** : Système distribué via MQTT
- ✅ **A.3 - DevKit** : Scaffolding + tests + stubs + templates
- ✅ **A.4 - PWA Dashboard** : Interface temps réel + widgets + responsiveness
- ✅ **Architecture production-ready** : 15 endpoints API + validation + documentation

## Phase Actuelle

### 🎯 Phase B - Noyau Utile (PRÊT à démarrer)
- ✅ Plugin Hosts opérationnel (heartbeats + Wake-on-LAN + monitoring temps réel)
- ✅ Plugin Notes opérationnel (CRUD distribué via MQTT complet)
- ✅ Dashboard web temps réel (PWA fonctionnel avec widgets dynamiques)
- ⏳ Plugin Journal Auto unifié (prochaine étape prioritaire)
- ⏳ Règles contextuelles avancées pour notes avec SSID/horaires
- ⏳ Context Engine v2 avec détection automatique environnement

## Commandes Utiles

### 🦀 Backend Rust
```bash
# Build workspace complet
cargo build --workspace

# Run kernel avec API key
cd symbion-kernel && SYMBION_API_KEY="s3cr3t-42" cargo run

# Tests & Linting
cargo test --workspace
cargo clippy --workspace  
cargo fmt --workspace
```

### 📱 Frontend PWA 
```bash
# Setup dashboard (première fois)
cd pwa-dashboard && npm install

# Mode développement avec proxy API + hot reload
npm run dev  # → http://localhost:3000 (fonctionnel avec fixes récents)

# Build production + service worker PWA
npm run build && npm run serve
```

### 🛠️ DevKit 
```bash
# Génération d'un nouveau plugin
python3 devkit/scaffold-plugin.py my-plugin --contracts heartbeat@v2 --description "Mon plugin"

# Tests contractuels (avec kernel + plugins actifs)  
python3 devkit/contract-tester.py --duration 15

# Tests DevKit
cd devkit && cargo test
```

### 🔍 Test API complet
```bash
curl -H "x-api-key: s3cr3t-42" http://localhost:8080/system/health
curl -H "x-api-key: s3cr3t-42" http://localhost:8080/plugins
curl -H "x-api-key: s3cr3t-42" http://localhost:8080/ports/memo
curl -H "x-api-key: s3cr3t-42" -X POST -H "Content-Type: application/json" \
  -d '{"content": "Test memo distribué", "urgent": true, "context": "test"}' http://localhost:8080/ports/memo
```