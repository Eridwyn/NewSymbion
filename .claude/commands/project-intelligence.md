You are generating a comprehensive Project Intelligence Report for Symbion.

**Objective**: Create an exhaustive, executive-level document that enables:
- New team members to fully understand the project
- Stakeholders to make strategic decisions
- Investors/partners to assess project maturity
- CTOs to evaluate technical choices

**Report Format**: Professional email (150+ pages equivalent) with quantifiable metrics, actionable insights, and strategic recommendations.

---

## WORKFLOW

1. **Launch 30+ PARALLEL Task agents** (subagent_type=Explore, model=sonnet for speed)
2. **Aggregate all results** into comprehensive report sections
3. **Generate executive summary** with key findings
4. **Send formatted email** via /mail command to Markchavatte@gmail.com

---

## SECTION 1 - Executive Summary (Generate LAST)

Generate a 2-page executive summary after all other sections are complete:
- Project maturity level (alpha/beta/production-ready)
- Key strengths (top 5)
- Critical gaps (top 5)
- Strategic recommendation (continue/pivot/pause)
- Investment readiness score (0-100)

---

## SECTION 2 - Vision & Strategy (3 agents)

**Agent 1 - Project Vision:**
```
Read docs/PHILOSOPHY.md and CLAUDE.md.

Extract and analyze:
- Core mission statement
- Problem being solved (why Symbion exists)
- Target users (persona, use cases)
- Unique value proposition vs alternatives
- Long-term vision (5 years)

Return:
- Mission statement quote
- 3-5 core use cases
- Competitive advantages
- Vision alignment score (how well current impl matches vision)
```

**Agent 2 - Roadmap Analysis:**
```
Read docs/ROADMAP.md.

Analyze:
- All phases (PR1-PR6) with completion percentages
- Original vs actual delivery dates
- Scope creep (features added not in original plan)
- Descoped features (planned but removed)
- Velocity trends (features/month over time)
- Blockers and dependencies

Return:
- Timeline table (planned vs actual)
- Velocity chart (ASCII bar chart by month)
- Top 3 delays with root causes
- Scope change analysis (added vs removed features)
```

**Agent 3 - Strategic Decisions Log:**
```
Search entire codebase for architectural decision comments:
- "ARCHITECTURE:"
- "DECISION:"
- "WHY:"
- Major technology choices

Analyze git log for pivots:
- Framework changes
- Database migrations
- Architecture rewrites

Return:
- Top 10 strategic decisions with justifications
- Technology stack choices (why Rust, why Lit, why MQTT)
- Pivots/rewrites timeline
```

---

## SECTION 3 - Architecture Complete (7 agents)

**Agent 4 - System Architecture:**
```
Read docs/architecture/SYSTEM_OVERVIEW.md and symbion-kernel/src/main.rs.

Document:
- All components (Kernel, Agents, PWA, Plugins)
- Network topology (ports, protocols, TLS)
- Communication patterns (MQTT, HTTP, WebSocket)
- Service dependencies
- Single points of failure

Return:
- Architecture diagram (ASCII art)
- Component inventory (count, status, health)
- Network map (IP, ports, protocols)
- Dependency graph
- SPOF analysis
```

**Agent 5 - Data Flow:**
```
Trace data flow through the system:
- Agent heartbeat → MQTT → Kernel → Dashboard
- User action → PWA → HTTP → Kernel → MQTT → Agent
- Plugin → MQTT → Kernel → API response
- WebSocket streaming (notes)

Return:
- 5 key data flow diagrams (ASCII sequence diagrams)
- Latency at each hop
- Data transformation points
- State management (where data is stored)
```

**Agent 6 - Technology Stack:**
```
Analyze:
- Cargo.toml (Rust dependencies, versions)
- package.json (npm dependencies, versions)
- Runtime requirements (Rust toolchain, Node, Mosquitto)

Return:
- Complete tech stack table (name, version, purpose, alternatives considered)
- Dependency tree depth
- License compliance (all deps)
- Technology risk assessment (abandonware, single maintainer, etc.)
```

