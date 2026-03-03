/**
 * Command History SQLite queries
 *
 * CRUD operations for persisting command audit trail.
 * Follows dual-write pattern: HashMap in-memory (existing) + SQLite (new).
 */

use anyhow::{Context, Result};
use super::Database;

/// Row representation of a command history entry.
#[derive(Debug, Clone)]
pub struct CommandHistoryRow {
    pub command_id: String,
    pub agent_id: String,
    pub command_type: String,
    pub parameters_json: Option<String>,
    pub status: String,
    pub output_json: Option<String>,
    pub error_json: Option<String>,
    pub timeout_seconds: i64,
    pub created_at: String,
    pub updated_at: String,
    pub completed_at: Option<String>,
}

/// Insert a new command into history.
pub fn insert_command(db: &Database, row: &CommandHistoryRow) -> Result<()> {
    let conn = db.conn();
    conn.execute(
        "INSERT OR REPLACE INTO command_history
         (command_id, agent_id, command_type, parameters_json, status,
          output_json, error_json, timeout_seconds, created_at, updated_at, completed_at)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11)",
        rusqlite::params![
            row.command_id, row.agent_id, row.command_type,
            row.parameters_json, row.status,
            row.output_json, row.error_json, row.timeout_seconds,
            row.created_at, row.updated_at, row.completed_at,
        ],
    ).context("Failed to insert command history")?;
    Ok(())
}

/// Update a command's status and output/error after completion.
pub fn update_command_status(
    db: &Database,
    command_id: &str,
    status: &str,
    output_json: Option<&str>,
    error_json: Option<&str>,
    completed_at: Option<&str>,
) -> Result<bool> {
    let conn = db.conn();
    let now = time::OffsetDateTime::now_utc()
        .format(&time::format_description::well_known::Rfc3339)
        .unwrap_or_default();
    let rows = conn.execute(
        "UPDATE command_history SET status = ?1, output_json = ?2, error_json = ?3,
         updated_at = ?4, completed_at = ?5 WHERE command_id = ?6",
        rusqlite::params![status, output_json, error_json, now, completed_at, command_id],
    ).context("Failed to update command status")?;
    Ok(rows > 0)
}

/// Get command history for a specific agent, newest first.
pub fn get_agent_history(
    db: &Database,
    agent_id: &str,
    limit: i64,
    offset: i64,
) -> Result<Vec<CommandHistoryRow>> {
    let conn = db.conn();
    let mut stmt = conn.prepare(
        "SELECT command_id, agent_id, command_type, parameters_json, status,
                output_json, error_json, timeout_seconds, created_at, updated_at, completed_at
         FROM command_history WHERE agent_id = ?1
         ORDER BY created_at DESC LIMIT ?2 OFFSET ?3"
    ).context("Failed to prepare get_agent_history")?;

    let rows = stmt.query_map(rusqlite::params![agent_id, limit, offset], |row| {
        Ok(CommandHistoryRow {
            command_id: row.get(0)?,
            agent_id: row.get(1)?,
            command_type: row.get(2)?,
            parameters_json: row.get(3)?,
            status: row.get(4)?,
            output_json: row.get(5)?,
            error_json: row.get(6)?,
            timeout_seconds: row.get(7)?,
            created_at: row.get(8)?,
            updated_at: row.get(9)?,
            completed_at: row.get(10)?,
        })
    }).context("Failed to query agent history")?;

    rows.collect::<Result<Vec<_>, _>>().context("Failed to collect history rows")
}

/// Delete entries older than N days.
pub fn cleanup_old_entries(db: &Database, max_age_days: i64) -> Result<usize> {
    let conn = db.conn();
    let cutoff = time::OffsetDateTime::now_utc() - time::Duration::days(max_age_days);
    let cutoff_str = cutoff.format(&time::format_description::well_known::Rfc3339)
        .unwrap_or_default();
    let rows = conn.execute(
        "DELETE FROM command_history WHERE created_at < ?1",
        rusqlite::params![cutoff_str],
    ).context("Failed to cleanup old command history")?;
    Ok(rows)
}

