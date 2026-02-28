/**
 * SYMBION KERNEL - Database Schema Migration System
 *
 * ROLE: Integer-based schema versioning for SQLite.
 * Each migration is a (version, SQL) pair. Applied migrations are tracked
 * in the schema_version table.
 *
 * RULE: Never modify an existing migration. Only append new ones.
 */

use anyhow::{Context, Result};
use super::Database;

/// All migrations, in order. Append-only.
const MIGRATIONS: &[(i64, &str)] = &[
    (1, include_str!("sql/v001_initial.sql")),
    (2, include_str!("sql/v002_remaining_tables.sql")),
    (3, include_str!("sql/v003_remaining_json.sql")),
];

/// Ensure schema_version table exists, then apply pending migrations.
pub fn run_migrations(db: &Database) -> Result<()> {
    let conn = db.conn();

    conn.execute_batch(
        "CREATE TABLE IF NOT EXISTS schema_version (
            version INTEGER PRIMARY KEY,
            applied_at TEXT NOT NULL DEFAULT (datetime('now'))
        );"
    ).context("Failed to create schema_version table")?;

    let current_version: i64 = conn
        .query_row(
            "SELECT COALESCE(MAX(version), 0) FROM schema_version",
            [],
            |row| row.get(0),
        )
        .context("Failed to read schema version")?;

    for &(version, sql) in MIGRATIONS {
        if version > current_version {
            eprintln!("[database] applying migration v{}", version);
            conn.execute_batch(sql)
                .with_context(|| format!("Migration v{} failed", version))?;
            conn.execute(
                "INSERT INTO schema_version (version) VALUES (?1)",
                rusqlite::params![version],
            ).with_context(|| format!("Failed to record migration v{}", version))?;
            eprintln!("[database] migration v{} applied", version);
        }
    }

    let final_version = MIGRATIONS.last().map(|m| m.0).unwrap_or(0);
    eprintln!(
        "[database] schema at v{} ({} migrations checked)",
        current_version.max(final_version),
        MIGRATIONS.len()
    );

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_migrations_run_on_fresh_db() {
        let db = Database::open_in_memory().unwrap();
        // Migrations already ran in open_in_memory, verify tables exist
        let conn = db.conn();

        let version: i64 = conn
            .query_row("SELECT MAX(version) FROM schema_version", [], |r| r.get(0))
            .unwrap();
        assert_eq!(version, 3);

        // Verify v1 tables
        let count: i64 = conn
            .query_row("SELECT COUNT(*) FROM sensor_environments", [], |r| r.get(0))
            .unwrap();
        assert_eq!(count, 0);

        let count: i64 = conn
            .query_row("SELECT COUNT(*) FROM automation_history", [], |r| r.get(0))
            .unwrap();
        assert_eq!(count, 0);

        // Verify v2 tables
        let tables = [
            "users", "device_tokens", "webauthn_credentials", "agents",
            "modes", "automations", "schedule_rules", "schedule_config",
            "notifications", "notification_configs",
            "trust_action_stats", "trust_agent_stats", "trust_global",
            "training_samples",
        ];
        for table in tables {
            let count: i64 = conn
                .query_row(&format!("SELECT COUNT(*) FROM {}", table), [], |r| r.get(0))
                .unwrap();
            assert_eq!(count, 0, "Table {} should be empty", table);
        }

        // Verify v3 tables
        let v3_tables = [
            "context_history", "context_state", "learned_patterns",
            "pending_actions", "sensors",
        ];
        for table in v3_tables {
            let count: i64 = conn
                .query_row(&format!("SELECT COUNT(*) FROM {}", table), [], |r| r.get(0))
                .unwrap();
            assert_eq!(count, 0, "Table {} should be empty", table);
        }
    }

    #[test]
    fn test_migrations_idempotent() {
        let db = Database::open_in_memory().unwrap();
        // Run again — should not error
        run_migrations(&db).unwrap();

        let conn = db.conn();
        let version: i64 = conn
            .query_row("SELECT MAX(version) FROM schema_version", [], |r| r.get(0))
            .unwrap();
        assert_eq!(version, 3);
    }
}
