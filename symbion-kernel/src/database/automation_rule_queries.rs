/**
 * SYMBION KERNEL - Automation Rules SQLite Queries
 *
 * ROLE: Typed queries for automation rule definitions (not history — that's in automation_queries.rs).
 * Complex fields (triggers, conditions, actions) stored as JSON columns.
 * All functions take &Database and return anyhow::Result.
 */

use anyhow::Result;
use rusqlite::params;
use super::Database;

pub struct AutomationRuleRow {
    pub id: String,
    pub name: String,
    pub description: Option<String>,
    pub category: Option<String>,
    pub goal_mode: Option<String>,
    pub enabled: bool,
    pub triggers_json: Option<String>,
    pub conditions_json: Option<String>,
    pub actions_json: String,
    pub cooldown_seconds: i32,
    pub trusted: Option<bool>,
    pub skip_if_same_mode: Option<bool>,
    pub auto_created: Option<bool>,
    pub last_executed_at: Option<String>,
    pub execution_count: i64,
    pub created_at: Option<String>,
    pub updated_at: Option<String>,
    pub deleted_at: Option<String>,
}

pub fn upsert_automation(db: &Database, row: &AutomationRuleRow) -> Result<()> {
    let conn = db.conn();
    conn.execute(
        "INSERT INTO automations
         (id, name, description, category, goal_mode, enabled,
          triggers_json, conditions_json, actions_json, cooldown_seconds,
          trusted, skip_if_same_mode, auto_created,
          last_executed_at, execution_count, created_at, updated_at, deleted_at)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15, ?16, ?17, ?18)
         ON CONFLICT(id) DO UPDATE SET
             name = excluded.name,
             description = excluded.description,
             category = excluded.category,
             goal_mode = excluded.goal_mode,
             enabled = excluded.enabled,
             triggers_json = excluded.triggers_json,
             conditions_json = excluded.conditions_json,
             actions_json = excluded.actions_json,
             cooldown_seconds = excluded.cooldown_seconds,
             trusted = excluded.trusted,
             skip_if_same_mode = excluded.skip_if_same_mode,
             auto_created = excluded.auto_created,
             last_executed_at = excluded.last_executed_at,
             execution_count = excluded.execution_count,
             updated_at = excluded.updated_at,
             deleted_at = excluded.deleted_at",
        params![
            row.id,
            row.name,
            row.description,
            row.category,
            row.goal_mode,
            row.enabled as i32,
            row.triggers_json,
            row.conditions_json,
            row.actions_json,
            row.cooldown_seconds,
            row.trusted.map(|b| b as i32),
            row.skip_if_same_mode.map(|b| b as i32),
            row.auto_created.map(|b| b as i32),
            row.last_executed_at,
            row.execution_count,
            row.created_at,
            row.updated_at,
            row.deleted_at,
        ],
    )?;
    Ok(())
}

pub fn get_automation(db: &Database, id: &str) -> Result<Option<AutomationRuleRow>> {
    let conn = db.conn();
    let mut stmt = conn.prepare(
        "SELECT id, name, description, category, goal_mode, enabled,
                triggers_json, conditions_json, actions_json, cooldown_seconds,
                trusted, skip_if_same_mode, auto_created,
                last_executed_at, execution_count, created_at, updated_at, deleted_at
         FROM automations WHERE id = ?1"
    )?;

    let mut rows = stmt.query_map(params![id], map_automation_row)?;
    match rows.next() {
        Some(row) => Ok(Some(row?)),
        None => Ok(None),
    }
}

/// List automations excluding soft-deleted.
pub fn list_automations(db: &Database) -> Result<Vec<AutomationRuleRow>> {
    let conn = db.conn();
    let mut stmt = conn.prepare(
        "SELECT id, name, description, category, goal_mode, enabled,
                triggers_json, conditions_json, actions_json, cooldown_seconds,
                trusted, skip_if_same_mode, auto_created,
                last_executed_at, execution_count, created_at, updated_at, deleted_at
         FROM automations WHERE deleted_at IS NULL
         ORDER BY name"
    )?;

    let rows = stmt.query_map([], map_automation_row)?;

    let mut result = Vec::new();
    for row in rows {
        result.push(row?);
    }
    Ok(result)
}