/// Count total entries for an agent (used for pagination).
pub fn count_agent_history(db: &Database, agent_id: &str) -> Result<i64> {
    let conn = db.conn();
    conn.query_row(
        "SELECT COUNT(*) FROM command_history WHERE agent_id = ?1",
        rusqlite::params![agent_id],
        |row| row.get(0),
    ).context("Failed to count agent history")
}

#[cfg(test)]
mod tests {
    use super::*;

    fn setup_db() -> std::sync::Arc<Database> {
        std::sync::Arc::new(Database::open_in_memory().unwrap())
    }

    fn now_rfc3339() -> String {
        time::OffsetDateTime::now_utc()
            .format(&time::format_description::well_known::Rfc3339)
            .unwrap()
    }

    fn make_row(id: &str, agent: &str, cmd_type: &str, status: &str) -> CommandHistoryRow {
        let now = now_rfc3339();
        CommandHistoryRow {
            command_id: id.to_string(),
            agent_id: agent.to_string(),
            command_type: cmd_type.to_string(),
            parameters_json: None,
            status: status.to_string(),
            output_json: None,
            error_json: None,
            timeout_seconds: 30,
            created_at: now.clone(),
            updated_at: now,
            completed_at: None,
        }
    }

    #[test]
    fn test_command_history_insert_get() {
        let db = setup_db();
        let row = make_row("cmd-1", "agent-a", "shutdown", "Sent");
        insert_command(&db, &row).unwrap();
        let history = get_agent_history(&db, "agent-a", 50, 0).unwrap();
        assert_eq!(history.len(), 1);
        assert_eq!(history[0].command_id, "cmd-1");
        assert_eq!(history[0].command_type, "shutdown");
    }

    #[test]
    fn test_command_history_update_status() {
        let db = setup_db();
        let row = make_row("cmd-2", "agent-b", "reboot", "Sent");
        insert_command(&db, &row).unwrap();
        let updated = update_command_status(
            &db, "cmd-2", "Completed",
            Some(r#"{"message":"done"}"#), None, Some(&now_rfc3339()),
        ).unwrap();
        assert!(updated);
        let history = get_agent_history(&db, "agent-b", 50, 0).unwrap();
        assert_eq!(history[0].status, "Completed");
        assert!(history[0].output_json.is_some());
    }

    #[test]
    fn test_command_history_pagination() {
        let db = setup_db();
        for i in 0..10 {
            let row = make_row(&format!("cmd-{}", i), "agent-c", "run_command", "Completed");
            insert_command(&db, &row).unwrap();
        }
        let page1 = get_agent_history(&db, "agent-c", 3, 0).unwrap();
        assert_eq!(page1.len(), 3);
        let page2 = get_agent_history(&db, "agent-c", 3, 3).unwrap();
        assert_eq!(page2.len(), 3);
        let total = count_agent_history(&db, "agent-c").unwrap();
        assert_eq!(total, 10);
    }

    #[test]
    fn test_command_history_cleanup_30_days() {
        let db = setup_db();
        // Insert with old timestamp
        let old_time = (time::OffsetDateTime::now_utc() - time::Duration::days(31))
            .format(&time::format_description::well_known::Rfc3339)
            .unwrap();
        let row = CommandHistoryRow {
            command_id: "old-cmd".to_string(),
            agent_id: "agent-d".to_string(),
            command_type: "shutdown".to_string(),
            parameters_json: None,
            status: "Completed".to_string(),
            output_json: None,
            error_json: None,
            timeout_seconds: 30,
            created_at: old_time,
            updated_at: now_rfc3339(),
            completed_at: None,
        };
        insert_command(&db, &row).unwrap();
        // Insert a recent one
        let recent = make_row("new-cmd", "agent-d", "reboot", "Completed");
        insert_command(&db, &recent).unwrap();

        let deleted = cleanup_old_entries(&db, 30).unwrap();
        assert_eq!(deleted, 1);
        let remaining = get_agent_history(&db, "agent-d", 50, 0).unwrap();
        assert_eq!(remaining.len(), 1);
        assert_eq!(remaining[0].command_id, "new-cmd");
    }
}
