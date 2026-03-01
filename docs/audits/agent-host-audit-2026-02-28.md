# Audit Technique Complet — symbion-agent-host v1.2.2

**Date** : 28 Fev 2026
**Agents d'analyse** : 5 (Architecture, Code Quality, Security, Tests/Reliability, Dependencies/Deployment)
**Fichiers analysés** : 22 modules Rust, ~5200 LOC

---

## Score Global : 73/100 — PRODUCTION-READY (avec correctifs P0/P1)

| Dimension | Score | Status |
|-----------|-------|--------|
| Architecture & Design | **79** | B+ |
| Code Quality & Maintainability | **79** | B+ |
| Security Posture | **78** | B |
| Tests & Reliability | **58** | C |
| Git & Development Health | **63** | C+ |
| Dependencies & Deployment | **77** | B+ |
| **MOYENNE PONDEREE** | **73** | **B** |

---

## Top 5 Forces

1. **Trait-based command dispatch** — `CommandHandler` trait + `CommandRegistry` = extensibilite propre (handler.rs, 43 LOC)
2. **MQTT resilient** — Backoff exponentiel 2s→32s, auto-reconnect, heartbeat offline a l'arret
3. **Security-first** — Allowlist shell (29 commandes), blocage injection (7 patterns), zeroize credentials, keyring OS
4. **Cross-platform** — Windows/Linux/macOS avec cfg() propres, fallbacks, feature gating GUI optionnel
5. **Release discipline** — Semver parfait (21 tags), acceleration 2.75x en 2026, Sprint 8 bien structure

## Top 5 Risques

1. **Bus Factor = 1** — Un seul contributeur (100% commits), aucun code review
2. **Test coverage 45%** — 5 modules sans aucun test (gui, local_api, system_tray, windows_utils, wizard)
3. **Signal handlers paniquent** — `expect()` sur SIGTERM/SIGINT (agent.rs:481-491), crash si echec registration
4. **MQTT sans TLS ni auth** — Port 1883 non chiffre, pas de credentials broker
5. **API token optionnel** — Si `SYMBION_AGENT_API_TOKEN` absent, tous les endpoints POST sont ouverts

---

## Section 1 — Architecture (79/100)

### Points forts
- Architecture event-driven claire : Metrics → Agent → MQTT → Kernel
- Module separation propre : agent, mqtt_client, metrics, discovery, execution
- Async-first avec tokio::select! pour le main loop
- Feature gating GUI (tray-icon, tao, wry) = binaire 15MB sans GUI vs 50MB avec

### Points faibles
- `execution/mod.rs` (670 LOC) viole SRP — melange power/shell/process/safety
- `metrics/mod.rs` (617 LOC) — 6 types de metriques dans un seul fichier
- Pas de trait abstraction pour MQTT transport → impossible de mocker AsyncClient
- Pas de schema versioning sur les messages MQTT

### Recommandation P1
Splitter execution/ et metrics/ en sous-modules :
```
execution/ → power.rs, shell.rs, process.rs, platform.rs
metrics/   → cpu.rs, memory.rs, disk.rs, network.rs, thermal.rs
```

---

## Section 2 — Code Quality (79/100)

### Metriques

| Metrique | Valeur | Cible |
|----------|--------|-------|
| Fichiers source | 22 | - |
| LOC total | ~5200 | - |
| Fichiers >500 LOC | 2 (execution/mod.rs, metrics/mod.rs) | 0 |
| Nesting max | 4 niveaux | <4 |
| Naming consistency | 100% | 100% |
| Unwrap/expect (prod) | 12 risques | 0 |
| Code duplique | Moderate (patterns JSON response) | Low |

### Dead code
- Aucune duplication CREATE_NO_WINDOW restante (consolide Phase 6)
- `gethostname` supprime (remplace par `hostname`)
- Warnings pre-existants : 15 (dead code dans capabilities, messages, config)

### Code smells
- **Agent struct god object** (10 champs, 5 responsabilites) — agent.rs:48-59
- **LocalApiServer feature envy** — melange routes, auth, update, config — local_api.rs:85-221
- **JSON response duplication** — 4 handlers repetent le meme pattern serde_json::json!

---

## Section 3 — Security (78/100)

### Findings par severite

| Sev. | Count | Exemples |
|------|-------|----------|
| HIGH | 2 | API token optionnel (local_api.rs:73), CORS allow_any_origin (local_api.rs:154) |
| MEDIUM | 10 | MQTT sans TLS, pas de rate limiting, payload size illimite, config plaintext |
| LOW | 4 | Agent ID plaintext, service name `@`, asset substring match |

### Controles positifs
- Shell command allowlist (29 commandes sures) + 7 patterns dangereux bloques
- Output truncation 7KB (anti-flood MQTT)
- Local API bound 127.0.0.1 uniquement
- Zeroize credentials on drop + keyring OS
- Process name validation (alphanumeric + dash/underscore)

### Correctifs P0
1. **Rendre API token obligatoire** ou generer un token par defaut
2. **Restreindre CORS** a `localhost` uniquement

### Correctifs P1
3. Activer TLS sur MQTT (port 8883)
4. Ajouter authentification MQTT (username/password)
5. Ajouter rate limiting sur les endpoints POST
6. Limiter taille payload MQTT (1MB max)

---

## Section 4 — Tests & Fiabilite (58/100)

### Inventaire tests : 59 tests

