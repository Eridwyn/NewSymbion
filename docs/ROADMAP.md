# Symbion Roadmap - Development Plan

**Last Updated**: 15 November 2025
**Current Version**: v0.3.0-alpha.1
**Target Stable**: v1.0.0 (Q2 2026)

---

## 📊 Global Progress

| Phase | Status | Completion | Target Date |
|-------|--------|------------|-------------|
| **PR1** - Context Engine v2 | 🟢 Production Ready | 100% | ✅ Done |
| **PR2** - Security Hardening | 🟢 Production Ready | 100% | ✅ Done |
| **PR3** - Decision Engine | 🟢 Production Ready | 100% | ✅ Done |
| **PR4** - Metrics & Observability | 🟡 Infra Ready | 75% | Dec 2025 |
| **PR5** - Kernel Reliability | 🔴 In Progress | 30% | Jan 2026 |
| **PR6** - Production Readiness | ⚪ Not Started | 5% | Feb 2026 |

**Overall Progress**: 77% (462/600 estimated tasks)

---

## 🎯 PR1 - Context Engine v2 (v0.2.0-alpha.1)

**Status**: 🟢 **100% Complete** - Production Ready

### Objectives

Improve context detection accuracy and eliminate timezone/hysteresis bugs.

### Completed Tasks ✅

- [x] **IANA timezone support** - Europe/Zurich with DST handling
  - Library: `time-tz` crate integrated
  - File: `symbion-kernel/Cargo.toml:33`
  - Implementation: `symbion-kernel/src/context.rs:107-148`

- [x] **Monotonic Instant for hysteresis** - Prevent clock drift bugs
  - Using Rust `Instant` instead of SystemTime
  - Implementation: `symbion-kernel/src/context.rs:192-235`
  - Threshold: 120 seconds (2 minutes)

- [x] **Week-end detection** - Automatic Intime mode Saturdays/Sundays
  - Logic: `context.rs:120-127`

- [x] **Night mode (23h-7h)** - Force Neutre mode during sleep hours
  - Logic: `context.rs:128-135`

- [x] **Manual override** - User can force mode for X hours
  - API: `POST /context/override` with duration
  - Expiration: Automatic return to auto-detection after duration

### Future Enhancements (Post-v1.0) 🔮

- [ ] **SSID-based detection** - Switch mode based on WiFi network (home vs office)
  - Requires: System-level WiFi API integration (platform-specific)
  - Use case: Multiple physical locations (office vs home)
  - Priority: P2 (not needed for current single-location setup)

- [ ] **Geolocation fallback** - Use GPS if SSID unavailable (mobile agents)
  - Use case: Mobile agents on smartphones
  - Priority: P3 (future mobile app development)

### Testing

- ✅ Timezone DST transitions tested manually (October 2025)
- ✅ Hysteresis prevents flapping (tested with rapid mode changes)
- ✅ Override expiration works correctly
- ✅ Week-end/night mode switching verified
- ✅ Manual override duration tested

### Documentation

- ✅ `CLAUDE.md` updated with context engine details
- ✅ API documentation: `docs/api/context.md`

---

## 🔐 PR2 - Security Hardening (v0.2.0-alpha.2)

**Status**: 🟢 **100% Complete** - Production-ready (backend + frontend)

### Objectives

Remediate 4 CRITICAL vulnerabilities from security audit (2025-11-12).

### Backend Completed ✅

#### Authentication
- [x] **JWT authentication** - HS256 signing with 8h expiry
  - File: `symbion-kernel/src/auth.rs`
  - Secret: 128 hex chars (64 bytes) in `.env`
  - Rate limiting: 5 attempts / 15 min per username

- [x] **Bcrypt cost 12** - Hardened from 10 to 12 (VULN-005)
  - File: `symbion-kernel/src/auth.rs:92-110`
  - Post-audit fix: 14 November 2025

