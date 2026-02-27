/**
 * SYMBION KERNEL - Trust Stats SQLite Queries
 *
 * ROLE: Typed queries for trust action stats, agent stats, and global counters.
 * All functions take &Database and return anyhow::Result.
 */

use anyhow::Result;
use rusqlite::params;
use super::Database;

// ============================================================================
// Action Stats
// ============================================================================

pub struct ActionStatsRow {
    pub action_type: String,
    pub total_executions: i64,
    pub successful: i64,
    pub failed: i64,
    pub blocked: i64,
    pub current_trust_modifier: f64,
    pub last_updated: String,
}

pub fn upsert_action_stats(db: &Database, row: &ActionStatsRow) -> Result<()> {
    let conn = db.conn();
    conn.execute(
        "INSERT INTO trust_action_stats
         (action_type, total_executions, successful, failed, blocked, current_trust_modifier, last_updated)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)
         ON CONFLICT(action_type) DO UPDATE SET
             total_executions = excluded.total_executions,
             successful = excluded.successful,
             failed = excluded.failed,
             blocked = excluded.blocked,
             current_trust_modifier = excluded.current_trust_modifier,
             last_updated = excluded.last_updated",
        params![
            row.action_type,
            row.total_executions,
            row.successful,
            row.failed,
            row.blocked,
            row.current_trust_modifier,
            row.last_updated,
        ],
    )?;
    Ok(())
}

pub fn get_action_stats(db: &Database, action_type: &str) -> Result<Option<ActionStatsRow>> {
    let conn = db.conn();
    let mut stmt = conn.prepare(
        "SELECT action_type, total_executions, successful, failed, blocked, current_trust_modifier, last_updated
         FROM trust_action_stats WHERE action_type = ?1"
    )?;

    let mut rows = stmt.query_map(params![action_type], |row| {
        Ok(ActionStatsRow {
            action_type: row.get(0)?,
            total_executions: row.get(1)?,
            successful: row.get(2)?,
            failed: row.get(3)?,
            blocked: row.get(4)?,
            current_trust_modifier: row.get(5)?,
            last_updated: row.get(6)?,
        })
    })?;

    match rows.next() {
        Some(row) => Ok(Some(row?)),
        None => Ok(None),
    }
}

pub fn list_action_stats(db: &Database) -> Result<Vec<ActionStatsRow>> {
    let conn = db.conn();
    let mut stmt = conn.prepare(
        "SELECT action_type, total_executions, successful, failed, blocked, current_trust_modifier, last_updated
         FROM trust_action_stats ORDER BY action_type"
    )?;

    let rows = stmt.query_map([], |row| {
        Ok(ActionStatsRow {
            action_type: row.get(0)?,
            total_executions: row.get(1)?,
            successful: row.get(2)?,
            failed: row.get(3)?,
            blocked: row.get(4)?,
            current_trust_modifier: row.get(5)?,
            last_updated: row.get(6)?,
        })
    })?;

    let mut result = Vec::new();
    for row in rows {
        result.push(row?);
    }
    Ok(result)
}

// ============================================================================
// Agent Stats
// ============================================================================

pub struct AgentStatsRow {
    pub agent_id: String,
    pub total_commands: i64,
    pub successful: i64,
    pub failed: i64,
    pub current_trust_modifier: f64,
    pub last_updated: String,
}

pub fn upsert_agent_stats(db: &Database, row: &AgentStatsRow) -> Result<()> {
    let conn = db.conn();
    conn.execute(
        "INSERT INTO trust_agent_stats
         (agent_id, total_commands, successful, failed, current_trust_modifier, last_updated)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6)
         ON CONFLICT(agent_id) DO UPDATE SET
             total_commands = excluded.total_commands,
             successful = excluded.successful,
             failed = excluded.failed,
             current_trust_modifier = excluded.current_trust_modifier,
             last_updated = excluded.last_updated",
        params![
            row.agent_id,
            row.total_commands,
            row.successful,
            row.failed,
            row.current_trust_modifier,
            row.last_updated,
        ],
    )?;
    Ok(())
}

pub fn get_agent_trust_stats(db: &Database, agent_id: &str) -> Result<Option<AgentStatsRow>> {
    let conn = db.conn();
    let mut stmt = conn.prepare(
        "SELECT agent_id, total_commands, successful, failed, current_trust_modifier, last_updated
         FROM trust_agent_stats WHERE agent_id = ?1"
    )?;

    let mut rows = stmt.query_map(params![agent_id], |row| {
        Ok(AgentStatsRow {
            agent_id: row.get(0)?,
            total_commands: row.get(1)?,
            successful: row.get(2)?,
            failed: row.get(3)?,
            current_trust_modifier: row.get(4)?,
            last_updated: row.get(5)?,
        })
    })?;

    match rows.next() {
        Some(row) => Ok(Some(row?)),
        None => Ok(None),
    }
}

