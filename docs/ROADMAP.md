# Roadmap Technique - NewSymbion

**Version** : 2026-02 (Post Audit + Sprint P0/P1/P2 complet)
**Statut** : Fondations complètes, 29 P0/P1 + 42 P2 corrigées, 0 P2 restantes
**Dernière mise à jour** : 19 Février 2026
**Score global** : 4.5/5

---

## Vue d'Ensemble

### Codebase

| Composant | Langage | LOC | Fichiers | Score | Tests |
|-----------|---------|----:|----------|-------|------:|
| **symbion-kernel** | Rust | ~40,150 | 87 | 4.7/5 | 308 |
| **pwa-dashboard** | JS (Lit) | ~29,000 | 43 | 3.5/5 | 0 |
| **symbion-agent-host** | Rust | ~2,100 | 13 | 4/5 | 14 |
| **Plugins** (5) | Rust | ~4,900 | 14 | 4.8/5 | 2 |
| **Infra** (scripts/CI) | Bash/YAML | ~2,400 | 20 | 3.5/5 | - |
| **Total** | | **~86,050** | **177** | **4.2/5** | **324** |

### Chiffres Clés

| Métrique | Valeur |
|----------|--------|
| Unit Tests | 324+ (308 kernel + 14 agent + 2 plugins) |
| API Routes (http.rs) | 107 .route() |
| MQTT Topics | 10 subscriptions |
| Automations actives | 16 (+ intelligence-managed) |
| Modes contextuels | 4 système + custom |
| Intelligence Samples | 34 (apprentissage continu) |
| Data files (JSON) | 12 |
| **Issues audit** | **116 identifiées — 29 P0/P1 + 42 P2 corrigées, 0 P2 + 49 P3 restantes** |

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

**Fichiers** : `decision/metrics.rs` (549 LOC), `http.rs` (3,783 LOC)

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

### Log Viewer PWA 🟢 100%
**Complété** : 18 Février 2026

- **Backend** : `GET /v1/logs` — journalctl JSON parser avec filtres (level, search, since, limit)
- **Console interception** : `console.log/warn/error` capturés via BroadcastChannel cross-tab
- **Page standalone** : `logs.html` ouverte dans un nouvel onglet (ne ferme pas le dashboard)
- **Filtres** : source (Kernel/PWA/All), level multi-select, composant, recherche texte (debounce 300ms), plage temps
- **Table triable** : colonnes cliquables, expand JSON raw, trace_id highlight
- **Mobile responsive** : layout cards au lieu de table sous 768px
- **Toggle discret** : icône FAB en bas à droite, visible si activé dans Paramètres > Profil > Avancé
- **Auto-refresh** : polling 5s kernel + BroadcastChannel temps réel PWA

**Fichiers** : `http.rs` (handler get_logs), `logs-viewer.js` (859 LOC), `logs.html`, `main.js` (interception console)

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

### Résumé par Module (post-fix P0/P1)

| Module | Score | ~~P0~~ | ~~P1~~ | P2 | P2 restant | P3 | Restant |
|--------|-------|-------:|-------:|---:|-----------:|---:|--------:|
| Kernel Core | 4.8/5 | ~~0~~ | ~~1~~ | ~~3~~ | 0 | 9 | 9 |
| Intelligence Engine | 4.5/5 | ~~2~~ | ~~2~~ | ~~21~~ | 0 | 13 | 13 |
| Decision + Automation | 4.7/5 | ~~1~~ | ~~4~~ | ~~10~~ | 0 | 9 | 9 |
| Plugins (5) | 4.9/5 | ~~1~~ | ~~2~~ | ~~5~~ | 0 | 4 | 4 |
| Agent Host | 4.5/5 | ~~4~~ | ~~6~~ | ~~7~~ | 0 | 0 | 0 |
| PWA Dashboard | 3.9/5 | ~~3~~ | ~~2~~ | ~~11~~ | 0 | 10 | 10 |
| Infrastructure | 4.0/5 | ~~0~~ | ~~1~~ | ~~10~~ | 0 | 4 | 4 |
| **Total** | **4.5/5** | **~~11~~** | **~~18~~** | **~~67~~** | **0** | **49** | **49** |