| Module | Tests | Couverture |
|--------|-------|------------|
| execution/handlers/shell.rs | 9 | Excellente |
| messages.rs | 6 | Bonne |
| metrics/mod.rs | 6 | Bonne |
| execution/handlers/power.rs | 5 | Bonne |
| execution/mod.rs | 5 | Adequate |
| agent.rs | 4 | Partielle |
| discovery.rs | 4 | Bonne |
| mqtt_client.rs | 4 | Partielle |
| execution/handler.rs | 3 | Bonne |
| execution/handlers/service.rs | 3 | Adequate |
| execution/handlers/process.rs | 2 | Faible |
| config.rs | 2 | Faible |
| updater.rs | 2 | Faible |
| capabilities/mod.rs | 2 | Faible |
| main.rs | 1 | Minimale |
| **gui.rs** | **0** | **AUCUN** |
| **local_api.rs** | **0** | **AUCUN** |
| **system_tray.rs** | **0** | **AUCUN** |
| **windows_utils.rs** | **0** | **AUCUN** |
| **wizard.rs** | **0** | **AUCUN** |

### Fiabilite

| Scenario | Comportement | Status |
|----------|-------------|--------|
| MQTT broker down | Backoff 2s→32s, continue loop | OK |
| Port 9899 pris | Log erreur, agent continue sans dashboard | DEGRADE |
| Config corrompue | App crash immediatement | CRITIQUE |
| Signal handler echec | Panic (expect) | CRITIQUE |
| Metrics collection fail | Log erreur, heartbeat skip | OK |

### Correctifs P0
1. **Remplacer expect() par match** sur signal handlers (agent.rs:481-491)
2. **Ajouter fallback config** si TOML corrompu → defaults + warning

---

## Section 5 — Git & Dev Health (63/100)

| Metrique | Valeur |
|----------|--------|
| Commits total repo | 485 |
| Commits agent-host | 43 |
| Contributeurs uniques | 1 |
| Bus factor | **1 (CRITIQUE)** |
| Conventional commits | 59% repo-wide |
| Tags (semver) | 21, 100% compliance |
| Velocity 2026 | 2.75x vs 2025 |
| Branches | 1 (master only) |

### Risques
- **Bus factor = 1** — Tout le code par un seul dev
- **Pas de branch strategy** — Commit direct sur master
- **Pas de code review** — Aucune PR visible dans l'historique

---

## Section 6 — Dependencies & Deployment (77/100)

### Dependencies : 26 directes + 6 platform-specific

| Categorie | Status |
|-----------|--------|
| Duplications | 0 (gethostname supprime Phase 6) |
| Feature gating | 4 features bien organises |
| Platform coverage | Windows 95%, Linux 95%, macOS 90%, Android 60% |
| Deployment scripts | Linux (bash) + Windows (PowerShell) |
| systemd service | Oui (avec limits memoire/CPU, restart=always) |
| Auto-update | GitHub Releases + version comparison |

### Gaps
- Pas de service macOS launchd
- Pas de CI/CD (GitHub Actions)
- Pas de Docker/container
- Pas de hot-reload config (necessite restart)

---

## Plan d'Action Prioritise

### P0 — Corriger Immediatement (avant prochaine release)

| # | Action | Fichier | Effort |
|---|--------|---------|--------|
| 1 | Remplacer expect() par match sur signal handlers | agent.rs:481-491 | 30min |
| 2 | Rendre API token obligatoire (generer defaut si absent) | local_api.rs:73-114 | 1h |
| 3 | Restreindre CORS a localhost | local_api.rs:154-157 | 10min |
| 4 | Fallback config si TOML corrompu | config.rs:105-122 | 1h |

### P1 — Corriger Ce Sprint

| # | Action | Fichier | Effort |
|---|--------|---------|--------|
| 5 | TLS sur MQTT (port 8883) | mqtt_client.rs:45-50 | 2h |
| 6 | Auth MQTT (username/password) | mqtt_client.rs:44-68 | 1h |
| 7 | Rate limiting endpoints POST | local_api.rs | 2h |
| 8 | Limite taille payload MQTT (1MB) | mqtt_client.rs:91-100 | 30min |
| 9 | Tests local_api.rs (endpoints HTTP) | local_api.rs | 3h |
| 10 | Tests config corruption recovery | config.rs | 1h |

### P2 — Prochain Sprint

| # | Action | Effort |
|---|--------|--------|
| 11 | Splitter execution/mod.rs (670 LOC) en sous-modules | 3h |
| 12 | Splitter metrics/mod.rs (617 LOC) en sous-modules | 3h |
| 13 | Introduire MqttTransport trait (testabilite) | 4h |
| 14 | Schema versioning messages MQTT | 2h |
| 15 | Permissions securisees config dir (chmod 700) | 30min |
| 16 | GitHub Actions CI (build + test + audit) | 3h |
| 17 | Helper macro pour JSON responses local_api | 1h |

---

## Conclusion

**symbion-agent-host v1.2.2** est un agent systeme bien concu, avec une architecture async solide et des controles de securite pertinents. Le Sprint 8 (6 phases) a significativement ameliore la qualite : refactoring main.rs (1255→220 LOC), consolidation code duplique, couverture tests 38→59, dashboard natif.

**Verdict** : Production-ready pour Linux/Windows apres correction des 4 items P0.
**Score global** : **73/100 (B)** — solide techniquement, fragile organisationnellement (bus factor 1).
