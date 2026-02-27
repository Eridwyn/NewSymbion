/**
 * SYMBION KERNEL - Agent SQLite Queries
 *
 * ROLE: Typed queries for agent registry persistence.
 * Complex nested structs (status, network, capabilities) stored as JSON columns.
 * All functions take &Database and return anyhow::Result.
 */

use anyhow::Result;
use rusqlite::params;
use super::Database;

/// Agent row — status/network/capabilities as JSON text columns.
pub struct AgentRow {
    pub agent_id: String,
    pub hostname: String,
    pub os: String,
    pub architecture: String,
    pub capabilities_json: String,
    pub network_json: String,
    pub version: Option<String>,
    pub status_json: String,
    pub last_seen: String,
    pub registration_time: String,
    pub deleted_at: Option<String>,
}

pub fn upsert_agent(db: &Database, row: &AgentRow) -> Result<()> {
    let conn = db.conn();
    conn.execute(
        "INSERT INTO agents
         (agent_id, hostname, os, architecture, capabilities_json, network_json,
          version, status_json, last_seen, registration_time, deleted_at)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11)
         ON CONFLICT(agent_id) DO UPDATE SET
             hostname = excluded.hostname,
             os = excluded.os,
             architecture = excluded.architecture,
             capabilities_json = excluded.capabilities_json,
             network_json = excluded.network_json,
             version = excluded.version,
             status_json = excluded.status_json,
             last_seen = excluded.last_seen,
             deleted_at = excluded.deleted_at",
        params![
            row.agent_id,
            row.hostname,
            row.os,
            row.architecture,
            row.capabilities_json,
            row.network_json,
            row.version,
            row.status_json,
            row.last_seen,
            row.registration_time,
            row.deleted_at,
        ],
    )?;
    Ok(())
}

pub fn get_agent(db: &Database, agent_id: &str) -> Result<Option<AgentRow>> {
    let conn = db.conn();
    let mut stmt = conn.prepare(
        "SELECT agent_id, hostname, os, architecture, capabilities_json, network_json,
                version, status_json, last_seen, registration_time, deleted_at
         FROM agents WHERE agent_id = ?1"
    )?;

    let mut rows = stmt.query_map(params![agent_id], map_agent_row)?;

    match rows.next() {
        Some(row) => Ok(Some(row?)),
        None => Ok(None),
    }
}

/// List agents excluding soft-deleted ones.
pub fn list_agents(db: &Database) -> Result<Vec<AgentRow>> {
    let conn = db.conn();
    let mut stmt = conn.prepare(
        "SELECT agent_id, hostname, os, architecture, capabilities_json, network_json,
                version, status_json, last_seen, registration_time, deleted_at
         FROM agents WHERE deleted_at IS NULL
         ORDER BY registration_time"
    )?;

    let rows = stmt.query_map([], map_agent_row)?;

    let mut result = Vec::new();
    for row in rows {
        result.push(row?);
    }
    Ok(result)
}

/// List all agents including soft-deleted.
pub fn list_all_agents(db: &Database) -> Result<Vec<AgentRow>> {
    let conn = db.conn();
    let mut stmt = conn.prepare(
        "SELECT agent_id, hostname, os, architecture, capabilities_json, network_json,
                version, status_json, last_seen, registration_time, deleted_at
         FROM agents ORDER BY registration_time"
    )?;

    let rows = stmt.query_map([], map_agent_row)?;

    let mut result = Vec::new();
    for row in rows {
        result.push(row?);
    }
    Ok(result)
}

pub fn soft_delete_agent(db: &Database, agent_id: &str, deleted_at: &str) -> Result<bool> {
    let conn = db.conn();
    let updated = conn.execute(
        "UPDATE agents SET deleted_at = ?1 WHERE agent_id = ?2 AND deleted_at IS NULL",
        params![deleted_at, agent_id],
    )?;
    Ok(updated > 0)
}

pub fn count_agents(db: &Database) -> Result<i64> {
    let conn = db.conn();
    conn.query_row(
        "SELECT COUNT(*) FROM agents WHERE deleted_at IS NULL",
        [],
        |row| row.get(0),
    ).map_err(Into::into)
}

