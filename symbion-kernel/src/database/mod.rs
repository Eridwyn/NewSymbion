/**
 * SYMBION KERNEL - SQLite Database Layer (Phase 1)
 *
 * ROLE: Centralized database access with WAL mode, migration system,
 * and graceful degradation to JSON fallback.
 *
 * PATTERN: r2d2 connection pool with SqliteConnectionManager.
 * Pool allows concurrent reads (WAL mode) while serializing writes.
 * conn() returns a PooledConnection that Derefs to rusqlite::Connection,
 * so all 115+ call sites remain unchanged.
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
use r2d2::Pool;
use r2d2_sqlite::SqliteConnectionManager;
use rusqlite::Connection;
use std::path::Path;
use std::sync::Arc;

/// Custom initializer that applies pragmas on each new connection from the pool.
#[derive(Debug)]
struct PragmaCustomizer;

impl r2d2::CustomizeConnection<Connection, rusqlite::Error> for PragmaCustomizer {
    fn on_acquire(&self, conn: &mut Connection) -> std::result::Result<(), rusqlite::Error> {
        conn.execute_batch("PRAGMA journal_mode=WAL;")?;
        conn.execute_batch("PRAGMA synchronous=NORMAL;")?;
        conn.execute_batch("PRAGMA foreign_keys=ON;")?;
        conn.busy_timeout(std::time::Duration::from_secs(5))?;
        Ok(())
    }
}

/// Thread-safe database handle backed by r2d2 connection pool.
pub struct Database {
    pool: Pool<SqliteConnectionManager>,
}

/// Shared reference, matching project convention (Arc<T>).
pub type SharedDatabase = Arc<Database>;

impl Database {
    /// Open or create database at the given path.
    /// Creates an r2d2 pool with up to 8 connections, WAL mode, and runs pending migrations.
    pub fn open(path: impl AsRef<Path>) -> Result<Self> {
        let path = path.as_ref();

        // Ensure parent directory exists
        if let Some(parent) = path.parent() {
            if !parent.exists() {
                std::fs::create_dir_all(parent)
                    .with_context(|| format!("Failed to create database directory: {:?}", parent))?;
            }
        }

        let manager = SqliteConnectionManager::file(path);
        let pool = Pool::builder()
            .max_size(8)
            .connection_timeout(std::time::Duration::from_secs(30))
            .connection_customizer(Box::new(PragmaCustomizer))
            .build(manager)
            .with_context(|| format!("Failed to create connection pool for {:?}", path))?;

        let db = Self { pool };

        // Run schema migrations
        migrations::run_migrations(&db)?;

        Ok(db)
    }

    /// Open in-memory database (for tests).
    /// Uses max_size(1) because in-memory DBs are per-connection.
    pub fn open_in_memory() -> Result<Self> {
        let manager = SqliteConnectionManager::memory();
        let pool = Pool::builder()
            .max_size(1)
            .connection_customizer(Box::new(PragmaCustomizer))
            .build(manager)
            .context("Failed to create in-memory connection pool")?;

        let db = Self { pool };

        // Run schema migrations
        migrations::run_migrations(&db)?;

        Ok(db)
    }

    /// Access a connection from the pool.
    /// Panics if the pool is exhausted after the configured timeout (30s).
    /// This matches the previous parking_lot::Mutex behavior (never returns Result).
    pub fn conn(&self) -> r2d2::PooledConnection<SqliteConnectionManager> {
        self.pool.get().expect("Failed to acquire database connection from pool (30s timeout)")
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

    #[test]
    fn test_pool_multiple_connections() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("pool_test.db");
        let db = Database::open(&path).unwrap();

        // Acquire multiple connections concurrently (WAL allows concurrent reads)
        let conn1 = db.conn();
        let conn2 = db.conn();

        let mode1: String = conn1.query_row("PRAGMA journal_mode", [], |r| r.get(0)).unwrap();
        let mode2: String = conn2.query_row("PRAGMA journal_mode", [], |r| r.get(0)).unwrap();

        assert_eq!(mode1, "wal");
        assert_eq!(mode2, "wal");
    }
}
