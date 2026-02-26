/**
 * SYMBION KERNEL - Automation History SQLite Queries
 *
 * ROLE: Typed queries for automation execution history persistence.
 * All functions take &Database and return anyhow::Result.
 */

use anyhow::Result;
use rusqlite::params;
use super::Database;

/// Single history row (maps to/from automation_history table).
pub struct HistoryRow {
    pub automation_id: String,
    pub automation_name: String,
    pub executed_at: String,      // ISO 8601
    pub trigger_event: String,
    pub conditions_met: bool,
    pub success: bool,
    pub error: Option<String>,
    pub trust_score: Option<f64>,
    pub decision_outcome: Option<String>,
    pub actions_json: String,     // Serialized Vec<ActionResult>
}

/// Insert a single history record. Returns the row ID.
pub fn insert_history(db: &Database, row: &HistoryRow) -> Result<i64> {
    let conn = db.conn();
    conn.execute(
        "INSERT INTO automation_history
         (automation_id, automation_name, executed_at, trigger_event,
          conditions_met, success, error, trust_score, decision_outcome, actions_json)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10)",
        params![
            row.automation_id,
            row.automation_name,
            row.executed_at,
            row.trigger_event,
            row.conditions_met as i32,
            row.success as i32,
            row.error,
            row.trust_score,
            row.decision_outcome,
            row.actions_json,
        ],
    )?;
    Ok(conn.last_insert_rowid())
}

/// Get history records, newest first, with limit.
pub fn get_history(db: &Database, limit: usize) -> Result<Vec<HistoryRow>> {
    let conn = db.conn();
    let mut stmt = conn.prepare(
        "SELECT automation_id, automation_name, executed_at, trigger_event,
                conditions_met, success, error, trust_score, decision_outcome, actions_json
         FROM automation_history
         ORDER BY executed_at DESC
         LIMIT ?1"
    )?;

    let rows = stmt.query_map(params![limit as i64], |row| {
        Ok(HistoryRow {
            automation_id: row.get(0)?,
            automation_name: row.get(1)?,
            executed_at: row.get(2)?,
            trigger_event: row.get(3)?,
            conditions_met: row.get::<_, i32>(4)? != 0,
            success: row.get::<_, i32>(5)? != 0,
            error: row.get(6)?,
            trust_score: row.get(7)?,
            decision_outcome: row.get(8)?,
            actions_json: row.get(9)?,
        })
    })?;

    let mut result = Vec::new();
    for row in rows {
        result.push(row?);
    }
    Ok(result)
}

/// Get history filtered by automation ID, newest first.
pub fn get_history_by_automation(
    db: &Database,
    automation_id: &str,
    limit: usize,
) -> Result<Vec<HistoryRow>> {
    let conn = db.conn();
    let mut stmt = conn.prepare(
        "SELECT automation_id, automation_name, executed_at, trigger_event,
                conditions_met, success, error, trust_score, decision_outcome, actions_json
         FROM automation_history
         WHERE automation_id = ?1
         ORDER BY executed_at DESC
         LIMIT ?2"
    )?;

    let rows = stmt.query_map(params![automation_id, limit as i64], |row| {
        Ok(HistoryRow {
            automation_id: row.get(0)?,
            automation_name: row.get(1)?,
            executed_at: row.get(2)?,
            trigger_event: row.get(3)?,
            conditions_met: row.get::<_, i32>(4)? != 0,
            success: row.get::<_, i32>(5)? != 0,
            error: row.get(6)?,
            trust_score: row.get(7)?,
            decision_outcome: row.get(8)?,
            actions_json: row.get(9)?,
        })
    })?;

    let mut result = Vec::new();
    for row in rows {
        result.push(row?);
    }
    Ok(result)
}

/// Count history records (for stats and import detection).
pub fn count_history(db: &Database) -> Result<i64> {
    let conn = db.conn();
    conn.query_row("SELECT COUNT(*) FROM automation_history", [], |row| row.get(0))
        .map_err(Into::into)
}