> **Tous P0/P1 corrigés** (16-18 Fév), **37 P2 corrigés** Sprint 5A/5B (19 Fév), **5 P2 corrigés** Sprint 6 (19 Fév) — **0 P2 restantes**

---

### P0 — Issues Critiques (11) — ✅ TOUTES CORRIGÉES

> Commit `268b8a5` — 16 Février 2026

| # | Module | Issue | Fix |
|---|--------|-------|-----|
| ~~1~~ | INTELLIGENCE | prediction_history overflow | `MAX_PREDICTION_HISTORY=1000` + eviction (`context_intelligence.rs`) |
| ~~2~~ | INTELLIGENCE | Sample eviction race condition | Evict-before-push `swap_remove` O(n) (`inference.rs:add_sample`) |
| ~~3~~ | INTELLIGENCE | NaN si half_life=0 | `.max(1.0)` guard (`inference.rs:128`) |
| ~~4~~ | AUTOMATION | Sync context building | `async build_decision_context()` (`engine.rs`) |
| ~~5~~ | SSL PLUGIN | Fingerprint events non émis | `publish_fingerprint_change()` (`ssl/mqtt.rs`) |
| ~~6~~ | PWA | XSS innerHTML | `DOMPurify.sanitize()` (`environment-widget.js`) |
| ~~7~~ | PWA | Device token pas effacé logout | `removeItem('symbion_device_token')` (`auth-service.js`) |
| ~~8~~ | PWA | API non sanitizées | `_sanitizeResponse()` récursif (`api-service.js`) |
| ~~9~~ | AGENT | MQTT always online | `Arc<AtomicBool>` état réel (`main.rs`) |
| ~~10~~ | AGENT | Disk metrics Windows | `sysinfo::Disks` cross-platform (`metrics/mod.rs`) |
| ~~11~~ | INFRA | Secrets hardcodés | `${VAR:?error}` (`docker-compose.yml`, `deploy-dashboard.yml`) |

---

### P1 — Issues Importants (18) — ✅ TOUTES CORRIGÉES

> 5 corrigées avant sprint (commits précédents), 13 corrigées commit `4f5cbce` — 18 Février 2026

| # | Module | Issue | Fix |
|---|--------|-------|-----|
| ~~1~~ | KERNEL | Contract validation manquante | Vérification champs `required` JSON schema (`contracts.rs`) |
| ~~2~~ | INTELLIGENCE | .json.tmp orphelins | `remove_file()` sur erreur write/rename (`inference.rs`) |
| ~~3~~ | INTELLIGENCE | Confidence non propagée | Pondération `contribution × confidence` (`vector.rs`) |
| ~~4~~ | INTELLIGENCE | I/O synchrone bloquant | `std::thread::spawn` pour file I/O (`inference.rs`) |
| ~~5~~ | DECISION | Trust sans decay | Decay exponentiel half-life 30j (`trust_tracker.rs`) |
| ~~6~~ | DECISION | PendingActions en mémoire | Persistence JSON atomique (`pending_actions.rs`) |
| ~~7~~ | DECISION | Validation cleanup jamais appelé | `cleanup_expired()` périodique 10 min (`kernel/main.rs`) |
| ~~8~~ | DECISION | Override cleanup jamais appelé | `cleanup_expired()` périodique 10 min (`kernel/main.rs`) |
| ~~9~~ | AUTOMATION | SSID hardcodé "local" | `FeatureRegistry.get_string("net.ssid")` (`engine.rs`) |
| ~~10~~ | AUTOMATION | Cooldown gap | `record_execution()` avant `execute_actions()` (`listener.rs`) |
| ~~11~~ | SSL | State save .ok() silencieux | `if let Err(e) = ... { warn!() }` (`ssl/main.rs`) |
| ~~12~~ | FREEBOX | mem::forget MQTT | `_shutdown_tx` stocké dans struct (`freebox/mqtt.rs`) |
| ~~13~~ | AGENT | MQTT retry fixe 5s | Backoff exponentiel 2→4→8→16→32s (`agent/main.rs`) |
| ~~14~~ | AGENT | /reconnect non implémenté | `mpsc` channel → main loop re-registration (`local_api.rs`) |
| ~~15~~ | AGENT | Agent ID non persisté | `~/.config/symbion/agent-id` (`discovery.rs`) |
| ~~16~~ | AGENT | Child process orphelins timeout | PID-based kill sur timeout (`execution/mod.rs`) |
| ~~17~~ | PWA | Agent cache unbounded | LRU eviction max 50 (`mqtt-service.js`) |
| ~~18~~ | PWA | Passkey URL hardcodée | `window.location.origin` (`passkey-manager.js`) |