- [x] **MFA/TOTP support** - RFC 6238 with QR code generation
  - File: `symbion-kernel/src/mfa.rs` (327 lines)
  - Backup codes: 5 single-use codes generated
  - Status: Feature complete, validation pending

#### CSRF Protection
- [x] **Single-use nonces** - 5 min TTL, invalidated after use
  - File: `symbion-kernel/src/csrf.rs` (287 lines)
  - Middleware: `http.rs:117-157`
  - Endpoints: All POST/PUT/DELETE under `/v1/*`

#### Rate Limiting
- [x] **auth.rs rate limiting** - Username-based (not IP)
  - Fix: tower_governor removed (VULN-009) due to IP extraction failures
  - Implementation: `auth.rs:145-171`
  - Scope: Login + MFA endpoints only
  - ⚠️ No global rate limiting (DoS risk on API endpoints)

#### TLS/HTTPS
- [x] **TLS 1.3 encryption** - Port 8443 with mkcert certificates
  - Cert path: `/etc/mosquitto/certs/cert-mkcert.pem`
  - Key permissions: 600 (VULN-001 fix)
  - CA download: `GET /ca-certificate` endpoint

- [x] **HTTP→HTTPS automatic redirect** - Port 8080 redirects to 8443
  - Implementation: `symbion-kernel/src/http.rs:352-375`
  - Status code: 301 Moved Permanently
  - Deployed: 14 November 2025

- [x] **HSTS headers** - Force HTTPS in browsers (VULN-007)
  - Header: `Strict-Transport-Security: max-age=31536000; includeSubDomains`
  - Implementation: `symbion-kernel/src/http.rs:336-348`
  - Max age: 1 year (31536000 seconds)
  - Deployed: 14 November 2025

#### Secrets Management
- [x] **Secrets rotation** - 90-day cycle implemented
  - Last rotation: 14 November 2025
  - Next rotation: 12 February 2026
  - Procedure: `docs/security/procedures/SECRETS_ROTATION_PROCEDURE.md`

### Frontend Completed ✅

- [x] **Login page** - Integrated in boot sequence (`boot-terminal.js` 1665 lines)
  - JWT token acquisition with username + password
  - TOTP step if MFA enabled
  - WebAuthn/biometric support (36 references)
  - Autofill detection (Bitwarden compatible)
  - Multi-phase boot: booting → login → authenticating → done

- [x] **MFA setup wizard** - Complete in `user-settings-page.js` (1418 lines)
  - QR code display (200x200px SVG)
  - TOTP code verification
  - Backup codes generation
  - Enable/disable toggle
  - Status monitoring

- [x] **CSRF service** - Full auto-management (`csrf-service.js` 214 lines)
  - Auto-refresh before expiration (TTL 5 min)
  - `fetchWithCsrf()` wrapper
  - Events: csrf:fetched, csrf:expired, csrf:error
  - Integrated in decision-service and destructive operations

- [x] **User settings page** - Complete authentication management
  - Password change
  - MFA configuration
  - Session info
  - Security settings

### Testing

- ✅ JWT login/logout tested via curl
- ✅ CSRF nonce validation tested (one-time use verified)
- ✅ Rate limiting tested (5+ failed logins blocked)
- ✅ MFA TOTP verified with Google Authenticator
- ⚠️ No automated integration tests yet

### Documentation

- ✅ Security audit report: `docs/security/audits/SECURITY_AUDIT_2025-11-12.md`
- ✅ Phase 2 tracker: `docs/security/archive/SECURITY_HARDENING_PHASE2.md`
- ✅ Implementation guides: `docs/security/implementation/`
- ✅ API security architecture: `docs/api/security.md`

---

## ✨ Post-Phase 2 Improvements (November 14-15, 2025)

**Context**: Critical UX/scalability enhancements implemented immediately after Security Hardening completion.

### 🎨 Organic Bioluminescent Loader

**Status**: 🟢 Complete (November 15, 2025)

