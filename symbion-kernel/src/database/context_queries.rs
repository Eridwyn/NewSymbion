/**
 * SYMBION KERNEL - Context History & State SQLite Queries
 *
 * ROLE: Typed queries for context mode history and current state (KV) persistence.
 * Replaces context-history.json and context-state.json.
 * All functions take &Database and return anyhow::Result.
 */

use anyhow::Result;
use rusqlite::params;
use super::Database;

/// Single context history row (maps to/from context_history table).
pub struct ContextHistoryRow {
    pub mode: String,
    pub mode_slug: Option<String>,
    pub timestamp: String, // ISO 8601
    pub reason: Option<String>,
    pub was_manual: bool,
}

/// Key-value pair for context state.
pub struct ContextStateKV {
    pub key: String,
    pub value: String,
}

// ============================================================================
// Context History
// ============================================================================

/// Insert a single history entry. Returns the row ID.
pub fn insert_history_entry(db: &Database, row: &ContextHistoryRow) -> Result<i64> {
    let conn = db.conn();
    conn.execute(
        "INSERT INTO context_history (mode, mode_slug, timestamp, reason, was_manual)
         VALUES (?1, ?2, ?3, ?4, ?5)",
        params![
            row.mode,
            row.mode_slug,
            row.timestamp,
            row.reason,
            row.was_manual as i32,
        ],
    )?;
    Ok(conn.last_insert_rowid())
}

/// List history entries, newest first, with limit.
pub fn list_history(db: &Database, limit: usize) -> Result<Vec<ContextHistoryRow>> {
    let conn = db.conn();
    let mut stmt = conn.prepare(
        "SELECT mode, mode_slug, timestamp, reason, was_manual
         FROM context_history
         ORDER BY timestamp DESC
         LIMIT ?1"
    )?;

    let rows = stmt.query_map(params![limit as i64], |row| {
        Ok(ContextHistoryRow {
            mode: row.get(0)?,
            mode_slug: row.get(1)?,
            timestamp: row.get(2)?,
            reason: row.get(3)?,
            was_manual: row.get::<_, i32>(4)? != 0,
        })
    })?;

    let mut result = Vec::new();
    for row in rows {
        result.push(row?);
    }
    Ok(result)
}

/// Count history entries.
pub fn count_history(db: &Database) -> Result<i64> {
    let conn = db.conn();
    conn.query_row("SELECT COUNT(*) FROM context_history", [], |row| row.get(0))
        .map_err(Into::into)
}

/// Delete oldest entries beyond `keep` count.
pub fn delete_old_history(db: &Database, keep: usize) -> Result<usize> {
    let conn = db.conn();
    let deleted = conn.execute(
        "DELETE FROM context_history WHERE id NOT IN (
            SELECT id FROM context_history ORDER BY timestamp DESC LIMIT ?1
        )",
        params![keep as i64],
    )?;
    Ok(deleted)
}

// ============================================================================
// Context State (KV)
// ============================================================================

/// Set a state key-value pair (INSERT OR REPLACE).
pub fn set_state(db: &Database, key: &str, value: &str) -> Result<()> {
    let conn = db.conn();
    conn.execute(
        "INSERT OR REPLACE INTO context_state (key, value) VALUES (?1, ?2)",
        params![key, value],
    )?;
    Ok(())
}

/// Get a single state value by key.
pub fn get_state(db: &Database, key: &str) -> Result<Option<String>> {
    let conn = db.conn();
    let result = conn.query_row(
        "SELECT value FROM context_state WHERE key = ?1",
        params![key],
        |row| row.get(0),
    );
    match result {
        Ok(value) => Ok(Some(value)),
        Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
        Err(e) => Err(e.into()),
    }
}

/// Get all state key-value pairs.
pub fn get_all_state(db: &Database) -> Result<Vec<ContextStateKV>> {
    let conn = db.conn();
    let mut stmt = conn.prepare("SELECT key, value FROM context_state")?;

    let rows = stmt.query_map([], |row| {
        Ok(ContextStateKV {
            key: row.get(0)?,
            value: row.get(1)?,
        })
    })?;

    let mut result = Vec::new();
    for row in rows {
        result.push(row?);
    }
    Ok(result)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_db() -> Database {
        Database::open_in_memory().unwrap()
    }

    // === History tests ===

    #[test]
    fn test_insert_and_count_history() {
        let db = test_db();
        let row = ContextHistoryRow {
            mode: "pro".to_string(),
            mode_slug: Some("pro".to_string()),
            timestamp: "2026-02-28T10:00:00Z".to_string(),
            reason: Some("Manual override".to_string()),
            was_manual: true,
        };
        let id = insert_history_entry(&db, &row).unwrap();
        assert!(id > 0);
        assert_eq!(count_history(&db).unwrap(), 1);
    }

    #[test]
    fn test_list_history_ordered() {
        let db = test_db();
        for i in 0..5 {
            let row = ContextHistoryRow {
                mode: "maison".to_string(),
                mode_slug: Some("maison".to_string()),
                timestamp: format!("2026-02-28T10:{:02}:00Z", i),
                reason: Some(format!("Reason {}", i)),
                was_manual: i % 2 == 0,
            };
            insert_history_entry(&db, &row).unwrap();
        }

        let history = list_history(&db, 3).unwrap();
        assert_eq!(history.len(), 3);
        // Newest first
        assert_eq!(history[0].timestamp, "2026-02-28T10:04:00Z");
        assert_eq!(history[2].timestamp, "2026-02-28T10:02:00Z");
    }

    #[test]
    fn test_delete_old_history() {
        let db = test_db();
        for i in 0..10 {
            let row = ContextHistoryRow {
                mode: "veille".to_string(),
                mode_slug: None,
                timestamp: format!("2026-02-28T10:{:02}:00Z", i),
                reason: None,
                was_manual: false,
            };
            insert_history_entry(&db, &row).unwrap();
        }
        assert_eq!(count_history(&db).unwrap(), 10);

        let deleted = delete_old_history(&db, 5).unwrap();
        assert_eq!(deleted, 5);
        assert_eq!(count_history(&db).unwrap(), 5);

        // Kept entries should be the newest 5
        let remaining = list_history(&db, 10).unwrap();
        assert_eq!(remaining.len(), 5);
        assert_eq!(remaining[0].timestamp, "2026-02-28T10:09:00Z");
    }

    // === State tests ===

    #[test]
    fn test_set_and_get_state() {
        let db = test_db();

        set_state(&db, "current_mode", "pro").unwrap();
        let val = get_state(&db, "current_mode").unwrap();
        assert_eq!(val, Some("pro".to_string()));
    }

    #[test]
    fn test_get_state_missing() {
        let db = test_db();
        let val = get_state(&db, "nonexistent").unwrap();
        assert_eq!(val, None);
    }

    #[test]
    fn test_set_state_upsert() {
        let db = test_db();

        set_state(&db, "mode", "pro").unwrap();
        set_state(&db, "mode", "maison").unwrap(); // Overwrite

        let val = get_state(&db, "mode").unwrap();
        assert_eq!(val, Some("maison".to_string()));
    }

    #[test]
    fn test_get_all_state() {
        let db = test_db();

        set_state(&db, "mode", "pro").unwrap();
        set_state(&db, "confidence", "0.85").unwrap();
        set_state(&db, "reason", "Morning work").unwrap();

        let all = get_all_state(&db).unwrap();
        assert_eq!(all.len(), 3);
    }
}
