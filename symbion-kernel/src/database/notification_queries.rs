/**
 * SYMBION KERNEL - Notification SQLite Queries
 *
 * ROLE: Typed queries for notification persistence.
 * All functions take &Database and return anyhow::Result.
 */

use anyhow::Result;
use rusqlite::params;
use super::Database;

pub struct NotificationRow {
    pub id: String,
    pub priority: String,
    pub title: String,
    pub body: String,
    pub source: String,
    pub timestamp: String,
    pub acknowledged: bool,
    pub acknowledged_at: Option<i64>,
    pub actions_json: String,
    pub data_json: Option<String>,
}

pub fn insert_notification(db: &Database, row: &NotificationRow) -> Result<()> {
    let conn = db.conn();
    conn.execute(
        "INSERT OR REPLACE INTO notifications
         (id, priority, title, body, source, timestamp, acknowledged, acknowledged_at, actions_json, data_json)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10)",
        params![
            row.id,
            row.priority,
            row.title,
            row.body,
            row.source,
            row.timestamp,
            row.acknowledged as i32,
            row.acknowledged_at,
            row.actions_json,
            row.data_json,
        ],
    )?;
    Ok(())
}

pub fn list_notifications(db: &Database, limit: usize) -> Result<Vec<NotificationRow>> {
    let conn = db.conn();
    let mut stmt = conn.prepare(
        "SELECT id, priority, title, body, source, timestamp, acknowledged, acknowledged_at, actions_json, data_json
         FROM notifications ORDER BY timestamp DESC LIMIT ?1"
    )?;

    let rows = stmt.query_map(params![limit as i64], map_notification_row)?;

    let mut result = Vec::new();
    for row in rows {
        result.push(row?);
    }
    Ok(result)
}

pub fn get_notification(db: &Database, id: &str) -> Result<Option<NotificationRow>> {
    let conn = db.conn();
    let mut stmt = conn.prepare(
        "SELECT id, priority, title, body, source, timestamp, acknowledged, acknowledged_at, actions_json, data_json
         FROM notifications WHERE id = ?1"
    )?;

    let mut rows = stmt.query_map(params![id], map_notification_row)?;
    match rows.next() {
        Some(row) => Ok(Some(row?)),
        None => Ok(None),
    }
}

pub fn acknowledge_notification(db: &Database, id: &str, acknowledged_at: i64) -> Result<bool> {
    let conn = db.conn();
    let updated = conn.execute(
        "UPDATE notifications SET acknowledged = 1, acknowledged_at = ?1 WHERE id = ?2",
        params![acknowledged_at, id],
    )?;
    Ok(updated > 0)
}

pub fn delete_notification(db: &Database, id: &str) -> Result<bool> {
    let conn = db.conn();
    let deleted = conn.execute("DELETE FROM notifications WHERE id = ?1", params![id])?;
    Ok(deleted > 0)
}

pub fn delete_all_notifications(db: &Database) -> Result<usize> {
    let conn = db.conn();
    let deleted = conn.execute("DELETE FROM notifications", [])?;
    Ok(deleted)
}

pub fn count_notifications(db: &Database) -> Result<i64> {
    let conn = db.conn();
    conn.query_row("SELECT COUNT(*) FROM notifications", [], |row| row.get(0))
        .map_err(Into::into)
}

pub fn import_notifications_json(db: &Database, json: &str) -> Result<usize> {
    let notifs: Vec<serde_json::Value> = serde_json::from_str(json)?;

    let mut conn = db.conn();
    let tx = conn.transaction()?;
    let mut count = 0;

    {
        let mut stmt = tx.prepare_cached(
            "INSERT OR IGNORE INTO notifications
             (id, priority, title, body, source, timestamp, acknowledged, acknowledged_at, actions_json, data_json)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10)"
        )?;

        for notif in &notifs {
            let id = notif.get("id").and_then(|v| v.as_str()).unwrap_or("").to_string();
            let priority = notif.get("priority").and_then(|v| v.as_str()).unwrap_or("P2").to_string();
            let title = notif.get("title").and_then(|v| v.as_str()).unwrap_or("").to_string();
            let body = notif.get("body").and_then(|v| v.as_str()).unwrap_or("").to_string();
            let source = notif.get("source").and_then(|v| v.as_str()).unwrap_or("").to_string();
            let timestamp = notif.get("timestamp")
                .and_then(|v| v.as_i64())
                .map(|ts| {
                    time::OffsetDateTime::from_unix_timestamp(ts)
                        .map(|dt| dt.format(&time::format_description::well_known::Rfc3339).unwrap_or_default())
                        .unwrap_or_default()
                })
                .or_else(|| notif.get("timestamp").and_then(|v| v.as_str()).map(|s| s.to_string()))
                .unwrap_or_default();
            let acknowledged = notif.get("acknowledged").and_then(|v| v.as_bool()).unwrap_or(false);
            let acknowledged_at = notif.get("acknowledged_at").and_then(|v| v.as_i64());
            let actions = notif.get("actions").map(|v| v.to_string()).unwrap_or_else(|| "[]".to_string());
            let data = notif.get("data").filter(|v| !v.is_null()).map(|v| v.to_string());

            if !id.is_empty() {
                stmt.execute(params![
                    id, priority, title, body, source, timestamp,
                    acknowledged as i32, acknowledged_at, actions, data,
                ])?;
                count += 1;
            }
        }
    }

    tx.commit()?;
    Ok(count)
}