pub fn import_agents_json(db: &Database, json: &str) -> Result<usize> {
    let agents: std::collections::HashMap<String, serde_json::Value> = serde_json::from_str(json)?;

    let mut conn = db.conn();
    let tx = conn.transaction()?;
    let mut count = 0;

    {
        let mut stmt = tx.prepare_cached(
            "INSERT OR IGNORE INTO agents
             (agent_id, hostname, os, architecture, capabilities_json, network_json,
              version, status_json, last_seen, registration_time, deleted_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11)"
        )?;

        for (agent_id, agent) in &agents {
            let hostname = agent.get("hostname").and_then(|v| v.as_str()).unwrap_or("").to_string();
            let os = agent.get("os").and_then(|v| v.as_str()).unwrap_or("").to_string();
            let arch = agent.get("architecture").and_then(|v| v.as_str()).unwrap_or("").to_string();
            let caps = agent.get("capabilities").map(|v| v.to_string()).unwrap_or_else(|| "[]".to_string());
            let network = agent.get("network").map(|v| v.to_string()).unwrap_or_else(|| "{}".to_string());
            let version = agent.get("version").and_then(|v| v.as_str()).map(|s| s.to_string());
            let status = agent.get("status").map(|v| v.to_string()).unwrap_or_else(|| "{}".to_string());
            let last_seen = agent.get("last_seen").and_then(|v| v.as_str()).unwrap_or("").to_string();
            let reg_time = agent.get("registration_time").and_then(|v| v.as_str()).unwrap_or("").to_string();
            let deleted = agent.get("deleted_at").and_then(|v| v.as_str()).map(|s| s.to_string());

            if !agent_id.is_empty() {
                stmt.execute(params![
                    agent_id, hostname, os, arch, caps, network,
                    version, status, last_seen, reg_time, deleted,
                ])?;
                count += 1;
            }
        }
    }

    tx.commit()?;
    Ok(count)
}

fn map_agent_row(row: &rusqlite::Row<'_>) -> rusqlite::Result<AgentRow> {
    Ok(AgentRow {
        agent_id: row.get(0)?,
        hostname: row.get(1)?,
        os: row.get(2)?,
        architecture: row.get(3)?,
        capabilities_json: row.get(4)?,
        network_json: row.get(5)?,
        version: row.get(6)?,
        status_json: row.get(7)?,
        last_seen: row.get(8)?,
        registration_time: row.get(9)?,
        deleted_at: row.get(10)?,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_db() -> Database {
        Database::open_in_memory().unwrap()
    }

    fn sample_agent(id: &str) -> AgentRow {
        AgentRow {
            agent_id: id.to_string(),
            hostname: "pc-salon".to_string(),
            os: "linux".to_string(),
            architecture: "x86_64".to_string(),
            capabilities_json: r#"["monitor","execute"]"#.to_string(),
            network_json: r#"{"local_ip":"192.168.1.10"}"#.to_string(),
            version: Some("1.0.0".to_string()),
            status_json: r#"{"state":"online"}"#.to_string(),
            last_seen: "2026-02-26T10:00:00Z".to_string(),
            registration_time: "2026-02-01T00:00:00Z".to_string(),
            deleted_at: None,
        }
    }

    #[test]
    fn test_upsert_and_get_agent() {
        let db = test_db();
        upsert_agent(&db, &sample_agent("agent-1")).unwrap();

        let loaded = get_agent(&db, "agent-1").unwrap().unwrap();
        assert_eq!(loaded.hostname, "pc-salon");
        assert_eq!(loaded.os, "linux");
    }

    #[test]
    fn test_upsert_updates_existing() {
        let db = test_db();
        upsert_agent(&db, &sample_agent("agent-1")).unwrap();

        let mut updated = sample_agent("agent-1");
        updated.hostname = "pc-bureau".to_string();
        updated.last_seen = "2026-02-26T11:00:00Z".to_string();
        upsert_agent(&db, &updated).unwrap();

        assert_eq!(count_agents(&db).unwrap(), 1);
        let loaded = get_agent(&db, "agent-1").unwrap().unwrap();
        assert_eq!(loaded.hostname, "pc-bureau");
    }

    #[test]
    fn test_list_agents_excludes_deleted() {
        let db = test_db();
        upsert_agent(&db, &sample_agent("a1")).unwrap();
        upsert_agent(&db, &sample_agent("a2")).unwrap();

        soft_delete_agent(&db, "a1", "2026-02-26T12:00:00Z").unwrap();

        let active = list_agents(&db).unwrap();
        assert_eq!(active.len(), 1);
        assert_eq!(active[0].agent_id, "a2");

        let all = list_all_agents(&db).unwrap();
        assert_eq!(all.len(), 2);
    }

    #[test]
    fn test_import_agents_json() {
        let db = test_db();
        let json = r#"{
            "agent-1": {
                "hostname": "pc-salon",
                "os": "linux",
                "architecture": "x86_64",
                "capabilities": ["monitor"],
                "network": {"local_ip": "192.168.1.10"},
                "status": {"state": "online"},
                "last_seen": "2026-02-26T10:00:00Z",
                "registration_time": "2026-02-01T00:00:00Z"
            }
        }"#;
        let count = import_agents_json(&db, json).unwrap();
        assert_eq!(count, 1);
        assert_eq!(count_agents(&db).unwrap(), 1);
    }
}
