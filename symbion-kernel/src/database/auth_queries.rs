/**
 * SYMBION KERNEL - Auth SQLite Queries
 *
 * ROLE: Typed queries for users, device tokens, and WebAuthn credentials.
 * All functions take &Database and return anyhow::Result.
 */

use anyhow::Result;
use rusqlite::params;
use super::Database;

// ============================================================================
// Users
// ============================================================================

pub struct UserRow {
    pub username: String,
    pub password_hash: String,
    pub role: String,
    pub created_at: i64,
    pub mfa_config_json: Option<String>,
}

pub fn upsert_user(db: &Database, row: &UserRow) -> Result<()> {
    let conn = db.conn();
    conn.execute(
        "INSERT INTO users (username, password_hash, role, created_at, mfa_config_json)
         VALUES (?1, ?2, ?3, ?4, ?5)
         ON CONFLICT(username) DO UPDATE SET
             password_hash = excluded.password_hash,
             role = excluded.role,
             mfa_config_json = excluded.mfa_config_json",
        params![
            row.username,
            row.password_hash,
            row.role,
            row.created_at,
            row.mfa_config_json,
        ],
    )?;
    Ok(())
}

pub fn get_user(db: &Database, username: &str) -> Result<Option<UserRow>> {
    let conn = db.conn();
    let mut stmt = conn.prepare(
        "SELECT username, password_hash, role, created_at, mfa_config_json
         FROM users WHERE username = ?1"
    )?;

    let mut rows = stmt.query_map(params![username], |row| {
        Ok(UserRow {
            username: row.get(0)?,
            password_hash: row.get(1)?,
            role: row.get(2)?,
            created_at: row.get(3)?,
            mfa_config_json: row.get(4)?,
        })
    })?;

    match rows.next() {
        Some(row) => Ok(Some(row?)),
        None => Ok(None),
    }
}

pub fn list_users(db: &Database) -> Result<Vec<UserRow>> {
    let conn = db.conn();
    let mut stmt = conn.prepare(
        "SELECT username, password_hash, role, created_at, mfa_config_json
         FROM users ORDER BY username"
    )?;

    let rows = stmt.query_map([], |row| {
        Ok(UserRow {
            username: row.get(0)?,
            password_hash: row.get(1)?,
            role: row.get(2)?,
            created_at: row.get(3)?,
            mfa_config_json: row.get(4)?,
        })
    })?;

    let mut result = Vec::new();
    for row in rows {
        result.push(row?);
    }
    Ok(result)
}

pub fn delete_user(db: &Database, username: &str) -> Result<bool> {
    let conn = db.conn();
    let deleted = conn.execute("DELETE FROM users WHERE username = ?1", params![username])?;
    Ok(deleted > 0)
}

pub fn count_users(db: &Database) -> Result<i64> {
    let conn = db.conn();
    conn.query_row("SELECT COUNT(*) FROM users", [], |row| row.get(0))
        .map_err(Into::into)
}

pub fn import_users_json(db: &Database, json: &str) -> Result<usize> {
    let users: std::collections::HashMap<String, serde_json::Value> = serde_json::from_str(json)?;

    let mut conn = db.conn();
    let tx = conn.transaction()?;
    let mut count = 0;

    {
        let mut stmt = tx.prepare_cached(
            "INSERT OR IGNORE INTO users (username, password_hash, role, created_at, mfa_config_json)
             VALUES (?1, ?2, ?3, ?4, ?5)"
        )?;

        for (username, user) in &users {
            let password_hash = user.get("password_hash")
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_string();
            let role = user.get("role")
                .and_then(|v| v.as_str())
                .unwrap_or("user")
                .to_string();
            let created_at = user.get("created_at")
                .and_then(|v| v.as_i64())
                .unwrap_or(0);
            let mfa_config = user.get("mfa_config")
                .filter(|v| !v.is_null())
                .map(|v| v.to_string());

            stmt.execute(params![username, password_hash, role, created_at, mfa_config])?;
            count += 1;
        }
    }

    tx.commit()?;
    Ok(count)
}

