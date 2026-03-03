-- v004: Command history for audit trail
-- Tracks all commands sent to agents with their results

CREATE TABLE IF NOT EXISTS command_history (
    command_id TEXT PRIMARY KEY,
    agent_id TEXT NOT NULL,
    command_type TEXT NOT NULL,
    parameters_json TEXT,
    status TEXT NOT NULL DEFAULT 'Sent',
    output_json TEXT,
    error_json TEXT,
    timeout_seconds INTEGER NOT NULL DEFAULT 30,
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL,
    completed_at TEXT
);

CREATE INDEX IF NOT EXISTS idx_cmd_agent_id ON command_history(agent_id);
CREATE INDEX IF NOT EXISTS idx_cmd_created_at ON command_history(created_at);
