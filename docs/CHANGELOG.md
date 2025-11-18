# Changelog - Symbion

Historique des améliorations et changements majeurs du projet.

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
