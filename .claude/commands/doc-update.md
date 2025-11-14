Systematic documentation update after ANY code modification.

**Trigger**: After EVERY code change (Edit/Write tool usage)

**Purpose**: Keep documentation synchronized with codebase automatically.

**Workflow** (MANDATORY after code changes):

1. **Detect Modified Files**
   ```bash
   git status --porcelain | grep -E "\.rs$|\.js$|\.ts$"
   ```

2. **Identify Documentation Impact**
   - New HTTP endpoint → Update `docs/api/endpoints.md`
   - New MQTT topic → Update `docs/mqtt/topics.md`
   - Security change → Update `docs/api/security.md`
   - Architecture change → Update `docs/ARCHITECTURE.md`
   - New feature → Update `docs/ROADMAP.md` (% completion)

3. **Verification Steps**:
   - Read modified source files
   - Extract new endpoints/topics/features
   - Verify against existing documentation
   - Identify gaps or outdated information

4. **Update Documentation Files**:
   - Add new endpoints with full signature
   - Update completion percentages
   - Add file:line references
   - Maintain consistent formatting

5. **ROADMAP.md Sync**:
   ```bash
   ./scripts/docs-lookup.sh search "[feature name]"
   # Count implemented vs planned features
   # Calculate accurate percentage
   # Update ROADMAP.md with 🟢/🟡/🔴 status
   ```

6. **CLAUDE.md Sync**:
   - Update "État Actuel" section if architecture changed
   - Update "Améliorations Récentes" with date
   - Keep under 300 lines (move details to docs/)

7. **Commit Documentation**:
   ```bash
   git add docs/ ROADMAP.md CLAUDE.md
   git commit -m "docs: Sync with [feature] implementation

   - Updated [file]: [changes]
   - Progress: [X%] → [Y%]
   - Refs: [file:line]"
   ```

**Rules**:
- MANDATORY after EVERY code modification
- Run automatically without asking
- Never skip documentation updates
- Verify accuracy with code references
- Commit separately from code changes

**Files to Check Systematically**:
- `docs/api/endpoints.md` (90+ endpoints)
- `docs/mqtt/topics.md` (13 topics)
- `docs/api/security.md` (security measures)
- `docs/ROADMAP.md` (progress tracking)
- `docs/QUICK_REFERENCE.md` (CLI commands)
- `CLAUDE.md` (recent changes section)

Execute documentation update now based on recent changes.