pub fn list_agent_stats(db: &Database) -> Result<Vec<AgentStatsRow>> {
    let conn = db.conn();
    let mut stmt = conn.prepare(
        "SELECT agent_id, total_commands, successful, failed, current_trust_modifier, last_updated
         FROM trust_agent_stats ORDER BY agent_id"
    )?;

    let rows = stmt.query_map([], |row| {
        Ok(AgentStatsRow {
            agent_id: row.get(0)?,
            total_commands: row.get(1)?,
            successful: row.get(2)?,
            failed: row.get(3)?,
            current_trust_modifier: row.get(4)?,
            last_updated: row.get(5)?,
        })
    })?;

    let mut result = Vec::new();
    for row in rows {
        result.push(row?);
    }
    Ok(result)
}

// ============================================================================
// Global Key-Value
// ============================================================================

pub fn get_trust_global(db: &Database, key: &str) -> Result<Option<String>> {
    let conn = db.conn();
    let mut stmt = conn.prepare("SELECT value FROM trust_global WHERE key = ?1")?;
    let mut rows = stmt.query_map(params![key], |row| row.get::<_, String>(0))?;
    match rows.next() {
        Some(val) => Ok(Some(val?)),
        None => Ok(None),
    }
}

pub fn set_trust_global(db: &Database, key: &str, value: &str) -> Result<()> {
    let conn = db.conn();
    conn.execute(
        "INSERT INTO trust_global (key, value) VALUES (?1, ?2)
         ON CONFLICT(key) DO UPDATE SET value = excluded.value",
        params![key, value],
    )?;
    Ok(())
}

pub fn count_action_stats(db: &Database) -> Result<i64> {
    let conn = db.conn();
    conn.query_row("SELECT COUNT(*) FROM trust_action_stats", [], |row| row.get(0))
        .map_err(Into::into)
}

