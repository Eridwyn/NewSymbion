/**
 * SYMBION KERNEL - Config SQLite Queries (Modes, Schedule, Notification Configs)
 *
 * ROLE: Typed queries for configuration entities.
 * All functions take &Database and return anyhow::Result.
 */

use anyhow::Result;
use rusqlite::params;
use super::Database;

// ============================================================================
// Modes
// ============================================================================

pub struct ModeRow {
    pub id: String,
    pub name: String,
    pub slug: String,
    pub icon: String,
    pub theme_json: String,
    pub is_system: bool,
    pub created_at: String,
    pub display_order: i32,
}

pub fn upsert_mode(db: &Database, row: &ModeRow) -> Result<()> {
    let conn = db.conn();
    conn.execute(
        "INSERT INTO modes (id, name, slug, icon, theme_json, is_system, created_at, display_order)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)
         ON CONFLICT(id) DO UPDATE SET
             name = excluded.name,
             slug = excluded.slug,
             icon = excluded.icon,
             theme_json = excluded.theme_json,
             is_system = excluded.is_system,
             display_order = excluded.display_order",
        params![
            row.id,
            row.name,
            row.slug,
            row.icon,
            row.theme_json,
            row.is_system as i32,
            row.created_at,
            row.display_order,
        ],
    )?;
    Ok(())
}

pub fn list_modes(db: &Database) -> Result<Vec<ModeRow>> {
    let conn = db.conn();
    let mut stmt = conn.prepare(
        "SELECT id, name, slug, icon, theme_json, is_system, created_at, display_order
         FROM modes ORDER BY display_order, name"
    )?;

    let rows = stmt.query_map([], |row| {
        Ok(ModeRow {
            id: row.get(0)?,
            name: row.get(1)?,
            slug: row.get(2)?,
            icon: row.get(3)?,
            theme_json: row.get(4)?,
            is_system: row.get::<_, i32>(5)? != 0,
            created_at: row.get(6)?,
            display_order: row.get(7)?,
        })
    })?;

    let mut result = Vec::new();
    for row in rows {
        result.push(row?);
    }
    Ok(result)
}

pub fn get_mode(db: &Database, id: &str) -> Result<Option<ModeRow>> {
    let conn = db.conn();
    let mut stmt = conn.prepare(
        "SELECT id, name, slug, icon, theme_json, is_system, created_at, display_order
         FROM modes WHERE id = ?1"
    )?;

    let mut rows = stmt.query_map(params![id], |row| {
        Ok(ModeRow {
            id: row.get(0)?,
            name: row.get(1)?,
            slug: row.get(2)?,
            icon: row.get(3)?,
            theme_json: row.get(4)?,
            is_system: row.get::<_, i32>(5)? != 0,
            created_at: row.get(6)?,
            display_order: row.get(7)?,
        })
    })?;

    match rows.next() {
        Some(row) => Ok(Some(row?)),
        None => Ok(None),
    }
}

pub fn delete_mode(db: &Database, id: &str) -> Result<bool> {
    let conn = db.conn();
    let deleted = conn.execute("DELETE FROM modes WHERE id = ?1", params![id])?;
    Ok(deleted > 0)
}

pub fn count_modes(db: &Database) -> Result<i64> {
    let conn = db.conn();
    conn.query_row("SELECT COUNT(*) FROM modes", [], |row| row.get(0))
        .map_err(Into::into)
}

pub fn import_modes_json(db: &Database, json: &str) -> Result<usize> {
    let modes: std::collections::HashMap<String, serde_json::Value> = serde_json::from_str(json)?;

    let mut conn = db.conn();
    let tx = conn.transaction()?;
    let mut count = 0;

    {
        let mut stmt = tx.prepare_cached(
            "INSERT OR IGNORE INTO modes
             (id, name, slug, icon, theme_json, is_system, created_at, display_order)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)"
        )?;

        for (id, mode) in &modes {
            let name = mode.get("name").and_then(|v| v.as_str()).unwrap_or("").to_string();
            let slug = mode.get("slug").and_then(|v| v.as_str()).unwrap_or("").to_string();
            let icon = mode.get("icon").and_then(|v| v.as_str()).unwrap_or("").to_string();
            let theme = mode.get("theme").map(|v| v.to_string()).unwrap_or_else(|| "{}".to_string());
            let is_system = mode.get("is_system").and_then(|v| v.as_bool()).unwrap_or(false);
            let created_at = mode.get("created_at").and_then(|v| v.as_str()).unwrap_or("").to_string();
            let display_order = mode.get("display_order").and_then(|v| v.as_i64()).unwrap_or(0) as i32;

            stmt.execute(params![id, name, slug, icon, theme, is_system as i32, created_at, display_order])?;
            count += 1;
        }
    }

    tx.commit()?;
    Ok(count)
}

