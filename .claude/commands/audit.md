You are performing a comprehensive codebase audit.

**Objective**: Audit the entire codebase in parallel for documentation drift, implementation discrepancies, and undocumented features.

**Workflow**:
1. Launch PARALLEL Task agents (subagent_type=Explore) for different audit areas:
   - Security implementation audit (docs/security/ vs actual code)
   - API endpoints audit (docs/api/endpoints.md vs HTTP routes)
   - MQTT topics audit (docs/mqtt/topics.md vs actual subscriptions)
   - ROADMAP progress audit (claimed % vs actual implementation)
   - Architecture audit (docs/architecture/ vs current system)
   - Test coverage audit (claimed test count vs actual tests)

2. Aggregate all audit results into comprehensive report

3. Identify HIGH/MEDIUM/LOW priority documentation fixes

4. Update ALL affected documentation files with accurate information

5. Commit changes with descriptive message and push

6. Send audit summary email via /mail slash command

**Rules**:
- Launch agents IN PARALLEL (single message with multiple Task calls)
- ALWAYS verify claims by reading actual code
- Update percentages based on real implementation counts
- Include file paths and line numbers in documentation
- Flag discrepancies clearly (e.g., "CLAIMED: 109 tests, ACTUAL: 93 tests")
- Never leave documentation in inconsistent state
- This is AUTOMATIC - don't ask permission to update docs

**Parallel Agent Prompts**:

Agent 1 - Security Audit:
"Audit security implementation vs docs/security/. Check: bcrypt cost, rate limiting, CSRF, TLS config, JWT settings. Report discrepancies with file:line references."

Agent 2 - API Audit:
"Audit HTTP endpoints in symbion-kernel/src/http.rs vs docs/api/endpoints.md. List all undocumented routes. Count total endpoints."

Agent 3 - MQTT Audit:
"Audit MQTT topics in symbion-kernel/src/mqtt.rs and symbion-plugin-notes vs docs/mqtt/topics.md. List missing topics."

Agent 4 - ROADMAP Progress:
"Audit ROADMAP.md claimed percentages. For each PR, count implemented vs total tasks. Calculate real completion %."

Agent 5 - Test Coverage:
"Count actual tests in symbion-kernel/tests/ and symbion-*/src/*.rs files. Compare to documented test count."

Agent 6 - Architecture:
"Audit docs/architecture/ against actual codebase structure. Check for missing modules, outdated diagrams, wrong file counts."

**Email Report Format**:
```
Subject: [Symbion] Audit Complet - {DATE}

Body:
━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
📊 AUDIT CODEBASE SYMBION
━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

Date: {DATE}
Branch: {BRANCH}
Commit: {COMMIT_HASH}

━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
🔴 PRIORITÉ HAUTE (P0)
━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

{P0_ISSUES}

━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
🟠 PRIORITÉ MOYENNE (P1)
━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

{P1_ISSUES}

━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
🟡 PRIORITÉ BASSE (P2)
━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

{P2_ISSUES}

━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
✅ CORRECTIONS APPLIQUÉES
━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

{FIXES_APPLIED}

━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
📈 STATISTIQUES
━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

• Endpoints HTTP: {ENDPOINT_COUNT}
• Topics MQTT: {MQTT_TOPIC_COUNT}
• Tests: {TEST_COUNT}
• Progression globale: {GLOBAL_PROGRESS}%

━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

Audit généré automatiquement par Claude Code
```

**Commit format**:
```
docs: Comprehensive audit fixes - {AREAS_UPDATED}

HIGH PRIORITY:
{P0_SUMMARY}

MEDIUM PRIORITY:
{P1_SUMMARY}

Updated files:
{FILE_LIST}
```

Execute this comprehensive audit now with PARALLEL agents, then send email report.