pub fn delete_automation(db: &Database, id: &str, deleted_at: &str) -> Result<bool> {
    let conn = db.conn();
    let updated = conn.execute(
        "UPDATE automations SET deleted_at = ?1 WHERE id = ?2 AND deleted_at IS NULL",
        params![deleted_at, id],
    )?;
    Ok(updated > 0)
}

pub fn count_automations(db: &Database) -> Result<i64> {
    let conn = db.conn();
    conn.query_row(
        "SELECT COUNT(*) FROM automations WHERE deleted_at IS NULL",
        [],
        |row| row.get(0),
    ).map_err(Into::into)
}

pub fn import_automations_json(db: &Database, json: &str) -> Result<usize> {
    let automations: std::collections::HashMap<String, serde_json::Value> = serde_json::from_str(json)?;

    let mut conn = db.conn();
    let tx = conn.transaction()?;
    let mut count = 0;

    {
        let mut stmt = tx.prepare_cached(
            "INSERT OR IGNORE INTO automations
             (id, name, description, category, goal_mode, enabled,
              triggers_json, conditions_json, actions_json, cooldown_seconds,
              trusted, skip_if_same_mode, auto_created,
              last_executed_at, execution_count, created_at, updated_at, deleted_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15, ?16, ?17, ?18)"
        )?;

        for (id, auto) in &automations {
            let name = auto.get("name").and_then(|v| v.as_str()).unwrap_or("").to_string();
            let description = auto.get("description").and_then(|v| v.as_str()).map(|s| s.to_string());
            let category = auto.get("category").and_then(|v| v.as_str()).map(|s| s.to_string());
            let goal_mode = auto.get("goal_mode").and_then(|v| v.as_str()).map(|s| s.to_string());
            let enabled = auto.get("enabled").and_then(|v| v.as_bool()).unwrap_or(true);

            // Merge legacy trigger + triggers into triggers_json
            let triggers = auto.get("triggers")
                .or_else(|| auto.get("trigger"))
                .map(|v| v.to_string());
            let conditions = auto.get("conditions").map(|v| v.to_string());
            let actions = auto.get("actions").map(|v| v.to_string()).unwrap_or_else(|| "[]".to_string());
            let cooldown = auto.get("cooldown_seconds").and_then(|v| v.as_i64()).unwrap_or(300) as i32;
            let trusted = auto.get("trusted").and_then(|v| v.as_bool()).map(|b| b as i32);
            let skip = auto.get("skip_if_same_mode").and_then(|v| v.as_bool()).map(|b| b as i32);
            let auto_created = auto.get("auto_created").and_then(|v| v.as_bool()).map(|b| b as i32);
            let last_exec = auto.get("last_executed_at").and_then(|v| v.as_str()).map(|s| s.to_string());
            let exec_count = auto.get("execution_count").and_then(|v| v.as_i64()).unwrap_or(0);
            let created = auto.get("created_at").and_then(|v| v.as_str()).map(|s| s.to_string());
            let updated = auto.get("updated_at").and_then(|v| v.as_str()).map(|s| s.to_string());
            let deleted = auto.get("deleted_at").and_then(|v| v.as_str()).map(|s| s.to_string());

            stmt.execute(params![
                id, name, description, category, goal_mode, enabled as i32,
                triggers, conditions, actions, cooldown,
                trusted, skip, auto_created,
                last_exec, exec_count, created, updated, deleted,
            ])?;
            count += 1;
        }
    }

    tx.commit()?;
    Ok(count)
}