// ============================================================================
// Schedule Rules
// ============================================================================

pub struct ScheduleRuleRow {
    pub id: String,
    pub mode_id: String,
    pub days_json: String,
    pub start_time: String,
    pub end_time: String,
    pub priority: i32,
    pub enabled: bool,
    pub name: Option<String>,
    pub created_at: String,
}

pub fn upsert_schedule_rule(db: &Database, row: &ScheduleRuleRow) -> Result<()> {
    let conn = db.conn();
    conn.execute(
        "INSERT INTO schedule_rules
         (id, mode_id, days_json, start_time, end_time, priority, enabled, name, created_at)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)
         ON CONFLICT(id) DO UPDATE SET
             mode_id = excluded.mode_id,
             days_json = excluded.days_json,
             start_time = excluded.start_time,
             end_time = excluded.end_time,
             priority = excluded.priority,
             enabled = excluded.enabled,
             name = excluded.name",
        params![
            row.id,
            row.mode_id,
            row.days_json,
            row.start_time,
            row.end_time,
            row.priority,
            row.enabled as i32,
            row.name,
            row.created_at,
        ],
    )?;
    Ok(())
}

pub fn list_schedule_rules(db: &Database) -> Result<Vec<ScheduleRuleRow>> {
    let conn = db.conn();
    let mut stmt = conn.prepare(
        "SELECT id, mode_id, days_json, start_time, end_time, priority, enabled, name, created_at
         FROM schedule_rules ORDER BY priority DESC"
    )?;

    let rows = stmt.query_map([], |row| {
        Ok(ScheduleRuleRow {
            id: row.get(0)?,
            mode_id: row.get(1)?,
            days_json: row.get(2)?,
            start_time: row.get(3)?,
            end_time: row.get(4)?,
            priority: row.get(5)?,
            enabled: row.get::<_, i32>(6)? != 0,
            name: row.get(7)?,
            created_at: row.get(8)?,
        })
    })?;

    let mut result = Vec::new();
    for row in rows {
        result.push(row?);
    }
    Ok(result)
}

pub fn delete_schedule_rule(db: &Database, id: &str) -> Result<bool> {
    let conn = db.conn();
    let deleted = conn.execute("DELETE FROM schedule_rules WHERE id = ?1", params![id])?;
    Ok(deleted > 0)
}

pub fn count_schedule_rules(db: &Database) -> Result<i64> {
    let conn = db.conn();
    conn.query_row("SELECT COUNT(*) FROM schedule_rules", [], |row| row.get(0))
        .map_err(Into::into)
}

/// Get a schedule config value by key.
pub fn get_schedule_config(db: &Database, key: &str) -> Result<Option<String>> {
    let conn = db.conn();
    let mut stmt = conn.prepare("SELECT value FROM schedule_config WHERE key = ?1")?;
    let mut rows = stmt.query_map(params![key], |row| row.get::<_, String>(0))?;
    match rows.next() {
        Some(val) => Ok(Some(val?)),
        None => Ok(None),
    }
}

/// Set a schedule config value (upsert).
pub fn set_schedule_config(db: &Database, key: &str, value: &str) -> Result<()> {
    let conn = db.conn();
    conn.execute(
        "INSERT INTO schedule_config (key, value) VALUES (?1, ?2)
         ON CONFLICT(key) DO UPDATE SET value = excluded.value",
        params![key, value],
    )?;
    Ok(())
}