- **Objective**: Replace mechanical SVG animations with organic CSS blob morphing for cohesive bio-aesthetic
- **Implementation**:
  - OrganicLoader Web Component (`organic-loader.js` - 243 lines)
  - Standalone CSS with 5 keyframe animations (`organic-loader.css` - 154 lines)
  - Radial gradient light propagation with blob morphing
  - Integration: Page load, Notes widget, Agents network widget
  - WebSocket streaming service for progressive note loading (`notes-stream-service.js` - 231 lines)

- **Backend WebSocket Support**:
  - New `notes_ws.rs` module (182 lines)
  - Real-time note streaming over WebSocket
  - Fallback to HTTP polling for compatibility

- **Files Added**:
  - `pwa-dashboard/src/components/organic-loader.js`
  - `pwa-dashboard/src/styles/organic-loader.css`
  - `pwa-dashboard/src/services/notes-stream-service.js`
  - `symbion-kernel/src/notes_ws.rs`

- **Commits**: `d5f50cb`, `cc91a40`, `1f45730`

### 📡 MQTT Streaming Pagination for Notes

**Status**: 🟢 Complete (November 14-15, 2025)

- **Problem**: HTTP 504 timeout on `/ports/memo` with >5 notes (MQTT 10KB limit exceeded)
- **Solution**: Stream notes one-by-one with `ListEnd` marker protocol
- **Impact**: Scales to 100+ notes with no payload size limitations

- **Implementation**:
  - Plugin emits 1 note/message + final `ListEnd` marker
  - Kernel aggregates stream into complete list
  - MQTT client buffer increased (10 → 200 messages)
  - MQTT packet size limit raised to 1MB

- **Files Modified**:
  - `symbion-plugin-notes/src/main.rs:329-368` (streaming emitter)
  - `symbion-kernel/src/notes_bridge.rs:154-241` (aggregation receiver)
  - `symbion-kernel/src/mqtt.rs:56` (buffer config)

- **Commits**: `6f4deb5`, `cea078e`, `9aa4f4f`

### 💾 AgentRegistry Debounced Persistence

**Status**: 🟢 Complete (November 14, 2025)

- **Problem**: Agent heartbeats not persisted, causing total data loss on kernel restart
- **Solution**: Dirty flag pattern with periodic save (every 5 minutes)
- **Impact**: Maximum 5 min data loss vs 100% loss before

- **Implementation**:
  - `AtomicBool` dirty flag (thread-safe)
  - Debounced I/O: marks dirty on update, saves periodically
  - No disk write on every heartbeat (performance gain)

- **Files Modified**:
  - `symbion-kernel/src/agents.rs:258-605`

- **Commit**: `cee08f9`

### Documentation

- ✅ `docs/CHANGELOG.md` updated with all 3 improvements
- ⏳ ROADMAP.md updated (this section)

---

## 🤖 PR3 - Decision Engine (v0.2.0-beta.1)

**Status**: 🟢 **100% Complete** - Production-ready (backend + frontend)

### Objectives

Implement intelligent decision-making for agent actions with trust scoring and idempotence.

### Completed Tasks ✅

#### Core Engine
- [x] **Decision engine** - Guards-first evaluation architecture
  - File: `symbion-kernel/src/decision/engine.rs`
  - Logic: Validate guards → Calculate trust → Execute/defer decision

- [x] **Trust score calculator** - Weighted factor scoring
  - File: `symbion-kernel/src/decision/trust.rs` (332 lines)
  - Factors: Context confidence (30%), telemetry freshness (25%), historical success (25%), network latency (10%), local presence (10%)

- [x] **Idempotence system** - Command deduplication via command_id
  - File: `symbion-kernel/src/decision/idempotence.rs` (264 lines)
  - Prevents duplicate execution of same command

- [x] **Impact levels** - Low/Medium/High classification
  - File: `symbion-kernel/src/decision/impact.rs`
  - Determines auto-approval vs manual validation

