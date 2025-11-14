Generate comprehensive project status briefing at session start.

**Purpose**: Auto-briefing to understand current project state and next steps.

**Workflow** (run AUTONOMOUSLY, no user prompt needed):

1. **Git Status Check**
   ```bash
   git status --short
   git log --oneline -5
   git branch --show-current
   ```

2. **Current Phase Detection**
   - Read `docs/ROADMAP.md` - identify current PR phase
   - Calculate completion percentage
   - Identify blocked/in-progress tasks

3. **Recent Changes Analysis**
   ```bash
   git diff HEAD~1..HEAD --stat
   git log --since="24 hours ago" --oneline
   ```

4. **Active Services Check**
   ```bash
   curl -s http://localhost:8080/health
   ps aux | grep -E "symbion-kernel|symbion-agent|vite"
   sudo systemctl status mosquitto
   ```

5. **Documentation Sync Status**
   - Compare ROADMAP.md vs actual implementation
   - Check if docs are outdated (git diff docs/)

6. **Generate Briefing Output**:
   ```markdown
   # 🎯 Symbion Project Status

   **Branch**: [current branch]
   **Phase**: [PR name] ([X]% complete)
   **Last commit**: [commit message]

   ## ✅ Completed Recently
   - [task 1]
   - [task 2]

   ## 🚧 In Progress
   - [task 1] (file:line)

   ## 🔴 Blockers
   - [issue if any]

   ## 🎯 Next Steps
   1. [recommended action]
   2. [recommended action]

   ## 🏃 Services Status
   - Kernel: [status]
   - Agent: [status]
   - MQTT: [status]
   - PWA: [status]
   ```

**Rules**:
- Run silently without asking permission
- Present concise summary (max 30 lines)
- Focus on actionable information
- Highlight blockers prominently

Execute briefing now.