pub fn import_schedule_json(db: &Database, json: &str) -> Result<usize> {
    let data: serde_json::Value = serde_json::from_str(json)?;

    let mut conn = db.conn();
    let tx = conn.transaction()?;
    let mut count = 0;

    // Import rules
    if let Some(rules) = data.get("rules").and_then(|v| v.as_array()) {
        let mut stmt = tx.prepare_cached(
            "INSERT OR IGNORE INTO schedule_rules
             (id, mode_id, days_json, start_time, end_time, priority, enabled, name, created_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)"
        )?;

        for rule in rules {
            let id = rule.get("id").and_then(|v| v.as_str()).unwrap_or("").to_string();
            let mode_id = rule.get("mode_id").and_then(|v| v.as_str()).unwrap_or("").to_string();
            let days = rule.get("days").map(|v| v.to_string()).unwrap_or_else(|| "[]".to_string());
            let start = rule.get("start_time").and_then(|v| v.as_str()).unwrap_or("").to_string();
            let end = rule.get("end_time").and_then(|v| v.as_str()).unwrap_or("").to_string();
            let priority = rule.get("priority").and_then(|v| v.as_i64()).unwrap_or(0) as i32;
            let enabled = rule.get("enabled").and_then(|v| v.as_bool()).unwrap_or(true);
            let name = rule.get("name").and_then(|v| v.as_str()).map(|s| s.to_string());
            let created_at = rule.get("created_at").and_then(|v| v.as_str()).unwrap_or("").to_string();

            if !id.is_empty() {
                stmt.execute(params![id, mode_id, days, start, end, priority, enabled as i32, name, created_at])?;
                count += 1;
            }
        }
    }

    // Import default_mode_id config
    if let Some(default_mode) = data.get("default_mode_id").and_then(|v| v.as_str()) {
        tx.execute(
            "INSERT OR REPLACE INTO schedule_config (key, value) VALUES ('default_mode_id', ?1)",
            params![default_mode],
        )?;
    }

    tx.commit()?;
    Ok(count)
}

// ============================================================================
// Notification Configs
// ============================================================================

pub struct NotifConfigRow {
    pub type_id: String,
    pub display_name: String,
    pub description: String,
    pub category: String,
    pub enabled: bool,
    pub title_template: String,
    pub body_template: String,
    pub priority: String,
    pub available_variables_json: String,
}

pub fn upsert_notif_config(db: &Database, row: &NotifConfigRow) -> Result<()> {
    let conn = db.conn();
    conn.execute(
        "INSERT INTO notification_configs
         (type_id, display_name, description, category, enabled,
          title_template, body_template, priority, available_variables_json)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)
         ON CONFLICT(type_id) DO UPDATE SET
             display_name = excluded.display_name,
             description = excluded.description,
             category = excluded.category,
             enabled = excluded.enabled,
             title_template = excluded.title_template,
             body_template = excluded.body_template,
             priority = excluded.priority,
             available_variables_json = excluded.available_variables_json",
        params![
            row.type_id,
            row.display_name,
            row.description,
            row.category,
            row.enabled as i32,
            row.title_template,
            row.body_template,
            row.priority,
            row.available_variables_json,
        ],
    )?;
    Ok(())
}

pub fn list_notif_configs(db: &Database) -> Result<Vec<NotifConfigRow>> {
    let conn = db.conn();
    let mut stmt = conn.prepare(
        "SELECT type_id, display_name, description, category, enabled,
                title_template, body_template, priority, available_variables_json
         FROM notification_configs ORDER BY category, type_id"
    )?;

    let rows = stmt.query_map([], |row| {
        Ok(NotifConfigRow {
            type_id: row.get(0)?,
            display_name: row.get(1)?,
            description: row.get(2)?,
            category: row.get(3)?,
            enabled: row.get::<_, i32>(4)? != 0,
            title_template: row.get(5)?,
            body_template: row.get(6)?,
            priority: row.get(7)?,
            available_variables_json: row.get(8)?,
        })
    })?;

    let mut result = Vec::new();
    for row in rows {
        result.push(row?);
    }
    Ok(result)
}

pub fn get_notif_config(db: &Database, type_id: &str) -> Result<Option<NotifConfigRow>> {
    let conn = db.conn();
    let mut stmt = conn.prepare(
        "SELECT type_id, display_name, description, category, enabled,
                title_template, body_template, priority, available_variables_json
         FROM notification_configs WHERE type_id = ?1"
    )?;

    let mut rows = stmt.query_map(params![type_id], |row| {
        Ok(NotifConfigRow {
            type_id: row.get(0)?,
            display_name: row.get(1)?,
            description: row.get(2)?,
            category: row.get(3)?,
            enabled: row.get::<_, i32>(4)? != 0,
            title_template: row.get(5)?,
            body_template: row.get(6)?,
            priority: row.get(7)?,
            available_variables_json: row.get(8)?,
        })
    })?;

    match rows.next() {
        Some(row) => Ok(Some(row?)),
        None => Ok(None),
    }
}

