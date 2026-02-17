# Roadmap Technique - NewSymbion

**Version** : 2026-02 (Post Audit Complet)
**Statut** : Fondations complètes, 116 issues identifiées par audit exhaustif
**Dernière mise à jour** : 17 Février 2026
**Score global** : 3.8/5

---

## Vue d'Ensemble

### Codebase

| Composant | Langage | LOC | Fichiers | Score | Tests |
|-----------|---------|----:|----------|-------|------:|
| **symbion-kernel** | Rust | ~40,000 | 87 | 4.5/5 | 288 |
| **pwa-dashboard** | JS (Lit) | ~28,100 | 40 | 3/5 | 0 |
| **symbion-agent-host** | Rust | ~2,000 | 13 | 3.5/5 | 7 |
| **Plugins** (5) | Rust | ~4,900 | 14 | 4.7/5 | 2 |
| **Infra** (scripts/CI) | Bash/YAML | ~2,400 | 20 | 3.5/5 | - |
| **Total** | | **~85,000** | **174** | **3.8/5** | **297** |

### Chiffres Clés

| Métrique | Valeur |
|----------|--------|
| Unit Tests | 415+ (288 kernel + 96 decision + 31 intelligence + 7 agent) |
| API Routes (http.rs) | 106 .route() |
| MQTT Topics | 10 subscriptions |
| Automations actives | 19 (+ intelligence-managed) |
| Modes contextuels | 4 système + custom |
| Intelligence Samples | 29 (apprentissage continu) |
| Data files (JSON) | 12 |
| **Issues audit** | **116 (11 P0, 18 P1, 67 P2, 49 P3)** |

---

## Phases Complétées

### PR1 — Context Engine v2 🟢 100%
**Complété** : Novembre 2025

- IANA timezone (Europe/Zurich DST)
- Hysteresis monotonic (120s threshold)
- Weekend auto-detection
- Night mode (23h-7h)
- Manual override avec durée
- Modes dynamiques (Pro, Focus, Maison, Veille)

**Fichiers** : `context.rs` (934 LOC), `modes/registry.rs`, `modes/types.rs`

---

### PR2 — Security Hardening 🟢 100%
**Complété** : Novembre 2025

- JWT HS256 (8h expiry, 128 hex secret)
- Bcrypt cost 12 (~400ms/op)
- MFA/TOTP + QR code + 5 backup codes
- WebAuthn/Passkeys biométriques
- CSRF single-use nonces (5 min TTL)
- Rate limiting auth (5 attempts/15min par username)
- TLS 1.3 (port 8443, mkcert)
- HTTP→HTTPS redirect (port 8080)
- HSTS (max-age 31536000)

**Fichiers** : `auth.rs` (885 LOC), `csrf.rs` (287), `webauthn.rs` (737), `mfa.rs` (321)

---

### PR3 — Decision Engine 🟢 100%
**Complété** : Novembre 2025

- Guards-first evaluation (Time/Agent/Context)
- Trust score 5 facteurs pondérés (context 30%, telemetry 25%, history 25%, network 10%, presence 10%)
- Idempotence (command_id deduplication)
- Impact levels (Low 0.3 / Medium 0.5 / High 0.7 / VeryHigh 0.9)
- Trust evolution asymétrique (+0.01/succès, -0.05/échec)
- Validation workflow + audit trail
- 93 unit tests

**Fichiers** : `decision/` (13 modules, 6,177 LOC)

---

### PR4 — Metrics & Observability 🟢 100%
**Complété** : Novembre 2025

- `GET /metrics` — 36 métriques Prometheus
- `GET /v1/metrics/system` — Kernel overview JSON
- `GET /v1/metrics/agents` — Télémétrie par agent
- `GET /health` — Liveness check
- Agent telemetry (30s heartbeat : CPU, RAM, disk, network, processes)
- Structured logging par catégorie

**Fichiers** : `decision/metrics.rs` (549 LOC), `http.rs` (3,633 LOC)

---

### PR5 — Kernel Reliability 🟢 100%
**Complété** : Novembre 2025