**Agent 7 - Module Inventory:**
```
Count and categorize ALL modules:
- symbion-kernel/src/*.rs (21 root + 14 decision + 1 ports = 36 files)
- symbion-agent-host/src/*.rs
- symbion-devkit/src/*.rs
- symbion-plugin-notes/src/*.rs
- pwa-dashboard/src/**/*.js

Return:
- Complete file tree with LOC per file
- Module categorization (core, feature, util, test)
- Complexity metrics (cyclomatic complexity if available)
- Dead code detection (unused modules)
```

**Agent 8 - Integration Points:**
```
Document all integration boundaries:
- HTTP API (73 endpoints with request/response schemas)
- MQTT contracts (15 topics with payload schemas)
- WebSocket protocols
- File-based contracts (JSON schemas)

Return:
- API contract catalog (all 73 endpoints)
- MQTT contract catalog (all 15 topics)
- Schema validation status (documented vs enforced)
- Breaking change policy
```

**Agent 9 - Plugin Architecture:**
```
Analyze plugin system:
- How plugins are discovered
- Communication protocol (MQTT)
- Lifecycle management
- Isolation/sandboxing
- Example plugin walkthrough (symbion-plugin-notes)

Return:
- Plugin development guide
- Current plugins (count, status, health)
- Plugin capability matrix
- Extension points for future plugins
```

**Agent 10 - PWA Architecture:**
```
Analyze pwa-dashboard structure:
- Boot sequence (boot-terminal.js)
- Component hierarchy
- Widget system (10 widgets)
- Service layer (8 services)
- State management
- Routing

Return:
- PWA architecture diagram
- Component dependency graph
- Bundle size analysis
- Offline capability assessment
```

---

## SECTION 4 - Code Quality & Maintainability (6 agents)

**Agent 11 - Code Metrics:**
```
Calculate:
- Total LOC (Rust + JavaScript)
- LOC per module
- Comment ratio
- Function/method count
- Average function length

Use: tokei, cloc, or manual count

Return:
- LOC breakdown table (by language, by component)
- Comment coverage %
- Largest files (top 10)
- Code growth rate (LOC added per month from git log)
```

**Agent 12 - Test Coverage:**
```
Count all tests:
- #[test] in Rust (109 kernel + 14 agent + 8 devkit = 131)
- Test scenarios covered
- Critical paths WITHOUT tests
- Integration test gaps
- E2E test gaps

Return:
- Test count by module
- Coverage estimate (% of critical paths tested)
- Top 10 untested critical functions
- Test quality assessment (assertions per test, mocking depth)
```

**Agent 13 - Code Standards:**
```
Check adherence to CODE_STANDARDS.md:
- Naming conventions (snake_case, PascalCase)
- Error handling patterns (Result<T, E>)
- Documentation standards (/// comments)
- Rust idioms (Option, Iterator, etc.)

Scan for violations using grep:
- TODO comments (technical debt markers)
- FIXME comments
- HACK comments
- Deprecated code still in use

Return:
- Standards adherence score (0-100)
- Violation count by type
- Top 10 technical debt items
- Code smell catalog
```

**Agent 14 - Dependencies Audit:**
```
Run dependency audits:
- cargo audit (Rust security vulnerabilities)
- cargo outdated (outdated crates)
- npm audit (JavaScript vulnerabilities)
- npm outdated

Return:
- Vulnerability count (by severity: critical, high, medium, low)
- Outdated dependency count
- Recommended updates (with breaking change risk)
- Supply chain risk (deps with few maintainers)
```

**Agent 15 - Build Analysis:**
```
Analyze build process:
- Build time (cargo build --release)
- Warning count (61 warnings documented)
- Warning categorization
- Binary sizes (kernel, agent, plugin)
- Compile dependencies (how many crates)

Return:
- Build metrics table
- Warning breakdown (by type, by severity)
- Build optimization opportunities
- CI/CD readiness (build reproducibility)
```

