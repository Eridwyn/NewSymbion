-- V1: Sensor environments + automation history
-- Phase 1 of JSON-to-SQLite migration

CREATE TABLE IF NOT EXISTS sensor_environments (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    sensor_id TEXT NOT NULL,
    room_id TEXT NOT NULL,
    temperature_c REAL,
    humidity_pct REAL,
    status TEXT NOT NULL DEFAULT 'safe',
    recorded_at TEXT NOT NULL,
    UNIQUE(sensor_id, recorded_at)
);

CREATE INDEX IF NOT EXISTS idx_env_sensor_id ON sensor_environments(sensor_id);
CREATE INDEX IF NOT EXISTS idx_env_recorded_at ON sensor_environments(recorded_at);
CREATE INDEX IF NOT EXISTS idx_env_room_id ON sensor_environments(room_id);

CREATE TABLE IF NOT EXISTS automation_history (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    automation_id TEXT NOT NULL,
    automation_name TEXT NOT NULL DEFAULT '',
    executed_at TEXT NOT NULL,
    trigger_event TEXT NOT NULL DEFAULT '',
    conditions_met INTEGER NOT NULL DEFAULT 1,
    success INTEGER NOT NULL DEFAULT 1,
    error TEXT,
    trust_score REAL,
    decision_outcome TEXT,
    actions_json TEXT NOT NULL DEFAULT '[]'
);

CREATE INDEX IF NOT EXISTS idx_hist_automation_id ON automation_history(automation_id);
CREATE INDEX IF NOT EXISTS idx_hist_executed_at ON automation_history(executed_at);
