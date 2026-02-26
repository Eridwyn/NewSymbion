/**
 * SYMBION KERNEL - Sensor Environment SQLite Queries
 *
 * ROLE: Typed queries for sensor environment persistence.
 * All functions take &Database and return anyhow::Result.
 */

use anyhow::Result;
use rusqlite::params;
use super::Database;

/// Single environment reading row (maps to/from sensor_environments table).
pub struct EnvRow {
    pub sensor_id: String,
    pub room_id: String,
    pub temperature_c: Option<f64>,
    pub humidity_pct: Option<f64>,
    pub status: String,
    pub recorded_at: String, // ISO 8601
}

/// Insert a single reading. Used in the periodic save path.
pub fn insert_reading(db: &Database, row: &EnvRow) -> Result<()> {
    let conn = db.conn();
    conn.execute(
        "INSERT OR IGNORE INTO sensor_environments
         (sensor_id, room_id, temperature_c, humidity_pct, status, recorded_at)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
        params![
            row.sensor_id,
            row.room_id,
            row.temperature_c,
            row.humidity_pct,
            row.status,
            row.recorded_at,
        ],
    )?;
    Ok(())
}

/// Insert a batch of environment readings within a transaction.
/// Used for both incremental writes and bulk import.
pub fn insert_readings(db: &Database, rows: &[EnvRow]) -> Result<usize> {
    let mut conn = db.conn();
    let tx = conn.transaction()?;

    let mut count = 0;
    {
        let mut stmt = tx.prepare_cached(
            "INSERT OR IGNORE INTO sensor_environments
             (sensor_id, room_id, temperature_c, humidity_pct, status, recorded_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6)"
        )?;

        for row in rows {
            stmt.execute(params![
                row.sensor_id,
                row.room_id,
                row.temperature_c,
                row.humidity_pct,
                row.status,
                row.recorded_at,
            ])?;
            count += 1;
        }
    }

    tx.commit()?;
    Ok(count)
}

/// Load latest N readings per sensor (for rebuilding in-memory state at startup).
/// Returns readings ordered oldest-first per sensor (ready for VecDeque push_back).
pub fn load_latest_per_sensor(db: &Database, max_per_sensor: usize) -> Result<Vec<EnvRow>> {
    let conn = db.conn();
    let mut stmt = conn.prepare(
        "SELECT sensor_id, room_id, temperature_c, humidity_pct, status, recorded_at
         FROM (
             SELECT *, ROW_NUMBER() OVER (PARTITION BY sensor_id ORDER BY recorded_at DESC) AS rn
             FROM sensor_environments
         ) WHERE rn <= ?1
         ORDER BY sensor_id, recorded_at ASC"
    )?;

    let rows = stmt.query_map(params![max_per_sensor as i64], |row| {
        Ok(EnvRow {
            sensor_id: row.get(0)?,
            room_id: row.get(1)?,
            temperature_c: row.get(2)?,
            humidity_pct: row.get(3)?,
            status: row.get(4)?,
            recorded_at: row.get(5)?,
        })
    })?;

    let mut result = Vec::new();
    for row in rows {
        result.push(row?);
    }
    Ok(result)
}

/// Count total readings (for health/stats and import detection).
pub fn count_readings(db: &Database) -> Result<i64> {
    let conn = db.conn();
    conn.query_row("SELECT COUNT(*) FROM sensor_environments", [], |row| row.get(0))
        .map_err(Into::into)
}

/// Prune readings older than N days (maintenance task).
pub fn prune_old_readings(db: &Database, days: i64) -> Result<usize> {
    let conn = db.conn();
    let cutoff = chrono::Utc::now() - chrono::Duration::days(days);
    let cutoff_str = cutoff.to_rfc3339();
    let deleted = conn.execute(
        "DELETE FROM sensor_environments WHERE recorded_at < ?1",
        params![cutoff_str],
    )?;
    Ok(deleted)
}