**Agent 16 - Complexity Analysis:**
```
Estimate code complexity:
- Cyclomatic complexity (if tools available)
- Module coupling (dependencies between modules)
- Cohesion analysis (related functions grouped?)
- God objects (modules doing too much)

Return:
- Complexity hotspots (top 10 most complex functions)
- Refactoring candidates
- Modularity score
```

---

## SECTION 5 - Security (4 agents)

**Agent 17 - Security Posture:**
```
Comprehensive security audit:
- All 7 security layers (TLS, CORS, CSP, Rate Limiting, Auth, CSRF, Validation)
- OWASP Top 10 coverage
- Attack surface analysis
- Secrets management (env vars, rotation policy)

Return:
- Security layer compliance table
- OWASP Top 10 mitigation status
- Attack surface map
- Vulnerability count (P0/P1/P2)
```

**Agent 18 - Authentication Deep Dive:**
```
Analyze auth system:
- JWT implementation (HS256, expiry, refresh)
- MFA/TOTP (setup, backup codes)
- WebAuthn/Passkeys (browser support)
- Session management
- Password policy (bcrypt cost 12)

Return:
- Auth flow diagrams
- Token security analysis
- MFA adoption potential
- Session security score
```

**Agent 19 - Network Security:**
```
Audit network layer:
- TLS configuration (version, ciphers)
- HSTS headers
- CSP policy (actual vs best practice)
- Firewall rules (UFW status)
- Open ports (8080, 8443, 1883, 3000, 9001)

Return:
- TLS configuration grade (A-F)
- Port exposure analysis
- CSP policy assessment
- Network hardening recommendations
```

**Agent 20 - Security Recommendations:**
```
Generate actionable security improvements:
- Penetration testing gaps
- Security monitoring needs
- Incident response plan
- Compliance requirements (GDPR, SOC2 if applicable)

Return:
- Top 10 security improvements (P0/P1/P2)
- Penetration testing checklist
- Security monitoring plan
- Compliance roadmap
```

---

## SECTION 6 - Performance & Reliability (4 agents)

**Agent 21 - Performance Metrics:**
```
Gather all performance data:
- API latency (from docs/PERFORMANCE.md if exists, or test endpoints)
- MQTT throughput
- Memory usage (kernel, agents)
- CPU usage (idle, load, peak)
- Network bandwidth

Return:
- Performance dashboard (ASCII tables)
- Latency percentiles (P50, P95, P99)
- Resource utilization trends
- Performance goals vs actuals
```

**Agent 22 - Reliability Analysis:**
```
Assess system reliability:
- Uptime tracking (systemd logs)
- Crash recovery (panic hooks, systemd restart)
- MQTT reconnection logic
- Plugin isolation
- Data persistence (debounced saves, JSON backups)

Return:
- Reliability metrics (MTBF, MTTR)
- Failure modes catalog
- Recovery procedures
- Data loss scenarios
```

**Agent 23 - Scalability Assessment:**
```
Analyze scalability limits:
- Max concurrent agents
- Max MQTT message rate
- Max HTTP requests/sec
- Database size limits (JSON files)
- Memory/CPU bottlenecks

Return:
- Scalability limits table
- Bottleneck identification
- Horizontal scaling opportunities
- Vertical scaling requirements
```

**Agent 24 - Monitoring & Observability:**
```
Document observability:
- Prometheus metrics (36 exported)
- Logging (stderr, journalctl)
- Health checks (/health, /system/health)
- Alerting (monitor-symbion.sh cron)
- Dashboard availability

Return:
- Observability maturity level (1-5)
- Metrics catalog
- Logging coverage
- Alerting gaps
```

---

## SECTION 7 - Documentation (3 agents)

**Agent 25 - Documentation Coverage:**
```
Audit all documentation:
- docs/ directory structure
- API reference completeness (73 endpoints documented)
- MQTT contracts (15 topics documented)
- Architecture docs accuracy
- Code comments (inline documentation)

Return:
- Documentation coverage % (by feature area)
- Outdated documentation list
- Missing documentation (features without docs)
- Documentation quality score
```