#### Comprehensive Testing
- [x] **109 unit tests** - Across 14 decision engine modules
  - Coverage: Engine, trust, idempotence, guards, factors, impact
  - Status: All tests passing

#### Module Structure
```
symbion-kernel/src/decision/
├── engine.rs         # Core decision logic
├── trust.rs          # Trust score calculation (332 lines)
├── idempotence.rs    # Command deduplication (264 lines)
├── guards.rs         # Pre-decision validation
├── factors/          # Trust factor modules
│   ├── context.rs
│   ├── telemetry.rs
│   ├── history.rs
│   ├── network.rs
│   └── presence.rs
├── impact.rs         # Impact level classification
└── tests/            # 109 unit tests
```

#### Frontend Complete ✅

- [x] **Approval interface UI** - User validation for Medium/High impact decisions
  - File: `pwa-dashboard/src/components/user-settings-page.js` (lines 757-1400+)
  - Tab "Decisions" avec interface complète
  - Features:
    - ⏳ Validations en attente (approve/reject buttons)
    - 📊 Métriques temps réel (total, approved, rejected, pending)
    - 📋 Validations expirées (suppression individuelle/masse)
    - 🧪 Générateur de test pour PR3
  - Service: `pwa-dashboard/src/services/decision-service.js` (280 lines)
  - Status: ✅ Fully functional, deployed

- [x] **Decision Service API client** - Complete API integration
  - GET /v1/decision/validations/pending
  - POST /v1/decision/validation/:id/resolve
  - DELETE /v1/decision/validation/:id
  - GET /v1/decision/stats, /audit, /metrics
  - CSRF protection integrated

### Future Enhancements (Post-v1.0) 🔮

- [ ] **Consent management** - Durable consent for repeated actions
  - Scope: Limited by action type, time window, conditions
  - Revocation: Via approval interface
  - Priority: P2 (nice-to-have)

- [ ] **Trust score tuning** - Adjust weights based on real-world usage
  - Current weights: Placeholder values (context 30%, etc.)
  - Requires: Historical data collection and analysis
  - Priority: P2 (optimization)

### Testing

- ✅ 109 unit tests passing
- ✅ Trust score calculation validated with mock data
- ✅ Idempotence prevents duplicate commands
- ✅ Frontend approval UI tested manually (approve/reject workflow)
- ✅ API integration tested (validations/pending, resolve, stats)
- ⚠️ No automated integration tests yet

### Documentation

- ✅ Decision engine architecture: `CLAUDE.md` (lines 299-476)
- ✅ Trust score formula documented
- ✅ Intention structure defined
- ✅ User approval workflow: UI in `user-settings-page.js`
- ✅ API endpoints: `docs/api/endpoints.md` (decision engine section)

---

## 📊 PR4 - Metrics & Observability (v0.2.1)

**Status**: 🟡 **75% Complete** - Infrastructure ready, API endpoints missing

### Objectives

Production-grade monitoring with Prometheus metrics and health checks.

### Completed Tasks ✅

- [x] **Metrics infrastructure** - prometheus-client crate integrated
  - File: `symbion-kernel/Cargo.toml`
  - Collector initialized: `AppState.metrics`

- [x] **Agent telemetry collection** - CPU, RAM, uptime via MQTT
  - Agents publish metrics every 30s heartbeat
  - Storage: In-memory agent registry

- [x] **Health check endpoint** - `GET /health` and `GET /system/health`
  - Basic kernel liveness check implemented
  - Returns: Kernel status, agent count, MQTT status

- [x] **Logging system** - Structured logs with timestamps
  - Format: `[category] message`
  - Categories: auth, security, mqtt, plugin, agent

### Missing HTTP Endpoints 🔴

- [ ] **`GET /metrics`** - Prometheus scraping endpoint
  - Format: Prometheus exposition format
  - Metrics: HTTP request count, latency, error rate, agent count, context switches
  - Priority: P0 (required for production monitoring)