// ============================================================================
// Device Tokens
// ============================================================================

pub struct DeviceTokenRow {
    pub token: String,
    pub username: String,
    pub device_fingerprint: String,
    pub created_at: i64,
    pub expires_at: i64,
    pub last_used_at: i64,
}

pub fn upsert_device_token(db: &Database, row: &DeviceTokenRow) -> Result<()> {
    let conn = db.conn();
    conn.execute(
        "INSERT INTO device_tokens (token, username, device_fingerprint, created_at, expires_at, last_used_at)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6)
         ON CONFLICT(token) DO UPDATE SET
             last_used_at = excluded.last_used_at",
        params![
            row.token,
            row.username,
            row.device_fingerprint,
            row.created_at,
            row.expires_at,
            row.last_used_at,
        ],
    )?;
    Ok(())
}

pub fn get_device_token(db: &Database, token: &str) -> Result<Option<DeviceTokenRow>> {
    let conn = db.conn();
    let mut stmt = conn.prepare(
        "SELECT token, username, device_fingerprint, created_at, expires_at, last_used_at
         FROM device_tokens WHERE token = ?1"
    )?;

    let mut rows = stmt.query_map(params![token], |row| {
        Ok(DeviceTokenRow {
            token: row.get(0)?,
            username: row.get(1)?,
            device_fingerprint: row.get(2)?,
            created_at: row.get(3)?,
            expires_at: row.get(4)?,
            last_used_at: row.get(5)?,
        })
    })?;

    match rows.next() {
        Some(row) => Ok(Some(row?)),
        None => Ok(None),
    }
}

pub fn list_user_tokens(db: &Database, username: &str) -> Result<Vec<DeviceTokenRow>> {
    let conn = db.conn();
    let mut stmt = conn.prepare(
        "SELECT token, username, device_fingerprint, created_at, expires_at, last_used_at
         FROM device_tokens WHERE username = ?1 ORDER BY created_at DESC"
    )?;

    let rows = stmt.query_map(params![username], |row| {
        Ok(DeviceTokenRow {
            token: row.get(0)?,
            username: row.get(1)?,
            device_fingerprint: row.get(2)?,
            created_at: row.get(3)?,
            expires_at: row.get(4)?,
            last_used_at: row.get(5)?,
        })
    })?;

    let mut result = Vec::new();
    for row in rows {
        result.push(row?);
    }
    Ok(result)
}

pub fn delete_device_token(db: &Database, token: &str) -> Result<bool> {
    let conn = db.conn();
    let deleted = conn.execute("DELETE FROM device_tokens WHERE token = ?1", params![token])?;
    Ok(deleted > 0)
}

pub fn delete_expired_tokens(db: &Database, now: i64) -> Result<usize> {
    let conn = db.conn();
    let deleted = conn.execute(
        "DELETE FROM device_tokens WHERE expires_at < ?1",
        params![now],
    )?;
    Ok(deleted)
}

pub fn import_device_tokens_json(db: &Database, json: &str) -> Result<usize> {
    let tokens: std::collections::HashMap<String, serde_json::Value> = serde_json::from_str(json)?;

    let mut conn = db.conn();
    let tx = conn.transaction()?;
    let mut count = 0;

    {
        let mut stmt = tx.prepare_cached(
            "INSERT OR IGNORE INTO device_tokens
             (token, username, device_fingerprint, created_at, expires_at, last_used_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6)"
        )?;

        for (_token_id, token) in &tokens {
            let t = token.get("token").and_then(|v| v.as_str()).unwrap_or("").to_string();
            let username = token.get("username").and_then(|v| v.as_str()).unwrap_or("").to_string();
            let fingerprint = token.get("device_fingerprint").and_then(|v| v.as_str()).unwrap_or("").to_string();
            let created_at = token.get("created_at").and_then(|v| v.as_i64()).unwrap_or(0);
            let expires_at = token.get("expires_at").and_then(|v| v.as_i64()).unwrap_or(0);
            let last_used_at = token.get("last_used_at").and_then(|v| v.as_i64()).unwrap_or(0);

            if !t.is_empty() {
                stmt.execute(params![t, username, fingerprint, created_at, expires_at, last_used_at])?;
                count += 1;
            }
        }
    }

    tx.commit()?;
    Ok(count)
}