/// Import sensor environments from JSON string (one-shot migration).
/// Expects the JSON format: HashMap<sensor_id, RoomEnvironmentState> with history VecDeque.
pub fn import_from_json(db: &Database, json: &str) -> Result<usize> {
    // Parse as generic JSON to extract readings from each sensor's history
    let data: serde_json::Value = serde_json::from_str(json)?;

    let obj = data.as_object()
        .ok_or_else(|| anyhow::anyhow!("Expected JSON object for sensor environments"))?;

    let mut rows = Vec::new();

    for (sensor_id, env_state) in obj {
        let room_id = env_state.get("room_id")
            .and_then(|v| v.as_str())
            .unwrap_or("unknown")
            .to_string();

        let status = env_state.get("status")
            .and_then(|v| v.as_str())
            .unwrap_or("safe")
            .to_string();

        // Import history entries
        if let Some(history) = env_state.get("history").and_then(|v| v.as_array()) {
            for entry in history {
                let temp = entry.get("temperature_c").and_then(|v| v.as_f64());
                let humidity = entry.get("humidity_pct").and_then(|v| v.as_f64());
                let timestamp = entry.get("timestamp")
                    .and_then(|v| v.as_str())
                    .unwrap_or("")
                    .to_string();

                if !timestamp.is_empty() {
                    rows.push(EnvRow {
                        sensor_id: sensor_id.clone(),
                        room_id: room_id.clone(),
                        temperature_c: temp,
                        humidity_pct: humidity,
                        status: status.clone(),
                        recorded_at: timestamp,
                    });
                }
            }
        }

        // Also import current reading
        if let Some(current) = env_state.get("current") {
            let temp = current.get("temperature_c").and_then(|v| v.as_f64());
            let humidity = current.get("humidity_pct").and_then(|v| v.as_f64());
            let timestamp = current.get("timestamp")
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_string();

            if !timestamp.is_empty() {
                rows.push(EnvRow {
                    sensor_id: sensor_id.clone(),
                    room_id: room_id.clone(),
                    temperature_c: temp,
                    humidity_pct: humidity,
                    status: status.clone(),
                    recorded_at: timestamp,
                });
            }
        }
    }

    insert_readings(db, &rows)
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
        let row = EnvRow {
            sensor_id: "esp32-chambre".to_string(),
            room_id: "chambre".to_string(),
            temperature_c: Some(22.5),
            humidity_pct: Some(55.0),
            status: "safe".to_string(),
            recorded_at: "2026-02-26T10:00:00Z".to_string(),
        };
        insert_reading(&db, &row).unwrap();
        assert_eq!(count_readings(&db).unwrap(), 1);
    }

    #[test]
    fn test_duplicate_ignored() {
        let db = test_db();
        let row = EnvRow {
            sensor_id: "esp32-chambre".to_string(),
            room_id: "chambre".to_string(),
            temperature_c: Some(22.5),
            humidity_pct: Some(55.0),
            status: "safe".to_string(),
            recorded_at: "2026-02-26T10:00:00Z".to_string(),
        };
        insert_reading(&db, &row).unwrap();
        insert_reading(&db, &row).unwrap(); // Duplicate — should be ignored
        assert_eq!(count_readings(&db).unwrap(), 1);
    }

    #[test]
    fn test_batch_insert() {
        let db = test_db();
        let rows = vec![
            EnvRow {
                sensor_id: "esp32-a".to_string(),
                room_id: "salon".to_string(),
                temperature_c: Some(20.0),
                humidity_pct: Some(50.0),
                status: "safe".to_string(),
                recorded_at: "2026-02-26T10:00:00Z".to_string(),
            },
            EnvRow {
                sensor_id: "esp32-a".to_string(),
                room_id: "salon".to_string(),
                temperature_c: Some(21.0),
                humidity_pct: Some(52.0),
                status: "safe".to_string(),
                recorded_at: "2026-02-26T10:00:30Z".to_string(),
            },
            EnvRow {
                sensor_id: "esp32-b".to_string(),
                room_id: "chambre".to_string(),
                temperature_c: Some(19.0),
                humidity_pct: Some(60.0),
                status: "safe".to_string(),
                recorded_at: "2026-02-26T10:00:00Z".to_string(),
            },
        ];
        let count = insert_readings(&db, &rows).unwrap();
        assert_eq!(count, 3);
        assert_eq!(count_readings(&db).unwrap(), 3);
    }

    #[test]
    fn test_load_latest_per_sensor() {
        let db = test_db();
        // Insert 5 readings for sensor A, 3 for sensor B
        let mut rows = Vec::new();
        for i in 0..5 {
            rows.push(EnvRow {
                sensor_id: "sensor-a".to_string(),
                room_id: "salon".to_string(),
                temperature_c: Some(20.0 + i as f64),
                humidity_pct: Some(50.0),
                status: "safe".to_string(),
                recorded_at: format!("2026-02-26T10:{:02}:00Z", i),
            });
        }
        for i in 0..3 {
            rows.push(EnvRow {
                sensor_id: "sensor-b".to_string(),
                room_id: "chambre".to_string(),
                temperature_c: Some(18.0 + i as f64),
                humidity_pct: Some(55.0),
                status: "safe".to_string(),
                recorded_at: format!("2026-02-26T10:{:02}:00Z", i),
            });
        }
        insert_readings(&db, &rows).unwrap();

        // Load latest 3 per sensor
        let loaded = load_latest_per_sensor(&db, 3).unwrap();
        // Should get 3 from sensor-a + 3 from sensor-b = 6
        assert_eq!(loaded.len(), 6);

        // Sensor A should have the 3 most recent (i=2,3,4)
        let sensor_a: Vec<_> = loaded.iter().filter(|r| r.sensor_id == "sensor-a").collect();
        assert_eq!(sensor_a.len(), 3);
        // Ordered oldest first
        assert_eq!(sensor_a[0].recorded_at, "2026-02-26T10:02:00Z");
        assert_eq!(sensor_a[2].recorded_at, "2026-02-26T10:04:00Z");
    }

    #[test]
    fn test_prune_old_readings() {
        let db = test_db();
        let rows = vec![
            EnvRow {
                sensor_id: "s1".to_string(),
                room_id: "r1".to_string(),
                temperature_c: Some(20.0),
                humidity_pct: Some(50.0),
                status: "safe".to_string(),
                recorded_at: "2020-01-01T00:00:00Z".to_string(), // Very old
            },
            EnvRow {
                sensor_id: "s1".to_string(),
                room_id: "r1".to_string(),
                temperature_c: Some(21.0),
                humidity_pct: Some(51.0),
                status: "safe".to_string(),
                recorded_at: chrono::Utc::now().to_rfc3339(), // Recent
            },
        ];
        insert_readings(&db, &rows).unwrap();
        assert_eq!(count_readings(&db).unwrap(), 2);

        let pruned = prune_old_readings(&db, 30).unwrap(); // Prune older than 30 days
        assert_eq!(pruned, 1);
        assert_eq!(count_readings(&db).unwrap(), 1);
    }

    #[test]
    fn test_null_readings() {
        let db = test_db();
        let row = EnvRow {
            sensor_id: "offline-sensor".to_string(),
            room_id: "bureau".to_string(),
            temperature_c: None,
            humidity_pct: None,
            status: "safe".to_string(),
            recorded_at: "2026-02-26T10:00:00Z".to_string(),
        };
        insert_reading(&db, &row).unwrap();

        let loaded = load_latest_per_sensor(&db, 10).unwrap();
        assert_eq!(loaded.len(), 1);
        assert!(loaded[0].temperature_c.is_none());
        assert!(loaded[0].humidity_pct.is_none());
    }

    #[test]
    fn test_import_from_json() {
        let db = test_db();
        let json = r#"{
            "esp32-chambre": {
                "room_id": "chambre",
                "status": "safe",
                "current": {
                    "temperature_c": 22.5,
                    "humidity_pct": 55.0,
                    "timestamp": "2026-02-26T10:00:00Z"
                },
                "history": [
                    {"temperature_c": 21.0, "humidity_pct": 50.0, "timestamp": "2026-02-26T09:59:00Z"},
                    {"temperature_c": 21.5, "humidity_pct": 52.0, "timestamp": "2026-02-26T09:59:30Z"}
                ]
            }
        }"#;
        let count = import_from_json(&db, json).unwrap();
        assert_eq!(count, 3); // 2 history + 1 current
        assert_eq!(count_readings(&db).unwrap(), 3);
    }
}
