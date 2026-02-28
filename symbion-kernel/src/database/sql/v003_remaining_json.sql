-- v003: Migrate remaining JSON files to SQLite
-- Files: context-history.json, context-state.json, learned_patterns.json,
--        data/pending_actions.json, data/sensors.json

-- Context mode history (from context-history.json)
CREATE TABLE IF NOT EXISTS context_history (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    mode TEXT NOT NULL,
    mode_slug TEXT,
    timestamp TEXT NOT NULL,
    reason TEXT,
    was_manual INTEGER NOT NULL DEFAULT 0
);
CREATE INDEX IF NOT EXISTS idx_ch_timestamp ON context_history(timestamp);

-- Context current state as key-value (from context-state.json)
CREATE TABLE IF NOT EXISTS context_state (
    key TEXT PRIMARY KEY,
    value TEXT NOT NULL
);

-- Learned behavior patterns (from learned_patterns.json)
CREATE TABLE IF NOT EXISTS learned_patterns (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    mode TEXT NOT NULL,
    day_of_week INTEGER NOT NULL,
    hour INTEGER NOT NULL,
    confidence REAL NOT NULL DEFAULT 0.0,
    occurrences INTEGER NOT NULL DEFAULT 0,
    last_seen TEXT NOT NULL,
    source TEXT NOT NULL DEFAULT 'Historical'
);
CREATE INDEX IF NOT EXISTS idx_lp_mode ON learned_patterns(mode);

-- Pending validation actions (from data/pending_actions.json)
CREATE TABLE IF NOT EXISTS pending_actions (
    validation_id TEXT PRIMARY KEY,
    automation_id TEXT NOT NULL,
    automation_name TEXT NOT NULL,
    action_json TEXT NOT NULL,
    action_index INTEGER NOT NULL DEFAULT 0,
    trust_score REAL,
    target_mode TEXT,
    created_at TEXT NOT NULL
);

-- Sensor metadata registry (from data/sensors.json)
CREATE TABLE IF NOT EXISTS sensors (
    sensor_id TEXT PRIMARY KEY,
    sensor_type TEXT NOT NULL,
    room_id TEXT NOT NULL,
    firmware_version TEXT,
    registered_at TEXT NOT NULL,
    last_seen TEXT NOT NULL,
    status TEXT NOT NULL DEFAULT 'unknown',
    battery_pct REAL,
    signal_rssi REAL,
    deleted_at TEXT
);
CREATE INDEX IF NOT EXISTS idx_sensors_room ON sensors(room_id);