- Graceful shutdown SIGTERM (plugin cleanup, MQTT close)
- MQTT reconnection exponential backoff (5 retries: 2s→32s)
- Plugin isolation (crash ≠ kernel crash)
- Panic hook + context logging
- Systemd service (Restart=always, RestartSec=5s)
- Resource limits (MemoryMax=512M, CPUQuota=200%)

**Fichiers** : `main.rs` (573 LOC), `symbion-kernel.service`

---

### Intelligence Engine v2 🟢 100%
**Complété** : Février 2026

- **Feature Registry** : TTL 60-300s, cleanup auto, 4 types (Bool, Float, String, StringList)
- **Vector Builder** : 5 dimensions normalisées (home_prob, work_prob, focus_prob, sleep_prob, pc_active)
- **Inference Engine** : Cosine similarity + k-NN (top-10), weighted voting
- **Session Manager** : Hysteresis 4 couches (entry/exit gap, 3 consécutifs, 5 min min, 30 min cooldown)
- **Source Weighting** : UserCorrection 1.3x, MfaConfirmed 1.0x, Automation 0.8x, Bootstrap 0.5x
- **Time Decay** : Half-life 30j (normal), 7j (bootstrap)
- **v2 Stabilization** : Shadow mode, 6 guards avant auto-apply

**Fichiers** : `intelligence/` (8 modules, 3,977 LOC), `context_intelligence.rs` (1,784 LOC)

---

### Automation Engine 🟢 100%
**Complété** : Janvier-Février 2026

- **7 types de triggers** : mode_change, sensor_alert, agent_status, manual, plugin_health, scheduled, polling
- **9 types de conditions** : mode, time_range, day_of_week, sensor_threshold, agent_online, custom, AND, OR, NOT
- **Actions** : MQTT publish, mode change, notification, webhook
- **Scheduler** : Cron-like avec timezone configurable (`SYMBION_TIMEZONE`)
- **Decision Bridge** : Intégration Decision Engine pour validation
- **Pending Actions** : Workflow approbation manuelle
- **Broadcast channel** : Capacité 512 events
- **Persistence** : `data/automations.json` + `data/automations_history.json`

**Fichiers** : `automations/` (12 modules, 6,280 LOC)

---

### F1 — Environment Monitoring 🟢 100%
**Complété** : Novembre 2025

- RoomEnvironmentState + circular buffer (20,160 readings = 7 jours)
- DewPointAlertLevel : 6 niveaux (Safe → Danger)
- Calcul point de rosée Magnus (±0.4°C)
- Plugin sensors ESP32 BME280 (821 LOC)
- PWA widget dynamique (Chart.js dual Y-axis, 7j historique)
- 44 unit tests

**Fichiers** : `environment.rs` (558 LOC), `dew_point_alerts.rs` (650 LOC), `decision/environment.rs` (431 LOC)

---

### F4 — Notifications 🟢 100%
**Complété** : Décembre 2025

- NotificationManager avec priority queue (P0, P1, P2)
- Multi-canal : Firebase FCM, SMTP email, ntfy.sh, MQTT
- Deduplication (content hash) + Rate limiting par source
- Persistence (1,000 history limit)
- Interactive actions (approve, reject, snooze)

**Fichiers** : `notifications.rs` (756 LOC), `notification_config.rs`

---

### Plugin System 🟢 100%
**Complété** : Décembre 2025 - Février 2026

| Plugin | LOC | Rôle |
|--------|----:|------|
| **symbion-plugin-sensors** | 821 | ESP32 BME280 monitoring |
| **symbion-plugin-notes** | 1,119 | Mémoire externe (markdown + tags) |
| **symbion-plugin-ssl** | 1,601 | Monitoring certificats multi-domaines |
| **symbion-plugin-freebox** | 1,104 | LAN discovery, présence, internet status |
| **symbion-plugin-common** | 255 | Shared utilities |

---

### Dynamic Themes & Modes 🟢 100%
**Complété** : Février 2026

- ModeRegistry avec persistence `data/modes.json`
- 4 modes système (Pro, Focus, Maison, Veille) + custom
- Logo header colorisé dynamiquement (hexToHSL → CSS filter)

