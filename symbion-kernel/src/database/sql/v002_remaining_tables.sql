-- V2: Remaining JSON-persisted modules → SQLite
-- Phase 2 of JSON-to-SQLite migration (14 tables)

-- Users (from users.json)
CREATE TABLE IF NOT EXISTS users (
    username TEXT PRIMARY KEY,
    password_hash TEXT NOT NULL,
    role TEXT NOT NULL DEFAULT 'user',
    created_at INTEGER NOT NULL,
    mfa_config_json TEXT
);

-- Device trust tokens (from device_tokens.json)
CREATE TABLE IF NOT EXISTS device_tokens (
    token TEXT PRIMARY KEY,
    username TEXT NOT NULL,
    device_fingerprint TEXT NOT NULL,
    created_at INTEGER NOT NULL,
    expires_at INTEGER NOT NULL,
    last_used_at INTEGER NOT NULL
);
CREATE INDEX IF NOT EXISTS idx_dt_username ON device_tokens(username);
CREATE INDEX IF NOT EXISTS idx_dt_expires ON device_tokens(expires_at);

-- WebAuthn credentials (from data/webauthn_credentials.json)
CREATE TABLE IF NOT EXISTS webauthn_credentials (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    username TEXT NOT NULL,
    credential_id BLOB NOT NULL,
    credential_json TEXT NOT NULL,
    friendly_name TEXT NOT NULL DEFAULT '',
    created_at INTEGER NOT NULL,
    last_used_at INTEGER
);
CREATE INDEX IF NOT EXISTS idx_wa_username ON webauthn_credentials(username);

-- Agents (from data/agents.json) — status/network as JSON columns
CREATE TABLE IF NOT EXISTS agents (
    agent_id TEXT PRIMARY KEY,
    hostname TEXT NOT NULL,
    os TEXT NOT NULL,
    architecture TEXT NOT NULL,
    capabilities_json TEXT NOT NULL DEFAULT '[]',
    network_json TEXT NOT NULL,
    version TEXT,
    status_json TEXT NOT NULL,
    last_seen TEXT NOT NULL,
    registration_time TEXT NOT NULL,
    deleted_at TEXT
);

-- Modes (from data/modes.json)
CREATE TABLE IF NOT EXISTS modes (
    id TEXT PRIMARY KEY,
    name TEXT NOT NULL,
    slug TEXT NOT NULL UNIQUE,
    icon TEXT NOT NULL DEFAULT '',
    theme_json TEXT NOT NULL,
    is_system INTEGER NOT NULL DEFAULT 0,
    created_at TEXT NOT NULL,
    display_order INTEGER NOT NULL DEFAULT 0
);

-- Automation rules (from data/automations.json) — not history (already in v001)
CREATE TABLE IF NOT EXISTS automations (
    id TEXT PRIMARY KEY,
    name TEXT NOT NULL,
    description TEXT,
    category TEXT DEFAULT 'custom',
    goal_mode TEXT,
    enabled INTEGER NOT NULL DEFAULT 1,
    triggers_json TEXT,
    conditions_json TEXT,
    actions_json TEXT NOT NULL DEFAULT '[]',
    cooldown_seconds INTEGER NOT NULL DEFAULT 60,
    trusted INTEGER,
    skip_if_same_mode INTEGER,
    auto_created INTEGER,
    last_executed_at TEXT,
    execution_count INTEGER NOT NULL DEFAULT 0,
    created_at TEXT,
    updated_at TEXT,
    deleted_at TEXT
);

-- Schedule rules (from data/schedule.json)
CREATE TABLE IF NOT EXISTS schedule_rules (
    id TEXT PRIMARY KEY,
    mode_id TEXT NOT NULL,
    days_json TEXT NOT NULL,
    start_time TEXT NOT NULL,
    end_time TEXT NOT NULL,
    priority INTEGER NOT NULL DEFAULT 0,
    enabled INTEGER NOT NULL DEFAULT 1,
    name TEXT,
    created_at TEXT NOT NULL
);

-- Schedule config (key-value pairs from schedule.json)
CREATE TABLE IF NOT EXISTS schedule_config (
    key TEXT PRIMARY KEY,
    value TEXT NOT NULL
);

-- Notifications (from /var/lib/symbion/notifications.json)
CREATE TABLE IF NOT EXISTS notifications (
    id TEXT PRIMARY KEY,
    priority TEXT NOT NULL,
    title TEXT NOT NULL,
    body TEXT NOT NULL,
    source TEXT NOT NULL,
    timestamp TEXT NOT NULL,
    acknowledged INTEGER NOT NULL DEFAULT 0,
    acknowledged_at INTEGER,
    actions_json TEXT NOT NULL DEFAULT '[]',
    data_json TEXT
);
CREATE INDEX IF NOT EXISTS idx_notif_timestamp ON notifications(timestamp);

-- Notification type configs (from /var/lib/symbion/notification_configs.json)
CREATE TABLE IF NOT EXISTS notification_configs (
    type_id TEXT PRIMARY KEY,
    display_name TEXT NOT NULL,
    description TEXT NOT NULL DEFAULT '',
    category TEXT NOT NULL DEFAULT 'system',
    enabled INTEGER NOT NULL DEFAULT 1,
    title_template TEXT NOT NULL,
    body_template TEXT NOT NULL,
    priority TEXT NOT NULL DEFAULT 'P2',
    available_variables_json TEXT NOT NULL DEFAULT '[]'
);

-- Trust: action stats (from data/trust_stats.json → action_stats)
CREATE TABLE IF NOT EXISTS trust_action_stats (
    action_type TEXT PRIMARY KEY,
    total_executions INTEGER NOT NULL DEFAULT 0,
    successful INTEGER NOT NULL DEFAULT 0,
    failed INTEGER NOT NULL DEFAULT 0,
    blocked INTEGER NOT NULL DEFAULT 0,
    current_trust_modifier REAL NOT NULL DEFAULT 0.0,
    last_updated TEXT NOT NULL
);

-- Trust: agent stats (from data/trust_stats.json → agent_stats)
CREATE TABLE IF NOT EXISTS trust_agent_stats (
    agent_id TEXT PRIMARY KEY,
    total_commands INTEGER NOT NULL DEFAULT 0,
    successful INTEGER NOT NULL DEFAULT 0,
    failed INTEGER NOT NULL DEFAULT 0,
    current_trust_modifier REAL NOT NULL DEFAULT 0.0,
    last_updated TEXT NOT NULL
);

-- Trust: global counters (total_decisions, last_updated)
CREATE TABLE IF NOT EXISTS trust_global (
    key TEXT PRIMARY KEY,
    value TEXT NOT NULL
);

-- Training samples (from data/inference_samples.json)
CREATE TABLE IF NOT EXISTS training_samples (
    id TEXT PRIMARY KEY,
    vector_json TEXT NOT NULL,
    chosen_mode TEXT NOT NULL,
    source TEXT NOT NULL,
    timestamp TEXT NOT NULL,
    base_weight REAL NOT NULL DEFAULT 1.0
);
CREATE INDEX IF NOT EXISTS idx_ts_chosen_mode ON training_samples(chosen_mode);
CREATE INDEX IF NOT EXISTS idx_ts_timestamp ON training_samples(timestamp);
