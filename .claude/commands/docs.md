Search in Symbion documentation using the built-in docs-lookup.sh tool.

**Usage**: `/docs [search term]`

**Workflow**:
1. Extract search term from user query
2. Run `./scripts/docs-lookup.sh search "[term]"`
3. Parse results and present relevant documentation
4. If multiple matches, show summary with file:line references
5. Always cite source (e.g., `docs/api/endpoints.md:123`)

**Available searches**:
- `/docs endpoints` - List all HTTP endpoints
- `/docs mqtt` - List all MQTT topics
- `/docs auth` - Authentication guide
- `/docs webauthn` - WebAuthn/Passkeys guide
- `/docs security` - Security architecture
- `/docs quick` - Quick reference cheat sheet
- `/docs [any term]` - Full-text search

**Important**:
- ALWAYS use this tool BEFORE answering technical questions
- Prefer documentation over code exploration when possible
- Update docs if information is missing or outdated

Execute search now based on user query.