---

### P2 — Améliorations (67 identifiées — 45 corrigées, 17 faux positifs, 5 restantes)

#### Kernel (3) — ✅ TOUS CORRIGÉS

| Issue | Fichier | Status |
|-------|---------|--------|
| ~~Notification path hardcodé~~ | `notifications.rs` | ✅ `SYMBION_DATA_DIR` env var |
| ~~Chemins config process_categories.toml hardcodés~~ | `mqtt.rs` | ✅ `SYMBION_CONFIG_DIR` env var |
| ~~SMTP port parse silently fallback 587~~ | `notifications.rs` | ✅ `eprintln!` on parse failure |

#### Intelligence (21) — ✅ TOUS CORRIGÉS (18 fixés + 3 faux positifs)

| Issue | Fichier | Status |
|-------|---------|--------|
| ~~Race condition add_sample~~ | `inference.rs` | ✅ Documenté (write lock atomique) |
| ~~Normalisation zero-sum silencieuse~~ | `vector.rs` | ✅ Warning log si sum < 1e-6 |
| ~~Confidence non validée~~ | `vector.rs` | ✅ `.clamp(0.0, 1.0)` |
| ~~Clock skew cooldown permanent~~ | `sessions.rs` | ✅ Returns false + warning |
| ~~Override expiry bypass~~ | `sessions.rs` | ✅ Log inconsistance |
| ~~Double-count classifier~~ | `classifier.rs` | ✅ Documenté (by-design) |
| ~~TOML weights non validés~~ | `classifier.rs` | ✅ Validation [0.0, 2.0] |
| ~~Cleanup contention features~~ | `features.rs` | ✅ AtomicBool guard |
| ~~Clock skew features is_expired~~ | `features.rs` | ✅ Negative age check |
| ~~Bootstrap zero-sum vector~~ | `bootstrap.rs` | ✅ Uniform 0.25 distribution |
| ~~Bootstrap ignore config~~ | — | 🔇 Faux positif |
| ~~Bootstrap modes non validés~~ | `bootstrap.rs` | ✅ VALID_MODES const |
| ~~Config cross-field validation~~ | `config.rs` | ✅ `validate()` method |
| ~~Timezone fallback~~ | — | 🔇 Faux positif (déjà configurable) |
| ~~v1/v2 decay inconsistency~~ | `context_intelligence.rs` | ✅ Automation 1.0→0.8 |
| ~~Stats 24h jamais reset~~ | `context_intelligence.rs` | ✅ Timestamp + auto-reset |
| ~~Agent metrics mauvais agent~~ | — | 🔇 Faux positif |
| ~~Focus missing init_patterns~~ | `context_intelligence.rs` | ✅ Documenté (enum legacy) |
| ~~save_patterns sans retry~~ | `context_intelligence.rs` | ✅ 1 retry après 500ms |
| ~~exit_threshold non exposé~~ | `config.rs` | ✅ `session_exit_threshold` field |
| ~~Source weight sans cap~~ | `inference.rs` | ✅ Documenté (intentionnel, max 1.3) |

#### Decision + Automation (10) — 9 corrigés, 1 différé

| Issue | Fichier | Status |
|-------|---------|--------|
| ~~Decay modifier calculé 2x~~ | — | 🔇 Déjà corrigé P1 |
| ~~Trust score fallback 0.7~~ | — | 🔇 Déjà corrigé P1 |
| ~~Validation cleanup_expired()~~ | — | 🔇 Déjà corrigé P1 |
| ~~Override cleanup_expired()~~ | — | 🔇 Déjà corrigé P1 |
| ~~TOCTOU race override~~ | — | 🔇 Déjà corrigé P1 |
| ~~SSID hardcodé "local"~~ | — | 🔇 Déjà corrigé P1 |
| ~~Actions si Decision error~~ | `engine.rs` | ✅ `catch_unwind` |
| ~~Cooldown pendant execution~~ | `listener.rs` | ✅ `Mutex<HashSet>` execution lock |
| ~~Feature registry silently false~~ | — | 🔇 Déjà corrigé P1 |
| ~~Test coverage minimale~~ | `engine.rs` | ✅ 24 tests (Sprint 6) |

