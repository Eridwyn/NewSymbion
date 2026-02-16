# Roadmap Technique - NewSymbion

**Version** : 2026-02 (Post Intelligence v2 + Automations)
**Statut** : Fondations complètes, système intelligent opérationnel
**Dernière mise à jour** : 16 Février 2026

---

## Vue d'Ensemble

### Codebase

| Composant | Langage | LOC | Fichiers |
|-----------|---------|----:|----------|
| **symbion-kernel** | Rust | 36,304 | 34 modules |
| **pwa-dashboard** | JS (Lit) | 28,966 | 13+ composants |
| **symbion-agent-host** | Rust | 4,292 | Multi-platform |
| **Plugins** (5) | Rust | 4,900 | 5 crates |
| **Total** | | **~74,500** | |

### Chiffres Clés

| Métrique | Valeur |
|----------|--------|
| Unit Tests | 280 |
| API Routes (http.rs) | 176 |
| MQTT Topics | 10 subscriptions |
| Automations actives | 16 |
| Modes contextuels | 4 système + custom |
| Intelligence Samples | 26+ (apprentissage) |
| Data files (JSON) | 11 |

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

**Fichiers** : `decision/` (16 modules, 6,120 LOC)

---

### PR4 — Metrics & Observability 🟢 100%
**Complété** : Novembre 2025

- `GET /metrics` — 36 métriques Prometheus
- `GET /v1/metrics/system` — Kernel overview JSON
- `GET /v1/metrics/agents` — Télémétrie par agent
- `GET /health` — Liveness check
- Agent telemetry (30s heartbeat : CPU, RAM, disk, network, processes)
- Structured logging par catégorie

**Fichiers** : `decision/metrics.rs` (549 LOC), `http.rs` (3,605 LOC)

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

Moteur d'inférence case-based reasoning avec apprentissage continu.

- **Feature Registry** : TTL 60-300s, cleanup auto, 4 types (Bool, Float, String, StringList)
- **Vector Builder** : 5 dimensions normalisées (home_prob, work_prob, focus_prob, sleep_prob, pc_active)
- **Inference Engine** : Cosine similarity + k-NN (top-10), weighted voting
- **Session Manager** : Hysteresis 4 couches (entry/exit gap, 3 consécutifs, 5 min min, 30 min cooldown)
- **Source Weighting** : UserCorrection 1.3x, MfaConfirmed 1.0x, Automation 0.8x, Bootstrap 0.5x
- **Time Decay** : Half-life 30j (normal), 7j (bootstrap)
- **v2 Stabilization** : Shadow mode, 6 guards avant auto-apply
- **Persistence atomique** : temp file + rename (P0 fix)
- **Normalisation** : Seuil 1e-6 (P0 fix)

**Fichiers** : `intelligence/` (9 modules, 3,821 LOC), `context_intelligence.rs` (1,784 LOC)

---

### Automation Engine 🟢 100%
**Complété** : Janvier-Février 2026

Moteur event-driven avec 16 automations actives.

- **7 types de triggers** : mode_change, sensor_alert, agent_status, manual, plugin_health, scheduled, polling
- **9 types de conditions** : mode, time_range, day_of_week, sensor_threshold, agent_online, custom, AND, OR, NOT
- **Actions** : MQTT publish, mode change, notification, webhook
- **Scheduler** : Cron-like avec timezone Europe/Paris
- **Decision Bridge** : Intégration Decision Engine pour validation
- **Pending Actions** : Workflow approbation manuelle
- **Broadcast channel** : Capacité 512 events (P0 fix)
- **Persistence** : `data/automations.json` + `data/automations_history.json`

**Fichiers** : `automations/` (11 modules, 6,156 LOC)

---

### F1 — Environment Monitoring 🟢 100%
**Complété** : Novembre 2025

- RoomEnvironmentState + circular buffer (20,160 readings = 7 jours)
- DewPointAlertLevel : 6 niveaux (Safe → Danger)
- Calcul point de rosée Magnus (±0.4°C)
- 5 niveaux d'alertes progressifs (Weak → Danger)
- 5 API endpoints (`/v1/environment/...`)
- Plugin sensors ESP32 BME280 (821 LOC)
- PWA widget dynamique (Chart.js dual Y-axis, 7j historique)
- 44 unit tests (environment + dew point)

**Fichiers** : `environment.rs` (558 LOC), `dew_point_alerts.rs` (650 LOC), `decision/environment.rs` (431 LOC)

---

### F4 — Notifications 🟢 100%
**Complété** : Décembre 2025

- NotificationManager avec priority queue (P0, P1, P2)
- Multi-canal : Firebase FCM, SMTP email, ntfy.sh, MQTT
- Priority-based retry (P0: immédiat + escalation, P1: retry 15min, P2: best effort)
- Deduplication (content hash)
- Rate limiting par source
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

- Discovery automatique Unix socket
- Health monitoring + alertes
- Routes dynamiques `/v1/plugin-api/{plugin}/*`
- Features automation (`ssl.{domain}.*`, `freebox.presence.*`)

---

### Dynamic Themes & Modes 🟢 100%
**Complété** : Février 2026

