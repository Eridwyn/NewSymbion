# NewSymbion

**Version**: v0.3.0-alpha.3
**Status**: Production-Ready Core (PR1-PR5 Complete)
**License**: MIT

Système d'automatisation personnelle et domotique intelligent avec architecture IoT distribuée.

---

## Architecture

NewSymbion utilise une architecture hub-and-spoke en trois composants :

- **symbion-kernel** : Hub central (Rust) - Event bus MQTT, API REST, gestion des plugins
- **symbion-agent-host** : Agents système (Rust) - Monitoring et contrôle des machines
- **pwa-dashboard** : Interface web (Lit + Vite) - Dashboard adaptatif temps réel

### Communication

- **MQTT** : Bus d'événements temps réel (15 topics actifs, QoS 1)
- **HTTP/REST** : API JSON (~107 routes, 80 handlers)
- **WebSocket** : Streaming temps réel (notes, métriques)

---

## Fonctionnalités Implémentées

### Kernel (Hub Central)

#### Sécurité (7 Couches)
- TLS 1.3 (port 8443, redirect automatique depuis 8080)
- HSTS headers (max-age=31536000)
- CSP headers (strict default-deny)
- JWT authentication (HS256, 8h expiry)
- MFA/TOTP (RFC 6238, QR code, 5 backup codes)
- WebAuthn passkeys (biometric auth)
- CSRF protection (single-use nonces, 5 min TTL)
- Rate limiting (5 attempts / 15 min, auth endpoints)
- Bcrypt cost 12 (~250ms/hash)

#### Context Engine
- 3 modes : Cravate (pro), Intime (maison), Neutre (surveillance)
- IANA timezone support (Europe/Zurich + DST)
- Hysteresis anti-flapping (120s threshold)
- Détection automatique (week-end, nuit 23h-7h)
- Override manuel avec expiration
- History persistence (JSON)

#### Decision Engine
- Trust scoring (5 facteurs pondérés : context 30%, telemetry 25%, history 25%, network 10%, presence 10%)
- Impact levels (Low/Medium/High)
- Idempotence (command_id deduplication)
- Validation workflow (pending/approved/rejected)
- Audit trail complet
- 93 unit tests (all passing)

#### Metrics & Observability
- 22 métriques Prometheus (GET /metrics)
- JSON metrics endpoints (/v1/metrics/system, /v1/metrics/agents)
- Health checks (/health, /system/health)
- Structured logging ([category] message)

#### Reliability
- Graceful shutdown (SIGTERM)
- MQTT auto-reconnect (exponential backoff, 5 retries)
- Plugin isolation (panic recovery)
- Panic hook with context logging
- Systemd service (Restart=always, 5s RestartSec)
- AgentRegistry persistence (5 min debounced I/O)

#### Plugin System
- symbion-plugin-notes : CRUD notes, tags auto, streaming MQTT (1-by-1 + ListEnd marker)
- Architecture extensible pour nouveaux plugins

### Agents (Monitoring & Contrôle)

#### Commandes Supportées
- `shutdown` : Extinction machine
- `reboot` : Redémarrage
- `hibernate` : Hibernation
- `kill_process` : Kill par PID
- `run_command` : Shell whitelist (safe commands only)
- `get_metrics` : Collecte métriques
- `list_processes` : Liste processus actifs

#### Métriques Collectées
- CPU : Usage %, load average (1/5/15 min), core count
- Memory : Total/used/available MB, % used
- Disk : Total/used/free GB par mount, % used
- Network : Bytes sent/recv par interface, is_up status
- Processes : Total count, running count, top CPU/memory
- Services (Linux) : Critical services status (systemctl)

#### Features
- Auto-discovery (OS, hostname, network, MAC)
- Heartbeat 30s (MQTT)
- Local API server (port 9899)
- System tray (Linux/Windows)
- Auto-update check
- GUI mode (Windows) / Terminal mode (Linux)
- Multi-platform (Linux, Windows)

### PWA Dashboard

#### Components (6)
- `boot-terminal.js` : Boot sequence multi-phase (login, MFA, WebAuthn)
- `dashboard-app.js` : Application principale
- `notes-page.js` : Interface CRUD notes complète
- `organic-loader.js` : Loader bioluminescent (blob morphing CSS)
- `passkey-manager.js` : Gestion passkeys WebAuthn
- `user-settings-page.js` : Settings (password, MFA, decisions, security)