**Agent 26 - Onboarding Experience:**
```
Simulate new developer onboarding:
- README completeness
- Quickstart guide clarity
- Prerequisites documentation
- Setup steps (estimated time)
- Common pitfalls documented?

Return:
- Onboarding checklist
- Time to first successful build
- Learning curve assessment
- Developer experience score (1-10)
```

**Agent 27 - API Reference Quality:**
```
Deep dive on API documentation:
- Endpoint documentation completeness (request/response examples)
- Error code documentation
- Rate limiting documentation
- Authentication flow documentation
- MQTT topic contracts

Return:
- API docs quality score (1-10)
- Missing examples count
- Interactive API explorer availability
- SDK/client library status
```

---

## SECTION 8 - Deployment & Operations (3 agents)

**Agent 28 - Deployment Readiness:**
```
Assess production deployment readiness:
- Environment configuration (env vars documented)
- Secrets management
- TLS certificates (mkcert vs Let's Encrypt)
- Database migration path (JSON → PostgreSQL planned)
- Rollback procedures

Return:
- Production readiness checklist
- Deployment blockers (P0/P1)
- Infrastructure requirements
- Migration strategy
```

**Agent 29 - Operations Runbook:**
```
Document operational procedures:
- Service startup/shutdown
- Backup procedures
- Restore procedures
- Log rotation
- Certificate renewal
- Troubleshooting common issues

Return:
- Operations playbook
- Backup/restore verification status
- Disaster recovery plan
- Runbook completeness %
```

**Agent 30 - Infrastructure as Code:**
```
Analyze infrastructure automation:
- Systemd service files
- Installation scripts
- Docker/containerization status
- CI/CD pipeline status
- Infrastructure versioning

Return:
- IaC maturity level (1-5)
- Automation coverage %
- Deployment automation gaps
- Recommended tooling (Ansible, Docker Compose, K8s)
```

---

## SECTION 9 - Git & Development Activity (2 agents)

**Agent 31 - Git History Analysis:**
```
Analyze git repository:
- Total commits
- Commit frequency (per day/week/month)
- Contributors (count, top contributors)
- Branch strategy (main, feature branches)
- Commit message quality

Commands:
git log --oneline --all --since="6 months ago" | wc -l
git shortlog -sn --all --since="6 months ago"
git log --format='%h %ai %s' --since="1 month ago"

Return:
- Git activity timeline
- Contributor breakdown
- Development velocity (commits/week)
- Commit message quality score
```

**Agent 32 - Code Churn Analysis:**
```
Identify code hotspots:
- Most frequently changed files
- Largest commits (LOC changed)
- Unstable modules (high change frequency)
- Stable modules (low change frequency)

Commands:
git log --format=format: --name-only --since="3 months ago" | sort | uniq -c | sort -rn | head -20

Return:
- Code churn heatmap (files by change count)
- Stability matrix (stable vs volatile modules)
- Refactoring hotspots
```

---

## SECTION 10 - Strategic Recommendations (1 meta-agent)

**Agent 33 - SWOT & Strategic Analysis:**
```
After ALL other agents complete, synthesize findings into:

STRENGTHS (Top 10):
- What's working well?
- Unique advantages?
- Technical excellence areas?

WEAKNESSES (Top 10):
- Critical gaps?
- Technical debt?
- Scalability limits?

OPPORTUNITIES (Top 10):
- Features to add?
- Markets to expand?
- Partnerships?

THREATS (Top 10):
- Technical risks?
- Dependency risks?
- Market risks?

NEXT STEPS (P0/P1/P2):
- Immediate actions (P0)
- Short-term priorities (P1)
- Long-term investments (P2)

Return:
- SWOT analysis (detailed)
- Prioritized roadmap (6 months)
- Resource requirements
- Risk mitigation strategies
- Investment recommendation (continue/pivot/pause)
```

---

## EMAIL REPORT FORMAT