#### Plugins (5) — ✅ TOUS CORRIGÉS (2 fixés + 3 faux positifs)

| Issue | Fichier | Status |
|-------|---------|--------|
| ~~Notes socket cleanup errors ignorés~~ | `notes/main.rs` | ✅ Error logging |
| ~~SSL state save .ok()~~ | — | 🔇 Déjà corrigé P1 |
| ~~SSL reqwest inutilisée~~ | `ssl/Cargo.toml` | ✅ Dépendance supprimée |
| ~~Freebox downloads API~~ | — | 🔇 Faux positif |
| ~~Freebox chemins config~~ | — | 🔇 By-design (API Freebox) |

#### Agent Host (7) — ✅ TOUS CORRIGÉS

| Issue | Fichier | Status |
|-------|---------|--------|
| ~~Password non masqué~~ | `wizard.rs` | ✅ `rpassword` crate |
| ~~Password non zéroizé~~ | `config.rs` | ✅ `zeroize` + Drop impl |
| ~~Load average 0.0 Windows~~ | — | 🔇 By-design (cross-platform) |
| ~~Dead code tray.rs~~ | — | 🔇 By-design (conditional) |
| ~~GitHub API sans token~~ | `updater.rs` | ✅ `GITHUB_TOKEN` optionnel |
| ~~Gestion services cross-platform~~ | `execution/mod.rs` | ✅ ServiceManager (Sprint 6) |
| ~~Kill + restart process~~ | `execution/mod.rs` | ✅ kill_and_restart (Sprint 6) |

#### PWA Dashboard (11) — ✅ TOUS CORRIGÉS

| Issue | Fichier | Status |
|-------|---------|--------|
| ~~Device token sans HTTPS check~~ | — | 🔇 Déjà HTTPS obligatoire |
| ~~CSRF nonce response validation~~ | — | 🔇 Faux positif (JSON garanti) |
| ~~CSRF nonce fetch sans timeout~~ | `csrf-service.js` | ✅ AbortController 10s |
| ~~API_BASE sans cert pinning~~ | — | 🔇 N/A (LAN only, mkcert) |
| ~~MQTT reconnection counter~~ | — | 🔇 Déjà corrigé |
| ~~HSL bounds check~~ | — | 🔇 Faux positif (toujours positif) |
| ~~Login form double-submit~~ | `boot-terminal.js` | ✅ disabled + loading state (Sprint 6) |
| ~~WebAuthn sans timeout~~ | `passkey-manager.js` | ✅ Promise.race 60s |
| ~~`<br>` au lieu de `<br/>`~~ | `sanitization.js` | ✅ Self-closing tag |
| ~~Request batching~~ | — | 🔇 N/A (déjà optimisé) |
| ~~aria-expanded manquant~~ | `dashboard-app.js` | ✅ Attribut ajouté |

#### Infrastructure (10) — ✅ TOUS CORRIGÉS

| Issue | Fichier | Status |
|-------|---------|--------|
| ~~send-mail.sh pipefail~~ | `send-mail.sh` | ✅ `set -euo pipefail` |
| ~~Dashboard systemd WorkingDirectory~~ | `symbion-dashboard.service` | ✅ Chemin corrigé |
| ~~Plugins systemd tilde~~ | — | 🔇 Faux positif (bash source OK) |
| ~~Docker resource limits~~ | `docker-compose.yml` | ✅ Memory + CPU limits |
| ~~Release GPG signing~~ | `release.yml` | ✅ GPG conditionnel (Sprint 6) |
| ~~CLAUDE.md exclu git~~ | — | 🔇 Intentionnel |
| ~~Chemins hardcodés backup~~ | `backup-symbion.sh` | ✅ `SCRIPT_DIR` + env vars |
| ~~stat GNU-only~~ | — | 🔇 Faux positif (dual syntax déjà) |
| ~~Dockerfile errors silencieux~~ | — | 🔇 Faux positif (RUN set -e) |
| ~~Nginx security headers~~ | `nginx-dashboard.conf` | ✅ X-Content-Type, X-Frame, HSTS, CSP |

