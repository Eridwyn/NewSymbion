# Changelog - Symbion

Historique des améliorations et changements majeurs du projet.

---

## Extension templates actions à 5 plugins + fix register stale routes (11 Mai 2026)

**Status**: 🟢 Complété (commits `4248b0c` → `7405804`)

### Plugins enrichis avec templates pour le rule builder PWA
- `ssl` : 1 action (check_now — POST /check, raw, force vérif certs)
- `notes` : 2 actions (create_note, delete_note — wrap v1 sur /actions)
- `telegram` : 2 actions (send_notification, send_message — wrap v1)
- `sensors` : 3 actions (list_sensors, get_environment, get_sensor — wrap v1)
- `coffee` : 4 actions inchangées (power_on, power_off, brew, stop — raw)

**Total : 12 actions templates sur 5 plugins**. Plus de payload JSON à taper.

### Plugins sans templates (volontairement)
- `library` : POST /nodes prend une structure trop complexe (parent_id, template_id,
  fields dynamiques). Pas pertinent pour automation simple.
- `freebox` : pas de POST utile (lecture uniquement via /health + MQTT)

### Bug racine corrigé (`symbion-kernel/src/plugin_proxy.rs::register_plugin`)
**Symptôme** : malgré `PluginRegistrationBuilder.action()` qui POST 200 OK,
`GET /v1/plugins` montrait parfois 0 actions pour certains plugins.

**Cause** : l'auto-discovery au boot kernel inscrivait chaque plugin avec route
racine `""` (catch-all). Le plugin re-register ensuite avec ses vraies routes
(`/power`, `/brew`, ...). L'ancien entry `""` persistait dans le HashMap car
`insert` ne purgeait pas les autres. `list_plugins()` agrégeait par `name` avec
ordre HashMap non-déterministe → renvoyait parfois l'instance stale (routes `[""]`,
actions vides).

**Fix** : `retain()` purge toutes les entries du même plugin name AVANT l'insert.
Plus de conflit auto-discovery vs manual register.

### Comment ajouter des templates à un plugin
Voir [`docs/PLUGIN_DEVELOPMENT_GUIDE.md §6`](PLUGIN_DEVELOPMENT_GUIDE.md). Pattern
minimal :
```rust
PluginRegistrationBuilder::new(PLUGIN_ID, SOCKET_PATH)
    .route("/health")
    .action(PluginAction { name, label, route, method, impact_level, wrap_protocol, params })
    .register().await
```

---

## Action plugin_command + templates structurés par plugin (10 Mai 2026)

**Status**: 🟢 Complété (commits `aa992bf` → `6da3a53`)

### Objectif
Permettre aux automations d'appeler n'importe quel endpoint plugin via Unix socket
(ex: « préchauffer la machine à café à 7h », « réindexer la library la nuit »)
sans ajouter de logique métier au kernel.

### Backend kernel
- Nouveau variant `ActionDefinition::PluginCommand { plugin, route, payload, impact_level }`
  dans `symbion-kernel/src/automations/types.rs`
- `PluginCommandExecutor` dans `automations/executors.rs` : POST HTTP via
  `tokio::net::UnixStream` + `hyper::client::conn::http1`. Résolution socket via
  `PluginRegistry::find_socket(/v1/plugin-api/{plugin}/{route})`.
- Branchement complet : `engine.rs` (auto trigger), `automations_http.rs` (run manuel),
  `http/decision.rs` (exécution post-validation manuelle), `decision_bridge.rs` (description)
- Validations executor : trim + rejet whitespace dans route, auto-parse payload string→json
  (cas du textarea PWA qui sérialise la valeur saisie en string)

### Schema rule builder enrichi
- Features dans le dropdown : 8 hardcodées → **41** (énumération `FeatureRegistry` runtime
  + dédoublonnage via BTreeMap + icône par préfixe `coffee.*`/`library.*`/`ssl.*`/...)
- Plugins dans le dropdown : 4 hardcodés → **6+** (énumération `PluginRegistry` runtime
  — coffee, library, telegram apparaissent maintenant)
