/**
 * SYMBION KERNEL - Inference Engine SQLite Queries
 *
 * ROLE: Typed queries for training samples persistence.
 * All functions take &Database and return anyhow::Result.
 */

use anyhow::Result;
use rusqlite::params;
use super::Database;

pub struct SampleRow {
    pub id: String,
    pub vector_json: String,
    pub chosen_mode: String,
    pub source: String,
    pub timestamp: String,
    pub base_weight: f64,
}

pub fn insert_sample(db: &Database, row: &SampleRow) -> Result<()> {
    let conn = db.conn();
    conn.execute(
        "INSERT OR REPLACE INTO training_samples
         (id, vector_json, chosen_mode, source, timestamp, base_weight)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
        params![
            row.id,
            row.vector_json,
            row.chosen_mode,
            row.source,
            row.timestamp,
            row.base_weight,
        ],
    )?;
    Ok(())
}

pub fn list_samples(db: &Database) -> Result<Vec<SampleRow>> {
    let conn = db.conn();
    let mut stmt = conn.prepare(
        "SELECT id, vector_json, chosen_mode, source, timestamp, base_weight
         FROM training_samples ORDER BY timestamp DESC"
    )?;

    let rows = stmt.query_map([], map_sample_row)?;

    let mut result = Vec::new();
    for row in rows {
        result.push(row?);
    }
    Ok(result)
}

pub fn count_samples(db: &Database) -> Result<i64> {
    let conn = db.conn();
    conn.query_row("SELECT COUNT(*) FROM training_samples", [], |row| row.get(0))
        .map_err(Into::into)
}

pub fn delete_sample(db: &Database, id: &str) -> Result<bool> {
    let conn = db.conn();
    let deleted = conn.execute("DELETE FROM training_samples WHERE id = ?1", params![id])?;
    Ok(deleted > 0)
}

pub fn delete_old_samples(db: &Database, cutoff: &str) -> Result<usize> {
    let conn = db.conn();
    let deleted = conn.execute(
        "DELETE FROM training_samples WHERE timestamp < ?1",
        params![cutoff],
    )?;
    Ok(deleted)
}

/// Replace all samples atomically (used after compaction).
pub fn replace_all_samples(db: &Database, samples: &[SampleRow]) -> Result<usize> {
    let mut conn = db.conn();
    let tx = conn.transaction()?;

    tx.execute("DELETE FROM training_samples", [])?;

    let mut count = 0;
    {
        let mut stmt = tx.prepare_cached(
            "INSERT INTO training_samples
             (id, vector_json, chosen_mode, source, timestamp, base_weight)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6)"
        )?;

        for row in samples {
            stmt.execute(params![
                row.id, row.vector_json, row.chosen_mode,
                row.source, row.timestamp, row.base_weight,
            ])?;
            count += 1;
        }
    }

    tx.commit()?;
    Ok(count)
}

pub fn import_samples_json(db: &Database, json: &str) -> Result<usize> {
    let samples: Vec<serde_json::Value> = serde_json::from_str(json)?;

    let mut conn = db.conn();
    let tx = conn.transaction()?;
    let mut count = 0;

    {
        let mut stmt = tx.prepare_cached(
            "INSERT OR IGNORE INTO training_samples
             (id, vector_json, chosen_mode, source, timestamp, base_weight)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6)"
        )?;

        for sample in &samples {
            let id = sample.get("id").and_then(|v| v.as_str()).unwrap_or("").to_string();
            let vector = sample.get("vector").map(|v| v.to_string()).unwrap_or_else(|| "{}".to_string());
            let chosen_mode = sample.get("chosen_mode").and_then(|v| v.as_str()).unwrap_or("").to_string();
            let source = sample.get("source").and_then(|v| v.as_str()).unwrap_or("Bootstrap").to_string();
            let timestamp = sample.get("timestamp").and_then(|v| v.as_str()).unwrap_or("").to_string();
            let base_weight = sample.get("base_weight").and_then(|v| v.as_f64()).unwrap_or(1.0);

            if !id.is_empty() {
                stmt.execute(params![id, vector, chosen_mode, source, timestamp, base_weight])?;
                count += 1;
            }
        }
    }

    tx.commit()?;
    Ok(count)
}