---

### P3 — Issues Mineures (49)

#### Kernel (9)

| Issue | Fichier |
|-------|---------|
| MFA test coverage limitée (backup codes, TOTP window, QR) | `mfa.rs` |
| Dead code markers (#[allow(dead_code)]) non nettoyés | `agents.rs:202-213` |
| Old enum naming : Cravate/Intime/Neutre au lieu de Pro/Focus/Maison | `context.rs` |
| http.rs monolithique (3,783 LOC) — devrait être split en sub-routers | `http.rs` |
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

## Issues Corrigées — Historique

### P0 — 11/11 Corrigés ✅

**Commit `268b8a5` — 16 Février 2026 (audit initial P0)**
- [x] ~~prediction_history overflow~~ → `MAX_PREDICTION_HISTORY=1000` + eviction
- [x] ~~Sample eviction race~~ → evict-before-push `swap_remove` O(n)
- [x] ~~NaN half_life=0~~ → `.max(1.0)` guard
- [x] ~~Sync context building~~ → `async build_decision_context()`
- [x] ~~SSL fingerprint events~~ → `publish_fingerprint_change()`
- [x] ~~XSS innerHTML~~ → `DOMPurify.sanitize()`
- [x] ~~Device token logout~~ → `removeItem('symbion_device_token')`
- [x] ~~API non sanitizées~~ → `_sanitizeResponse()` récursif
- [x] ~~MQTT always online~~ → `Arc<AtomicBool>` état réel
- [x] ~~Disk metrics Windows~~ → `sysinfo::Disks` cross-platform
- [x] ~~Secrets hardcodés~~ → `${VAR:?error}`

**Pré-audit (commits antérieurs)**
- [x] ~~Broadcast channel overflow~~ → Capacité 100→512 (`events.rs:116`)
- [x] ~~Persistence non-atomique~~ → temp file + rename (`inference.rs:309-334`)
- [x] ~~Normalisation trop conservatrice~~ → Seuil 0.1→1e-6 (`vector.rs:419`)

### P1 — 18/18 Corrigés ✅

**Commit `4f5cbce` — 18 Février 2026 (sprint fiabilité P1)**
- [x] ~~Contract validation~~ → vérification champs `required` JSON schema
- [x] ~~.json.tmp orphelins~~ → `remove_file()` sur erreur
- [x] ~~cleanup_expired() validation~~ → timer périodique 10 min (kernel main.rs)
- [x] ~~cleanup_expired() override~~ → timer périodique 10 min (kernel main.rs)
- [x] ~~SSID hardcodé~~ → `FeatureRegistry.get_string("net.ssid")`
- [x] ~~Cooldown gap~~ → `record_execution()` avant actions
- [x] ~~SSL .ok() silencieux~~ → `warn!` logging
- [x] ~~Freebox mem::forget~~ → `_shutdown_tx` struct field
- [x] ~~MQTT retry fixe~~ → backoff exponentiel 2→32s
- [x] ~~/reconnect non implémenté~~ → mpsc channel → main loop
- [x] ~~Agent ID non persisté~~ → `~/.config/symbion/agent-id`
- [x] ~~Child process orphelins~~ → PID-based kill on timeout
- [x] ~~Agent cache unbounded~~ → LRU eviction max 50
- [x] ~~Passkey URL hardcodée~~ → `window.location.origin`

**Pré-audit (commits antérieurs)**
- [x] ~~I/O synchrone bloquant~~ → File I/O via `std::thread::spawn`
- [x] ~~Confidence non propagée~~ → pondération par `confidence`
- [x] ~~PendingActions en mémoire~~ → Persistence JSON atomique
- [x] ~~PC_ACTIVE non normalisé~~ → Weighted cosine similarity ×0.3
- [x] ~~Trust Tracker sans decay~~ → Decay exponentiel half-life 30j

### P2 — 8 Corrigés (17 Février 2026)
- [x] ~~Tie-breaking top-k~~ → timestamp critère secondaire (`inference.rs:404`)
- [x] ~~Bootstrap multi-slot~~ → 7 time-slot samples (`bootstrap.rs:148-210`)
- [x] ~~SSID case-insensitive~~ → `eq_ignore_ascii_case` (`trust.rs:119`)
- [x] ~~Timezone configurable~~ → `SYMBION_TIMEZONE` env var (`intelligence/mod.rs:137-165`)
- [x] ~~Clock skew protection~~ → Duration clamped à zero (`sessions.rs:144`)
- [x] ~~Compaction périodique~~ → compact() toutes les 100 cycles (`context_intelligence.rs`)
- [x] ~~Stability score decay~~ → decay exponentiel half-life 60 min (`sessions.rs:184-192`)
- [x] ~~symbion-devkit obsolète~~ → Supprimé du workspace

### P2 — 42 Corrigés Sprint 5A/5B + Sprint 6 (19 Février 2026)

**Commit `1789789` — Kernel core + Intelligence + Decision (26 issues)**
- [x] ~~Notification path hardcodé~~ → `SYMBION_DATA_DIR` env var (`notifications.rs`)
- [x] ~~process_categories.toml hardcodé~~ → `SYMBION_CONFIG_DIR` env var (`mqtt.rs`)
- [x] ~~SMTP port silent fallback~~ → `eprintln!` on parse failure (`notifications.rs`)
- [x] ~~add_sample race condition~~ → documenté write lock scope (`inference.rs`)
- [x] ~~Zero-sum normalize~~ → warning log (`vector.rs`)
- [x] ~~Confidence non validée~~ → `.clamp(0.0, 1.0)` (`vector.rs`)
- [x] ~~Clock skew cooldown~~ → returns false + log warning (`sessions.rs`)
- [x] ~~Override expiry bypass~~ → log inconsistance (`sessions.rs`)
- [x] ~~Double-count classifier~~ → documenté by-design (`classifier.rs`)
- [x] ~~TOML weights négatifs~~ → validation [0.0, 2.0] (`classifier.rs`)
- [x] ~~Cleanup contention~~ → `AtomicBool` guard (`features.rs`)
- [x] ~~Clock skew is_expired~~ → negative age check (`features.rs`)
- [x] ~~Bootstrap zero-sum~~ → uniform 0.25 distribution (`bootstrap.rs`)
- [x] ~~Bootstrap modes invalides~~ → `VALID_MODES` const validation (`bootstrap.rs`)
- [x] ~~Config cross-field~~ → `validate()` method (`config.rs`)
- [x] ~~v1/v2 decay inconsistency~~ → Automation 1.0→0.8 (`context_intelligence.rs`)
- [x] ~~Stats 24h reset~~ → timestamp + auto-reset (`context_intelligence.rs`)
- [x] ~~Focus init_patterns~~ → documenté enum legacy (`context_intelligence.rs`)
- [x] ~~save_patterns retry~~ → 1 retry après 500ms (`context_intelligence.rs`)
- [x] ~~exit_threshold non exposé~~ → `session_exit_threshold` field (`config.rs`)
- [x] ~~Source weight cap~~ → documenté intentionnel max 1.3 (`inference.rs`)
- [x] ~~Decision Engine error~~ → `catch_unwind` safety (`engine.rs`)
- [x] ~~Cooldown pendant execution~~ → `Mutex<HashSet>` lock (`listener.rs`)

**Commit `4cc359e` — Plugins (2 issues)**
- [x] ~~Notes socket cleanup~~ → error logging (`notes/main.rs`)
- [x] ~~SSL reqwest inutilisée~~ → dépendance supprimée (`ssl/Cargo.toml`)

**Commit `1d01460` — Agent Host (3 issues)**
- [x] ~~Password non masqué~~ → `rpassword` crate (`wizard.rs`)
- [x] ~~Password non zéroizé~~ → `zeroize` + Drop impl (`config.rs`)
- [x] ~~GitHub API sans token~~ → `GITHUB_TOKEN` optionnel (`updater.rs`)

**Commit `08da280` — PWA Dashboard (4 issues)**
- [x] ~~CSRF fetch sans timeout~~ → AbortController 10s (`csrf-service.js`)
- [x] ~~WebAuthn sans timeout~~ → Promise.race 60s (`passkey-manager.js`)
- [x] ~~`<br>` → `<br/>`~~ → self-closing tag (`sanitization.js`)
- [x] ~~aria-expanded manquant~~ → attribut ajouté (`dashboard-app.js`)

**Commit `ce6ded9` — Infrastructure (5 issues)**
- [x] ~~send-mail.sh pipefail~~ → `set -euo pipefail`
- [x] ~~Dashboard systemd path~~ → WorkingDirectory corrigé
- [x] ~~Docker resource limits~~ → memory + CPU limits
- [x] ~~Backup paths hardcodés~~ → `SCRIPT_DIR` + env vars
- [x] ~~Nginx security headers~~ → X-Content-Type, X-Frame, HSTS, CSP, Referrer

**Commit `f1b1951` — Sprint 6 : P2 Différés (5 issues)**
- [x] ~~Test coverage automation~~ → 24 tests engine.rs (308 total kernel)
- [x] ~~ServiceManager cross-platform~~ → systemctl/sc/launchctl (`execution/mod.rs`)
- [x] ~~Kill + restart process~~ → kill_and_restart par PID + nom (`execution/mod.rs`)
- [x] ~~GPG signing release~~ → étape conditionnelle (`release.yml`)
- [x] ~~Login double-submit~~ → disabled + loading state (`boot-terminal.js`)

---

## Plan d'Action — Sprints

### Sprint 1-2 — Sécurité P0 ✅ TERMINÉ (16 Fév 2026)
- [x] Fix XSS PWA (DOMPurify)
- [x] Fix device token logout
- [x] Sanitizer réponses API
- [x] Fix prediction_history overflow
- [x] Fix half_life=0 NaN
- [x] Fix sample eviction race
- [x] Fix async context building
- [x] SSL fingerprint events
- [x] Agent MQTT status réel
- [x] Agent disk metrics cross-platform
- [x] Secrets hardcodés supprimés

### Sprint 3-4 — Fiabilité P1 ✅ TERMINÉ (18 Fév 2026)
- [x] MQTT contract validation
- [x] Cleanup .json.tmp orphelins
- [x] cleanup_expired() auto-scheduled (10 min)
- [x] SSID depuis FeatureRegistry
- [x] Cooldown avant actions
- [x] SSL state save error handling
- [x] Freebox mem::forget → struct field
- [x] MQTT recovery exponential backoff
- [x] Agent /reconnect endpoint fonctionnel
- [x] Agent ID persisté sur disque
- [x] Agent child process cleanup on timeout
- [x] PWA agent cache LRU (max 50)
- [x] PWA passkey-manager window.location.origin

### Sprint 5A/5B — P2 Améliorations ✅ TERMINÉ (19 Fév 2026)
- [x] Kernel core : env vars pour paths, SMTP logging (3 issues)
- [x] Intelligence : 18 issues (confidence, clock skew, bootstrap, config, decay, retry)
- [x] Decision : catch_unwind + execution lock (2 issues)
- [x] Plugins : notes cleanup + SSL reqwest (2 issues)
- [x] Agent : rpassword + zeroize + GITHUB_TOKEN (3 issues)
- [x] PWA : CSRF/WebAuthn timeout + br + aria-expanded (4 issues)
- [x] Infra : pipefail + systemd + docker + backup + nginx (5 issues)

### Sprint 6 — P2 Différés ✅ TERMINÉ (19 Fév 2026)
- [x] D10 : 24 tests automation engine (308 total kernel)
- [x] A6 : ServiceManager cross-platform (systemctl/sc/launchctl)
- [x] A7 : kill_and_restart process
- [x] I5 : GPG signing conditionnel dans release.yml
- [x] PWA-7 : Login double-submit disabled + loading state

### Sprint 7 (Prochain) — PR6 + Features
- [ ] PR6 : Let's Encrypt ACME
- [ ] PR6 : SQLite migration (12 fichiers JSON)
- [ ] Tests PWA (objectif 50%+ coverage)

### Sprint 8+ (Long terme) — Features + P3
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