/// Import automation history from JSON string (one-shot migration).
/// Expects the JSON format: Vec<ExecutionRecord>.
pub fn import_from_json(db: &Database, json: &str) -> Result<usize> {
    let records: Vec<serde_json::Value> = serde_json::from_str(json)?;

    let mut conn = db.conn();
    let tx = conn.transaction()?;
    let mut count = 0;

    {
        let mut stmt = tx.prepare_cached(
            "INSERT OR IGNORE INTO automation_history
             (automation_id, automation_name, executed_at, trigger_event,
              conditions_met, success, error, trust_score, decision_outcome, actions_json)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10)"
        )?;

        for record in &records {
            let automation_id = record.get("automation_id")
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_string();
            let automation_name = record.get("automation_name")
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_string();
            let executed_at = record.get("executed_at")
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_string();
            let trigger_event = record.get("trigger_event")
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_string();
            let conditions_met = record.get("conditions_met")
                .and_then(|v| v.as_bool())
                .unwrap_or(true);
            let success = record.get("success")
                .and_then(|v| v.as_bool())
                .unwrap_or(true);
            let error = record.get("error")
                .and_then(|v| v.as_str())
                .map(|s| s.to_string());
            let trust_score = record.get("trust_score")
                .and_then(|v| v.as_f64());
            let decision_outcome = record.get("decision_outcome")
                .and_then(|v| v.as_str())
                .map(|s| s.to_string());
            let actions_json = record.get("actions_executed")
                .map(|v| v.to_string())
                .unwrap_or_else(|| "[]".to_string());

            if !executed_at.is_empty() {
                stmt.execute(params![
                    automation_id,
                    automation_name,
                    executed_at,
                    trigger_event,
                    conditions_met as i32,
                    success as i32,
                    error,
                    trust_score,
                    decision_outcome,
                    actions_json,
                ])?;
                count += 1;
            }
        }
    }

    tx.commit()?;
    Ok(count)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_db() -> Database {
        Database::open_in_memory().unwrap()
    }

    #[test]
    fn test_insert_and_count() {
        let db = test_db();
        let row = HistoryRow {
            automation_id: "auto-1".to_string(),
            automation_name: "Test Automation".to_string(),
            executed_at: "2026-02-26T10:00:00Z".to_string(),
            trigger_event: "sensor.temperature".to_string(),
            conditions_met: true,
            success: true,
            error: None,
            trust_score: Some(0.85),
            decision_outcome: Some("approved".to_string()),
            actions_json: "[]".to_string(),
        };
        let id = insert_history(&db, &row).unwrap();
        assert!(id > 0);
        assert_eq!(count_history(&db).unwrap(), 1);
    }

    #[test]
    fn test_get_history_ordered() {
        let db = test_db();
        for i in 0..5 {
            let row = HistoryRow {
                automation_id: "auto-1".to_string(),
                automation_name: "Test".to_string(),
                executed_at: format!("2026-02-26T10:{:02}:00Z", i),
                trigger_event: "test".to_string(),
                conditions_met: true,
                success: i != 2, // One failure
                error: if i == 2 { Some("test error".to_string()) } else { None },
                trust_score: None,
                decision_outcome: None,
                actions_json: "[]".to_string(),
            };
            insert_history(&db, &row).unwrap();
        }

        let history = get_history(&db, 3).unwrap();
        assert_eq!(history.len(), 3);
        // Newest first
        assert_eq!(history[0].executed_at, "2026-02-26T10:04:00Z");
        assert_eq!(history[2].executed_at, "2026-02-26T10:02:00Z");
    }

    #[test]
    fn test_get_history_by_automation() {
        let db = test_db();
        for auto_id in ["auto-1", "auto-2"] {
            for i in 0..3 {
                let row = HistoryRow {
                    automation_id: auto_id.to_string(),
                    automation_name: format!("Automation {}", auto_id),
                    executed_at: format!("2026-02-26T10:{:02}:00Z", i),
                    trigger_event: "test".to_string(),
                    conditions_met: true,
                    success: true,
                    error: None,
                    trust_score: None,
                    decision_outcome: None,
                    actions_json: "[]".to_string(),
                };
                insert_history(&db, &row).unwrap();
            }
        }

        let auto1_history = get_history_by_automation(&db, "auto-1", 10).unwrap();
        assert_eq!(auto1_history.len(), 3);
        assert!(auto1_history.iter().all(|h| h.automation_id == "auto-1"));
    }

    #[test]
    fn test_failed_execution_with_error() {
        let db = test_db();
        let row = HistoryRow {
            automation_id: "auto-fail".to_string(),
            automation_name: "Failing Auto".to_string(),
            executed_at: "2026-02-26T10:00:00Z".to_string(),
            trigger_event: "plugin.health".to_string(),
            conditions_met: true,
            success: false,
            error: Some("Connection refused".to_string()),
            trust_score: Some(0.3),
            decision_outcome: Some("blocked".to_string()),
            actions_json: r#"[{"action_type":"restart","success":false}]"#.to_string(),
        };
        insert_history(&db, &row).unwrap();

        let history = get_history(&db, 1).unwrap();
        assert_eq!(history.len(), 1);
        assert!(!history[0].success);
        assert_eq!(history[0].error.as_deref(), Some("Connection refused"));
        assert_eq!(history[0].decision_outcome.as_deref(), Some("blocked"));
    }

    #[test]
    fn test_import_from_json() {
        let db = test_db();
        let json = r#"[
            {
                "automation_id": "auto-1",
                "automation_name": "Env Alert",
                "executed_at": "2026-02-26T09:00:00Z",
                "trigger_event": "sensor.humidity",
                "conditions_met": true,
                "actions_executed": [{"action_type": "notify", "success": true, "duration_ms": 50}],
                "success": true
            },
            {
                "automation_id": "auto-2",
                "automation_name": "Plugin Restart",
                "executed_at": "2026-02-26T09:30:00Z",
                "trigger_event": "plugin.health",
                "conditions_met": true,
                "actions_executed": [],
                "success": false,
                "error": "Plugin offline",
                "trust_score": 0.5,
                "decision_outcome": "approved"
            }
        ]"#;
        let count = import_from_json(&db, json).unwrap();
        assert_eq!(count, 2);
        assert_eq!(count_history(&db).unwrap(), 2);
    }
}