- Nouvelle action `plugin_command` exposée dans le rule builder

### Templates structurés par plugin (option 3 d'UX)
- `PluginRegistration` (common) : nouveau champ `actions: Vec<PluginAction>`
- `PluginAction { name, label, icon, route, method, impact_level, params }`
- `PluginActionParam { name, label, param_type, required, default, options, min, max, placeholder }`
  avec types `bool`/`int`/`float`/`string`/`select`/`text_area`
- `PluginRegistrationBuilder.action(...)` pour déclarer les templates
- `PluginInfo` (kernel) expose `actions` via `GET /v1/plugins`
- PWA `renderPluginCommandConfig()` : 2 selects cascadés (plugin → action) + sub-form
  généré dynamiquement depuis `params`. Fallback gracieux vers textarea libre si
  plugin sans templates.

### Plugin coffee = exemple de référence
4 actions déclarées : `power_on`, `power_off`, `brew` (3 params : drink/temperature/cups), `stop`.
Plus de payload JSON à taper dans le rule builder pour ces actions.

### Bugs collatéraux fixés (même session)
- `set_mode_natural` ne republiait pas sur MQTT → le retain stale après auto-apply
  Intelligence v2. Fix : republier dans `spawn_intelligence_monitor` + au boot
  du kernel pour synchroniser le retain avec l'état mémoire chargé depuis disk
- `handle_list_plugins` reconstruit le JSON manuellement → tout nouveau champ
  `PluginInfo` reste invisible côté API tant qu'on ne l'ajoute pas explicitement
  (anti-pattern documenté en mémoire pour audit futur)
- Conditions des automations café (`water_low`, `beans_low`, `descale`) : passé
  de `coffee.online=true` à `coffee.ready=true` (mainstate=2). En standby la machine
  renvoie `level=0` mais reste « online HTTP » → faux positifs d'alerte