#### Widgets (10)
- Context widget : Mode actuel + confidence
- Context stats : Temps par mode
- Context settings : Override manuel
- Agents network : Réseau agents (status, metrics)
- Notes widget : Quick view avec loader
- Plugins widget : Status plugins
- System health : Uptime, memory, MQTT
- Agent control : Contrôles shutdown/reboot
- Hosts widget : Liste agents
- Widget registry : Dynamic loading

#### Services
- MQTT service : Client WebSocket
- Decision service : API client (CSRF protected)
- CSRF service : Auto token management
- Notes stream service : WebSocket streaming

---

## Installation

### Prérequis

- Rust 1.70+ (`rustc --version`)
- Node.js 18+ (`node --version`)
- Mosquitto MQTT broker (`sudo apt install mosquitto`)
- TLS certificates (mkcert pour développement)

### 1. Kernel

```bash
cd symbion-kernel

# Variables d'environnement
export SYMBION_API_KEY="s3cr3t-42"
export SYMBION_MQTT_BROKER="127.0.0.1:1883"
export SYMBION_JWT_SECRET="<64 bytes hex>" # 128 chars

# Lancement
cargo run --release
```

Le kernel démarre sur :
- HTTPS : `https://localhost:8443`
- HTTP redirect : `http://localhost:8080` → 8443

### 2. Agent

```bash
cd symbion-agent-host
cargo run --release

# Setup wizard interactif première fois
# Configuration : ~/.config/symbion-agent/config.toml
```

### 3. PWA Dashboard

```bash
cd pwa-dashboard
npm install
npm run dev

# Accès : http://localhost:3000
```

---

## Configuration

### Kernel

Fichier : `.env` à la racine de `symbion-kernel/`

```bash
SYMBION_API_KEY=s3cr3t-42
SYMBION_MQTT_BROKER=127.0.0.1:1883
SYMBION_JWT_SECRET=<128 hex chars>
```

### Agent

Fichier : `~/.config/symbion-agent/config.toml`

```toml
[mqtt]
broker_host = "localhost"
broker_port = 1883
client_id = "symbion-agent-<hostname>" # optionnel

[update]
auto_update = false
```

### MQTT Broker (Mosquitto)

Fichier : `/etc/mosquitto/mosquitto.conf`

```
listener 1883 127.0.0.1
allow_anonymous true

# WSS (optionnel pour PWA)
listener 9001
protocol websockets
```

---

## État du Projet

### Progression Globale : 67% (41/61 tâches)

| Phase | Status | P1 Core | Total | Completion |
|-------|--------|---------|-------|------------|
| PR1 - Context Engine | 🟢 Done | 5/5 | 5/7 | 100% P1 |
| PR2 - Security Hardening | 🟢 Done | 13/13 | 13/13 | 100% |
| PR3 - Decision Engine | 🟢 Done | 7/7 | 7/9 | 100% P1 |
| PR4 - Metrics & Observability | 🟢 Done | 7/7 | 7/10 | 100% P1 |
| PR5 - Kernel Reliability | 🟢 Done | 7/7 | 7/11 | 100% P1 |
| PR6 - Production Readiness | 🟡 In Progress | 2/2 | 2/11 | 18% (CSP only) |

**P1 Core Features** : 100% Complete ✅
**Production-Ready** : PR1-PR5 deployment-ready
**Target v1.0.0** : Q2 2026

### Prochaines Étapes (PR6 - Deferred Q1 2026)

- Let's Encrypt integration (automatic cert renewal)
- PostgreSQL migration (replace JSON files)
- Docker containerization
- CI/CD pipeline (GitHub Actions)

### Métriques Qualité

- **Tests** : 131 total (109 kernel + 14 agent + 8 devkit)
- **Sécurité** : 0 vulnérabilités critiques (audit 12 Nov 2025)
- **Code** : 36 modules Rust kernel, 11,297 lignes JS PWA
- **Performance** : Kernel 23.6 MB RAM, heartbeat <100ms

---

## Documentation

- **[CLAUDE.md](CLAUDE.md)** : Vision système et workflow documentation
- **[docs/ROADMAP.md](docs/ROADMAP.md)** : Feuille de route détaillée
- **[docs/CHANGELOG.md](docs/CHANGELOG.md)** : Historique changements
- **[docs/architecture/SYSTEM_OVERVIEW.md](docs/architecture/SYSTEM_OVERVIEW.md)** : Architecture complète
- **[docs/api/endpoints.md](docs/api/endpoints.md)** : Référence API (107 routes)
- **[docs/mqtt/topics.md](docs/mqtt/topics.md)** : Référence MQTT (15 topics)
- **[docs/QUICK_REFERENCE.md](docs/QUICK_REFERENCE.md)** : Cheat sheet commandes
- **[docs/TROUBLESHOOTING.md](docs/TROUBLESHOOTING.md)** : Diagnostic problèmes
- **[docs/DEPLOYMENT.md](docs/DEPLOYMENT.md)** : Guide déploiement
- **[docs/PERFORMANCE.md](docs/PERFORMANCE.md)** : Benchmarks performance