pub fn count_notif_configs(db: &Database) -> Result<i64> {
    let conn = db.conn();
    conn.query_row("SELECT COUNT(*) FROM notification_configs", [], |row| row.get(0))
        .map_err(Into::into)
}

pub fn import_notif_configs_json(db: &Database, json: &str) -> Result<usize> {
    let configs: std::collections::HashMap<String, serde_json::Value> = serde_json::from_str(json)?;

    let mut conn = db.conn();
    let tx = conn.transaction()?;
    let mut count = 0;

    {
        let mut stmt = tx.prepare_cached(
            "INSERT OR IGNORE INTO notification_configs
             (type_id, display_name, description, category, enabled,
              title_template, body_template, priority, available_variables_json)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)"
        )?;

        for (type_id, config) in &configs {
            let display_name = config.get("display_name").and_then(|v| v.as_str()).unwrap_or("").to_string();
            let description = config.get("description").and_then(|v| v.as_str()).unwrap_or("").to_string();
            let category = config.get("category").and_then(|v| v.as_str()).unwrap_or("system").to_string();
            let enabled = config.get("enabled").and_then(|v| v.as_bool()).unwrap_or(true);
            let title = config.get("title_template").and_then(|v| v.as_str()).unwrap_or("").to_string();
            let body = config.get("body_template").and_then(|v| v.as_str()).unwrap_or("").to_string();
            let priority = config.get("priority").and_then(|v| v.as_str()).unwrap_or("P2").to_string();
            let vars = config.get("available_variables").map(|v| v.to_string()).unwrap_or_else(|| "[]".to_string());

            stmt.execute(params![type_id, display_name, description, category, enabled as i32, title, body, priority, vars])?;
            count += 1;
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

    // --- Modes ---

    #[test]
    fn test_upsert_and_list_modes() {
        let db = test_db();
        let row = ModeRow {
            id: "mode-pro".to_string(),
            name: "Pro".to_string(),
            slug: "pro".to_string(),
            icon: "briefcase".to_string(),
            theme_json: r##"{"primary":"#2563eb","background":"#f8fafc","accent":"#1e40af"}"##.to_string(),
            is_system: true,
            created_at: "2026-02-26T10:00:00Z".to_string(),
            display_order: 0,
        };
        upsert_mode(&db, &row).unwrap();

        let modes = list_modes(&db).unwrap();
        assert_eq!(modes.len(), 1);
        assert_eq!(modes[0].name, "Pro");
        assert!(modes[0].is_system);
    }

    #[test]
    fn test_mode_upsert_updates() {
        let db = test_db();
        let row = ModeRow {
            id: "m1".to_string(),
            name: "Old".to_string(),
            slug: "old".to_string(),
            icon: "x".to_string(),
            theme_json: "{}".to_string(),
            is_system: false,
            created_at: "2026-01-01T00:00:00Z".to_string(),
            display_order: 0,
        };
        upsert_mode(&db, &row).unwrap();

        let updated = ModeRow {
            id: "m1".to_string(),
            name: "New".to_string(),
            slug: "new".to_string(),
            icon: "y".to_string(),
            theme_json: r##"{"primary":"#fff"}"##.to_string(),
            is_system: false,
            created_at: "2026-01-01T00:00:00Z".to_string(),
            display_order: 1,
        };
        upsert_mode(&db, &updated).unwrap();

        assert_eq!(count_modes(&db).unwrap(), 1);
        let mode = get_mode(&db, "m1").unwrap().unwrap();
        assert_eq!(mode.name, "New");
    }

    #[test]
    fn test_delete_mode() {
        let db = test_db();
        let row = ModeRow {
            id: "m1".to_string(),
            name: "Test".to_string(),
            slug: "test".to_string(),
            icon: "".to_string(),
            theme_json: "{}".to_string(),
            is_system: false,
            created_at: "2026-01-01T00:00:00Z".to_string(),
            display_order: 0,
        };
        upsert_mode(&db, &row).unwrap();
        assert!(delete_mode(&db, "m1").unwrap());
        assert_eq!(count_modes(&db).unwrap(), 0);
    }

    #[test]
    fn test_import_modes_json() {
        let db = test_db();
        let json = r##"{
            "mode-pro": {
                "name": "Pro",
                "slug": "pro",
                "icon": "briefcase",
                "theme": {"primary": "#2563eb"},
                "is_system": true,
                "created_at": "2026-01-01T00:00:00Z",
                "display_order": 0
            }
        }"##;
        let count = import_modes_json(&db, json).unwrap();
        assert_eq!(count, 1);
    }

    // --- Schedule Rules ---

    #[test]
    fn test_upsert_and_list_schedule_rules() {
        let db = test_db();
        let row = ScheduleRuleRow {
            id: "rule-1".to_string(),
            mode_id: "mode-pro".to_string(),
            days_json: "[0,1,2,3,4]".to_string(),
            start_time: "08:00".to_string(),
            end_time: "18:00".to_string(),
            priority: 10,
            enabled: true,
            name: Some("Workday".to_string()),
            created_at: "2026-02-26T10:00:00Z".to_string(),
        };
        upsert_schedule_rule(&db, &row).unwrap();

        let rules = list_schedule_rules(&db).unwrap();
        assert_eq!(rules.len(), 1);
        assert_eq!(rules[0].mode_id, "mode-pro");
    }

    #[test]
    fn test_schedule_config() {
        let db = test_db();
        set_schedule_config(&db, "default_mode_id", "mode-veille").unwrap();
        let val = get_schedule_config(&db, "default_mode_id").unwrap();
        assert_eq!(val.as_deref(), Some("mode-veille"));

        // Upsert
        set_schedule_config(&db, "default_mode_id", "mode-pro").unwrap();
        let val = get_schedule_config(&db, "default_mode_id").unwrap();
        assert_eq!(val.as_deref(), Some("mode-pro"));
    }

    #[test]
    fn test_import_schedule_json() {
        let db = test_db();
        let json = r#"{
            "default_mode_id": "mode-veille",
            "rules": [
                {
                    "id": "r1",
                    "mode_id": "mode-pro",
                    "days": [0,1,2,3,4],
                    "start_time": "08:00",
                    "end_time": "18:00",
                    "priority": 10,
                    "enabled": true,
                    "created_at": "2026-01-01T00:00:00Z"
                }
            ]
        }"#;
        let count = import_schedule_json(&db, json).unwrap();
        assert_eq!(count, 1);

        let config = get_schedule_config(&db, "default_mode_id").unwrap();
        assert_eq!(config.as_deref(), Some("mode-veille"));
    }

    // --- Notification Configs ---

    #[test]
    fn test_upsert_and_list_notif_configs() {
        let db = test_db();
        let row = NotifConfigRow {
            type_id: "plugin_offline".to_string(),
            display_name: "Plugin Offline".to_string(),
            description: "When a plugin goes offline".to_string(),
            category: "plugin_health".to_string(),
            enabled: true,
            title_template: "{plugin_name} is offline".to_string(),
            body_template: "Plugin {plugin_name} went offline".to_string(),
            priority: "P1".to_string(),
            available_variables_json: r#"[{"name":"plugin_name"}]"#.to_string(),
        };
        upsert_notif_config(&db, &row).unwrap();

        let configs = list_notif_configs(&db).unwrap();
        assert_eq!(configs.len(), 1);
        assert_eq!(configs[0].display_name, "Plugin Offline");
    }

    #[test]
    fn test_import_notif_configs_json() {
        let db = test_db();
        let json = r#"{
            "plugin_offline": {
                "display_name": "Plugin Offline",
                "description": "test",
                "category": "plugin_health",
                "enabled": true,
                "title_template": "{name} offline",
                "body_template": "Plugin went offline",
                "priority": "P1",
                "available_variables": [{"name": "name"}]
            }
        }"#;
        let count = import_notif_configs_json(&db, json).unwrap();
        assert_eq!(count, 1);
        assert_eq!(count_notif_configs(&db).unwrap(), 1);
    }
}