// ============================================================================
// WebAuthn Credentials
// ============================================================================

pub struct WebauthnRow {
    pub id: Option<i64>,
    pub username: String,
    pub credential_id: Vec<u8>,
    pub credential_json: String,
    pub friendly_name: String,
    pub created_at: i64,
    pub last_used_at: Option<i64>,
}

pub fn insert_credential(db: &Database, row: &WebauthnRow) -> Result<i64> {
    let conn = db.conn();
    conn.execute(
        "INSERT INTO webauthn_credentials
         (username, credential_id, credential_json, friendly_name, created_at, last_used_at)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
        params![
            row.username,
            row.credential_id,
            row.credential_json,
            row.friendly_name,
            row.created_at,
            row.last_used_at,
        ],
    )?;
    Ok(conn.last_insert_rowid())
}

pub fn list_credentials(db: &Database, username: &str) -> Result<Vec<WebauthnRow>> {
    let conn = db.conn();
    let mut stmt = conn.prepare(
        "SELECT id, username, credential_id, credential_json, friendly_name, created_at, last_used_at
         FROM webauthn_credentials WHERE username = ?1 ORDER BY created_at DESC"
    )?;

    let rows = stmt.query_map(params![username], |row| {
        Ok(WebauthnRow {
            id: Some(row.get(0)?),
            username: row.get(1)?,
            credential_id: row.get(2)?,
            credential_json: row.get(3)?,
            friendly_name: row.get(4)?,
            created_at: row.get(5)?,
            last_used_at: row.get(6)?,
        })
    })?;

    let mut result = Vec::new();
    for row in rows {
        result.push(row?);
    }
    Ok(result)
}

pub fn list_all_credentials(db: &Database) -> Result<Vec<WebauthnRow>> {
    let conn = db.conn();
    let mut stmt = conn.prepare(
        "SELECT id, username, credential_id, credential_json, friendly_name, created_at, last_used_at
         FROM webauthn_credentials ORDER BY username, created_at DESC"
    )?;

    let rows = stmt.query_map([], |row| {
        Ok(WebauthnRow {
            id: Some(row.get(0)?),
            username: row.get(1)?,
            credential_id: row.get(2)?,
            credential_json: row.get(3)?,
            friendly_name: row.get(4)?,
            created_at: row.get(5)?,
            last_used_at: row.get(6)?,
        })
    })?;

    let mut result = Vec::new();
    for row in rows {
        result.push(row?);
    }
    Ok(result)
}

pub fn update_credential_last_used(db: &Database, credential_id: i64, last_used: i64) -> Result<()> {
    let conn = db.conn();
    conn.execute(
        "UPDATE webauthn_credentials SET last_used_at = ?1 WHERE id = ?2",
        params![last_used, credential_id],
    )?;
    Ok(())
}

pub fn delete_credential(db: &Database, credential_id: i64) -> Result<bool> {
    let conn = db.conn();
    let deleted = conn.execute(
        "DELETE FROM webauthn_credentials WHERE id = ?1",
        params![credential_id],
    )?;
    Ok(deleted > 0)
}

pub fn count_credentials(db: &Database) -> Result<i64> {
    let conn = db.conn();
    conn.query_row("SELECT COUNT(*) FROM webauthn_credentials", [], |row| row.get(0))
        .map_err(Into::into)
}

