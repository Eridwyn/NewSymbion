/**
 * SYMBION KERNEL - Sensor Metadata SQLite Queries
 *
 * ROLE: Typed queries for sensor registry metadata persistence.
 * Replaces data/sensors.json (metadata only, not environment readings).
 * All functions take &Database and return anyhow::Result.
 */

use anyhow::Result;
use rusqlite::params;
use super::Database;

/// Single sensor metadata row (maps to/from sensors table).
pub struct SensorMetaRow {
    pub sensor_id: String,
    pub sensor_type: String,
    pub room_id: String,
    pub firmware_version: Option<String>,
    pub registered_at: String, // ISO 8601
    pub last_seen: String,     // ISO 8601
    pub status: String,
    pub battery_pct: Option<f64>,
    pub signal_rssi: Option<f64>,
    pub deleted_at: Option<String>, // ISO 8601 or None
}

/// Upsert sensor metadata (INSERT OR REPLACE).
pub fn upsert_sensor(db: &Database, row: &SensorMetaRow) -> Result<()> {
    let conn = db.conn();
    conn.execute(
        "INSERT OR REPLACE INTO sensors
         (sensor_id, sensor_type, room_id, firmware_version, registered_at,
          last_seen, status, battery_pct, signal_rssi, deleted_at)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10)",
        params![
            row.sensor_id,
            row.sensor_type,
            row.room_id,
            row.firmware_version,
            row.registered_at,
            row.last_seen,
            row.status,
            row.battery_pct,
            row.signal_rssi,
            row.deleted_at,
        ],
    )?;
    Ok(())
}

/// List all sensors (including soft-deleted).
pub fn list_sensors(db: &Database) -> Result<Vec<SensorMetaRow>> {
    let conn = db.conn();
    let mut stmt = conn.prepare(
        "SELECT sensor_id, sensor_type, room_id, firmware_version, registered_at,
                last_seen, status, battery_pct, signal_rssi, deleted_at
         FROM sensors
         ORDER BY registered_at DESC"
    )?;

    let rows = stmt.query_map([], |row| {
        Ok(SensorMetaRow {
            sensor_id: row.get(0)?,
            sensor_type: row.get(1)?,
            room_id: row.get(2)?,
            firmware_version: row.get(3)?,
            registered_at: row.get(4)?,
            last_seen: row.get(5)?,
            status: row.get(6)?,
            battery_pct: row.get(7)?,
            signal_rssi: row.get(8)?,
            deleted_at: row.get(9)?,
        })
    })?;

    let mut result = Vec::new();
    for row in rows {
        result.push(row?);
    }
    Ok(result)
}

/// Count sensors (excluding soft-deleted).
pub fn count_sensors(db: &Database) -> Result<i64> {
    let conn = db.conn();
    conn.query_row(
        "SELECT COUNT(*) FROM sensors WHERE deleted_at IS NULL",
        [],
        |row| row.get(0),
    )
    .map_err(Into::into)
}

/// Delete a sensor by ID (hard delete).
pub fn delete_sensor(db: &Database, sensor_id: &str) -> Result<bool> {
    let conn = db.conn();
    let deleted = conn.execute(
        "DELETE FROM sensors WHERE sensor_id = ?1",
        params![sensor_id],
    )?;
    Ok(deleted > 0)
}

/// Batch upsert all sensors (for bulk save).
pub fn upsert_all_sensors(db: &Database, rows: &[SensorMetaRow]) -> Result<usize> {
    let mut conn = db.conn();
    let tx = conn.transaction()?;

    {
        let mut stmt = tx.prepare_cached(
            "INSERT OR REPLACE INTO sensors
             (sensor_id, sensor_type, room_id, firmware_version, registered_at,
              last_seen, status, battery_pct, signal_rssi, deleted_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10)"
        )?;

        for row in rows {
            stmt.execute(params![
                row.sensor_id,
                row.sensor_type,
                row.room_id,
                row.firmware_version,
                row.registered_at,
                row.last_seen,
                row.status,
                row.battery_pct,
                row.signal_rssi,
                row.deleted_at,
            ])?;
        }
    }

    tx.commit()?;
    Ok(rows.len())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_db() -> Database {
        Database::open_in_memory().unwrap()
    }

    fn sample_sensor(id: &str) -> SensorMetaRow {
        SensorMetaRow {
            sensor_id: id.to_string(),
            sensor_type: "bme280".to_string(),
            room_id: "chambre".to_string(),
            firmware_version: Some("0.1.0".to_string()),
            registered_at: "2026-02-28T10:00:00Z".to_string(),
            last_seen: "2026-02-28T10:05:00Z".to_string(),
            status: "online".to_string(),
            battery_pct: Some(85.0),
            signal_rssi: Some(-45.0),
            deleted_at: None,
        }
    }

    #[test]
    fn test_upsert_and_list() {
        let db = test_db();

        upsert_sensor(&db, &sample_sensor("esp32-01")).unwrap();
        upsert_sensor(&db, &sample_sensor("esp32-02")).unwrap();

        let sensors = list_sensors(&db).unwrap();
        assert_eq!(sensors.len(), 2);
        assert_eq!(count_sensors(&db).unwrap(), 2);
    }

    #[test]
    fn test_upsert_overwrite() {
        let db = test_db();

        let mut sensor = sample_sensor("esp32-01");
        upsert_sensor(&db, &sensor).unwrap();

        sensor.status = "offline".to_string();
        sensor.last_seen = "2026-02-28T11:00:00Z".to_string();
        upsert_sensor(&db, &sensor).unwrap();

        assert_eq!(count_sensors(&db).unwrap(), 1);
        let loaded = list_sensors(&db).unwrap();
        assert_eq!(loaded[0].status, "offline");
    }

    #[test]
    fn test_soft_delete_excluded_from_count() {
        let db = test_db();

        upsert_sensor(&db, &sample_sensor("esp32-active")).unwrap();

        let mut deleted_sensor = sample_sensor("esp32-deleted");
        deleted_sensor.deleted_at = Some("2026-02-28T12:00:00Z".to_string());
        upsert_sensor(&db, &deleted_sensor).unwrap();

        // Count excludes soft-deleted
        assert_eq!(count_sensors(&db).unwrap(), 1);
        // List includes all
        assert_eq!(list_sensors(&db).unwrap().len(), 2);
    }

    #[test]
    fn test_delete_sensor() {
        let db = test_db();

        upsert_sensor(&db, &sample_sensor("esp32-del")).unwrap();
        assert_eq!(count_sensors(&db).unwrap(), 1);

        let deleted = delete_sensor(&db, "esp32-del").unwrap();
        assert!(deleted);
        assert_eq!(count_sensors(&db).unwrap(), 0);
    }

    #[test]
    fn test_upsert_all_sensors() {
        let db = test_db();

        let sensors: Vec<SensorMetaRow> = (0..5)
            .map(|i| sample_sensor(&format!("esp32-{:02}", i)))
            .collect();

        let count = upsert_all_sensors(&db, &sensors).unwrap();
        assert_eq!(count, 5);
        assert_eq!(count_sensors(&db).unwrap(), 5);
    }
}
