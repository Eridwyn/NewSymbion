/**
 * SYMBION KERNEL - Pending Actions SQLite Queries
 *
 * ROLE: Typed queries for pending validation actions persistence.
 * Replaces data/pending_actions.json.
 * All functions take &Database and return anyhow::Result.
 */

use anyhow::Result;
use rusqlite::params;
use super::Database;

/// Single pending action row (maps to/from pending_actions table).
pub struct PendingActionRow {
    pub validation_id: String,
    pub automation_id: String,
    pub automation_name: String,
    pub action_json: String,
    pub action_index: usize,
    pub trust_score: Option<f64>,
    pub target_mode: Option<String>,
    pub created_at: String, // ISO 8601
}

/// Upsert a pending action (INSERT OR REPLACE).
pub fn upsert_pending_action(db: &Database, row: &PendingActionRow) -> Result<()> {
    let conn = db.conn();
    conn.execute(
        "INSERT OR REPLACE INTO pending_actions
         (validation_id, automation_id, automation_name, action_json, action_index,
          trust_score, target_mode, created_at)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
        params![
            row.validation_id,
            row.automation_id,
            row.automation_name,
            row.action_json,
            row.action_index as i32,
            row.trust_score,
            row.target_mode,
            row.created_at,
        ],
    )?;
    Ok(())
}

/// Get a pending action by validation_id.
pub fn get_pending_action(db: &Database, validation_id: &str) -> Result<Option<PendingActionRow>> {
    let conn = db.conn();
    let result = conn.query_row(
        "SELECT validation_id, automation_id, automation_name, action_json,
                action_index, trust_score, target_mode, created_at
         FROM pending_actions WHERE validation_id = ?1",
        params![validation_id],
        |row| {
            Ok(PendingActionRow {
                validation_id: row.get(0)?,
                automation_id: row.get(1)?,
                automation_name: row.get(2)?,
                action_json: row.get(3)?,
                action_index: row.get::<_, i32>(4)? as usize,
                trust_score: row.get(5)?,
                target_mode: row.get(6)?,
                created_at: row.get(7)?,
            })
        },
    );
    match result {
        Ok(row) => Ok(Some(row)),
        Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
        Err(e) => Err(e.into()),
    }
}

/// List all pending actions.
pub fn list_pending_actions(db: &Database) -> Result<Vec<PendingActionRow>> {
    let conn = db.conn();
    let mut stmt = conn.prepare(
        "SELECT validation_id, automation_id, automation_name, action_json,
                action_index, trust_score, target_mode, created_at
         FROM pending_actions
         ORDER BY created_at DESC"
    )?;

    let rows = stmt.query_map([], |row| {
        Ok(PendingActionRow {
            validation_id: row.get(0)?,
            automation_id: row.get(1)?,
            automation_name: row.get(2)?,
            action_json: row.get(3)?,
            action_index: row.get::<_, i32>(4)? as usize,
            trust_score: row.get(5)?,
            target_mode: row.get(6)?,
            created_at: row.get(7)?,
        })
    })?;

    let mut result = Vec::new();
    for row in rows {
        result.push(row?);
    }
    Ok(result)
}

/// Delete a single pending action by validation_id.
pub fn delete_pending_action(db: &Database, validation_id: &str) -> Result<bool> {
    let conn = db.conn();
    let deleted = conn.execute(
        "DELETE FROM pending_actions WHERE validation_id = ?1",
        params![validation_id],
    )?;
    Ok(deleted > 0)
}

/// Delete all pending actions.
pub fn delete_all_pending_actions(db: &Database) -> Result<usize> {
    let conn = db.conn();
    let deleted = conn.execute("DELETE FROM pending_actions", [])?;
    Ok(deleted)
}

/// Count pending actions.
pub fn count_pending_actions(db: &Database) -> Result<i64> {
    let conn = db.conn();
    conn.query_row("SELECT COUNT(*) FROM pending_actions", [], |row| row.get(0))
        .map_err(Into::into)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_db() -> Database {
        Database::open_in_memory().unwrap()
    }

    fn sample_row(id: &str) -> PendingActionRow {
        PendingActionRow {
            validation_id: id.to_string(),
            automation_id: "auto-1".to_string(),
            automation_name: "Test Automation".to_string(),
            action_json: r#"{"type":"notify","message":"hello"}"#.to_string(),
            action_index: 0,
            trust_score: Some(0.75),
            target_mode: Some("focus".to_string()),
            created_at: "2026-02-28T10:00:00Z".to_string(),
        }
    }

    #[test]
    fn test_upsert_and_get() {
        let db = test_db();
        let row = sample_row("val-001");

        upsert_pending_action(&db, &row).unwrap();
        assert_eq!(count_pending_actions(&db).unwrap(), 1);

        let loaded = get_pending_action(&db, "val-001").unwrap().unwrap();
        assert_eq!(loaded.automation_name, "Test Automation");
        assert_eq!(loaded.trust_score, Some(0.75));
        assert_eq!(loaded.target_mode, Some("focus".to_string()));
    }

    #[test]
    fn test_get_missing() {
        let db = test_db();
        let result = get_pending_action(&db, "nonexistent").unwrap();
        assert!(result.is_none());
    }

    #[test]
    fn test_upsert_overwrite() {
        let db = test_db();

        let mut row = sample_row("val-002");
        upsert_pending_action(&db, &row).unwrap();

        // Update same key with new data
        row.automation_name = "Updated Automation".to_string();
        row.trust_score = Some(0.90);
        upsert_pending_action(&db, &row).unwrap();

        assert_eq!(count_pending_actions(&db).unwrap(), 1);
        let loaded = get_pending_action(&db, "val-002").unwrap().unwrap();
        assert_eq!(loaded.automation_name, "Updated Automation");
        assert_eq!(loaded.trust_score, Some(0.90));
    }

    #[test]
    fn test_list_pending_actions() {
        let db = test_db();

        for i in 0..3 {
            let mut row = sample_row(&format!("val-{:03}", i));
            row.created_at = format!("2026-02-28T10:{:02}:00Z", i);
            upsert_pending_action(&db, &row).unwrap();
        }

        let all = list_pending_actions(&db).unwrap();
        assert_eq!(all.len(), 3);
        // Newest first
        assert_eq!(all[0].validation_id, "val-002");
    }

    #[test]
    fn test_delete_pending_action() {
        let db = test_db();

        upsert_pending_action(&db, &sample_row("val-del")).unwrap();
        assert_eq!(count_pending_actions(&db).unwrap(), 1);

        let deleted = delete_pending_action(&db, "val-del").unwrap();
        assert!(deleted);
        assert_eq!(count_pending_actions(&db).unwrap(), 0);

        // Second delete returns false
        let deleted = delete_pending_action(&db, "val-del").unwrap();
        assert!(!deleted);
    }

    #[test]
    fn test_delete_all_pending_actions() {
        let db = test_db();

        for i in 0..5 {
            upsert_pending_action(&db, &sample_row(&format!("val-{}", i))).unwrap();
        }
        assert_eq!(count_pending_actions(&db).unwrap(), 5);

        let deleted = delete_all_pending_actions(&db).unwrap();
        assert_eq!(deleted, 5);
        assert_eq!(count_pending_actions(&db).unwrap(), 0);
    }
}