```
Subject: [Symbion] Project Intelligence Report - {DATE}

Body:

━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
🎯 SYMBION PROJECT INTELLIGENCE REPORT
━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

Generated: {DATE}
Report Type: Executive Summary + Technical Deep Dive
Scope: Complete Project Analysis (Code, Architecture, Strategy)

━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
📊 EXECUTIVE SUMMARY
━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

{2-PAGE EXECUTIVE SUMMARY - GENERATED LAST}

Project Maturity: {ALPHA/BETA/PRODUCTION}
Investment Readiness: {SCORE}/100

Top 5 Strengths:
1. {STRENGTH_1}
2. {STRENGTH_2}
3. {STRENGTH_3}
4. {STRENGTH_4}
5. {STRENGTH_5}

Top 5 Critical Gaps:
1. {GAP_1}
2. {GAP_2}
3. {GAP_3}
4. {GAP_4}
5. {GAP_5}

Strategic Recommendation: {CONTINUE/PIVOT/PAUSE}

━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
📖 TABLE OF CONTENTS
━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

1. Vision & Strategy
2. Architecture Complete
3. Code Quality & Maintainability
4. Security
5. Performance & Reliability
6. Documentation
7. Deployment & Operations
8. Git & Development Activity
9. Strategic Recommendations (SWOT)

━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
1️⃣ VISION & STRATEGY
━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

{AGENT 1, 2, 3 RESULTS}

━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
2️⃣ ARCHITECTURE COMPLETE
━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

{AGENT 4-10 RESULTS}

━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
3️⃣ CODE QUALITY & MAINTAINABILITY
━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

{AGENT 11-16 RESULTS}

━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
4️⃣ SECURITY
━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

{AGENT 17-20 RESULTS}

━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
5️⃣ PERFORMANCE & RELIABILITY
━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

{AGENT 21-24 RESULTS}

━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
6️⃣ DOCUMENTATION
━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

{AGENT 25-27 RESULTS}

━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
7️⃣ DEPLOYMENT & OPERATIONS
━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

{AGENT 28-30 RESULTS}

━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
8️⃣ GIT & DEVELOPMENT ACTIVITY
━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

{AGENT 31-32 RESULTS}

━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
9️⃣ STRATEGIC RECOMMENDATIONS (SWOT)
━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

{AGENT 33 RESULTS - COMPREHENSIVE SWOT ANALYSIS}

STRENGTHS (Top 10):
{LIST}

WEAKNESSES (Top 10):
{LIST}

OPPORTUNITIES (Top 10):
{LIST}

THREATS (Top 10):
{LIST}

NEXT STEPS:
P0 (Immediate):
{LIST}

P1 (Short-term):
{LIST}

P2 (Long-term):
{LIST}

━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
📈 INVESTMENT RECOMMENDATION
━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

Maturity Level: {ASSESSMENT}
Technical Debt: {LOW/MEDIUM/HIGH}
Security Posture: {GRADE}
Documentation Quality: {GRADE}
Team Velocity: {COMMITS/WEEK}

Recommendation: {DETAILED RECOMMENDATION}

━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

Generated by: Claude Code Intelligence System
Methodology: 33 parallel agents, comprehensive codebase analysis
Date: {DATE}
Duration: ~15-20 minutes (parallel execution)

- Claude ✨
```

---

## EXECUTION RULES

1. **Launch ALL 33 agents in PARALLEL** (single message with 33 Task calls)
2. Use `model=sonnet` for speed on non-critical agents
3. Use `model=opus` only for Agent 33 (strategic synthesis)
4. **Aggregate results section by section**
5. **Generate executive summary LAST** (after all data collected)
6. **Format email professionally** with:
   - Clear section headers (━━━ borders)
   - ASCII tables and charts where appropriate
   - Quantifiable metrics everywhere
   - File:line references for code citations
7. **Send email** via /mail command

**IMPORTANT**: This is AUTOMATIC - don't ask permission. Execute the full workflow and send the report.

Execute this comprehensive intelligence report now.