pub fn import_trust_json(db: &Database, json: &str) -> Result<usize> {
    let data: serde_json::Value = serde_json::from_str(json)?;
    let mut conn = db.conn();
    let tx = conn.transaction()?;
    let mut count = 0;

    // Import action_stats
    if let Some(action_stats) = data.get("action_stats").and_then(|v| v.as_object()) {
        let mut stmt = tx.prepare_cached(
            "INSERT OR IGNORE INTO trust_action_stats
             (action_type, total_executions, successful, failed, blocked, current_trust_modifier, last_updated)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)"
        )?;

        for (action_type, stats) in action_stats {
            let total = stats.get("total_executions").and_then(|v| v.as_i64()).unwrap_or(0);
            let success = stats.get("successful").and_then(|v| v.as_i64()).unwrap_or(0);
            let failed = stats.get("failed").and_then(|v| v.as_i64()).unwrap_or(0);
            let blocked = stats.get("blocked").and_then(|v| v.as_i64()).unwrap_or(0);
            let modifier = stats.get("current_trust_modifier").and_then(|v| v.as_f64()).unwrap_or(0.0);
            let updated = stats.get("last_updated").and_then(|v| v.as_str()).unwrap_or("").to_string();

            stmt.execute(params![action_type, total, success, failed, blocked, modifier, updated])?;
            count += 1;
        }
    }

    // Import agent_stats
    if let Some(agent_stats) = data.get("agent_stats").and_then(|v| v.as_object()) {
        let mut stmt = tx.prepare_cached(
            "INSERT OR IGNORE INTO trust_agent_stats
             (agent_id, total_commands, successful, failed, current_trust_modifier, last_updated)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6)"
        )?;

        for (agent_id, stats) in agent_stats {
            let total = stats.get("total_commands").and_then(|v| v.as_i64()).unwrap_or(0);
            let success = stats.get("successful").and_then(|v| v.as_i64()).unwrap_or(0);
            let failed = stats.get("failed").and_then(|v| v.as_i64()).unwrap_or(0);
            let modifier = stats.get("current_trust_modifier").and_then(|v| v.as_f64()).unwrap_or(0.0);
            let updated = stats.get("last_updated").and_then(|v| v.as_str()).unwrap_or("").to_string();

            stmt.execute(params![agent_id, total, success, failed, modifier, updated])?;
            count += 1;
        }
    }

    // Import global counters
    if let Some(total_decisions) = data.get("total_decisions").and_then(|v| v.as_u64()) {
        tx.execute(
            "INSERT OR REPLACE INTO trust_global (key, value) VALUES ('total_decisions', ?1)",
            params![total_decisions.to_string()],
        )?;
    }
    if let Some(last_updated) = data.get("last_updated").and_then(|v| v.as_str()) {
        tx.execute(
            "INSERT OR REPLACE INTO trust_global (key, value) VALUES ('last_updated', ?1)",
            params![last_updated],
        )?;
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

    // --- Action Stats ---

    #[test]
    fn test_upsert_and_get_action_stats() {
        let db = test_db();
        let row = ActionStatsRow {
            action_type: "change_mode".to_string(),
            total_executions: 10,
            successful: 8,
            failed: 1,
            blocked: 1,
            current_trust_modifier: 0.05,
            last_updated: "2026-02-26T10:00:00Z".to_string(),
        };
        upsert_action_stats(&db, &row).unwrap();

        let loaded = get_action_stats(&db, "change_mode").unwrap().unwrap();
        assert_eq!(loaded.total_executions, 10);
        assert_eq!(loaded.successful, 8);
        assert!((loaded.current_trust_modifier - 0.05).abs() < 0.001);
    }

    #[test]
    fn test_upsert_updates_action_stats() {
        let db = test_db();
        let row = ActionStatsRow {
            action_type: "restart".to_string(),
            total_executions: 5,
            successful: 5,
            failed: 0,
            blocked: 0,
            current_trust_modifier: 0.1,
            last_updated: "2026-02-26T10:00:00Z".to_string(),
        };
        upsert_action_stats(&db, &row).unwrap();

        let updated = ActionStatsRow {
            action_type: "restart".to_string(),
            total_executions: 6,
            successful: 5,
            failed: 1,
            blocked: 0,
            current_trust_modifier: 0.05,
            last_updated: "2026-02-26T11:00:00Z".to_string(),
        };
        upsert_action_stats(&db, &updated).unwrap();

        assert_eq!(count_action_stats(&db).unwrap(), 1);
        let loaded = get_action_stats(&db, "restart").unwrap().unwrap();
        assert_eq!(loaded.total_executions, 6);
        assert_eq!(loaded.failed, 1);
    }

    // --- Agent Stats ---

    #[test]
    fn test_upsert_and_get_agent_stats() {
        let db = test_db();
        let row = AgentStatsRow {
            agent_id: "pc-salon".to_string(),
            total_commands: 100,
            successful: 98,
            failed: 2,
            current_trust_modifier: 0.15,
            last_updated: "2026-02-26T10:00:00Z".to_string(),
        };
        upsert_agent_stats(&db, &row).unwrap();

        let loaded = get_agent_trust_stats(&db, "pc-salon").unwrap().unwrap();
        assert_eq!(loaded.total_commands, 100);
        assert_eq!(loaded.successful, 98);
    }

    #[test]
    fn test_list_agent_stats() {
        let db = test_db();
        for id in ["agent-a", "agent-b"] {
            let row = AgentStatsRow {
                agent_id: id.to_string(),
                total_commands: 10,
                successful: 10,
                failed: 0,
                current_trust_modifier: 0.0,
                last_updated: "2026-02-26T10:00:00Z".to_string(),
            };
            upsert_agent_stats(&db, &row).unwrap();
        }

        let list = list_agent_stats(&db).unwrap();
        assert_eq!(list.len(), 2);
    }

    // --- Global ---

    #[test]
    fn test_global_kv() {
        let db = test_db();
        set_trust_global(&db, "total_decisions", "42").unwrap();
        let val = get_trust_global(&db, "total_decisions").unwrap();
        assert_eq!(val.as_deref(), Some("42"));

        // Update
        set_trust_global(&db, "total_decisions", "43").unwrap();
        let val = get_trust_global(&db, "total_decisions").unwrap();
        assert_eq!(val.as_deref(), Some("43"));
    }

    // --- Import ---

    #[test]
    fn test_import_trust_json() {
        let db = test_db();
        let json = r#"{
            "action_stats": {
                "change_mode": {
                    "total_executions": 10,
                    "successful": 8,
                    "failed": 1,
                    "blocked": 1,
                    "current_trust_modifier": 0.05,
                    "last_updated": "2026-02-26T10:00:00Z"
                }
            },
            "agent_stats": {
                "pc-salon": {
                    "total_commands": 50,
                    "successful": 48,
                    "failed": 2,
                    "current_trust_modifier": 0.1,
                    "last_updated": "2026-02-26T10:00:00Z"
                }
            },
            "total_decisions": 100,
            "last_updated": "2026-02-26T10:00:00Z"
        }"#;
        let count = import_trust_json(&db, json).unwrap();
        assert_eq!(count, 2); // 1 action + 1 agent

        assert_eq!(count_action_stats(&db).unwrap(), 1);
        let global = get_trust_global(&db, "total_decisions").unwrap();
        assert_eq!(global.as_deref(), Some("100"));
    }
}