fn map_sample_row(row: &rusqlite::Row<'_>) -> rusqlite::Result<SampleRow> {
    Ok(SampleRow {
        id: row.get(0)?,
        vector_json: row.get(1)?,
        chosen_mode: row.get(2)?,
        source: row.get(3)?,
        timestamp: row.get(4)?,
        base_weight: row.get(5)?,
    })
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
        let row = SampleRow {
            id: "s1".to_string(),
            vector_json: r#"{"dimensions":{"hour":0.5,"day_type":1.0}}"#.to_string(),
            chosen_mode: "pro".to_string(),
            source: "UserCorrection".to_string(),
            timestamp: "2026-02-26T10:00:00Z".to_string(),
            base_weight: 1.0,
        };
        insert_sample(&db, &row).unwrap();
        assert_eq!(count_samples(&db).unwrap(), 1);
    }

    #[test]
    fn test_list_ordered() {
        let db = test_db();
        for i in 0..3 {
            let row = SampleRow {
                id: format!("s{}", i),
                vector_json: "{}".to_string(),
                chosen_mode: "pro".to_string(),
                source: "Bootstrap".to_string(),
                timestamp: format!("2026-02-26T10:{:02}:00Z", i),
                base_weight: 1.0,
            };
            insert_sample(&db, &row).unwrap();
        }

        let samples = list_samples(&db).unwrap();
        assert_eq!(samples.len(), 3);
        // Newest first
        assert_eq!(samples[0].id, "s2");
    }

    #[test]
    fn test_delete_sample() {
        let db = test_db();
        let row = SampleRow {
            id: "s1".to_string(),
            vector_json: "{}".to_string(),
            chosen_mode: "focus".to_string(),
            source: "Automation".to_string(),
            timestamp: "2026-02-26T10:00:00Z".to_string(),
            base_weight: 0.8,
        };
        insert_sample(&db, &row).unwrap();
        assert!(delete_sample(&db, "s1").unwrap());
        assert_eq!(count_samples(&db).unwrap(), 0);
    }

    #[test]
    fn test_delete_old_samples() {
        let db = test_db();
        let old = SampleRow {
            id: "old".to_string(),
            vector_json: "{}".to_string(),
            chosen_mode: "pro".to_string(),
            source: "Bootstrap".to_string(),
            timestamp: "2020-01-01T00:00:00Z".to_string(),
            base_weight: 1.0,
        };
        let recent = SampleRow {
            id: "recent".to_string(),
            vector_json: "{}".to_string(),
            chosen_mode: "pro".to_string(),
            source: "UserCorrection".to_string(),
            timestamp: "2026-02-26T10:00:00Z".to_string(),
            base_weight: 1.0,
        };
        insert_sample(&db, &old).unwrap();
        insert_sample(&db, &recent).unwrap();

        let deleted = delete_old_samples(&db, "2025-01-01T00:00:00Z").unwrap();
        assert_eq!(deleted, 1);
        assert_eq!(count_samples(&db).unwrap(), 1);
    }

    #[test]
    fn test_replace_all_samples() {
        let db = test_db();
        // Insert initial
        for i in 0..5 {
            let row = SampleRow {
                id: format!("old-{}", i),
                vector_json: "{}".to_string(),
                chosen_mode: "pro".to_string(),
                source: "Bootstrap".to_string(),
                timestamp: "2026-02-26T10:00:00Z".to_string(),
                base_weight: 1.0,
            };
            insert_sample(&db, &row).unwrap();
        }
        assert_eq!(count_samples(&db).unwrap(), 5);

        // Replace with 2 new ones
        let new_samples = vec![
            SampleRow {
                id: "new-1".to_string(),
                vector_json: r#"{"dimensions":{"hour":0.5}}"#.to_string(),
                chosen_mode: "focus".to_string(),
                source: "UserCorrection".to_string(),
                timestamp: "2026-02-26T11:00:00Z".to_string(),
                base_weight: 1.3,
            },
            SampleRow {
                id: "new-2".to_string(),
                vector_json: r#"{"dimensions":{"hour":0.8}}"#.to_string(),
                chosen_mode: "pro".to_string(),
                source: "MfaConfirmed".to_string(),
                timestamp: "2026-02-26T11:30:00Z".to_string(),
                base_weight: 1.0,
            },
        ];

        let count = replace_all_samples(&db, &new_samples).unwrap();
        assert_eq!(count, 2);
        assert_eq!(count_samples(&db).unwrap(), 2);
    }

    #[test]
    fn test_import_samples_json() {
        let db = test_db();
        let json = r#"[
            {
                "id": "s1",
                "vector": {"dimensions": {"hour": 0.5}},
                "chosen_mode": "pro",
                "source": "UserCorrection",
                "timestamp": "2026-02-26T10:00:00Z",
                "base_weight": 1.0
            },
            {
                "id": "s2",
                "vector": {"dimensions": {"hour": 0.8}},
                "chosen_mode": "focus",
                "source": "Automation",
                "timestamp": "2026-02-26T10:30:00Z",
                "base_weight": 0.8
            }
        ]"#;
        let count = import_samples_json(&db, json).unwrap();
        assert_eq!(count, 2);
        assert_eq!(count_samples(&db).unwrap(), 2);
    }
}