pub fn import_webauthn_json(db: &Database, json: &str) -> Result<usize> {
    let data: std::collections::HashMap<String, Vec<serde_json::Value>> = serde_json::from_str(json)?;

    let mut conn = db.conn();
    let tx = conn.transaction()?;
    let mut count = 0;

    {
        let mut stmt = tx.prepare_cached(
            "INSERT OR IGNORE INTO webauthn_credentials
             (username, credential_id, credential_json, friendly_name, created_at, last_used_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6)"
        )?;

        for (username, creds) in &data {
            for cred in creds {
                let credential_id_b64 = cred.get("credential_id")
                    .and_then(|v| v.as_array())
                    .map(|arr| arr.iter().filter_map(|v| v.as_u64().map(|n| n as u8)).collect::<Vec<u8>>())
                    .unwrap_or_default();
                let credential_json = cred.get("credential")
                    .map(|v| v.to_string())
                    .unwrap_or_else(|| "{}".to_string());
                let friendly_name = cred.get("friendly_name")
                    .and_then(|v| v.as_str())
                    .unwrap_or("")
                    .to_string();
                let created_at = cred.get("created_at")
                    .and_then(|v| v.as_i64())
                    .unwrap_or(0);
                let last_used_at = cred.get("last_used_at")
                    .and_then(|v| v.as_i64());

                stmt.execute(params![
                    username, credential_id_b64, credential_json,
                    friendly_name, created_at, last_used_at,
                ])?;
                count += 1;
            }
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

    // --- Users ---

    #[test]
    fn test_upsert_and_get_user() {
        let db = test_db();
        let row = UserRow {
            username: "admin".to_string(),
            password_hash: "$2b$12$hash".to_string(),
            role: "admin".to_string(),
            created_at: 1700000000,
            mfa_config_json: None,
        };
        upsert_user(&db, &row).unwrap();

        let loaded = get_user(&db, "admin").unwrap().unwrap();
        assert_eq!(loaded.username, "admin");
        assert_eq!(loaded.role, "admin");
        assert!(loaded.mfa_config_json.is_none());
    }

    #[test]
    fn test_upsert_updates_existing() {
        let db = test_db();
        let row = UserRow {
            username: "admin".to_string(),
            password_hash: "$2b$12$old".to_string(),
            role: "user".to_string(),
            created_at: 1700000000,
            mfa_config_json: None,
        };
        upsert_user(&db, &row).unwrap();

        let updated = UserRow {
            username: "admin".to_string(),
            password_hash: "$2b$12$new".to_string(),
            role: "admin".to_string(),
            created_at: 1700000000,
            mfa_config_json: Some(r#"{"secret":"abc"}"#.to_string()),
        };
        upsert_user(&db, &updated).unwrap();

        assert_eq!(count_users(&db).unwrap(), 1);
        let loaded = get_user(&db, "admin").unwrap().unwrap();
        assert_eq!(loaded.password_hash, "$2b$12$new");
        assert_eq!(loaded.role, "admin");
        assert!(loaded.mfa_config_json.is_some());
    }

    #[test]
    fn test_list_and_delete_users() {
        let db = test_db();
        for name in ["alice", "bob", "charlie"] {
            let row = UserRow {
                username: name.to_string(),
                password_hash: "hash".to_string(),
                role: "user".to_string(),
                created_at: 1700000000,
                mfa_config_json: None,
            };
            upsert_user(&db, &row).unwrap();
        }

        assert_eq!(list_users(&db).unwrap().len(), 3);
        assert!(delete_user(&db, "bob").unwrap());
        assert_eq!(count_users(&db).unwrap(), 2);
        assert!(!delete_user(&db, "nonexistent").unwrap());
    }

    #[test]
    fn test_import_users_json() {
        let db = test_db();
        let json = r#"{
            "admin": {
                "username": "admin",
                "password_hash": "$2b$12$hash",
                "role": "admin",
                "created_at": 1700000000
            },
            "user1": {
                "username": "user1",
                "password_hash": "$2b$12$hash2",
                "role": "user",
                "created_at": 1700000001,
                "mfa_config": {"secret": "test"}
            }
        }"#;
        let count = import_users_json(&db, json).unwrap();
        assert_eq!(count, 2);
        assert_eq!(count_users(&db).unwrap(), 2);
    }

    // --- Device Tokens ---

    #[test]
    fn test_upsert_and_get_device_token() {
        let db = test_db();
        let row = DeviceTokenRow {
            token: "tok-123".to_string(),
            username: "admin".to_string(),
            device_fingerprint: "fp-abc".to_string(),
            created_at: 1700000000,
            expires_at: 1702592000,
            last_used_at: 1700000000,
        };
        upsert_device_token(&db, &row).unwrap();

        let loaded = get_device_token(&db, "tok-123").unwrap().unwrap();
        assert_eq!(loaded.username, "admin");
        assert_eq!(loaded.device_fingerprint, "fp-abc");
    }

    #[test]
    fn test_list_and_delete_tokens() {
        let db = test_db();
        for i in 0..3 {
            let row = DeviceTokenRow {
                token: format!("tok-{}", i),
                username: "admin".to_string(),
                device_fingerprint: "fp".to_string(),
                created_at: 1700000000 + i,
                expires_at: 1702592000 + i,
                last_used_at: 1700000000 + i,
            };
            upsert_device_token(&db, &row).unwrap();
        }

        assert_eq!(list_user_tokens(&db, "admin").unwrap().len(), 3);
        assert!(delete_device_token(&db, "tok-1").unwrap());
        assert_eq!(list_user_tokens(&db, "admin").unwrap().len(), 2);
    }

    #[test]
    fn test_delete_expired_tokens() {
        let db = test_db();
        // One expired, one valid
        let expired = DeviceTokenRow {
            token: "expired".to_string(),
            username: "admin".to_string(),
            device_fingerprint: "fp".to_string(),
            created_at: 1700000000,
            expires_at: 1700000100, // past
            last_used_at: 1700000000,
        };
        let valid = DeviceTokenRow {
            token: "valid".to_string(),
            username: "admin".to_string(),
            device_fingerprint: "fp".to_string(),
            created_at: 1700000000,
            expires_at: 9999999999, // far future
            last_used_at: 1700000000,
        };
        upsert_device_token(&db, &expired).unwrap();
        upsert_device_token(&db, &valid).unwrap();

        let deleted = delete_expired_tokens(&db, 1700001000).unwrap();
        assert_eq!(deleted, 1);
        assert!(get_device_token(&db, "valid").unwrap().is_some());
        assert!(get_device_token(&db, "expired").unwrap().is_none());
    }

    // --- WebAuthn ---

    #[test]
    fn test_insert_and_list_credentials() {
        let db = test_db();
        let row = WebauthnRow {
            id: None,
            username: "admin".to_string(),
            credential_id: vec![1, 2, 3, 4],
            credential_json: r#"{"type":"public-key"}"#.to_string(),
            friendly_name: "iPhone 15".to_string(),
            created_at: 1700000000,
            last_used_at: None,
        };
        let id = insert_credential(&db, &row).unwrap();
        assert!(id > 0);

        let creds = list_credentials(&db, "admin").unwrap();
        assert_eq!(creds.len(), 1);
        assert_eq!(creds[0].friendly_name, "iPhone 15");
        assert_eq!(creds[0].credential_id, vec![1, 2, 3, 4]);
    }

    #[test]
    fn test_update_and_delete_credential() {
        let db = test_db();
        let row = WebauthnRow {
            id: None,
            username: "admin".to_string(),
            credential_id: vec![5, 6],
            credential_json: "{}".to_string(),
            friendly_name: "Test".to_string(),
            created_at: 1700000000,
            last_used_at: None,
        };
        let id = insert_credential(&db, &row).unwrap();

        update_credential_last_used(&db, id, 1700001000).unwrap();
        let creds = list_credentials(&db, "admin").unwrap();
        assert_eq!(creds[0].last_used_at, Some(1700001000));

        assert!(delete_credential(&db, id).unwrap());
        assert_eq!(count_credentials(&db).unwrap(), 0);
    }
}