- [ ] **`GET /v1/metrics/agents`** - Per-agent metrics dashboard
  - Returns: JSON with CPU/RAM/uptime per agent
  - Priority: P1

- [ ] **`GET /v1/metrics/system`** - Kernel performance metrics
  - Includes: Memory usage, MQTT message rate, plugin health
  - Priority: P1

### Prometheus Integration Pending 🔴

- [ ] **Grafana dashboards** - Pre-built monitoring dashboards
  - Dashboard templates: Kernel health, agent telemetry, security events
  - Priority: P2

- [ ] **Alerting rules** - PrometheusAlertmanager configuration
  - Alerts: Kernel down, agent offline, high error rate, auth failures
  - Delivery: Email, Slack, PagerDuty
  - Priority: P1

### Testing

- ✅ `/health` endpoint returns 200 OK
- ✅ Agent metrics collected and stored
- ⚠️ No Prometheus scraping tested yet (endpoint missing)

### Documentation

- ⚠️ Metrics documentation missing
- [ ] Prometheus setup guide needed
- [ ] Grafana dashboard screenshots needed

---

## 🛡️ PR5 - Kernel Reliability (v0.2.2)

**Status**: 🔴 **30% Complete** - In progress

### Objectives

Ensure kernel stability with panic recovery, graceful shutdown, and automatic restarts.

### Completed Tasks ✅

- [x] **Graceful shutdown on SIGTERM** - Clean plugin/agent disconnection
  - File: `symbion-kernel/src/main.rs`
  - Behavior: Stops plugins, closes MQTT, flushes logs

- [x] **MQTT reconnection** - Automatic reconnect with exponential backoff
  - Max retries: 5
  - Backoff: 2s, 4s, 8s, 16s, 32s

- [x] **Plugin isolation** - Plugins crash without killing kernel
  - Each plugin runs in separate task
  - Panic in plugin logs error, continues running

### Missing Critical Features 🔴

- [ ] **Panic recovery** - Catch panics in HTTP handlers and background tasks
  - Priority: P0 (critical for production stability)
  - Scope: All async tasks, HTTP routes, MQTT handlers

- [ ] **Systemd service file** - Auto-restart on crash
  - File: `/etc/systemd/system/symbion-kernel.service`
  - Restart: Always, delay 5s
  - Logging: journal
  - Priority: P0

- [ ] **Backup/restore** - Persist agent registry, context state, notes
  - Frequency: Every 5 minutes + on shutdown
  - Format: JSON files in `~/.symbion/state/`
  - Priority: P1

- [ ] **Health monitoring script** - External watchdog
  - Current: `scripts/monitor-symbion.sh` checks HTTP health
  - Missing: Automatic restart on failure
  - Priority: P2

### Testing

- ✅ Graceful shutdown tested (SIGTERM)
- ✅ MQTT reconnect tested (kill mosquitto, restart)
- ⚠️ Panic recovery not tested (not implemented)
- ⚠️ Systemd service not tested

### Documentation

- ⚠️ Deployment guide missing
- [ ] Systemd setup instructions needed
- [ ] Backup/restore procedure needed

---

## 🚀 PR6 - Production Readiness (v0.2.3)

**Status**: ⚪ **0% Complete** - Not started

### Objectives

Final production deployment requirements.

### Planned Tasks 📋

#### TLS Certificates
- [ ] **Let's Encrypt integration** - Automatic certificate renewal
  - Tool: certbot
  - Domain: symbion.yourdomain.com
  - Renewal: Automatic via cron (every 2 months)

- [x] **HSTS headers** - Force HTTPS, prevent downgrade attacks
  - Header: `Strict-Transport-Security: max-age=31536000; includeSubDomains`
  - ✅ Completed in PR2 (14 November 2025) - See PR2 section above

- [ ] **CSP headers** - Content Security Policy
  - Prevent XSS attacks
  - Priority: P0 (VULN-008)

