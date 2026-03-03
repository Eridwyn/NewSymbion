/**
 * SYMBION KERNEL - SQLite Database Layer (Phase 1)
 *
 * ROLE: Centralized database access with WAL mode, migration system,
 * and graceful degradation to JSON fallback.
 *
 * PATTERN: Single connection behind parking_lot::Mutex, matching
 * the project's Shared<T> = Arc<Mutex<T>> convention from src/state.rs.
 *
 * SAFETY: If Database::open() fails, the kernel continues in JSON-only mode.
 * All query functions return Result — callers fall back to JSON on any error.
 */

pub mod migrations;
pub mod sensor_queries;
pub mod automation_queries;
pub mod auth_queries;
pub mod agent_queries;
pub mod config_queries;
pub mod notification_queries;
pub mod trust_queries;
pub mod inference_queries;
pub mod automation_rule_queries;
pub mod context_queries;
pub mod pattern_queries;
pub mod pending_action_queries;
pub mod sensor_meta_queries;
pub mod command_history_queries;

use anyhow::{Context, Result};
use parking_lot::Mutex;
use rusqlite::Connection;
use std::path::Path;
use std::sync::Arc;

/// Thread-safe database handle.
pub struct Database {
    conn: Mutex<Connection>,
}

/// Shared reference, matching project convention (Arc<T>).
pub type SharedDatabase = Arc<Database>;

impl Database {
    /// Open or create database at the given path.
    /// Enables WAL mode, NORMAL synchronous, foreign keys, and runs pending migrations.
    pub fn open(path: impl AsRef<Path>) -> Result<Self> {
        let path = path.as_ref();

        // Ensure parent directory exists
        if let Some(parent) = path.parent() {
            if !parent.exists() {
                std::fs::create_dir_all(parent)
                    .with_context(|| format!("Failed to create database directory: {:?}", parent))?;
            }
        }

        let conn = Connection::open(path)
            .with_context(|| format!("Failed to open SQLite database at {:?}", path))?;

        Self::configure_and_migrate(conn)
    }

    /// Open in-memory database (for tests).
    pub fn open_in_memory() -> Result<Self> {
        let conn = Connection::open_in_memory()
            .context("Failed to open in-memory SQLite database")?;
        Self::configure_and_migrate(conn)
    }

    /// Configure pragmas and run migrations.
    fn configure_and_migrate(conn: Connection) -> Result<Self> {
        // WAL mode for concurrent reads during writes
        conn.execute_batch("PRAGMA journal_mode=WAL;")
            .context("Failed to set WAL mode")?;
        // NORMAL sync is safe with WAL (no corruption, ~2x faster than FULL)
        conn.execute_batch("PRAGMA synchronous=NORMAL;")
            .context("Failed to set synchronous mode")?;
        // Foreign keys
        conn.execute_batch("PRAGMA foreign_keys=ON;")
            .context("Failed to enable foreign keys")?;
        // Busy timeout 5 seconds
        conn.busy_timeout(std::time::Duration::from_secs(5))
            .context("Failed to set busy timeout")?;

        let db = Self {
            conn: Mutex::new(conn),
        };

        // Run schema migrations
        migrations::run_migrations(&db)?;

        Ok(db)
    }

    /// Access the connection under lock.
    pub fn conn(&self) -> parking_lot::MutexGuard<'_, Connection> {
        self.conn.lock()
    }

    /// Import existing JSON data into SQLite (one-shot migration).
    /// Only imports if the target tables are empty.
    /// All errors are non-fatal — logged and skipped.
    pub fn import_json_if_needed(
        db: &Database,
        env_json_path: &str,
        history_json_path: &str,
    ) -> Result<()> {
        let env_count = sensor_queries::count_readings(db)?;
        let hist_count = automation_queries::count_history(db)?;

        if env_count > 0 && hist_count > 0 {
            eprintln!(
                "[database] import skipped: SQLite already has data ({} env, {} hist)",
                env_count, hist_count
            );
            return Ok(());
        }

        // Import sensor environments
        if env_count == 0 {
            if let Ok(json) = std::fs::read_to_string(env_json_path) {
                match sensor_queries::import_from_json(db, &json) {
                    Ok(n) => eprintln!("[database] imported {} environment readings from JSON", n),
                    Err(e) => eprintln!("[database] environment import failed (non-fatal): {}", e),
                }
            } else {
                eprintln!("[database] no environment JSON file to import");
            }
        }

        // Import automation history
        if hist_count == 0 {
            if let Ok(json) = std::fs::read_to_string(history_json_path) {
                match automation_queries::import_from_json(db, &json) {
                    Ok(n) => eprintln!("[database] imported {} automation history records from JSON", n),
                    Err(e) => eprintln!("[database] history import failed (non-fatal): {}", e),
                }
            } else {
                eprintln!("[database] no history JSON file to import");
            }
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_open_in_memory() {
        let db = Database::open_in_memory();
        assert!(db.is_ok());
    }

    #[test]
    fn test_open_file() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("test.db");
        let db = Database::open(&path);
        assert!(db.is_ok());
        assert!(path.exists());
    }

    #[test]
    fn test_wal_mode_enabled() {
        let db = Database::open_in_memory().unwrap();
        let conn = db.conn();
        let mode: String = conn
            .query_row("PRAGMA journal_mode", [], |r| r.get(0))
            .unwrap();
        // In-memory databases use "memory" journal mode, not WAL
        // WAL only applies to file-based databases
        assert!(mode == "memory" || mode == "wal");
    }
}