fn map_notification_row(row: &rusqlite::Row<'_>) -> rusqlite::Result<NotificationRow> {
    Ok(NotificationRow {
        id: row.get(0)?,
        priority: row.get(1)?,
        title: row.get(2)?,
        body: row.get(3)?,
        source: row.get(4)?,
        timestamp: row.get(5)?,
        acknowledged: row.get::<_, i32>(6)? != 0,
        acknowledged_at: row.get(7)?,
        actions_json: row.get(8)?,
        data_json: row.get(9)?,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_db() -> Database {
        Database::open_in_memory().unwrap()
    }

    #[test]
    fn test_insert_and_get() {
        let db = test_db();
        let row = NotificationRow {
            id: "notif-1".to_string(),
            priority: "P1".to_string(),
            title: "Alert".to_string(),
            body: "Sensor offline".to_string(),
            source: "sensor-monitor".to_string(),
            timestamp: "2026-02-26T10:00:00Z".to_string(),
            acknowledged: false,
            acknowledged_at: None,
            actions_json: "[]".to_string(),
            data_json: None,
        };
        insert_notification(&db, &row).unwrap();

        let loaded = get_notification(&db, "notif-1").unwrap().unwrap();
        assert_eq!(loaded.title, "Alert");
        assert!(!loaded.acknowledged);
    }

    #[test]
    fn test_acknowledge() {
        let db = test_db();
        let row = NotificationRow {
            id: "n1".to_string(),
            priority: "P0".to_string(),
            title: "Critical".to_string(),
            body: "test".to_string(),
            source: "kernel".to_string(),
            timestamp: "2026-02-26T10:00:00Z".to_string(),
            acknowledged: false,
            acknowledged_at: None,
            actions_json: "[]".to_string(),
            data_json: None,
        };
        insert_notification(&db, &row).unwrap();

        assert!(acknowledge_notification(&db, "n1", 1700001000).unwrap());

        let loaded = get_notification(&db, "n1").unwrap().unwrap();
        assert!(loaded.acknowledged);
        assert_eq!(loaded.acknowledged_at, Some(1700001000));
    }

    #[test]
    fn test_list_ordered_and_limited() {
        let db = test_db();
        for i in 0..5 {
            let row = NotificationRow {
                id: format!("n{}", i),
                priority: "P2".to_string(),
                title: format!("Notif {}", i),
                body: "test".to_string(),
                source: "test".to_string(),
                timestamp: format!("2026-02-26T10:{:02}:00Z", i),
                acknowledged: false,
                acknowledged_at: None,
                actions_json: "[]".to_string(),
                data_json: None,
            };
            insert_notification(&db, &row).unwrap();
        }

        let list = list_notifications(&db, 3).unwrap();
        assert_eq!(list.len(), 3);
        // Newest first
        assert_eq!(list[0].id, "n4");
    }

    #[test]
    fn test_delete_all() {
        let db = test_db();
        for i in 0..3 {
            let row = NotificationRow {
                id: format!("n{}", i),
                priority: "P2".to_string(),
                title: "t".to_string(),
                body: "b".to_string(),
                source: "s".to_string(),
                timestamp: "2026-02-26T10:00:00Z".to_string(),
                acknowledged: false,
                acknowledged_at: None,
                actions_json: "[]".to_string(),
                data_json: None,
            };
            insert_notification(&db, &row).unwrap();
        }

        let deleted = delete_all_notifications(&db).unwrap();
        assert_eq!(deleted, 3);
        assert_eq!(count_notifications(&db).unwrap(), 0);
    }

    #[test]
    fn test_import_notifications_json() {
        let db = test_db();
        let json = r#"[
            {
                "id": "n1",
                "priority": "P1",
                "title": "Test",
                "body": "Body",
                "source": "kernel",
                "timestamp": "2026-02-26T10:00:00Z",
                "acknowledged": false,
                "actions": [{"id": "approve", "label": "OK", "action_type": "approve"}]
            }
        ]"#;
        let count = import_notifications_json(&db, json).unwrap();
        assert_eq!(count, 1);
        assert_eq!(count_notifications(&db).unwrap(), 1);
    }
}