- `auto_coffee_ready_morning` supprimé (notif inutile qui constate au lieu d'anticiper)

### Comment ajouter des templates à un plugin
Voir [`docs/PLUGIN_DEVELOPMENT_GUIDE.md`](PLUGIN_DEVELOPMENT_GUIDE.md) §6 « Déclarer
des actions templates pour le rule builder ».

---

## ☕ Coffee + Library — Intégration Intelligence Engine (9 Mai 2026)

**Status**: 🟢 Complété (commit `49e5d7d`)

### Bug schéma MQTT (silencieux depuis le déploiement)
Les plugins `coffee` et `library` publiaient sur `symbion/features/update` avec un schéma incompatible (`{"features": {...}}` wrappé) que le kernel rejetait silencieusement à la désérialisation. Aucune feature ne remontait au `FeatureRegistry`, le Decision Engine ignorait totalement l'état machine à café et la knowledge base.

### Fix
- **Refactor `publish_features`** sur les deux plugins : un message MQTT par feature, schéma `FeatureUpdate { source, feature_id, value, timestamp, ttl_seconds }` matchant `ExternalFeatureUpdate` côté kernel (`symbion-kernel/src/mqtt.rs:66-72`)
- Pattern de référence : `symbion-plugin-ssl/src/mqtt.rs:241-246`

### Coffee : nouvelles features + automations
- `MachineStatus` étendu : `brew_count_today`, `last_brew_at`, `brew_count_date` (volatile, reset à minuit)
- Hook `brewing/completed` incrémente le compteur et timestamp le dernier brew
- 11 features désormais exposées : `coffee.online`, `.ready`, `.brewing`, `.brew_progress`, `.water_level`, `.bean_level`, `.maintenance`, `.descale_status`, `.aquaclean_remaining`, `.brews_today`, `.last_brew_minutes_ago`
- 4 automations cibles dans le Decision Engine :
  - `auto_coffee_ready_morning` — notif Telegram P2 quand `coffee.ready=true` entre 6h-10h hors mode veille (cooldown 30 min)
  - `auto_coffee_water_low` — niveau eau < 20 % (cooldown 1 h)
  - `auto_coffee_beans_low` — niveau grains < 20 % (cooldown 1 h)
  - `auto_coffee_descale` — `descale_status <= 4` (logique inversée : 0 = besoin urgent, cooldown 24 h)

### Library : 3 features remontent désormais
- `library.nodes.count`, `library.sections.count`, `library.pending_links.count`

### Validation E2E
Run manuel `POST /v1/automations/auto_coffee_water_low/run` → Decision Engine approuve (trust 0.79) → notification émise → reçue par `symbion-plugin-telegram` via MQTT.

### Découvertes documentées (second brain)
- Plugins déployés selon 2 patterns différents : `target/release/` direct (coffee) vs `/opt/symbion/bin/` (library) — un restart après rebuild ne suffit pas pour ce dernier
- Moteur d'automations : dual storage SQLite (primary, override JSON au boot) + JSON (backup). Modifier `data/automations.json` à la main pendant runtime ne sert à rien.

---

## 🔐 PWA Auth & Widget Fixes (31 Mars 2026)

**Status**: 🟢 Complété

### Session Expiry Auto-Redirect
- **Fix** : La page login s'affiche automatiquement quand la session expire (critique sur mobile PWA standalone)
- **Root cause** : 3 problèmes combinés
  1. SW SWR cache servait des 200 cachés masquant les vrais 401
  2. SW vault continuait d'injecter un token périmé après AUTH_CLEAR
  3. z-index boot-terminal (1200) < overlays (9999) empêchant l'affichage
- **Corrections** :
  - SW vérifie JWT exp avant injection + purge SWR cache sur AUTH_CLEAR
  - AuthService écoute `auth:expired` pour nettoyer état + IndexedDB + SW vault
  - SymbionApp gère la transition boot↔dashboard via event `auth:expired`
  - Logout dispatch `auth:expired` au lieu de `window.location.reload()`
  - boot-terminal z-index → 99999

### Widget Race Condition au Reload
- **Fix** : `await authService.whenReady()` dans dashboard-app avant chargement widgets
- **Root cause** : Widgets démarraient le polling avant restauration du token depuis IndexedDB
- **Impact** : Élimine les erreurs 521/auth transitoires au rechargement de page

### Widget Santé Système "Déconnecté"
- **Fix** : `handleApiStatus` met `connected=true` quand status='online'
- **Root cause** : Le handler ne settait connected que sur offline, jamais sur online

### Mode Pro Couleur Incorrecte
- **Fix** : Ajout `modes` dans le regex nginx pour routes API
- **Root cause** : `/modes` non routé par nginx → retournait HTML au lieu de JSON

### SSL Widget Refresh
- **Fix** : Le bouton refresh appelle maintenant `POST /ssl/check` + `GET /ssl/domains`
- **Root cause** : Ne faisait que relire le cache MQTT local (données potentiellement obsolètes)

### Nouveaux Slash Commands
- `/debug-pwa` : Investigation automatique problèmes PWA avec agents parallèles
- `/fix-widget` : Fix rapide widget avec inventaire des sources de données
- `/deploy-pwa` : Build + deploy PWA (vite preview port 3001)
- `/nginx-check` : Vérification/mise à jour config nginx

---

## 🌡️ F1: Environment Sensors Plugin (18 Novembre 2025)

**Status**: 🟢 100% complété

### Plugin Sensors-Manager Implémenté

#### Nouvelle fonctionnalité : Plugin ESP32 Environment Sensors
- **Feature** : Plugin standalone pour monitoring environnement (température, humidité)
- **Architecture** :
  - Binary indépendant communiquant via MQTT
  - Auto-registration des capteurs ESP32 BME280
  - Stockage en mémoire thread-safe (RwLock)
  - Circular buffer (max 100 readings par room)
  - Status evaluation automatique (Normal, WarningVentilate >65%, RiskMold >70%, TempLow <16°C)
- **MQTT Topics** :
  - Subscribe: `symbion/sensors/registration@v1`, `symbion/sensors/+/env@v1`
  - Publish: `symbion/plugin/sensors/response@v1`, `symbion/dashboard/environment@v1`
- **Fichiers créés** :
  - `symbion-plugin-sensors/src/main.rs` (275 lignes)
  - `symbion-plugin-sensors/Cargo.toml`
  - `plugins/symbion-plugin-sensors.json` (manifest complet)
- **Documentation** : Guide complet de création de plugins ajouté
  - `docs/PLUGIN_DEVELOPMENT_GUIDE.md` (450+ lignes)
  - Checklist complète
  - Debugging guide
  - Pièges courants et solutions
- **Capteur actif** :
  - ESP32-CDE370 (chambre) : 22.9°C, 79.6% humidité
  - Fréquence : 5 secondes
  - Status : RiskMold détecté (>70%)

#### Corrections et Apprentissages

**Manifest du Plugin** - Tous les champs obligatoires :
- `contracts` : Array des topics MQTT (obligatoire, peut être vide)
- `restart_on_failure` : Boolean pour redémarrage automatique
- `startup_timeout_seconds`, `shutdown_timeout_seconds` : Timeouts
- `depends_on` : Array dépendances (obligatoire, peut être vide)
- `start_priority` : Nombre 0-100 pour ordre démarrage
- `env` : Object variables environnement (obligatoire, peut être vide)

**Chemin Binary** :
- ❌ `../target/release/...` (relatif depuis plugins/)
- ✅ `./target/release/...` (relatif depuis racine projet)
- Le kernel s'exécute depuis la racine, pas depuis `plugins/`

**Permissions** :
- Manifest JSON : `644 eridwyn:eridwyn` (readable par kernel)
- Binary : `755 eridwyn:eridwyn` (exécutable)

**Workspace** :
- Ajouter nouveau plugin dans `Cargo.toml` racine `members = [...]`

### F1 HTTP API Endpoints (18 Novembre 2025 - après-midi)

#### API REST pour Environnement IoT
- **Feature** : 5 nouveaux endpoints HTTP pour consultation données capteurs
- **Endpoints ajoutés** :
  - `GET /v1/environment/sensors` - Liste tous les capteurs
  - `GET /v1/environment/sensors/{sensor_id}` - Détails capteur + état actuel
  - `GET /v1/environment/sensors/{sensor_id}/history?hours=24` - Historique par capteur
  - `DELETE /v1/environment/sensors/{sensor_id}` - Désinscrire capteur (manuel)
  - `GET /v1/environment/{room_id}` - État actuel d'une pièce (aggregation multi-sensors)
  - `GET /v1/environment/{room_id}/history?hours=24` - Historique filtré par pièce
- **Architecture** :
  - SensorRegistry avec méthodes thread-safe (`get_environment_by_room()`)
  - Aggregation multi-sensors par room_id (sélection reading la plus récente)
  - History filtering par paramètre `?hours=N`
- **Fichiers modifiés** :
  - `symbion-kernel/src/sensors.rs:161-192` - Nouvelle méthode `get_environment_by_room()`
  - `symbion-kernel/src/environment_http.rs:58-161` - 2 nouveaux endpoints room-based
- **Tests** :
  - ✅ `GET /v1/environment/chambre` → 200 OK (22.9°C, 79.6%, RiskMold)
  - ✅ `GET /v1/environment/chambre/history?hours=24` → Array de readings filtrés
  - ✅ Axum route syntax corrigé (`:param` → `{param}`)

#### PWA Environment Widget Scalable + Modal Historique
- **Feature** : Widget dashboard pour N sensors/rooms avec graphiques historiques
- **Architecture scalable** :
  - Fetch dynamique de TOUS les sensors via API
  - Extraction automatic unique room_ids (Set)
  - Rendu N room cards (pas de hard-coding)
  - Auto-refresh toutes les 30 secondes
- **Modal Chart.js** (18 Nov après-midi) :
  - Click sur room card → modal plein écran avec historique 7 jours
  - Chart.js dual Y-axis (température + humidité)
  - DOM portal pattern (render direct à `document.body`)
  - 7-day data retention backend (2,100 readings max)
  - Fixes: Shadow DOM disabled, portal overlay escapes widget container
- **Fichiers créés/modifiés** :
  - `pwa-dashboard/src/widgets/environment-widget.js` (850+ lignes)
  - `symbion-plugin-sensors/src/main.rs:163-173` - 7-day retention policy
  - Intégration desktop: `dashboard-app.js:24,1048-1051`
  - Intégration mobile: `dashboard-app.js:1029-1031` (tab "Données")
- **Design** :
  - Cards avec status-based coloring (Normal vert, Humid jaune, RiskMold orange, Cold bleu)
  - Gradient border bioluminescent selon status
  - Lecture température + humidité + signal Wi-Fi
  - Empty state si 0 capteur
- **Mobile Layout** :
  - Widget placé en premier dans tab "📝 Données"
  - Accessible sur téléphone via navigation tabs fixe
  - Ordre: Environnement → Notes → Context Stats

#### Documentation Complète
- **Fichiers mis à jour** :
  - `docs/api/endpoints.md:922-1102` - Section "Environment & IoT Sensors" (181 lignes)
  - Endpoint count : 73 → 78 (+5)
  - Exemples JSON complets pour chaque endpoint
  - Notes sur aggregation multi-sensors et status calculation
- **Conformité roadmap** : 100% roadmap F1 API implémenté (lignes 186-199)

### F1 Mold Risk Alerts OR-based + Mobile Optimizations (18 Novembre 2025 - soir)

#### Système d'Alertes Moisissure Amélioré
- **Feature** : Logique OR-based pour détection risque moisissure
- **Backend** (`symbion-plugin-sensors/src/main.rs:183-251`) :
  - Simplification enum `EnvironmentStatus` : Normal, MoldRisk, TempLow
  - Suppression anciens états : WarningVentilate, RiskMold
  - 4 conditions OR pour déclencher `MoldRisk` :
    - >75% humidité pendant 10 minutes (96/120 readings)
    - >70% humidité pendant 2 heures (1152/1440 readings)
    - >60% humidité pendant 6 heures (3456/4320 readings)
    - >50% humidité pendant 12 heures (6912/8640 readings)
  - Tolérance 80% pour gaps de données (sensor dropouts)
  - Alerte température basse : <16°C → TempLow
- **Frontend** (`pwa-dashboard/src/widgets/environment-widget.js`) :
  - Mise à jour status enum : `mold_risk`, `temp_low`, `normal`
  - Suppression anciens status : `humid`, `risk_mold`, `cold`
  - Nouveaux labels : "Risque Moisissure", "Froid", "Normal"
  - Nouveaux icons : 🚨 (mold_risk), ❄ (temp_low), ✓ (normal)
  - CSS classes adaptées pour status-based coloring

#### Optimisations Mobile Chart.js
- **Responsive Modal** (@media max-width: 768px) :
  - Fullscreen sur mobile (`height: 100vh`)
  - Layout flexbox vertical : header sticky + body scrollable
  - Padding adaptatif : 16px mobile (vs 24px desktop)
  - Bottom padding 80px (espace navigation mobile)
- **Chart dimensions** :
  - Hauteur : 300px mobile (vs 400px desktop)
  - Legend position : bottom mobile (vs top desktop)
- **Typography adaptative** :
  - Legend font : 11px mobile (vs 14px desktop)
  - Tooltip font : 11-12px mobile (vs 13-14px desktop)
  - Axes titles : 10px mobile (vs 12px desktop)
  - Axes ticks : 9px mobile (vs 11px desktop)
- **X-axis optimisé** :
  - Rotation labels : 60° mobile (vs 45° desktop)
  - Max labels : 8 mobile (vs 12 desktop)
  - Auto-skip activé pour lisibilité
- **Touch interactions** :
  - Tooltip padding compact : 8px (vs 12px desktop)
  - Legend box width : 12px (vs 15px desktop)

#### Tests
- ✅ Plugin rebuilé avec nouveau système d'alertes
- ✅ Plugin redémarré : ESP32-CDE370 (21°C, 56.7%) → Status Normal
- ✅ Modal responsive testée sur mobile : affichage correct
- ✅ Chart adaptatif : fonts lisibles, labels non superposés
- **Commit** : `0dd2e88`

---

## 🔐 Phase 2: Security Hardening (14 Novembre 2025)

**Status**: 🟢 100% complété (5/5 tâches)

### Améliorations Post-Phase 2 (15 Novembre 2025)

#### Fix: Organic Loader Animation Synchronization
- **Problème** : Incohérence visuelle entre instances du loader (border vs radial gradient)
- **Solution** : Synchronisation animation ripples avec propagation lumineuse
- **Changements** :
  - Remplacement `border: 2px solid` par `radial-gradient` bioluminescent
  - Animation `ripple-propagate` → `light-propagate` (scale 0.5→3)
  - Cohérence visuelle complète sur tous les loaders
- **Fichier modifié** : `pwa-dashboard/src/components/organic-loader.js:53-69,170-182`
- **Commit** : `cc91a40`

### Améliorations Post-Phase 2 (14 Novembre 2025 - soir)

#### Fix: MQTT Streaming Pagination pour Notes
- **Problème** : HTTP 504 timeout sur `/ports/memo` avec >5 notes (dépassement limite 10KB MQTT)
- **Solution** : Implémentation protocole streaming (1 note/message + marker `ListEnd`)
- **Bénéfices** :
  - Scalable pour nombre arbitraire de notes
  - Pas de limite taille payload
  - Performance stable quelle que soit la quantité
- **Fichiers modifiés** :
  - `symbion-plugin-notes/src/main.rs:329-368` (streaming émetteur)
  - `symbion-kernel/src/notes_bridge.rs:154-241` (agrégation récepteur)
- **Commits** : `6f4deb5`, `cea078e`

#### Fix: AgentRegistry Persistence avec Dirty Flag
- **Problème** : Heartbeats d'agents pas sauvegardés (perte données au redémarrage kernel)
- **Solution** : Pattern debounced I/O avec dirty flag + periodic save (5 min)
- **Bénéfices** :
  - Max 5 min perte données vs perte totale avant
  - Pas de write I/O à chaque heartbeat (économie disque)
  - Thread-safe avec AtomicBool
- **Fichier modifié** : `symbion-kernel/src/agents.rs:258-605`
- **Commit** : `cee08f9`

#### Fix: MQTT Packet Size Limits
- **Problème** : Rejet payloads >10KB par défaut
- **Solution** : Augmentation limite 1MB pour kernel + agents
- **Fichiers modifiés** :
  - `symbion-kernel/src/mqtt.rs:29-30`
  - `symbion-agent-host/src/mqtt.rs` (limites similaires)
- **Commit** : `479b00d`

### Vulnérabilités Corrigées

#### VULN-005: Bcrypt Cost Factor
- ✅ Augmentation: 10 → 12 (~400ms hashing)
- Fichiers: `symbion-kernel/src/auth.rs:107`, `symbion-kernel/src/auth.rs:367`

#### VULN-009: Rate Limiting Auth
- ✅ Retrait `tower_governor` (HTTP 500 localhost/VPN)
- ✅ Maintien rate limiting basé username (5/15min)
- Fichiers: `symbion-kernel/Cargo.toml`, `symbion-kernel/src/http.rs`

#### VULN-004: Secrets Hardcodés
- ✅ Rotation complète JWT_SECRET + API_KEY (OpenSSL)
- ✅ `.env` mis à jour, `.env.example` créé
- Prochaine rotation: 12 février 2026 (90 jours)

#### VULN-001: Permissions Certificats TLS
- ✅ `/etc/mosquitto/certs/key-mkcert.pem` → 600 (propriétaire seul)
- Avant: 640 (lisible groupe)

#### PR1: MQTT Retain Context
- ✅ Déjà implémenté `retain=true` (`symbion-kernel/src/dashboard_events.rs:46`)

**Documentation**: `docs/security/`

---

## 🔧 Fix Critique MQTT (15 Octobre 2025)

### Problème
- MQTT status: "connecting" permanent
- Agents non synchronisés

### Root Cause
`mark_mqtt_connected()` jamais appelé après subscriptions MQTT

### Solution
`symbion-kernel/src/mqtt.rs:81-85` - Ajout appel après subscriptions

### Impact
- ✅ Agents synchronisés
- ✅ Métriques CPU/RAM temps réel
- ✅ Status online/offline précis
- ✅ Plugin notes fonctionnel

**Documentation**: `incidents/resolu/INCIDENT-2025-10-15-mqtt-not-connected.md`

---

## 📧 Monitoring Automatique (15 Octobre 2025)

### Surveillance Proactive
- **Fréquence**: Toutes les 15 minutes (cron)
- **Alertes**: Gmail (Markchavatte@gmail.com) via msmtp

### Scripts Créés
- `scripts/monitor-symbion.sh` - Surveillance complète
- `scripts/install-monitoring.sh` - Installation cron
- `scripts/README.md` - Documentation

### Checks
- ✅ Kernel alive (http://localhost:8080)
- ✅ MQTT connected/disconnected
- ✅ Agents online/offline + métriques
- ✅ Plugins running/failed

### Alertes Configurées
- 🚨 Kernel DOWN
- 🚨 MQTT Disconnected
- 🚨 All Agents Offline
- 🚨 Plugin Failed
- ✅ System Recovered

---

## 📝 Widget Notes UX (15 Octobre 2025)

### Améliorations
- ✅ Markdown rendering (`marked.js`)
- ✅ Preview 3 lignes + gradient fondu
- ✅ Expand/collapse ("📖 Lire plus" / "⬆️ Réduire")
- ✅ Styling avancé (titres, code, listes)

**Fichier**: `pwa-dashboard/src/widgets/notes-widget.js`

---

## 🎨 Context Engine (25 Octobre 2025)

### Détection Automatique
- ✅ Règles temporelles: Nuit (23h-7h) → Mode Neutre
- ✅ Week-end automatique → Mode Intime
- ✅ Expiration override

### API Contrôle Manuel
```
POST /context/override
POST /context/clear
GET /context/current
```

**Fichiers**:
- `symbion-kernel/src/context.rs:107-148,192-235`
- `pwa-dashboard/src/widgets/context-widget.js` (498 lignes)

---

## 🏷️ Notes Contextuelles (25 Octobre 2025)

### Auto-injection Backend
- ✅ Tagging automatique notes avec contexte actuel
- ✅ Format lowercase: "cravate", "intime", "neutre"
- ✅ Fallback intelligent

**Fichiers**:
- `symbion-kernel/src/http.rs:588-625`
- `pwa-dashboard/src/widgets/notes-widget.js:837-950`

---

## 📱 Interface Mobile Responsive (25 Octobre 2025)

### Tabs Catégorisées
- 🎛️ Contrôle: Context + Agent controls
- ⚙️ Système: Health + Agents list
- 📝 Données: Notes widget

### Theming Dynamique
Variables CSS selon mode actif:
- `--context-primary`
- `--context-bg`
- `--context-accent`

**Fichiers**:
- `pwa-dashboard/src/components/dashboard-app.js:107-178,268-297,567-624`
- `pwa-dashboard/src/services/context-service.js`

---

## 🔐 Setup Certificat TLS (26 Octobre 2025)

### Workflow Automatisé
- ✅ Endpoint public: `GET /ca-certificate`
- ✅ Téléchargement direct `symbion-ca.crt`
- ✅ Instructions plateformes (Windows/Linux/macOS/iOS/Android)
- ✅ Vérification installation automatique

### Navigation Mobile Fixe
- ✅ Tabs ancrés en bas viewport (`position: fixed`)
- ✅ Backdrop blur effet iOS
- ✅ Padding container (70px)

**Fichiers**:
- `symbion-kernel/src/http.rs:136,1105-1131`
- `pwa-dashboard/src/components/boot-terminal.js:716-779,904-914`
- `pwa-dashboard/src/components/dashboard-app.js:331-349`

---

## 📚 Légende

- 🟢 Complété
- 🟡 En cours
- 🔴 Bloqué
- ⚪ Non démarré
