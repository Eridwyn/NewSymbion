You are updating the documentation after code changes.

**Workflow**:
1. Use Task with subagent_type=Explore to audit recent code changes
2. Identify documentation that is now outdated (ROADMAP.md, architecture docs, implementation guides)
3. Update ALL affected documentation files with accurate information
4. Commit changes with descriptive message
5. Push to current branch

**Rules**:
- ALWAYS verify claims by reading actual code
- Update percentages based on real implementation
- Include file paths and line numbers in documentation
- Never leave documentation in inconsistent state
- This is AUTOMATIC - don't ask permission to update docs

**Files to check systematically**:
- docs/ROADMAP.md (PR progress percentages)
- docs/security/ (security implementation status)
- docs/architecture/ (system architecture)
- CLAUDE.md (project context - if it exists and is tracked)

**Commit format**:
```
docs: Update [area] documentation with current state

- Updated [file]: [what changed]
- Verified implementation at [file:line]
- Progress: X% → Y%
```

Execute this workflow now based on the current state of the codebase.