---

### Per-Agent Features 🟢 100%
**Complété** : Février 2026

- Features individuelles : `agent.{id}.online`, `agent.{id}.cpu`, `agent.{id}.memory`
- AgentRegistry debounced persistence (5 min save interval)

---

### Prediction Correction UI 🟢 100%
**Complété** : 16 Février 2026

- `POST /v1/intelligence/feedback` enrichi v1+v2
- Bouton "Corriger la prédiction" dans l'onglet Intelligence PWA

---

## Phase Active

### PR6 — Production Readiness 🟡 ~86%
**En cours** — Démarré Novembre 2025

**Complété** (12/14) :
- [x] CSP headers (strict default-deny)
- [x] HSTS headers
- [x] Security documentation
- [x] CI/CD pipelines (3 GitHub Actions workflows)
- [x] CI/CD test suite
- [x] Rate limiting auth (5 attempts/15min)
- [x] Rate limiting global IP-based (300 req/min)
- [x] Docker containerization (multi-stage)
- [x] Health probes Kubernetes-compatible (`/health/live`, `/health/ready`)
- [x] Log rotation journald (500M max, 30j rétention)
- [x] Database backups automatiques (timer quotidien 3h, rotation 30j)
- [x] Monitoring externe healthcheck.io

**Restant** (2/14) :
- [ ] Let's Encrypt ACME integration (auto-renouvellement certificats)
- [ ] SQLite migration (remplacer 12 fichiers JSON)

---

## Features Planifiées (Non Implémentées)

### F2 — Digital Hygiene ⚪ 0%
**Effort estimé** : 9 jours

- Activity tracker dans agent-host (process classification)
- MQTT topic `symbion/agents/activity@v1`
- Decision rules : pause 4h, alerte burnout 10h×3j
- PWA widget temps d'écran + page stats

### F3 — Intentions Log ⚪ ~5%
**Effort estimé** : 5 jours

- Type `Intention` défini (`decision/environment.rs:39-47`)
- Storage JSON/SQLite avec retention 90 jours
- API analytics + PWA page paginée + export CSV

### F5 — Light Actuator ⚪ 0%
**Effort estimé** : 10 jours

- Trait `LightActuator` (on/off, brightness, color temp, RGB)
- Backend Tuya local (LAN-only)
- Decision rules context-aware (Pro → froid 100%, Maison → chaud 30%)

---

## Audit Système Complet — 17 Février 2026

### Résumé par Module

| Module | Score | P0 | P1 | P2 | P3 | Total |
|--------|-------|---:|---:|---:|---:|------:|
| Kernel Core | 4.5/5 | 0 | 1 | 3 | 9 | 13 |
| Intelligence Engine | 3.5/5 | 2 | 2 | 21 | 13 | 38 |
| Decision + Automation | 4/5 | 1 | 4 | 10 | 9 | 24 |
| Plugins (5) | 4.7/5 | 1 | 2 | 5 | 4 | 12 |
| Agent Host | 3.5/5 | 4 | 6 | 7 | 0 | 17 |
| PWA Dashboard | 3/5 | 3 | 2 | 11 | 10 | 26 |
| Infrastructure | 3.5/5 | 0 | 1 | 10 | 4 | 15 |
| **Total** | **3.8/5** | **11** | **18** | **67** | **49** | **145** |

---

### P0 — Issues Critiques (11)

