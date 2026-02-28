/**
 * SYMBION KERNEL - Learned Patterns SQLite Queries
 *
 * ROLE: Typed queries for learned behavior patterns persistence.
 * Replaces learned_patterns.json.
 * All functions take &Database and return anyhow::Result.
 */

use anyhow::Result;
use rusqlite::params;
use super::Database;

/// Single pattern row (maps to/from learned_patterns table).
pub struct PatternRow {
    pub mode: String,
    pub day_of_week: u8,
    pub hour: u8,
    pub confidence: f32,
    pub occurrences: u32,
    pub last_seen: String, // ISO 8601
    pub source: String,    // "Historical", "UserCorrection", "Automation"
}

/// Replace all patterns atomically (DELETE ALL + INSERT in a transaction).
pub fn replace_all_patterns(db: &Database, rows: &[PatternRow]) -> Result<usize> {
    let mut conn = db.conn();
    let tx = conn.transaction()?;

    tx.execute("DELETE FROM learned_patterns", [])?;

    {
        let mut stmt = tx.prepare_cached(
            "INSERT INTO learned_patterns (mode, day_of_week, hour, confidence, occurrences, last_seen, source)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)"
        )?;

        for row in rows {
            stmt.execute(params![
                row.mode,
                row.day_of_week as i32,
                row.hour as i32,
                row.confidence as f64,
                row.occurrences as i32,
                row.last_seen,
                row.source,
            ])?;
        }
    }

    tx.commit()?;
    Ok(rows.len())
}

/// List all patterns.
pub fn list_patterns(db: &Database) -> Result<Vec<PatternRow>> {
    let conn = db.conn();
    let mut stmt = conn.prepare(
        "SELECT mode, day_of_week, hour, confidence, occurrences, last_seen, source
         FROM learned_patterns
         ORDER BY confidence DESC"
    )?;

    let rows = stmt.query_map([], |row| {
        Ok(PatternRow {
            mode: row.get(0)?,
            day_of_week: row.get::<_, i32>(1)? as u8,
            hour: row.get::<_, i32>(2)? as u8,
            confidence: row.get::<_, f64>(3)? as f32,
            occurrences: row.get::<_, i32>(4)? as u32,
            last_seen: row.get(5)?,
            source: row.get(6)?,
        })
    })?;

    let mut result = Vec::new();
    for row in rows {
        result.push(row?);
    }
    Ok(result)
}

/// Count patterns.
pub fn count_patterns(db: &Database) -> Result<i64> {
    let conn = db.conn();
    conn.query_row("SELECT COUNT(*) FROM learned_patterns", [], |row| row.get(0))
        .map_err(Into::into)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_db() -> Database {
        Database::open_in_memory().unwrap()
    }

    #[test]
    fn test_replace_all_and_list() {
        let db = test_db();

        let patterns = vec![
            PatternRow {
                mode: "pro".to_string(),
                day_of_week: 0, // Monday
                hour: 9,
                confidence: 0.85,
                occurrences: 12,
                last_seen: "2026-02-28T09:00:00Z".to_string(),
                source: "UserCorrection".to_string(),
            },
            PatternRow {
                mode: "maison".to_string(),
                day_of_week: 4, // Friday
                hour: 18,
                confidence: 0.70,
                occurrences: 8,
                last_seen: "2026-02-28T18:00:00Z".to_string(),
                source: "Historical".to_string(),
            },
        ];

        let count = replace_all_patterns(&db, &patterns).unwrap();
        assert_eq!(count, 2);
        assert_eq!(count_patterns(&db).unwrap(), 2);

        let loaded = list_patterns(&db).unwrap();
        assert_eq!(loaded.len(), 2);
        // Ordered by confidence DESC
        assert_eq!(loaded[0].mode, "pro");
        assert_eq!(loaded[0].confidence, 0.85);
        assert_eq!(loaded[1].mode, "maison");
    }

    #[test]
    fn test_replace_all_clears_previous() {
        let db = test_db();

        // Insert initial set
        let initial = vec![PatternRow {
            mode: "veille".to_string(),
            day_of_week: 6,
            hour: 23,
            confidence: 0.60,
            occurrences: 5,
            last_seen: "2026-02-28T23:00:00Z".to_string(),
            source: "Automation".to_string(),
        }];
        replace_all_patterns(&db, &initial).unwrap();
        assert_eq!(count_patterns(&db).unwrap(), 1);

        // Replace with new set
        let replacement = vec![
            PatternRow {
                mode: "pro".to_string(),
                day_of_week: 0,
                hour: 8,
                confidence: 0.90,
                occurrences: 20,
                last_seen: "2026-02-28T08:00:00Z".to_string(),
                source: "UserCorrection".to_string(),
            },
            PatternRow {
                mode: "focus".to_string(),
                day_of_week: 1,
                hour: 10,
                confidence: 0.75,
                occurrences: 10,
                last_seen: "2026-02-28T10:00:00Z".to_string(),
                source: "Historical".to_string(),
            },
        ];
        replace_all_patterns(&db, &replacement).unwrap();
        assert_eq!(count_patterns(&db).unwrap(), 2);

        // Old pattern should be gone
        let loaded = list_patterns(&db).unwrap();
        assert!(loaded.iter().all(|p| p.mode != "veille"));
    }

    #[test]
    fn test_replace_with_empty() {
        let db = test_db();

        let patterns = vec![PatternRow {
            mode: "pro".to_string(),
            day_of_week: 0,
            hour: 9,
            confidence: 0.85,
            occurrences: 12,
            last_seen: "2026-02-28T09:00:00Z".to_string(),
            source: "Historical".to_string(),
        }];
        replace_all_patterns(&db, &patterns).unwrap();
        assert_eq!(count_patterns(&db).unwrap(), 1);

        // Replace with empty
        replace_all_patterns(&db, &[]).unwrap();
        assert_eq!(count_patterns(&db).unwrap(), 0);
    }
}