---

## Technologies

### Backend
- **Rust** : Langage système (Kernel + Agents)
- **Axum** : Framework HTTP async
- **Tokio** : Runtime async
- **rumqttc** : Client MQTT
- **serde** : Sérialisation JSON
- **bcrypt** : Password hashing
- **jsonwebtoken** : JWT HS256
- **totp-rs** : TOTP MFA

### Frontend
- **Lit** : Web Components
- **Vite** : Build tool
- **JavaScript** : Vanilla JS (no framework)

### Infrastructure
- **Mosquitto** : MQTT broker
- **Systemd** : Service management
- **mkcert** : TLS certificates (dev)
- **Let's Encrypt** : TLS certificates (production, planned)

---

## API Quick Reference

### Authentication
- `POST /login` : JWT acquisition (username + password)
- `POST /logout` : Session termination
- `GET /ca-certificate` : Download CA cert

### Context
- `GET /context/mode` : Mode actuel
- `POST /context/override` : Force mode
- `GET /context/stats` : Statistiques modes

### Decision
- `GET /v1/decision/validations/pending` : Validations en attente
- `POST /v1/decision/validation/:id/resolve` : Approve/reject
- `GET /v1/decision/stats` : Statistiques décisions

### Agents
- `GET /agents` : Liste agents
- `POST /agents/:id/command` : Envoyer commande
- `GET /v1/metrics/agents` : Métriques agents

### Metrics
- `GET /metrics` : Prometheus format (22 metrics)
- `GET /v1/metrics/system` : Kernel overview JSON
- `GET /health` : Liveness check

### Notes
- `POST /ports/memo` : Create note
- `GET /ports/memo` : List notes (streaming)
- `PUT /ports/memo/:id` : Update note
- `DELETE /ports/memo/:id` : Delete note

---

## MQTT Topics

### Agent Lifecycle
- `symbion/agents/registration@v1` : Agents → Kernel (startup)
- `symbion/agents/heartbeat@v1` : Agents → Kernel (30s)
- `symbion/agents/response@v1` : Agents → Kernel (command result)

### Agent Control
- `symbion/agents/command@v1` : Kernel → Agents (remote commands)

### Plugin Communication
- `symbion/notes/command@v1` : Kernel → Plugin (CRUD requests)
- `symbion/notes/response@v1` : Plugin → Kernel (streaming response)

### Dashboard Updates
- `symbion/dashboard/context@v1` : Mode changes
- `symbion/dashboard/agents@v1` : Agent status
- `symbion/dashboard/health@v1` : System health
- `symbion/dashboard/notes@v1` : Note events
- `symbion/dashboard/stats@v1` : Statistics
- `symbion/dashboard/pattern@v1` : Pattern detection

### System Events
- `symbion/kernel/health@v1` : Kernel health (5 min broadcast)

---

## Développement

### Tests

```bash
# Kernel tests
cd symbion-kernel
cargo test

# Agent tests
cd symbion-agent-host
cargo test

# PWA (no tests currently)
```

### Build Release

```bash
# Kernel
cargo build --release -p symbion-kernel

# Agent
cargo build --release -p symbion-agent-host

# PWA
cd pwa-dashboard && npm run build
```

### Slash Commands (Claude Code)

- `/docs [terme]` : Recherche documentation
- `/status` : Briefing état projet
- `/audit` : Audit complet avec 6 agents parallèles + email
- `/sync-roadmap` : Synchronise ROADMAP.md

### Monitoring

```bash
# Logs kernel
tail -f /tmp/kernel.log

# Logs systemd
journalctl -u symbion-kernel -f

# Monitoring automatique (cron 15 min)
./scripts/monitor-symbion.sh
```

---

## Contribuer

Le projet est actuellement en développement actif avec un seul développeur (bus factor = 1).

Pour contribuer :
1. Fork le repo
2. Créer une branche feature (`git checkout -b feature/ma-feature`)
3. Commit avec conventional commits (`feat:`, `fix:`, `docs:`)
4. Push et créer une Pull Request

---

## Licence

MIT License - Voir [LICENSE](LICENSE) pour détails.

---

## Contact

**Maintainer** : Mark
**Email** : markchavatte@gmail.com
**Version** : v0.3.0-alpha.3
**Last Updated** : 16 Novembre 2025