| # | Module | Issue | Fichier |
|---|--------|-------|---------|
| 1 | INTELLIGENCE | prediction_history VecDeque overflow (cap 1000, pas d'eviction) | `inference.rs:prediction_history` |
| 2 | INTELLIGENCE | Sample eviction race condition (add_sample concurrent > max_samples) | `inference.rs:add_sample()` |
| 3 | INTELLIGENCE | NaN si half_life=0 dans effective_weight() | `inference.rs:128` |
| 4 | AUTOMATION | Sync context building synchrone en async (HashMap agents vide) | `engine.rs:634-652` |
| 5 | SSL PLUGIN | Fingerprint change events non émis (TODO dans le code) | `ssl/main.rs:317` |
| 6 | PWA | XSS innerHTML sans sanitization (DOMPurify installé mais pas utilisé) | `notification-center.js:618`, `environment-widget.js:785` |
| 7 | PWA | Device token localStorage pas effacé au logout → XSS re-auth | `auth-service.js:73-87` |
| 8 | PWA | Réponses API non sanitizées | `api-service.js:185-198` |
| 9 | AGENT | MQTT always shows online (mqtt_connected=true hardcodé) | `main.rs:984` |
| 10 | AGENT | Disk metrics broken Windows (utilise `df` Unix-only) | `metrics/mod.rs:205-262` |
| 11 | INFRA | Secrets hardcodés docker-compose + CI/CD workflows | `docker-compose.yml:35-36`, `deploy-dashboard.yml:44` |

---

### P1 — Issues Importants (18)

| # | Module | Issue | Fichier |
|---|--------|-------|---------|
| 1 | KERNEL | MQTT contract validation non implémentée | `contracts.rs:111` |
| 2 | INTELLIGENCE | Pas de cleanup fichiers .json.tmp orphelins | `inference.rs` |
| 3 | INTELLIGENCE | Confidence non propagée dans VectorBuilder | `vector.rs` |
| 4 | INTELLIGENCE | I/O synchrone bloquant (save_samples) | `inference.rs:345` |
| 5 | DECISION | Trust tracker sans temporal decay | `trust_tracker.rs` |
| 6 | DECISION | PendingActions en mémoire (non persisté) | `pending_actions.rs` |
| 7 | DECISION | Validation cleanup_expired() jamais auto-appelé | `validation.rs:221` |
| 8 | DECISION | Override cleanup_expired() jamais auto-appelé | `override.rs:169-190` |
| 9 | AUTOMATION | SSID hardcodé "local" dans conditions | `conditions.rs` |
| 10 | AUTOMATION | Cooldown enforcement gap entre triggers | `engine.rs` |
| 11 | SSL | State save silently fails (.ok()) | `ssl/main.rs:565` |
| 12 | FREEBOX | mem::forget pour MQTT loop | `freebox/mqtt.rs:86` |
| 13 | AGENT | No MQTT recovery feedback (5s fixed retry) | `main.rs:212` |
| 14 | AGENT | /reconnect endpoint not implemented | `local_api.rs:72` |
| 15 | AGENT | Agent ID non persisté (change si MAC change) | `discovery.rs:76` |
| 16 | AGENT | No child process cleanup on timeout | `execution/mod.rs:250` |
| 17 | PWA | MQTT agent cache unbounded (pas de LRU) | `mqtt-service.js:134` |
| 18 | PWA | Passkey manager hardcoded base URL | `passkey-manager.js:194` |

---

### P2 — Améliorations (67)

#### Kernel (3)

| Issue | Fichier |
|-------|---------|
| Notification path hardcodé `/var/lib/symbion/` non configurable | `notifications.rs:76` |
| Chemins config process_categories.toml hardcodés | `mqtt.rs` |
| SMTP port parse silently fallback 587 sans log | `notifications.rs:125-143` |

#### Intelligence (21)

| Issue | Fichier |
|-------|---------|
| Race condition sample eviction (add_sample concurrent) | `inference.rs:341-360` |
| Normalisation zero-sum silencieuse (vecteur non normalisé) | `vector.rs:427-433` |
| Confidence non validée (peut être >1.0 ou <0.0) | `vector.rs:401` |
| Clock skew : session bloquée en cooldown permanent | `sessions.rs:152-157` |
| Override expiry bypass si source change | `sessions.rs:161-165` |
| Double-count process classifier (case-insensitive match) | `classifier.rs:350` |
| TOML weights non validés (négatifs acceptés) | `classifier.rs:182-194` |
| Cleanup contention features : write lock pendant read | `features.rs:239-244` |
| Clock skew features : is_expired() faux négatif | `features.rs:86-92` |
| Bootstrap vecteurs non normalisés sur zero input | `bootstrap.rs:196-208` |
| Bootstrap ignore config weekday_work_mode | `bootstrap.rs:162-178` |
| Bootstrap modes non validés (pas de validation) | `bootstrap.rs:100-110` |
| Config : cross-field validation manquante (min_samples) | `config.rs:64-118` |
| Timezone fallback silencieux vers Europe/Paris | `mod.rs:143-157` |
| v1/v2 decay multiplier inconsistency | `context_intelligence.rs:260-268` |
| Stats 24h jamais reset (accumulation indéfinie) | `context_intelligence.rs:189-199` |
| Agent metrics peut choisir le mauvais agent | `context_intelligence.rs:548-552` |
| Mode "focus" missing dans init_patterns_from_history | `context_intelligence.rs:446-450` |
| save_patterns() sans retry si disque plein | `context_intelligence.rs:309-316` |
| v2 exit_threshold non exposé dans config | `sessions.rs` |
| Source weight multiplier sans cap (>2.0 accepté) | `inference.rs:46-53` |

#### Decision + Automation (10)

| Issue | Fichier |
|-------|---------|
| Decay modifier calculé 2x par action (redundant) | `trust_tracker.rs:56-93` |
| Trust score fallback 0.7 masque les vrais defaults | `trust.rs:190-201` |
| Validation cleanup_expired() jamais auto-appelé | `validation.rs:221` |
| Override cleanup_expired() jamais auto-appelé | `override.rs:169-190` |
| TOCTOU race condition sur override check | `engine.rs:307-329` |
| SSID toujours hardcodé "local" (conditions SSID cassées) | `engine.rs:636-637` |
| Actions continuent même si Decision Engine error | `engine.rs:281-580` |
| Cooldown non enforcé pendant exécution longue | `engine.rs:281-580` |
| Feature registry lookup silently returns false | `engine.rs:248-265` |
| Test coverage minimale (3 tests helper seulement) | `engine.rs:991-1035` |

#### Plugins (5)

| Issue | Fichier |
|-------|---------|
| Notes : socket cleanup errors ignorés (let _ =) | `notes/main.rs:832,923` |
| SSL : state save .ok() ignore erreurs silencieusement | `ssl/main.rs:565,587` |
| SSL : dépendance reqwest inutilisée | `ssl/Cargo.toml:17` |
| Freebox : downloads API graceful failure incomplet | `freebox/main.rs:244-246` |
| Freebox : chemins config hardcodés (mafreebox.freebox.fr) | `freebox/config.rs:101-147` |

#### Agent Host (7)

| Issue | Fichier |
|-------|---------|
| Password input non masqué dans wizard | `wizard.rs:359` |
| Password non zéroizé en mémoire (plain String) | `config.rs:34` |
| Load average hardcodé 0.0 sur Windows | `metrics/mod.rs:164-169` |
| Dead code tray.rs (Tauri dependency absente) | `tray.rs:1` |
| GitHub API updater sans token auth (rate limit 60/h) | `updater.rs:55` |
| Pas de gestion services cross-platform | — |
| Kill processes OK mais pas de restart | — |

#### PWA Dashboard (11)

| Issue | Fichier |
|-------|---------|
| Device token envoyé sans check transport HTTPS | `auth-service.js:116` |
| CSRF nonce response : pas de validation JSON | `csrf-service.js:85-90` |
| CSRF nonce fetch : pas de timeout (hang indéfini) | `csrf-service.js:67` |
| API_BASE sans certificate pinning | `api-service.js` |
| MQTT reconnection counter jamais reset | `mqtt-service.js:106` |
| HSL bounds check manquant (hue négatif possible) | `context-service.js:146` |
| Login form double-submit (pas de disabled state) | `boot-terminal.js:332` |
| WebAuthn credential creation sans timeout | `passkey-manager.js:286` |
| sanitization.js : `<br>` au lieu de `<br/>` (XHTML) | `sanitization.js:32` |
| Dashboard : pas de request batching au chargement | `dashboard-app.js` |
| User menu : aria-expanded manquant | `dashboard-app.js:1073` |

#### Infrastructure (10)

| Issue | Fichier |
|-------|---------|
| send-mail.sh : missing set -euo pipefail | `scripts/send-mail.sh:1-2` |
| Dashboard systemd : WorkingDirectory /var/www/ incorrect | `symbion-dashboard.service:13` |
| Plugins systemd : ~ non expansé dans bash source | `symbion-plugin-*.service:12` |
| Docker compose : pas de resource limits | `docker-compose.yml` |
| Release workflow : pas de GPG signing | `release.yml` |
| CLAUDE.md exclu du git (.gitignore) | `.gitignore:76` |
| Scripts : chemins absolus hardcodés non portables | `backup-symbion.sh:17` |
| backup-symbion.sh : stat GNU-only (casse macOS) | `backup-symbion.sh:50,66` |
| Dockerfile : cargo build errors silently swallowed | `kernel.Dockerfile:35` |
| Nginx : missing security headers (X-Content-Type, etc) | `nginx-dashboard.conf` |

---

### P3 — Issues Mineures (49)

#### Kernel (9)

| Issue | Fichier |
|-------|---------|
| MFA test coverage limitée (backup codes, TOTP window, QR) | `mfa.rs` |
| Dead code markers (#[allow(dead_code)]) non nettoyés | `agents.rs:202-213` |
| Old enum naming : Cravate/Intime/Neutre au lieu de Pro/Focus/Maison | `context.rs` |
| http.rs monolithique (3,633 LOC) — devrait être split en sub-routers | `http.rs` |
| Max packet size MQTT hardcodé 1MB | `mqtt.rs:84` |
| Doc comments manquants sur handlers HTTP | `http.rs` |
| Erreurs 500 génériques sans messages structurés | `http.rs` |
| OpenAPI/Swagger non généré pour 90+ endpoints | — |
| Startup performance : pas de timers initialisation | — |

#### Intelligence (13)

| Issue | Fichier |
|-------|---------|
| Fichiers .json.tmp orphelins si rename() échoue | `inference.rs:334-336` |
| Floating-point accumulation error après ~10 contributions | `vector.rs:400-404` |
| Why-chain unbounded (500+ entries possibles) | `vector.rs:406` |
| Decay formula doc mismatch entre sessions.rs et inference.rs | `sessions.rs:189` |
| PendingTransition sans timeout max (bloquée 5+ min) | `sessions.rs:410-418` |
| Magic number 3.0 dans confidence formula (arbitraire) | `classifier.rs:314-320` |
| feature_ids : strings hardcodés sans validation compile-time | `features.rs:286-327` |
| Signal weights : test vérifie sum=1.0 mais pas new()/default() | `config.rs:158-168` |
| decay_coefficients non documentés | `config.rs:36-38` |
| ShadowStats blocked_reasons HashMap unbounded | `types.rs:267` |
| PatternExport.decayed_confidence non-déterministe (depends on now) | `types.rs:137` |
| Expired features pas lazy-deleted dans get() | `features.rs:162-170` |
| Mixed chrono/time crate usage dans context_intelligence | `context_intelligence.rs:588` |

#### Decision + Automation (9)

| Issue | Fichier |
|-------|---------|
| Trust constants hardcodées (success +0.01, failure -0.05, max 0.2) | `trust_tracker.rs:141-145` |
| very_high threshold >1.0 intentionnel mais non documenté | `config.rs:99` |
| Edge case tests manquants (temperature_c=None, delta_t) | `environment.rs` |
| Lock poisoning : .expect() sur RwLock = panic potentiel | `engine.rs:187,198,205` |
| Mode strings hardcodés (cravate/intime/neutre) dupliqués du context | `engine.rs:688-698` |
| Pas de changement de mode atomique (set_override + set_mode séquentiels) | `engine.rs:759,772` |
| Trigger matching inefficiency : même automation évaluée plusieurs fois | `listener.rs` |
| Code dupliqué pour operator string conversion | `engine.rs:134-142,197-201` |
| Day of week : number_days_from_sunday() sémantique selon version time crate | `engine.rs:165` |

#### Plugins (4)

| Issue | Fichier |
|-------|---------|
| Notes : reload disk à chaque list_notes() (inefficient) | `notes/main.rs:310` |
| Notes : socket et storage paths hardcodés | `notes/main.rs:827,823` |
| Freebox : health tracking potential race (write/read concurrent) | `freebox/main.rs:188-196` |
| Freebox : mem::forget() pattern non conventionnel pour MQTT loop | `freebox/mqtt.rs:86` |

#### PWA Dashboard (10)

| Issue | Fichier |
|-------|---------|
| MQTT : pas de unsubscribe au disconnect | `mqtt-service.js` |
| Login : autocomplete="username" manquant | `boot-terminal.js` |
| Modals : role="dialog" et aria-modal manquants | `dashboard-app.js` |
| Modals : focus non reset après fermeture (focus trap absent) | `dashboard-app.js` |
| Boutons emoji sans aria-label descriptif | `dashboard-app.js` |
| Indicateurs couleur-only (dots rouge/vert) sans fallback texte | `dashboard-app.js` |
| Pas de keyboard navigation pour features mobile-only | `dashboard-app.js` |
| Widgets : pas de lazy loading (tout importé au top) | `dashboard-app.js:15-24` |
| CSS hover animations : 60 widgets × hover = repaints | `dashboard-app.js` |
| Agent cache unbounded (pas de LRU/eviction) | `mqtt-service.js:134` |

#### Infrastructure (4)

| Issue | Fichier |
|-------|---------|
| Artifact retention 90 jours (devrait être 7j) | `deploy-kernel.yml:105-110` |
| Rust version non pinnée (digest hash) dans Dockerfile | `kernel.Dockerfile:7` |
| SETUP.md et TESTING.md manquants | `docs/` |
| MONITORING.md et INCIDENT_RESPONSE.md manquants | `docs/` |

---

## Issues Précédemment Corrigées

### P0 — Corrigés (16 Février 2026)
- [x] ~~Broadcast channel overflow~~ → Capacité 100→512 (`events.rs:116`)
- [x] ~~Persistence non-atomique~~ → temp file + rename (`inference.rs:309-334`)
- [x] ~~Normalisation trop conservatrice~~ → Seuil 0.1→1e-6 (`vector.rs:419`)

### P1 — Corrigés (16 Février 2026)
- [x] ~~I/O synchrone bloquant~~ → File I/O via `std::thread::spawn` (`inference.rs:309-334`)
- [x] ~~Confidence non propagée~~ → pondération par `confidence` (`vector.rs:391-406`)
- [x] ~~PendingActions en mémoire~~ → Persistence JSON atomique (`pending_actions.rs`)
- [x] ~~PC_ACTIVE non normalisé~~ → Weighted cosine similarity ×0.3 (`inference.rs:677-710`)
- [x] ~~Trust Tracker sans decay~~ → Decay exponentiel half-life 30j (`trust_tracker.rs:262-272`)

### P2 — Corrigés (17 Février 2026)
- [x] ~~Tie-breaking top-k~~ → timestamp critère secondaire (`inference.rs:404`)
- [x] ~~Bootstrap multi-slot~~ → 7 time-slot samples (`bootstrap.rs:148-210`)
- [x] ~~SSID case-insensitive~~ → `eq_ignore_ascii_case` (`trust.rs:119`)
- [x] ~~Timezone configurable~~ → `SYMBION_TIMEZONE` env var (`intelligence/mod.rs:137-165`)
- [x] ~~Clock skew protection~~ → Duration clamped à zero (`sessions.rs:144`)
- [x] ~~Compaction périodique~~ → compact() toutes les 100 cycles (`context_intelligence.rs`)
- [x] ~~Stability score decay~~ → decay exponentiel half-life 60 min (`sessions.rs:184-192`)
- [x] ~~symbion-devkit obsolète~~ → Supprimé du workspace

---

## Plan d'Action — Sprints

### Sprint 1-2 (Immédiat) — Sécurité P0
- [ ] Fix XSS PWA : utiliser DOMPurify + escapeHtml() dans notification-center, environment-widget
- [ ] Fix device token : clear localStorage au logout
- [ ] Sanitizer réponses API dans api-service.js
- [ ] Fix prediction_history overflow (eviction/ring buffer)
- [ ] Valider half_life != 0 dans effective_weight()
- [ ] Fix sample eviction race condition (lock scope)
- [ ] Fix automation sync context building (async HashMap)
- [ ] SSL plugin : émettre événements fingerprint change
- [ ] Agent : fix MQTT status (pas toujours online)
- [ ] Agent : fix disk metrics Windows
- [ ] Supprimer secrets hardcodés docker-compose + CI

### Sprint 3-4 (Court terme) — Fiabilité P1
- [ ] MQTT contract validation au kernel
- [ ] Async I/O pour save_samples()
- [ ] Trust tracker temporal decay
- [ ] Validation + Override cleanup_expired() auto-scheduled
- [ ] Agent ID persisté sur disque
- [ ] MQTT recovery exponential backoff (agent)
- [ ] Plugins systemd resource limits
- [ ] SSL state save error handling
- [ ] Freebox mem::forget → JoinHandle
- [ ] Agent /reconnect endpoint
- [ ] Agent child process cleanup on timeout
- [ ] PWA MQTT cache LRU (max 100)
- [ ] PWA passkey-manager : utiliser apiService.baseUrl

### Sprint 5-6 (Moyen terme) — Améliorations P2
- [ ] PR6 : Let's Encrypt ACME
- [ ] PR6 : SQLite migration (12 fichiers JSON)
- [ ] Intelligence : clock skew protection, confidence validation, bootstrap config
- [ ] Automation : SSID réel, error propagation, cooldown during execution
- [ ] Agent : password masking, zeroize, load_avg Windows
- [ ] PWA : CSRF timeout, double-submit, request batching
- [ ] Infra : Docker resource limits, GPG signing, nginx security headers
- [ ] Tests PWA (objectif 50%+ coverage)

### Sprint 7+ (Long terme) — Features + P3
- [ ] F3 Intentions Log (type déjà défini)
- [ ] F2 Digital Hygiene (activity tracking + burnout)
- [ ] F5 Light Actuator (bridge hardware Tuya)
- [ ] Kernel http.rs split en sub-routers
- [ ] Old enum naming cleanup (Cravate→Pro)
- [ ] OpenAPI/Swagger documentation
- [ ] PWA accessibility (aria, focus trap, keyboard nav)
- [ ] PWA lazy loading widgets
- [ ] Docs : SETUP.md, TESTING.md, MONITORING.md

---

## Architecture Pipeline Intelligent

```
SIGNAUX (MQTT, Agents, Freebox, Sensors)
    ↓
FEATURE REGISTRY (features.rs) — TTL 60-300s, cleanup auto
    ↓  Features typées: Bool, Float, String, StringList
VECTOR BUILDER (vector.rs) — 5 dimensions normalisées 0→1
    ↓  home_prob | work_prob | focus_prob | sleep_prob | pc_active
INFERENCE ENGINE (inference.rs) — Cosine similarity + k-NN (top-10)
    ↓  Weighted voting: similarity × recency × source_weight
SESSION MANAGER (sessions.rs) — Hysteresis 4 couches
    ↓  Entry ≥ 0.50 | Exit ≥ 0.35 | 3 consécutifs | Min 5 min
DECISION ENGINE (decision/) — Guards → Trust → Threshold
    ↓  Impact: Low(0.3) | Medium(0.5) | High(0.7) | VeryHigh(0.9)
AUTOMATION ENGINE (automations/) — Trigger → Condition → Action
    ↓  19 automations actives, scheduler cron-like
```

---

## Contraintes Architecture

- **LAN-only** : Pas de cloud, 100% réseau local
- **Privacy-first** : Aucune donnée personnelle dans le code
- **Generic design** : Abstraction pour réutilisabilité
- **Production-grade** : Tests, error handling, observability
- **Systemd-first** : Kernel = service systemd, jamais lancé manuellement

---

**Document Maintenu Par** : Claude Code + Mark
**Git Branch** : master