fn map_automation_row(row: &rusqlite::Row<'_>) -> rusqlite::Result<AutomationRuleRow> {
    Ok(AutomationRuleRow {
        id: row.get(0)?,
        name: row.get(1)?,
        description: row.get(2)?,
        category: row.get(3)?,
        goal_mode: row.get(4)?,
        enabled: row.get::<_, i32>(5)? != 0,
        triggers_json: row.get(6)?,
        conditions_json: row.get(7)?,
        actions_json: row.get(8)?,
        cooldown_seconds: row.get(9)?,
        trusted: row.get::<_, Option<i32>>(10)?.map(|v| v != 0),
        skip_if_same_mode: row.get::<_, Option<i32>>(11)?.map(|v| v != 0),
        auto_created: row.get::<_, Option<i32>>(12)?.map(|v| v != 0),
        last_executed_at: row.get(13)?,
        execution_count: row.get(14)?,
        created_at: row.get(15)?,
        updated_at: row.get(16)?,
        deleted_at: row.get(17)?,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_db() -> Database {
        Database::open_in_memory().unwrap()
    }

    fn sample_rule(id: &str) -> AutomationRuleRow {
        AutomationRuleRow {
            id: id.to_string(),
            name: "Test Automation".to_string(),
            description: Some("Test description".to_string()),
            category: Some("custom".to_string()),
            goal_mode: Some("pro".to_string()),
            enabled: true,
            triggers_json: Some(r#"{"type":"mode_change","to_mode":"pro"}"#.to_string()),
            conditions_json: None,
            actions_json: r#"[{"type":"change_mode","target":"focus"}]"#.to_string(),
            cooldown_seconds: 300,
            trusted: Some(true),
            skip_if_same_mode: Some(false),
            auto_created: None,
            last_executed_at: None,
            execution_count: 0,
            created_at: Some("2026-02-26T10:00:00Z".to_string()),
            updated_at: None,
            deleted_at: None,
        }
    }

    #[test]
    fn test_upsert_and_get() {
        let db = test_db();
        upsert_automation(&db, &sample_rule("auto-1")).unwrap();

        let loaded = get_automation(&db, "auto-1").unwrap().unwrap();
        assert_eq!(loaded.name, "Test Automation");
        assert!(loaded.enabled);
        assert_eq!(loaded.trusted, Some(true));
    }

    #[test]
    fn test_upsert_updates_existing() {
        let db = test_db();
        upsert_automation(&db, &sample_rule("auto-1")).unwrap();

        let mut updated = sample_rule("auto-1");
        updated.name = "Updated".to_string();
        updated.execution_count = 5;
        updated.enabled = false;
        upsert_automation(&db, &updated).unwrap();

        assert_eq!(count_automations(&db).unwrap(), 1);
        let loaded = get_automation(&db, "auto-1").unwrap().unwrap();
        assert_eq!(loaded.name, "Updated");
        assert_eq!(loaded.execution_count, 5);
        assert!(!loaded.enabled);
    }

    #[test]
    fn test_list_excludes_deleted() {
        let db = test_db();
        upsert_automation(&db, &sample_rule("a1")).unwrap();
        upsert_automation(&db, &sample_rule("a2")).unwrap();

        delete_automation(&db, "a1", "2026-02-26T12:00:00Z").unwrap();

        let active = list_automations(&db).unwrap();
        assert_eq!(active.len(), 1);
        assert_eq!(active[0].id, "a2");
    }

    #[test]
    fn test_import_automations_json() {
        let db = test_db();
        let json = r#"{
            "auto-1": {
                "name": "Morning Mode",
                "description": "Switch to pro in morning",
                "category": "modes",
                "enabled": true,
                "triggers": {"type": "schedule", "time": "08:00"},
                "actions": [{"type": "change_mode", "target": "pro"}],
                "cooldown_seconds": 300,
                "trusted": true,
                "execution_count": 42,
                "created_at": "2026-01-01T00:00:00Z"
            }
        }"#;
        let count = import_automations_json(&db, json).unwrap();
        assert_eq!(count, 1);
        assert_eq!(count_automations(&db).unwrap(), 1);

        let loaded = get_automation(&db, "auto-1").unwrap().unwrap();
        assert_eq!(loaded.execution_count, 42);
    }
}