#### Database Migration
- [ ] **SQLite → PostgreSQL** - Production-grade database
  - Reason: Better concurrency, ACID guarantees
  - Migration script: Convert users.json + notes to PostgreSQL
  - Priority: P1

- [ ] **Database backups** - Automated daily backups
  - Tool: pg_dump
  - Retention: 30 days
  - Storage: Offsite (S3 or similar)

#### Deployment
- [ ] **Docker containerization** - Kernel + Agents in containers
  - Dockerfile: Multi-stage build (Rust → Alpine)
  - Compose: Kernel + Mosquitto + PostgreSQL + Grafana
  - Priority: P1

- [ ] **CI/CD pipeline** - Automated testing and deployment
  - Platform: GitHub Actions
  - Tests: cargo test, integration tests, security scan
  - Deploy: On tag (v0.x.x)

#### Documentation
- [ ] **Installation guide** - Step-by-step setup
- [ ] **Architecture diagrams** - System overview
- [ ] **API reference** - OpenAPI/Swagger specification
- [ ] **Troubleshooting guide** - Common issues and fixes

### Estimated Timeline

- Start: January 2026
- Target completion: February 2026
- Launch: v1.0.0 stable

---

## 📅 Timeline Summary

| Milestone | Target Date | Status |
|-----------|-------------|--------|
| PR1 Context Engine | ✅ October 2025 | Done |
| PR2 Security (backend) | ✅ November 2025 | Done |
| PR2 Security (frontend) | December 2025 | In Progress |
| PR3 Decision Engine | ✅ November 2025 | Done |
| PR4 Metrics endpoints | December 2025 | Pending |
| PR5 Panic recovery | January 2026 | Pending |
| PR6 Production deploy | February 2026 | Not Started |
| **v1.0.0 Stable Release** | **March 2026** | **Target** |

---

## 🎯 Next Actions (Priority Order)

### Immediate (P0)
1. **PR2**: Implement PWA login page + CSRF nonce management
2. **PR4**: Add `GET /metrics` Prometheus endpoint
3. **PR5**: Implement panic recovery middleware

### Short Term (P1)
4. **PR2**: Build MFA setup wizard in PWA
5. **PR4**: Configure Prometheus alerting rules
6. **PR5**: Create systemd service file with auto-restart

### Medium Term (P2)
7. **PR1**: Add SSID-based context detection
8. **PR3**: Implement mobile approval interface for decisions
9. **PR5**: Implement backup/restore system

### Long Term (Q1 2026)
10. **PR6**: Let's Encrypt integration
11. **PR6**: PostgreSQL migration
12. **PR6**: Docker containerization

---

## 📊 Metrics & Success Criteria

### Code Quality
- ✅ 109+ unit tests (target: 200+)
- ✅ Zero CRITICAL vulnerabilities (security audit)
- ⚠️ Test coverage: Unknown (target: 80%+)

### Performance
- ✅ Kernel memory: 23.6 MB (target: <50 MB)
- ✅ Agent heartbeat latency: <100ms
- ⚠️ HTTP API response time: Not measured (target: p95 <200ms)

### Reliability
- ⚠️ Kernel uptime: Not tracked (target: 99.9%)
- ⚠️ MQTT reconnection rate: Not measured
- ⚠️ Plugin crash recovery: Not measured

### Security
- ✅ JWT authentication: Production-ready
- ✅ CSRF protection: Production-ready
- ✅ Rate limiting: Partial (auth only)
- ⚠️ TLS: Dev certificates (production requires Let's Encrypt)

---

## 🔗 Related Documentation

- **Architecture**: `CLAUDE.md` - System philosophy and design
- **Security Audit**: `docs/security/audits/SECURITY_AUDIT_2025-11-12.md`
- **API Reference**: `docs/api/` - Endpoint specifications
- **Quick Reference**: `docs/QUICK_REFERENCE.md` - Common commands

---

**Maintained by**: Mark (with Claude Code assistance)
**Contact**: markchavatte@gmail.com
**Last Updated**: 14 November 2025