- ModeRegistry avec persistence `data/modes.json`
- 4 modes système (Pro #2563eb, Focus #6366f1, Maison #10b981, Veille #6b7280)
- Modes custom via API (POST /v1/modes)
- Logo header colorisé dynamiquement (hexToHSL → CSS filter)
- Variable CSS `--context-primary`

---

### Per-Agent Features 🟢 100%
**Complété** : Février 2026

- Features individuelles : `agent.{id}.online`, `agent.{id}.cpu`, `agent.{id}.memory`
- Features globales (Windows PC uniquement pour backward compat)
- AgentRegistry debounced persistence (5 min save interval)

**Fichiers** : `agents.rs` (772 LOC), `decision/agent_status.rs` (541 LOC)

---

### Prediction Correction UI 🟢 100%
**Complété** : 16 Février 2026

Correction prédiction intelligence directement depuis l'interface PWA.

- **Backend** : `POST /v1/intelligence/feedback` enrichi v1+v2
  - v1 : `record_feedback()` (pattern-based learning)
  - v2 : `record_correction()` (case-based inference, UserCorrection priority)
- **Frontend** : Bouton "Corriger la prédiction" dans l'onglet Intelligence
  - Panel modes avec icônes dynamiques
  - Mode prédit actuel désactivé (grisé)
  - Confirmation visuelle + auto-refresh 1.5s
  - Utilise `csrfService.fetchWithCsrf()` pour compatibilité mobile

**Fichiers** : `intelligence_http.rs:220-240`, `context-engine-page.js:5268-5291`

---

## Phase Active

### PR6 — Production Readiness 🟡 ~30%
**En cours** — Démarré Novembre 2025

**Complété** :
- [x] CSP headers (strict default-deny) (`http.rs:459-494`)
- [x] HSTS headers (`http.rs:445-457`)
- [x] Security documentation
- [x] CI/CD pipelines (3 GitHub Actions workflows)
  - `deploy-kernel.yml` — Multi-platform builds (Linux/Windows/macOS)
  - `deploy-dashboard.yml` — PWA build
  - `release.yml` — Agent releases

**Restant** :
- [ ] Let's Encrypt ACME integration
- [ ] SQLite/PostgreSQL migration (JSON files actuels)
- [ ] Docker containerization
- [ ] CI/CD test suite (builds OK, tests manquants dans CI)
- [ ] Database backups automatiques
- [ ] Rate limiting global (IP-based, pas seulement auth)
- [ ] Monitoring externe (healthcheck.io ou équivalent)
- [ ] Log rotation (structuré → fichier rotatif)

---

## Features Planifiées (Non Implémentées)

### F2 — Digital Hygiene ⚪ 0%
**Effort estimé** : 9 jours

Tracking activité PC (idle/work/game) et prévention burnout.

- Activity tracker dans agent-host (process classification)
- MQTT topic `symbion/agents/activity@v1`
- Decision rules : pause 4h, alerte burnout 10h×3j
- PWA widget temps d'écran + page stats

---

### F3 — Intentions Log ⚪ 0%
**Effort estimé** : 5 jours

Persistence et analytics historique des intentions Decision Engine.

- Storage JSON/SQLite avec retention 90 jours
- API analytics (filtres type, impact, date range)
- PWA page paginée + export CSV

---

### F5 — Light Actuator ⚪ 0%
**Effort estimé** : 10 jours

Contrôle lumières connectées avec abstraction générique.

- Trait `LightActuator` (on/off, brightness, color temp, RGB)
- Backend Tuya local (tuyapi protocol, LAN-only)
- Decision rules context-aware (Pro → froid 100%, Maison → chaud 30%)
- MQTT topics `symbion/lights/command@v1` + `symbion/lights/state@v1`
- PWA widget contrôle + scénarios prédéfinis

---

## Issues Connues

### P0 — Corrigés (16 Février 2026)
- [x] ~~Broadcast channel overflow~~ → Capacité 100→512 (`events.rs:116`)
- [x] ~~Persistence non-atomique~~ → temp file + rename (`inference.rs:309-334`)
- [x] ~~Normalisation trop conservatrice~~ → Seuil 0.1→1e-6 (`vector.rs:419`)

### P1 — À Traiter
- [ ] **Confidence non propagée** : Features ont une confidence (0-1) ignorée par VectorBuilder
- [ ] **I/O synchrone bloquant** : save_samples() synchrone après chaque add
- [ ] **PendingActions en mémoire** : Redémarrage kernel = validations perdues
- [ ] **PC_ACTIVE non normalisé** : Échelle différente des mode probs → biais cosine
- [ ] **Trust Tracker sans decay** : Pas de dégradation temporelle des stats

### P2 — Améliorations
- [ ] Tie-breaking top-k (timestamp comme critère secondaire)
- [ ] Bootstrap multi-slot (7+ samples à l'init)
- [ ] SSID case-insensitive (RFC compliance)
- [ ] Timezone configurable (hardcodé Europe/Paris)
- [ ] Clock skew protection (NTP drift → duration négative)
- [ ] Compaction périodique automatique
- [ ] Stability score decay temporel

---

## Recommandations Prochains Sprints

### Sprint Immédiat (Février 2026)
1. ~~**Correction prediction PWA**~~ ✅ — Bouton "corriger" dans l'onglet Intelligence
2. **Corriger P1 I/O synchrone** — save_samples() async ou background spawn
3. **Propager confidence** features → VectorBuilder (contribution × confidence)

### Court Terme (Mars 2026)
4. **F2 Digital Hygiene** — Activity tracking + burnout detection
5. **Persister PendingActions** sur disque
6. **Trust Tracker decay** (half-life 30 jours)

### Moyen Terme (Q2 2026)
7. **F3 Intentions Log** — Audit trail + analytics
8. **PR6 suite** — SQLite migration, Docker, CI/CD
9. **F5 Light Actuator** (si matériel Tuya confirmé)

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
    ↓  16 automations actives, scheduler cron-like
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
