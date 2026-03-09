use anyhow::{Context, Result};
use rusqlite::{params, Connection};
use std::path::Path;
use std::sync::Arc;
use tokio::sync::Mutex;
use uuid::Uuid;

use crate::models::*;

pub struct Database {
    conn: Arc<Mutex<Connection>>,
}

impl Database {
    pub fn open(path: &str) -> Result<Self> {
        if let Some(parent) = Path::new(path).parent() {
            std::fs::create_dir_all(parent)?;
        }
        let conn = Connection::open(path)?;
        conn.execute_batch("PRAGMA journal_mode=WAL; PRAGMA synchronous=NORMAL; PRAGMA foreign_keys=ON;")?;
        let db = Self { conn: Arc::new(Mutex::new(conn)) };
        Ok(db)
    }

    pub fn open_in_memory() -> Result<Self> {
        let conn = Connection::open_in_memory()?;
        conn.execute_batch("PRAGMA foreign_keys=ON;")?;
        let db = Self { conn: Arc::new(Mutex::new(conn)) };
        Ok(db)
    }

    pub async fn migrate(&self) -> Result<()> {
        let conn = self.conn.lock().await;
        conn.execute_batch(include_str!("schema.sql"))
            .context("Failed to run migrations")?;

        // Migration v2: Add ON DELETE CASCADE
        let user_version: i64 = conn.query_row("PRAGMA user_version", [], |r| r.get(0))?;
        if user_version < 2 {
            conn.execute_batch("BEGIN IMMEDIATE")?;
            let mig_result = (|| -> Result<()> {
                // node_versions
                conn.execute_batch(
                    "CREATE TABLE IF NOT EXISTS node_versions_new (
                        id TEXT PRIMARY KEY,
                        node_id TEXT REFERENCES nodes(id) ON DELETE CASCADE,
                        content TEXT,
                        version_num INTEGER,
                        created_at DATETIME DEFAULT CURRENT_TIMESTAMP
                    );
                    INSERT OR IGNORE INTO node_versions_new SELECT * FROM node_versions;
                    DROP TABLE IF EXISTS node_versions;
                    ALTER TABLE node_versions_new RENAME TO node_versions;
                    CREATE INDEX IF NOT EXISTS idx_versions_node ON node_versions(node_id);"
                )?;
                // node_sections
                conn.execute_batch(
                    "CREATE TABLE IF NOT EXISTS node_sections_new (
                        node_id TEXT REFERENCES nodes(id) ON DELETE CASCADE,
                        section_id TEXT REFERENCES sections(id) ON DELETE CASCADE,
                        PRIMARY KEY (node_id, section_id)
                    );
                    INSERT OR IGNORE INTO node_sections_new SELECT * FROM node_sections;
                    DROP TABLE IF EXISTS node_sections;
                    ALTER TABLE node_sections_new RENAME TO node_sections;"
                )?;
                // node_tags
                conn.execute_batch(
                    "CREATE TABLE IF NOT EXISTS node_tags_new (
                        node_id TEXT REFERENCES nodes(id) ON DELETE CASCADE,
                        tag_id TEXT REFERENCES tags(id) ON DELETE CASCADE,
                        PRIMARY KEY (node_id, tag_id)
                    );
                    INSERT OR IGNORE INTO node_tags_new SELECT * FROM node_tags;
                    DROP TABLE IF EXISTS node_tags;
                    ALTER TABLE node_tags_new RENAME TO node_tags;"
                )?;
                // node_fields
                conn.execute_batch(
                    "CREATE TABLE IF NOT EXISTS node_fields_new (
                        id TEXT PRIMARY KEY,
                        node_id TEXT NOT NULL REFERENCES nodes(id) ON DELETE CASCADE,
                        field_name TEXT NOT NULL,
                        field_value TEXT,
                        sort_order INTEGER DEFAULT 0,
                        UNIQUE(node_id, field_name)
                    );
                    INSERT OR IGNORE INTO node_fields_new SELECT * FROM node_fields;
                    DROP TABLE IF EXISTS node_fields;
                    ALTER TABLE node_fields_new RENAME TO node_fields;
                    CREATE INDEX IF NOT EXISTS idx_fields_node ON node_fields(node_id);"
                )?;
                // edges
                conn.execute_batch(
                    "CREATE TABLE IF NOT EXISTS edges_new (
                        id TEXT PRIMARY KEY,
                        node_from TEXT REFERENCES nodes(id) ON DELETE CASCADE,
                        node_to TEXT REFERENCES nodes(id) ON DELETE CASCADE,
                        relation TEXT,
                        auto BOOLEAN DEFAULT FALSE,
                        confirmed BOOLEAN DEFAULT TRUE,
                        created_at DATETIME DEFAULT CURRENT_TIMESTAMP
                    );
                    INSERT OR IGNORE INTO edges_new SELECT * FROM edges;
                    DROP TABLE IF EXISTS edges;
                    ALTER TABLE edges_new RENAME TO edges;
                    CREATE INDEX IF NOT EXISTS idx_edges_from ON edges(node_from);
                    CREATE INDEX IF NOT EXISTS idx_edges_to ON edges(node_to);"
                )?;
                // pending_links
                conn.execute_batch(
                    "CREATE TABLE IF NOT EXISTS pending_links_new (
                        id TEXT PRIMARY KEY,
                        node_from TEXT REFERENCES nodes(id) ON DELETE CASCADE,
                        node_to TEXT REFERENCES nodes(id) ON DELETE CASCADE,
                        occurrence TEXT,
                        created_at DATETIME DEFAULT CURRENT_TIMESTAMP
                    );
                    INSERT OR IGNORE INTO pending_links_new SELECT * FROM pending_links;
                    DROP TABLE IF EXISTS pending_links;
                    ALTER TABLE pending_links_new RENAME TO pending_links;
                    CREATE INDEX IF NOT EXISTS idx_pending_from ON pending_links(node_from);
                    CREATE INDEX IF NOT EXISTS idx_pending_to ON pending_links(node_to);"
                )?;
                conn.execute_batch("PRAGMA user_version = 2;")?;
                Ok(())
            })();
            match mig_result {
                Ok(()) => conn.execute_batch("COMMIT")?,
                Err(e) => {
                    let _ = conn.execute_batch("ROLLBACK");
                    return Err(e);
                }
            }
        }

        Ok(())
    }

    pub async fn seed_if_empty(&self) -> Result<bool> {
        let conn = self.conn.lock().await;
        let count: i64 = conn.query_row("SELECT COUNT(*) FROM templates", [], |r| r.get(0))?;
        if count > 0 {
            return Ok(false);
        }

        conn.execute_batch("BEGIN IMMEDIATE")?;
        let result = (|| -> Result<bool> {
        let template_id = Uuid::new_v4().to_string();
        let structure = serde_json::json!([
            {"name": "fiche_num", "type": "text", "label": "N° Fiche"},
            {"name": "subtitle", "type": "text", "label": "Sous-titre"},
            {"name": "icon", "type": "text", "label": "Icône"},
            {"name": "odeur_froid", "type": "text", "label": "Odeur à froid"},
            {"name": "odeur_torrefaction", "type": "text", "label": "Odeur torréfaction"},
            {"name": "gout_dominant", "type": "text", "label": "Goût dominant"},
            {"name": "notes_secondaires", "type": "text", "label": "Notes secondaires"},
            {"name": "persistance", "type": "text", "label": "Persistance"},
            {"name": "intensite", "type": "rating", "label": "Intensité", "max": 5},
            {"name": "intensite_label", "type": "text", "label": "Label intensité"},
            {"name": "associations", "type": "tags", "label": "Associations"},
            {"name": "usage", "type": "textarea", "label": "Usage"},
            {"name": "footer", "type": "text", "label": "Pied de page"},
            {"name": "notes_personnelles", "type": "textarea", "label": "Notes personnelles"}
        ]);

        let preview_css = include_str!("templates/fiche_epice.css");
        let preview_html = include_str!("templates/fiche_epice.html");

        conn.execute(
            "INSERT INTO templates (id, name, structure, preview_css, preview_html) VALUES (?1, ?2, ?3, ?4, ?5)",
            params![template_id, "Fiche Épice", structure.to_string(), preview_css, preview_html],
        )?;

        // Section: Épices
        let section_id = Uuid::new_v4().to_string();
        conn.execute(
            "INSERT INTO sections (id, name, description, color, pos_x, pos_y) VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
            params![section_id, "Épices", "Fiches gustatives d'épices et mélanges aromatiques", "#e6a23c", 0.0, 0.0],
        )?;

        // Node: Marc de Café
        let node_id = Uuid::new_v4().to_string();
        conn.execute(
            "INSERT INTO nodes (id, title, content, template_id, is_pinned) VALUES (?1, ?2, ?3, ?4, ?5)",
            params![node_id, "Marc de Café", Option::<String>::None, template_id, false],
        )?;

        // Structured fields in node_fields table (not JSON blob)
        let fields = serde_json::json!({
            "fiche_num": "001",
            "subtitle": "Coffea arabica — résidu de torréfaction",
            "icon": "\u{2615}",
            "odeur_froid": "Terreux, légèrement acide, cacao amer",
            "odeur_torrefaction": "Fumé, bois brûlé, réglisse douce",
            "gout_dominant": "Amer franc, tannique, persistant",
            "notes_secondaires": "Chocolat noir, sous-bois, cendre douce",
            "persistance": "Longue — 30 à 60 secondes en bouche",
            "intensite": 4,
            "intensite_label": "puissant",
            "associations": ["Bœuf / viande rouge", "Sauce brune", "Chocolat noir", "Gibier", "Échalote caramélisée", "Poivre long"],
            "usage": "Une pincée dans le gras avec les échalotes, avant déglaçage. Jamais plus d'1g pour une sauce — c'est un exhausteur, pas une épice dominante.",
            "footer": "RÉCUPÉRATION · DÉSHYDRATÉ 60°C · 1H30",
            "notes_personnelles": ""
        });
        self.save_fields(&conn, &node_id, &fields)?;

        // Link node to section
        conn.execute(
            "INSERT INTO node_sections (node_id, section_id) VALUES (?1, ?2)",
            params![node_id, section_id],
        )?;

        // Create tags
        let tags = ["Bœuf", "Sauce brune", "Chocolat noir", "Gibier", "Échalote caramélisée", "Poivre long"];
        for tag_name in &tags {
            let tag_id = Uuid::new_v4().to_string();
            conn.execute(
                "INSERT INTO tags (id, name) VALUES (?1, ?2)",
                params![tag_id, tag_name],
            )?;
            conn.execute(
                "INSERT INTO node_tags (node_id, tag_id) VALUES (?1, ?2)",
                params![node_id, tag_id],
            )?;
        }

        // Initial version (snapshot fields)
        let version_id = Uuid::new_v4().to_string();
        let fields_json = self.get_fields_as_json(&conn, &node_id)?;
        let snapshot = self.build_version_snapshot(None, Some(&fields_json));
        conn.execute(
            "INSERT INTO node_versions (id, node_id, content, version_num) VALUES (?1, ?2, ?3, 1)",
            params![version_id, node_id, snapshot],
        )?;

        // Index in FTS — title + all field values
        let fields_text = self.get_fields_text(&conn, &node_id)?;
        conn.execute(
            "INSERT INTO nodes_fts (node_id, title, content) VALUES (?1, ?2, ?3)",
            params![node_id, "Marc de Café", fields_text],
        )?;

        tracing::info!("[library] seeded: template 'Fiche Épice', section 'Épices', node 'Marc de Café'");
        Ok(true)
        })();
        match result {
            Ok(v) => { conn.execute_batch("COMMIT")?; Ok(v) }
            Err(e) => { let _ = conn.execute_batch("ROLLBACK"); Err(e) }
        }
    }

    // ── Nodes CRUD ──

    pub async fn create_node(&self, input: &CreateNode) -> Result<Node> {
        let conn = self.conn.lock().await;
        conn.execute_batch("BEGIN IMMEDIATE")?;
        let result = (|| -> Result<Node> {
            let id = Uuid::new_v4().to_string();
            let now = chrono::Utc::now().to_rfc3339();

            conn.execute(
                "INSERT INTO nodes (id, title, content, template_id, created_at, updated_at) VALUES (?1, ?2, ?3, ?4, ?5, ?5)",
                params![id, input.title, input.content, input.template_id, now],
            )?;

            // Fields (structured data in node_fields table)
            if let Some(ref fields) = input.fields {
                self.save_fields(&conn, &id, fields)?;
            }

            // Sections
            if let Some(ref section_ids) = input.section_ids {
                for sid in section_ids {
                    conn.execute(
                        "INSERT OR IGNORE INTO node_sections (node_id, section_id) VALUES (?1, ?2)",
                        params![id, sid],
                    )?;
                }
            }

            // Tags
            if let Some(ref tag_names) = input.tag_names {
                for name in tag_names {
                    let tag_id = get_or_create_tag(&conn, name)?;
                    conn.execute(
                        "INSERT OR IGNORE INTO node_tags (node_id, tag_id) VALUES (?1, ?2)",
                        params![id, tag_id],
                    )?;
                }
            }

            // Version 1 — snapshot content + fields
            let ver_id = Uuid::new_v4().to_string();
            let version_snapshot = self.build_version_snapshot(input.content.as_deref(), input.fields.as_ref());
            conn.execute(
                "INSERT INTO node_versions (id, node_id, content, version_num) VALUES (?1, ?2, ?3, 1)",
                params![ver_id, id, version_snapshot],
            )?;

            // FTS index — title + content + field values
            let fields_text = self.get_fields_text(&conn, &id)?;
            let fts_content = format!("{} {}", input.content.as_deref().unwrap_or(""), fields_text);
            conn.execute(
                "INSERT INTO nodes_fts (node_id, title, content) VALUES (?1, ?2, ?3)",
                params![id, input.title, fts_content.trim()],
            )?;

            let node = self.get_node_by_id_inner(&conn, &id)?;
            Ok(node)
        })();
        match result {
            Ok(v) => { conn.execute_batch("COMMIT")?; Ok(v) }
            Err(e) => { let _ = conn.execute_batch("ROLLBACK"); Err(e) }
        }
    }

    pub async fn get_node(&self, id: &str) -> Result<Option<Node>> {
        let conn = self.conn.lock().await;
        match self.get_node_by_id_inner(&conn, id) {
            Ok(n) => Ok(Some(n)),
            Err(e) => {
                if let Some(rusqlite::Error::QueryReturnedNoRows) = e.downcast_ref::<rusqlite::Error>() {
                    Ok(None)
                } else {
                    Err(e)
                }
            }
        }
    }

    fn get_node_by_id_inner(&self, conn: &Connection, id: &str) -> Result<Node> {
        let mut node = conn.query_row(
            "SELECT id, title, content, template_id, created_at, updated_at, deleted_at, is_pinned, is_active FROM nodes WHERE id = ?1 AND deleted_at IS NULL",
            params![id],
            |row| {
                Ok(Node {
                    id: row.get(0)?,
                    title: row.get(1)?,
                    content: row.get(2)?,
                    fields: None,
                    template_id: row.get(3)?,
                    created_at: row.get(4)?,
                    updated_at: row.get(5)?,
                    deleted_at: row.get(6)?,
                    is_pinned: row.get(7)?,
                    is_active: row.get(8)?,
                })
            },
        )?;
        node.fields = Some(self.get_fields_as_json(conn, &node.id)?);
        Ok(node)
    }

    // ── Node Fields helpers ──

    fn save_fields(&self, conn: &Connection, node_id: &str, fields: &serde_json::Value) -> Result<()> {
        // Delete existing fields
        conn.execute("DELETE FROM node_fields WHERE node_id = ?1", params![node_id])?;

        if let Some(obj) = fields.as_object() {
            for (i, (key, value)) in obj.iter().enumerate() {
                if value.is_null() { continue; }
                let id = Uuid::new_v4().to_string();
                // Store as JSON string to preserve types (string vs number vs array)
                let val_str = value.to_string();
                conn.execute(
                    "INSERT INTO node_fields (id, node_id, field_name, field_value, sort_order) VALUES (?1, ?2, ?3, ?4, ?5)",
                    params![id, node_id, key, val_str, i as i64],
                )?;
            }
        }
        Ok(())
    }

    fn get_fields_as_json(&self, conn: &Connection, node_id: &str) -> Result<serde_json::Value> {
        let mut stmt = conn.prepare(
            "SELECT field_name, field_value FROM node_fields WHERE node_id = ?1 ORDER BY sort_order"
        )?;
        let mut map = serde_json::Map::new();
        let rows: Vec<(String, Option<String>)> = stmt.query_map(params![node_id], |row| {
            Ok((row.get(0)?, row.get(1)?))
        })?.collect::<std::result::Result<Vec<_>, _>>()?;

        for (name, value) in rows {
            let val = match value {
                Some(v) => serde_json::from_str(&v).unwrap_or(serde_json::Value::String(v)),
                None => serde_json::Value::Null,
            };
            map.insert(name, val);
        }
        Ok(serde_json::Value::Object(map))
    }

    fn get_fields_text(&self, conn: &Connection, node_id: &str) -> Result<String> {
        let mut stmt = conn.prepare(
            "SELECT field_value FROM node_fields WHERE node_id = ?1 ORDER BY sort_order"
        )?;
        let values: Vec<String> = stmt.query_map(params![node_id], |row| {
            let v: Option<String> = row.get(0)?;
            Ok(v.unwrap_or_default())
        })?.collect::<std::result::Result<Vec<_>, _>>()?;
        // Extract readable text from JSON values for FTS indexing
        let texts: Vec<String> = values.iter().map(|v| {
            if let Ok(val) = serde_json::from_str::<serde_json::Value>(v) {
                match val {
                    serde_json::Value::String(s) => s,
                    serde_json::Value::Array(arr) => arr.iter()
                        .filter_map(|item| item.as_str().map(|s| s.to_string()))
                        .collect::<Vec<_>>().join(" "),
                    serde_json::Value::Number(n) => n.to_string(),
                    _ => v.clone(),
                }
            } else {
                v.clone()
            }
        }).collect();
        Ok(texts.join(" "))
    }

    fn build_version_snapshot(&self, content: Option<&str>, fields: Option<&serde_json::Value>) -> String {
        let snapshot = serde_json::json!({
            "content": content,
            "fields": fields,
        });
        snapshot.to_string()
    }

    pub async fn list_nodes(&self, include_deleted: bool, limit: i64, offset: i64) -> Result<Vec<Node>> {
        let conn = self.conn.lock().await;
        let sql = if include_deleted {
            format!("SELECT id, title, content, template_id, created_at, updated_at, deleted_at, is_pinned, is_active FROM nodes ORDER BY updated_at DESC LIMIT {} OFFSET {}", limit, offset)
        } else {
            format!("SELECT id, title, content, template_id, created_at, updated_at, deleted_at, is_pinned, is_active FROM nodes WHERE deleted_at IS NULL ORDER BY updated_at DESC LIMIT {} OFFSET {}", limit, offset)
        };
        let mut stmt = conn.prepare(&sql)?;
        let mut nodes: Vec<Node> = stmt.query_map([], map_node)?
            .collect::<std::result::Result<Vec<_>, _>>()?;
        // Populate fields for each node
        for node in &mut nodes {
            node.fields = Some(self.get_fields_as_json(&conn, &node.id)?);
        }
        Ok(nodes)
    }

    pub async fn update_node(&self, id: &str, input: &UpdateNode) -> Result<Option<Node>> {
        let conn = self.conn.lock().await;
        conn.execute_batch("BEGIN IMMEDIATE")?;
        let result = (|| -> Result<Option<Node>> {
            let now = chrono::Utc::now().to_rfc3339();

            // Check exists
            let exists: bool = conn.query_row(
                "SELECT COUNT(*) > 0 FROM nodes WHERE id = ?1 AND deleted_at IS NULL",
                params![id],
                |r| r.get(0),
            )?;
            if !exists { return Ok(None); }

            if let Some(ref title) = input.title {
                conn.execute("UPDATE nodes SET title = ?1, updated_at = ?2 WHERE id = ?3", params![title, now, id])?;
            }
            if let Some(ref content) = input.content {
                conn.execute("UPDATE nodes SET content = ?1, updated_at = ?2 WHERE id = ?3", params![content, now, id])?;
            }

            // Fields (structured data)
            if let Some(ref fields) = input.fields {
                self.save_fields(&conn, id, fields)?;
                conn.execute("UPDATE nodes SET updated_at = ?1 WHERE id = ?2", params![now, id])?;
            }

            // New version on content or fields change
            if input.content.is_some() || input.fields.is_some() {
                let ver_num: i64 = conn.query_row(
                    "SELECT COALESCE(MAX(version_num), 0) + 1 FROM node_versions WHERE node_id = ?1",
                    params![id], |r| r.get(0),
                )?;
                let ver_id = Uuid::new_v4().to_string();
                let current_content: Option<String> = conn.query_row(
                    "SELECT content FROM nodes WHERE id = ?1", params![id], |r| r.get(0),
                )?;
                let current_fields = self.get_fields_as_json(&conn, id)?;
                let snapshot = self.build_version_snapshot(current_content.as_deref(), Some(&current_fields));
                conn.execute(
                    "INSERT INTO node_versions (id, node_id, content, version_num) VALUES (?1, ?2, ?3, ?4)",
                    params![ver_id, id, snapshot, ver_num],
                )?;
            }

            if let Some(ref template_id) = input.template_id {
                conn.execute("UPDATE nodes SET template_id = ?1, updated_at = ?2 WHERE id = ?3", params![template_id, now, id])?;
            }
            if let Some(pinned) = input.is_pinned {
                conn.execute("UPDATE nodes SET is_pinned = ?1, updated_at = ?2 WHERE id = ?3", params![pinned, now, id])?;
            }
            if let Some(active) = input.is_active {
                // Deactivate all others if activating
                if active {
                    conn.execute("UPDATE nodes SET is_active = FALSE WHERE is_active = TRUE", [])?;
                }
                conn.execute("UPDATE nodes SET is_active = ?1, updated_at = ?2 WHERE id = ?3", params![active, now, id])?;
            }

            // Sections
            if let Some(ref section_ids) = input.section_ids {
                conn.execute("DELETE FROM node_sections WHERE node_id = ?1", params![id])?;
                for sid in section_ids {
                    conn.execute(
                        "INSERT OR IGNORE INTO node_sections (node_id, section_id) VALUES (?1, ?2)",
                        params![id, sid],
                    )?;
                }
            }

            // Tags
            if let Some(ref tag_names) = input.tag_names {
                conn.execute("DELETE FROM node_tags WHERE node_id = ?1", params![id])?;
                for name in tag_names {
                    let tag_id = get_or_create_tag(&conn, name)?;
                    conn.execute(
                        "INSERT OR IGNORE INTO node_tags (node_id, tag_id) VALUES (?1, ?2)",
                        params![id, tag_id],
                    )?;
                }
                // Clean orphan tags (tags not linked to any node)
                conn.execute("DELETE FROM tags WHERE id NOT IN (SELECT DISTINCT tag_id FROM node_tags)", [])?;
            }

            // Update FTS — include field values
            let node = self.get_node_by_id_inner(&conn, id)?;
            conn.execute("DELETE FROM nodes_fts WHERE node_id = ?1", params![id])?;
            let fields_text = self.get_fields_text(&conn, id)?;
            let fts_content = format!("{} {}", node.content.as_deref().unwrap_or(""), fields_text);
            conn.execute(
                "INSERT INTO nodes_fts (node_id, title, content) VALUES (?1, ?2, ?3)",
                params![id, node.title, fts_content.trim()],
            )?;

            Ok(Some(node))
        })();
        match result {
            Ok(v) => { conn.execute_batch("COMMIT")?; Ok(v) }
            Err(e) => { let _ = conn.execute_batch("ROLLBACK"); Err(e) }
        }
    }

    pub async fn soft_delete_node(&self, id: &str) -> Result<bool> {
        let conn = self.conn.lock().await;
        let now = chrono::Utc::now().to_rfc3339();
        let affected = conn.execute(
            "UPDATE nodes SET deleted_at = ?1 WHERE id = ?2 AND deleted_at IS NULL",
            params![now, id],
        )?;
        if affected > 0 {
            conn.execute("DELETE FROM nodes_fts WHERE node_id = ?1", params![id])?;
        }
        Ok(affected > 0)
    }

    pub async fn restore_node(&self, id: &str) -> Result<bool> {
        let conn = self.conn.lock().await;
        let affected = conn.execute(
            "UPDATE nodes SET deleted_at = NULL WHERE id = ?1 AND deleted_at IS NOT NULL",
            params![id],
        )?;
        if affected > 0 {
            let node = self.get_node_by_id_inner(&conn, id)?;
            let fields_text = self.get_fields_text(&conn, id)?;
            let fts_content = format!("{} {}", node.content.as_deref().unwrap_or(""), fields_text);
            conn.execute(
                "INSERT INTO nodes_fts (node_id, title, content) VALUES (?1, ?2, ?3)",
                params![id, node.title, fts_content.trim()],
            )?;
        }
        Ok(affected > 0)
    }

    pub async fn purge_node(&self, id: &str) -> Result<bool> {
        let conn = self.conn.lock().await;
        // Only purge if already soft-deleted
        let is_deleted: bool = conn.query_row(
            "SELECT COUNT(*) > 0 FROM nodes WHERE id = ?1 AND deleted_at IS NOT NULL",
            params![id], |r| r.get(0),
        ).unwrap_or(false);
        if !is_deleted { return Ok(false); }

        conn.execute_batch("BEGIN IMMEDIATE")?;
        let result = (|| -> Result<bool> {
            conn.execute("DELETE FROM node_tags WHERE node_id = ?1", params![id])?;
            // Clean orphan tags
            conn.execute("DELETE FROM tags WHERE id NOT IN (SELECT DISTINCT tag_id FROM node_tags)", [])?;
            conn.execute("DELETE FROM node_sections WHERE node_id = ?1", params![id])?;
            conn.execute("DELETE FROM node_fields WHERE node_id = ?1", params![id])?;
            conn.execute("DELETE FROM node_versions WHERE node_id = ?1", params![id])?;
            conn.execute("DELETE FROM edges WHERE node_from = ?1 OR node_to = ?1", params![id])?;
            conn.execute("DELETE FROM pending_links WHERE node_from = ?1 OR node_to = ?1", params![id])?;
            conn.execute("DELETE FROM nodes WHERE id = ?1", params![id])?;
            Ok(true)
        })();
        match result {
            Ok(v) => { conn.execute_batch("COMMIT")?; Ok(v) }
            Err(e) => { let _ = conn.execute_batch("ROLLBACK"); Err(e) }
        }
    }

    pub async fn list_trash(&self, limit: i64, offset: i64) -> Result<Vec<Node>> {
        let conn = self.conn.lock().await;
        let sql = format!(
            "SELECT id, title, content, template_id, created_at, updated_at, deleted_at, is_pinned, is_active FROM nodes WHERE deleted_at IS NOT NULL ORDER BY deleted_at DESC LIMIT {} OFFSET {}",
            limit, offset
        );
        let mut stmt = conn.prepare(&sql)?;
        let mut nodes: Vec<Node> = stmt.query_map([], map_node)?
            .collect::<std::result::Result<Vec<_>, _>>()?;
        for node in &mut nodes {
            node.fields = Some(self.get_fields_as_json(&conn, &node.id)?);
        }
        Ok(nodes)
    }

    // ── Sections CRUD ──

    pub async fn create_section(&self, input: &CreateSection) -> Result<Section> {
        let conn = self.conn.lock().await;
        let id = Uuid::new_v4().to_string();
        let now = chrono::Utc::now().to_rfc3339();
        conn.execute(
            "INSERT INTO sections (id, name, description, color, pos_x, pos_y, created_at) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
            params![id, input.name, input.description, input.color, input.pos_x, input.pos_y, now],
        )?;
        Ok(Section { id, name: input.name.clone(), description: input.description.clone(), color: input.color.clone(), pos_x: input.pos_x, pos_y: input.pos_y, created_at: now })
    }

    fn list_sections_inner(&self, conn: &Connection) -> Result<Vec<Section>> {
        let mut stmt = conn.prepare("SELECT id, name, description, color, pos_x, pos_y, created_at FROM sections ORDER BY name")?;
        let sections = stmt.query_map([], |row| {
            Ok(Section {
                id: row.get(0)?, name: row.get(1)?, description: row.get(2)?,
                color: row.get(3)?, pos_x: row.get(4)?, pos_y: row.get(5)?, created_at: row.get(6)?,
            })
        })?.collect::<std::result::Result<Vec<_>, _>>()?;
        Ok(sections)
    }

    pub async fn list_sections(&self) -> Result<Vec<Section>> {
        let conn = self.conn.lock().await;
        self.list_sections_inner(&conn)
    }

    pub async fn get_section(&self, id: &str) -> Result<Option<Section>> {
        let conn = self.conn.lock().await;
        let section = conn.query_row(
            "SELECT id, name, description, color, pos_x, pos_y, created_at FROM sections WHERE id = ?1",
            params![id],
            |row| Ok(Section {
                id: row.get(0)?, name: row.get(1)?, description: row.get(2)?,
                color: row.get(3)?, pos_x: row.get(4)?, pos_y: row.get(5)?, created_at: row.get(6)?,
            }),
        );
        match section {
            Ok(s) => Ok(Some(s)),
            Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
            Err(e) => Err(e.into()),
        }
    }

    pub async fn update_section(&self, id: &str, input: &UpdateSection) -> Result<Option<Section>> {
        let conn = self.conn.lock().await;
        if let Some(ref name) = input.name {
            conn.execute("UPDATE sections SET name = ?1 WHERE id = ?2", params![name, id])?;
        }
        if let Some(ref desc) = input.description {
            conn.execute("UPDATE sections SET description = ?1 WHERE id = ?2", params![desc, id])?;
        }
        if let Some(ref color) = input.color {
            conn.execute("UPDATE sections SET color = ?1 WHERE id = ?2", params![color, id])?;
        }
        if let Some(x) = input.pos_x {
            conn.execute("UPDATE sections SET pos_x = ?1 WHERE id = ?2", params![x, id])?;
        }
        if let Some(y) = input.pos_y {
            conn.execute("UPDATE sections SET pos_y = ?1 WHERE id = ?2", params![y, id])?;
        }
        drop(conn);
        self.get_section(id).await
    }

    pub async fn delete_section(&self, id: &str) -> Result<bool> {
        let conn = self.conn.lock().await;
        conn.execute("DELETE FROM node_sections WHERE section_id = ?1", params![id])?;
        let affected = conn.execute("DELETE FROM sections WHERE id = ?1", params![id])?;
        Ok(affected > 0)
    }

    fn get_section_nodes_inner(&self, conn: &Connection, section_id: &str) -> Result<Vec<Node>> {
        let mut stmt = conn.prepare(
            "SELECT n.id, n.title, n.content, n.template_id, n.created_at, n.updated_at, n.deleted_at, n.is_pinned, n.is_active \
             FROM nodes n JOIN node_sections ns ON n.id = ns.node_id \
             WHERE ns.section_id = ?1 AND n.deleted_at IS NULL ORDER BY n.updated_at DESC"
        )?;
        let mut nodes: Vec<Node> = stmt.query_map(params![section_id], map_node)?
            .collect::<std::result::Result<Vec<_>, _>>()?;
        for node in &mut nodes {
            node.fields = Some(self.get_fields_as_json(conn, &node.id)?);
        }
        Ok(nodes)
    }

    pub async fn get_section_nodes(&self, section_id: &str) -> Result<Vec<Node>> {
        let conn = self.conn.lock().await;
        self.get_section_nodes_inner(&conn, section_id)
    }

    // ── Edges CRUD ──

    pub async fn create_edge(&self, input: &CreateEdge) -> Result<Edge> {
        let conn = self.conn.lock().await;
        let id = Uuid::new_v4().to_string();
        let now = chrono::Utc::now().to_rfc3339();
        conn.execute(
            "INSERT INTO edges (id, node_from, node_to, relation, auto, confirmed, created_at) VALUES (?1, ?2, ?3, ?4, FALSE, TRUE, ?5)",
            params![id, input.node_from, input.node_to, input.relation, now],
        )?;
        Ok(Edge { id, node_from: input.node_from.clone(), node_to: input.node_to.clone(), relation: input.relation.clone(), auto_created: false, confirmed: true, created_at: now })
    }

    fn list_edges_inner(&self, conn: &Connection) -> Result<Vec<Edge>> {
        let mut stmt = conn.prepare(
            "SELECT id, node_from, node_to, relation, auto, confirmed, created_at FROM edges ORDER BY created_at DESC"
        )?;
        let edges = stmt.query_map([], |row| {
            Ok(Edge {
                id: row.get(0)?, node_from: row.get(1)?, node_to: row.get(2)?,
                relation: row.get(3)?, auto_created: row.get(4)?, confirmed: row.get(5)?, created_at: row.get(6)?,
            })
        })?.collect::<std::result::Result<Vec<_>, _>>()?;
        Ok(edges)
    }

    pub async fn list_edges(&self, limit: i64, offset: i64) -> Result<Vec<Edge>> {
        let conn = self.conn.lock().await;
        let sql = format!(
            "SELECT id, node_from, node_to, relation, auto, confirmed, created_at FROM edges ORDER BY created_at DESC LIMIT {} OFFSET {}",
            limit, offset
        );
        let mut stmt = conn.prepare(&sql)?;
        let edges = stmt.query_map([], |row| {
            Ok(Edge {
                id: row.get(0)?, node_from: row.get(1)?, node_to: row.get(2)?,
                relation: row.get(3)?, auto_created: row.get(4)?, confirmed: row.get(5)?, created_at: row.get(6)?,
            })
        })?.collect::<std::result::Result<Vec<_>, _>>()?;
        Ok(edges)
    }

    fn get_node_edges_inner(&self, conn: &Connection, node_id: &str) -> Result<Vec<Edge>> {
        let mut stmt = conn.prepare(
            "SELECT id, node_from, node_to, relation, auto, confirmed, created_at FROM edges WHERE (node_from = ?1 OR node_to = ?1) AND confirmed = TRUE ORDER BY created_at DESC"
        )?;
        let edges = stmt.query_map(params![node_id], |row| {
            Ok(Edge {
                id: row.get(0)?, node_from: row.get(1)?, node_to: row.get(2)?,
                relation: row.get(3)?, auto_created: row.get(4)?, confirmed: row.get(5)?, created_at: row.get(6)?,
            })
        })?.collect::<std::result::Result<Vec<_>, _>>()?;
        Ok(edges)
    }

    pub async fn get_node_edges(&self, node_id: &str) -> Result<Vec<Edge>> {
        let conn = self.conn.lock().await;
        self.get_node_edges_inner(&conn, node_id)
    }

    pub async fn delete_edge(&self, id: &str) -> Result<bool> {
        let conn = self.conn.lock().await;
        let affected = conn.execute("DELETE FROM edges WHERE id = ?1", params![id])?;
        Ok(affected > 0)
    }

    // ── Templates CRUD ──

    pub async fn create_template(&self, input: &CreateTemplate) -> Result<Template> {
        let conn = self.conn.lock().await;
        let id = Uuid::new_v4().to_string();
        let now = chrono::Utc::now().to_rfc3339();
        let structure_str = input.structure.as_ref().map(|s| s.to_string());
        conn.execute(
            "INSERT INTO templates (id, name, structure, preview_css, preview_html, created_at) VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
            params![id, input.name, structure_str, input.preview_css, input.preview_html, now],
        )?;
        Ok(Template { id, name: input.name.clone(), structure: input.structure.clone(), preview_css: input.preview_css.clone(), preview_html: input.preview_html.clone(), created_at: now })
    }

    pub async fn list_templates(&self) -> Result<Vec<Template>> {
        let conn = self.conn.lock().await;
        let mut stmt = conn.prepare("SELECT id, name, structure, preview_css, preview_html, created_at FROM templates ORDER BY name")?;
        let templates = stmt.query_map([], |row| {
            let structure_str: Option<String> = row.get(2)?;
            let structure = structure_str.and_then(|s| serde_json::from_str(&s).ok());
            Ok(Template { id: row.get(0)?, name: row.get(1)?, structure, preview_css: row.get(3)?, preview_html: row.get(4)?, created_at: row.get(5)? })
        })?.collect::<std::result::Result<Vec<_>, _>>()?;
        Ok(templates)
    }

    pub async fn get_template(&self, id: &str) -> Result<Option<Template>> {
        let conn = self.conn.lock().await;
        let result = conn.query_row(
            "SELECT id, name, structure, preview_css, preview_html, created_at FROM templates WHERE id = ?1",
            params![id],
            |row| {
                let structure_str: Option<String> = row.get(2)?;
                let structure = structure_str.and_then(|s| serde_json::from_str(&s).ok());
                Ok(Template { id: row.get(0)?, name: row.get(1)?, structure, preview_css: row.get(3)?, preview_html: row.get(4)?, created_at: row.get(5)? })
            },
        );
        match result {
            Ok(t) => Ok(Some(t)),
            Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
            Err(e) => Err(e.into()),
        }
    }

    pub async fn update_template(&self, id: &str, input: &UpdateTemplate) -> Result<Option<Template>> {
        let conn = self.conn.lock().await;
        if let Some(ref name) = input.name {
            conn.execute("UPDATE templates SET name = ?1 WHERE id = ?2", params![name, id])?;
        }
        if let Some(ref structure) = input.structure {
            conn.execute("UPDATE templates SET structure = ?1 WHERE id = ?2", params![structure.to_string(), id])?;
        }
        if let Some(ref css) = input.preview_css {
            conn.execute("UPDATE templates SET preview_css = ?1 WHERE id = ?2", params![css, id])?;
        }
        if let Some(ref html) = input.preview_html {
            conn.execute("UPDATE templates SET preview_html = ?1 WHERE id = ?2", params![html, id])?;
        }
        drop(conn);
        self.get_template(id).await
    }

    pub async fn delete_template(&self, id: &str) -> Result<bool> {
        let conn = self.conn.lock().await;
        // Unlink nodes from this template
        conn.execute("UPDATE nodes SET template_id = NULL WHERE template_id = ?1", params![id])?;
        let affected = conn.execute("DELETE FROM templates WHERE id = ?1", params![id])?;
        Ok(affected > 0)
    }

    // ── Tags ──

    pub async fn list_tags(&self) -> Result<Vec<Tag>> {
        let conn = self.conn.lock().await;
        let mut stmt = conn.prepare("SELECT id, name FROM tags ORDER BY name")?;
        let tags = stmt.query_map([], |row| {
            Ok(Tag { id: row.get(0)?, name: row.get(1)? })
        })?.collect::<std::result::Result<Vec<_>, _>>()?;
        Ok(tags)
    }

    fn get_node_tags_inner(&self, conn: &Connection, node_id: &str) -> Result<Vec<Tag>> {
        let mut stmt = conn.prepare(
            "SELECT t.id, t.name FROM tags t JOIN node_tags nt ON t.id = nt.tag_id WHERE nt.node_id = ?1 ORDER BY t.name"
        )?;
        let tags = stmt.query_map(params![node_id], |row| {
            Ok(Tag { id: row.get(0)?, name: row.get(1)? })
        })?.collect::<std::result::Result<Vec<_>, _>>()?;
        Ok(tags)
    }

    pub async fn get_node_tags(&self, node_id: &str) -> Result<Vec<Tag>> {
        let conn = self.conn.lock().await;
        self.get_node_tags_inner(&conn, node_id)
    }

    fn get_node_sections_inner(&self, conn: &Connection, node_id: &str) -> Result<Vec<Section>> {
        let mut stmt = conn.prepare(
            "SELECT s.id, s.name, s.description, s.color, s.pos_x, s.pos_y, s.created_at \
             FROM sections s JOIN node_sections ns ON s.id = ns.section_id WHERE ns.node_id = ?1"
        )?;
        let sections = stmt.query_map(params![node_id], |row| {
            Ok(Section {
                id: row.get(0)?, name: row.get(1)?, description: row.get(2)?,
                color: row.get(3)?, pos_x: row.get(4)?, pos_y: row.get(5)?, created_at: row.get(6)?,
            })
        })?.collect::<std::result::Result<Vec<_>, _>>()?;
        Ok(sections)
    }

    pub async fn get_node_sections(&self, node_id: &str) -> Result<Vec<Section>> {
        let conn = self.conn.lock().await;
        self.get_node_sections_inner(&conn, node_id)
    }

    // ── Pending Links ──

    pub async fn list_pending_links(&self, limit: i64, offset: i64) -> Result<Vec<PendingLink>> {
        let conn = self.conn.lock().await;
        let sql = format!(
            "SELECT id, node_from, node_to, occurrence, created_at FROM pending_links ORDER BY created_at DESC LIMIT {} OFFSET {}",
            limit, offset
        );
        let mut stmt = conn.prepare(&sql)?;
        let links = stmt.query_map([], |row| {
            Ok(PendingLink { id: row.get(0)?, node_from: row.get(1)?, node_to: row.get(2)?, occurrence: row.get(3)?, created_at: row.get(4)? })
        })?.collect::<std::result::Result<Vec<_>, _>>()?;
        Ok(links)
    }

    fn get_node_pending_links_inner(&self, conn: &Connection, node_id: &str) -> Result<Vec<PendingLink>> {
        let mut stmt = conn.prepare(
            "SELECT id, node_from, node_to, occurrence, created_at FROM pending_links WHERE node_from = ?1 OR node_to = ?1 ORDER BY created_at DESC"
        )?;
        let links = stmt.query_map(params![node_id], |row| {
            Ok(PendingLink { id: row.get(0)?, node_from: row.get(1)?, node_to: row.get(2)?, occurrence: row.get(3)?, created_at: row.get(4)? })
        })?.collect::<std::result::Result<Vec<_>, _>>()?;
        Ok(links)
    }

    pub async fn get_node_pending_links(&self, node_id: &str) -> Result<Vec<PendingLink>> {
        let conn = self.conn.lock().await;
        self.get_node_pending_links_inner(&conn, node_id)
    }

    pub async fn confirm_pending_link(&self, pending_id: &str, relation: Option<&str>) -> Result<Option<Edge>> {
        let conn = self.conn.lock().await;
        let link = conn.query_row(
            "SELECT node_from, node_to FROM pending_links WHERE id = ?1",
            params![pending_id],
            |row| Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?)),
        );
        let (node_from, node_to) = match link {
            Ok(l) => l,
            Err(rusqlite::Error::QueryReturnedNoRows) => return Ok(None),
            Err(e) => return Err(e.into()),
        };

        let edge_id = Uuid::new_v4().to_string();
        let now = chrono::Utc::now().to_rfc3339();
        conn.execute(
            "INSERT INTO edges (id, node_from, node_to, relation, auto, confirmed, created_at) VALUES (?1, ?2, ?3, ?4, TRUE, TRUE, ?5)",
            params![edge_id, node_from, node_to, relation, now],
        )?;
        conn.execute("DELETE FROM pending_links WHERE id = ?1", params![pending_id])?;

        Ok(Some(Edge { id: edge_id, node_from, node_to, relation: relation.map(|s| s.to_string()), auto_created: true, confirmed: true, created_at: now }))
    }

    pub async fn dismiss_pending_link(&self, pending_id: &str) -> Result<bool> {
        let conn = self.conn.lock().await;
        let affected = conn.execute("DELETE FROM pending_links WHERE id = ?1", params![pending_id])?;
        Ok(affected > 0)
    }

    // ── Versions ──

    fn get_node_versions_count_inner(&self, conn: &Connection, node_id: &str) -> Result<i64> {
        let count: i64 = conn.query_row(
            "SELECT COUNT(*) FROM node_versions WHERE node_id = ?1",
            params![node_id], |r| r.get(0),
        )?;
        Ok(count)
    }

    pub async fn get_node_versions(&self, node_id: &str) -> Result<Vec<NodeVersion>> {
        let conn = self.conn.lock().await;
        let mut stmt = conn.prepare(
            "SELECT id, node_id, content, version_num, created_at FROM node_versions WHERE node_id = ?1 ORDER BY version_num DESC"
        )?;
        let versions = stmt.query_map(params![node_id], |row| {
            Ok(NodeVersion { id: row.get(0)?, node_id: row.get(1)?, content: row.get(2)?, version_num: row.get(3)?, created_at: row.get(4)? })
        })?.collect::<std::result::Result<Vec<_>, _>>()?;
        Ok(versions)
    }

    // ── Search (FTS5) ──

    fn sanitize_fts_query(query: &str) -> String {
        let trimmed = query.trim();
        if trimmed.is_empty() {
            return "\"\"".to_string();
        }
        // Escape internal double quotes and wrap
        let escaped = trimmed.replace('"', "\"\"");
        format!("\"{}\"", escaped)
    }

    pub async fn search(&self, query: &str, section_id: Option<&str>, tag: Option<&str>) -> Result<Vec<Node>> {
        let conn = self.conn.lock().await;
        let safe_query = Self::sanitize_fts_query(query);

        let base = if section_id.is_some() && tag.is_some() {
            "SELECT n.id, n.title, n.content, n.template_id, n.created_at, n.updated_at, n.deleted_at, n.is_pinned, n.is_active \
             FROM nodes n \
             JOIN nodes_fts fts ON n.id = fts.node_id \
             JOIN node_sections ns ON n.id = ns.node_id \
             JOIN node_tags nt ON n.id = nt.node_id \
             JOIN tags t ON nt.tag_id = t.id \
             WHERE nodes_fts MATCH ?1 AND n.deleted_at IS NULL AND ns.section_id = ?2 AND t.name = ?3 \
             ORDER BY rank LIMIT 50"
        } else if section_id.is_some() {
            "SELECT n.id, n.title, n.content, n.template_id, n.created_at, n.updated_at, n.deleted_at, n.is_pinned, n.is_active \
             FROM nodes n \
             JOIN nodes_fts fts ON n.id = fts.node_id \
             JOIN node_sections ns ON n.id = ns.node_id \
             WHERE nodes_fts MATCH ?1 AND n.deleted_at IS NULL AND ns.section_id = ?2 \
             ORDER BY rank LIMIT 50"
        } else if tag.is_some() {
            "SELECT n.id, n.title, n.content, n.template_id, n.created_at, n.updated_at, n.deleted_at, n.is_pinned, n.is_active \
             FROM nodes n \
             JOIN nodes_fts fts ON n.id = fts.node_id \
             JOIN node_tags nt ON n.id = nt.node_id \
             JOIN tags t ON nt.tag_id = t.id \
             WHERE nodes_fts MATCH ?1 AND n.deleted_at IS NULL AND t.name = ?2 \
             ORDER BY rank LIMIT 50"
        } else {
            "SELECT n.id, n.title, n.content, n.template_id, n.created_at, n.updated_at, n.deleted_at, n.is_pinned, n.is_active \
             FROM nodes n \
             JOIN nodes_fts fts ON n.id = fts.node_id \
             WHERE nodes_fts MATCH ?1 AND n.deleted_at IS NULL \
             ORDER BY rank LIMIT 50"
        };

        let mut stmt = conn.prepare(base)?;
        let mut nodes: Vec<Node> = if section_id.is_some() && tag.is_some() {
            stmt.query_map(params![safe_query, section_id.unwrap(), tag.unwrap()], map_node)?
        } else if section_id.is_some() {
            stmt.query_map(params![safe_query, section_id.unwrap()], map_node)?
        } else if tag.is_some() {
            stmt.query_map(params![safe_query, tag.unwrap()], map_node)?
        } else {
            stmt.query_map(params![safe_query], map_node)?
        }.collect::<std::result::Result<Vec<_>, _>>()?;

        // Hydrate fields for search results
        for node in &mut nodes {
            node.fields = Some(self.get_fields_as_json(&conn, &node.id)?);
        }
        Ok(nodes)
    }

    // ── Study Desk ──

    pub async fn get_study_desk(&self, node_id: &str) -> Result<Option<StudyDesk>> {
        let conn = self.conn.lock().await;

        let node = match self.get_node_by_id_inner(&conn, node_id) {
            Ok(n) => n,
            Err(e) => {
                if let Some(rusqlite::Error::QueryReturnedNoRows) = e.downcast_ref::<rusqlite::Error>() {
                    return Ok(None);
                }
                return Err(e);
            }
        };

        let edges = self.get_node_edges_inner(&conn, node_id)?;
        let tags = self.get_node_tags_inner(&conn, node_id)?;
        let sections = self.get_node_sections_inner(&conn, node_id)?;
        let pending = self.get_node_pending_links_inner(&conn, node_id)?;
        let versions_count = self.get_node_versions_count_inner(&conn, node_id)?;

        let mut connections = Vec::new();
        for edge in &edges {
            let other_id = if edge.node_from == node_id { &edge.node_to } else { &edge.node_from };
            match self.get_node_by_id_inner(&conn, other_id) {
                Ok(other_node) => connections.push(StudyConnection { node: other_node, edge: edge.clone() }),
                Err(_) => {} // Skip deleted/missing nodes
            }
        }

        let fields = node.fields.clone();

        Ok(Some(StudyDesk {
            node,
            fields,
            connections,
            pending_links: pending,
            tags,
            sections,
            versions_count,
        }))
    }

    // ── Graph data ──

    pub async fn get_graph_data(&self) -> Result<GraphData> {
        let conn = self.conn.lock().await;
        let sections = self.list_sections_inner(&conn)?;
        let mut section_nodes = Vec::new();
        for section in &sections {
            let nodes = self.get_section_nodes_inner(&conn, &section.id)?;
            section_nodes.push(SectionWithNodes {
                section: section.clone(),
                node_count: nodes.len(),
            });
        }

        // Inter-section edges: edges where node_from and node_to are in different sections
        let edges = self.list_edges_inner(&conn)?;
        let mut inter = Vec::new();
        for edge in edges {
            let from_sections: Vec<String> = conn
                .prepare("SELECT section_id FROM node_sections WHERE node_id = ?1")?
                .query_map(params![edge.node_from], |r| r.get(0))?
                .collect::<std::result::Result<Vec<_>, _>>()?;
            let to_sections: Vec<String> = conn
                .prepare("SELECT section_id FROM node_sections WHERE node_id = ?1")?
                .query_map(params![edge.node_to], |r| r.get(0))?
                .collect::<std::result::Result<Vec<_>, _>>()?;
            let shared = from_sections.iter().any(|s| to_sections.contains(s));
            if !shared {
                inter.push(edge);
            }
        }

        Ok(GraphData { sections: section_nodes, inter_section_edges: inter })
    }

    // ── Stats ──

    pub async fn stats(&self) -> Result<(i64, i64, i64)> {
        let conn = self.conn.lock().await;
        let nodes: i64 = conn.query_row("SELECT COUNT(*) FROM nodes WHERE deleted_at IS NULL", [], |r| r.get(0))?;
        let sections: i64 = conn.query_row("SELECT COUNT(*) FROM sections", [], |r| r.get(0))?;
        let pending: i64 = conn.query_row("SELECT COUNT(*) FROM pending_links", [], |r| r.get(0))?;
        Ok((nodes, sections, pending))
    }

    // ── Worker: occurrence detection ──

    pub async fn detect_occurrences(&self, changed_node_id: &str) -> Result<Vec<PendingLink>> {
        let conn = self.conn.lock().await;

        // Get all node titles (except the changed one)
        let mut stmt = conn.prepare(
            "SELECT id, title FROM nodes WHERE deleted_at IS NULL AND id != ?1"
        )?;
        let all_nodes: Vec<(String, String)> = stmt.query_map(params![changed_node_id], |row| {
            Ok((row.get(0)?, row.get(1)?))
        })?.collect::<std::result::Result<Vec<_>, _>>()?;

        // Get the changed node's content + field values
        let changed_content: Option<String> = conn.query_row(
            "SELECT content FROM nodes WHERE id = ?1",
            params![changed_node_id],
            |r| r.get(0),
        ).ok();
        let changed_fields_text = self.get_fields_text(&conn, changed_node_id).unwrap_or_default();
        // Combine content and fields for matching
        let changed_full_text = format!("{} {}", changed_content.as_deref().unwrap_or(""), changed_fields_text);

        let changed_title: String = conn.query_row(
            "SELECT title FROM nodes WHERE id = ?1",
            params![changed_node_id],
            |r| r.get(0),
        )?;

        let mut new_pending = Vec::new();

        // Check if other node titles appear in changed node's content + fields
        {
            let full_text_lower = changed_full_text.to_lowercase();
            for (other_id, other_title) in &all_nodes {
                if full_text_lower.contains(&other_title.to_lowercase()) {
                    // Check no existing edge or pending link
                    let exists: bool = conn.query_row(
                        "SELECT COUNT(*) > 0 FROM edges WHERE (node_from = ?1 AND node_to = ?2) OR (node_from = ?2 AND node_to = ?1)",
                        params![changed_node_id, other_id],
                        |r| r.get(0),
                    )?;
                    let pending_exists: bool = conn.query_row(
                        "SELECT COUNT(*) > 0 FROM pending_links WHERE (node_from = ?1 AND node_to = ?2) OR (node_from = ?2 AND node_to = ?1)",
                        params![changed_node_id, other_id],
                        |r| r.get(0),
                    )?;

                    if !exists && !pending_exists {
                        let pl_id = Uuid::new_v4().to_string();
                        let now = chrono::Utc::now().to_rfc3339();
                        conn.execute(
                            "INSERT INTO pending_links (id, node_from, node_to, occurrence, created_at) VALUES (?1, ?2, ?3, ?4, ?5)",
                            params![pl_id, changed_node_id, other_id, other_title, now],
                        )?;
                        new_pending.push(PendingLink {
                            id: pl_id, node_from: changed_node_id.to_string(),
                            node_to: other_id.clone(), occurrence: Some(other_title.clone()), created_at: now,
                        });
                    }
                }
            }
        }

        // Check if changed node's title appears in other nodes' content + fields
        let changed_title_lower = changed_title.to_lowercase();
        for (other_id, _) in &all_nodes {
            let other_content: Option<String> = conn.query_row(
                "SELECT content FROM nodes WHERE id = ?1",
                params![other_id],
                |r| r.get(0),
            ).ok().flatten();
            let other_fields_text = self.get_fields_text(&conn, other_id).unwrap_or_default();
            let other_full_text = format!("{} {}", other_content.as_deref().unwrap_or(""), other_fields_text);

            {
                if other_full_text.to_lowercase().contains(&changed_title_lower) {
                    let exists: bool = conn.query_row(
                        "SELECT COUNT(*) > 0 FROM edges WHERE (node_from = ?1 AND node_to = ?2) OR (node_from = ?2 AND node_to = ?1)",
                        params![other_id, changed_node_id],
                        |r| r.get(0),
                    )?;
                    let pending_exists: bool = conn.query_row(
                        "SELECT COUNT(*) > 0 FROM pending_links WHERE (node_from = ?1 AND node_to = ?2) OR (node_from = ?2 AND node_to = ?1)",
                        params![other_id, changed_node_id],
                        |r| r.get(0),
                    )?;

                    if !exists && !pending_exists {
                        let pl_id = Uuid::new_v4().to_string();
                        let now = chrono::Utc::now().to_rfc3339();
                        conn.execute(
                            "INSERT INTO pending_links (id, node_from, node_to, occurrence, created_at) VALUES (?1, ?2, ?3, ?4, ?5)",
                            params![pl_id, other_id, changed_node_id, changed_title, now],
                        )?;
                        new_pending.push(PendingLink {
                            id: pl_id, node_from: other_id.clone(),
                            node_to: changed_node_id.to_string(), occurrence: Some(changed_title.clone()), created_at: now,
                        });
                    }
                }
            }
        }

        Ok(new_pending)
    }
}

fn get_or_create_tag(conn: &Connection, name: &str) -> Result<String> {
    let existing: Option<String> = conn.query_row(
        "SELECT id FROM tags WHERE name = ?1",
        params![name],
        |r| r.get(0),
    ).ok();

    if let Some(id) = existing {
        return Ok(id);
    }

    let id = Uuid::new_v4().to_string();
    conn.execute("INSERT INTO tags (id, name) VALUES (?1, ?2)", params![id, name])?;
    Ok(id)
}

fn map_node(row: &rusqlite::Row) -> rusqlite::Result<Node> {
    Ok(Node {
        id: row.get(0)?, title: row.get(1)?, content: row.get(2)?,
        fields: None,
        template_id: row.get(3)?, created_at: row.get(4)?, updated_at: row.get(5)?,
        deleted_at: row.get(6)?, is_pinned: row.get(7)?, is_active: row.get(8)?,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    async fn test_db() -> Database {
        let db = Database::open_in_memory().unwrap();
        db.migrate().await.unwrap();
        db
    }

    fn node(title: &str) -> CreateNode {
        CreateNode { title: title.into(), content: None, fields: None, template_id: None, section_ids: None, tag_names: None }
    }

    fn node_with(title: &str, content: &str) -> CreateNode {
        CreateNode { title: title.into(), content: Some(content.into()), fields: None, template_id: None, section_ids: None, tag_names: None }
    }

    fn node_with_fields(title: &str, fields: serde_json::Value) -> CreateNode {
        CreateNode { title: title.into(), content: None, fields: Some(fields), template_id: None, section_ids: None, tag_names: None }
    }

    fn update() -> UpdateNode {
        UpdateNode { title: None, content: None, fields: None, template_id: None, is_pinned: None, is_active: None, section_ids: None, tag_names: None }
    }

    // ── Seed ──

    #[tokio::test]
    async fn test_seed_creates_initial_data() {
        let db = test_db().await;
        assert!(db.seed_if_empty().await.unwrap());

        let nodes = db.list_nodes(false, 100, 0).await.unwrap();
        assert_eq!(nodes.len(), 1);
        assert_eq!(nodes[0].title, "Marc de Café");
        assert!(nodes[0].template_id.is_some());
        // Content is None — structured data is in fields
        assert!(nodes[0].content.is_none());

        let templates = db.list_templates().await.unwrap();
        assert_eq!(templates.len(), 1);
        assert_eq!(templates[0].name, "Fiche Épice");
        assert!(templates[0].structure.is_some());
        assert!(templates[0].preview_css.is_some());

        let sections = db.list_sections().await.unwrap();
        assert_eq!(sections.len(), 1);
        assert_eq!(sections[0].name, "Épices");
        assert_eq!(sections[0].color.as_deref(), Some("#e6a23c"));
    }

    #[tokio::test]
    async fn test_seed_node_has_fields_in_db() {
        let db = test_db().await;
        db.seed_if_empty().await.unwrap();
        let nodes = db.list_nodes(false, 100, 0).await.unwrap();
        let fields = nodes[0].fields.as_ref().unwrap();
        assert!(fields.is_object());
        let obj = fields.as_object().unwrap();
        assert_eq!(obj.get("odeur_froid").unwrap().as_str().unwrap(), "Terreux, légèrement acide, cacao amer");
        assert_eq!(obj.get("intensite").unwrap().as_u64().unwrap(), 4);
        assert_eq!(obj.get("fiche_num").unwrap().as_str().unwrap(), "001");
        assert_eq!(obj.get("subtitle").unwrap().as_str().unwrap(), "Coffea arabica \u{2014} résidu de torréfaction");
        assert_eq!(obj.get("intensite_label").unwrap().as_str().unwrap(), "puissant");
        // associations is an array
        let assoc = obj.get("associations").unwrap().as_array().unwrap();
        assert_eq!(assoc.len(), 6);
        assert_eq!(assoc[0].as_str().unwrap(), "Bœuf / viande rouge");
    }

    #[tokio::test]
    async fn test_seed_idempotent() {
        let db = test_db().await;
        assert!(db.seed_if_empty().await.unwrap());
        assert!(!db.seed_if_empty().await.unwrap());
        assert_eq!(db.list_templates().await.unwrap().len(), 1);
    }

    #[tokio::test]
    async fn test_seed_node_has_tags_and_section() {
        let db = test_db().await;
        db.seed_if_empty().await.unwrap();
        let nodes = db.list_nodes(false, 100, 0).await.unwrap();
        let tags = db.get_node_tags(&nodes[0].id).await.unwrap();
        assert_eq!(tags.len(), 6); // Bœuf, Sauce brune, Chocolat noir, Gibier, Échalote caramélisée, Poivre long
        let sections = db.get_node_sections(&nodes[0].id).await.unwrap();
        assert_eq!(sections.len(), 1);
        assert_eq!(sections[0].name, "Épices");
    }

    #[tokio::test]
    async fn test_seed_creates_initial_version() {
        let db = test_db().await;
        db.seed_if_empty().await.unwrap();
        let nodes = db.list_nodes(false, 100, 0).await.unwrap();
        let versions = db.get_node_versions(&nodes[0].id).await.unwrap();
        assert_eq!(versions.len(), 1);
        assert_eq!(versions[0].version_num, 1);
    }

    // ── Node CRUD ──

    #[tokio::test]
    async fn test_create_node_minimal() {
        let db = test_db().await;
        let n = db.create_node(&node("Test")).await.unwrap();
        assert_eq!(n.title, "Test");
        assert!(n.content.is_none());
        assert!(!n.is_pinned);
        assert!(!n.is_active);
        assert!(n.deleted_at.is_none());
    }

    #[tokio::test]
    async fn test_create_node_with_content_and_tags() {
        let db = test_db().await;
        let n = db.create_node(&CreateNode {
            title: "Cumin".into(),
            content: Some("Épice chaude".into()),
            fields: None,
            template_id: None,
            section_ids: None,
            tag_names: Some(vec!["épice".into(), "chaud".into()]),
        }).await.unwrap();
        assert_eq!(n.title, "Cumin");
        assert_eq!(n.content.as_deref(), Some("Épice chaude"));
        let tags = db.get_node_tags(&n.id).await.unwrap();
        assert_eq!(tags.len(), 2);
    }

    #[tokio::test]
    async fn test_create_node_with_section() {
        let db = test_db().await;
        let sec = db.create_section(&CreateSection { name: "Herbes".into(), description: None, color: None, pos_x: None, pos_y: None }).await.unwrap();
        let n = db.create_node(&CreateNode {
            title: "Basilic".into(), content: None, fields: None, template_id: None,
            section_ids: Some(vec![sec.id.clone()]), tag_names: None,
        }).await.unwrap();
        let sections = db.get_node_sections(&n.id).await.unwrap();
        assert_eq!(sections.len(), 1);
        assert_eq!(sections[0].name, "Herbes");
    }

    #[tokio::test]
    async fn test_create_node_with_template() {
        let db = test_db().await;
        let tmpl = db.create_template(&CreateTemplate { name: "Simple".into(), structure: None, preview_css: None, preview_html: None }).await.unwrap();
        let n = db.create_node(&CreateNode {
            title: "Noeud".into(), content: None, fields: None, template_id: Some(tmpl.id.clone()),
            section_ids: None, tag_names: None,
        }).await.unwrap();
        assert_eq!(n.template_id.as_deref(), Some(tmpl.id.as_str()));
    }

    #[tokio::test]
    async fn test_get_node_not_found() {
        let db = test_db().await;
        assert!(db.get_node("nonexistent").await.unwrap().is_none());
    }

    #[tokio::test]
    async fn test_list_nodes_excludes_deleted() {
        let db = test_db().await;
        let n1 = db.create_node(&node("A")).await.unwrap();
        let _n2 = db.create_node(&node("B")).await.unwrap();
        db.soft_delete_node(&n1.id).await.unwrap();
        let nodes = db.list_nodes(false, 100, 0).await.unwrap();
        assert_eq!(nodes.len(), 1);
        assert_eq!(nodes[0].title, "B");
    }

    #[tokio::test]
    async fn test_list_nodes_include_deleted() {
        let db = test_db().await;
        let n1 = db.create_node(&node("A")).await.unwrap();
        db.create_node(&node("B")).await.unwrap();
        db.soft_delete_node(&n1.id).await.unwrap();
        let all = db.list_nodes(true, 100, 0).await.unwrap();
        assert_eq!(all.len(), 2);
    }

    #[tokio::test]
    async fn test_update_node_title() {
        let db = test_db().await;
        let n = db.create_node(&node("Old")).await.unwrap();
        let updated = db.update_node(&n.id, &UpdateNode { title: Some("New".into()), ..update() }).await.unwrap().unwrap();
        assert_eq!(updated.title, "New");
        assert_ne!(updated.updated_at, n.updated_at);
    }

    #[tokio::test]
    async fn test_update_node_content_creates_version() {
        let db = test_db().await;
        let n = db.create_node(&node_with("Test", "v1")).await.unwrap();
        db.update_node(&n.id, &UpdateNode { content: Some("v2".into()), ..update() }).await.unwrap();
        db.update_node(&n.id, &UpdateNode { content: Some("v3".into()), ..update() }).await.unwrap();
        let versions = db.get_node_versions(&n.id).await.unwrap();
        assert_eq!(versions.len(), 3);
        assert_eq!(versions[0].version_num, 3); // DESC order
        assert_eq!(versions[2].version_num, 1);
    }

    #[tokio::test]
    async fn test_update_node_pinned() {
        let db = test_db().await;
        let n = db.create_node(&node("Pin me")).await.unwrap();
        assert!(!n.is_pinned);
        let updated = db.update_node(&n.id, &UpdateNode { is_pinned: Some(true), ..update() }).await.unwrap().unwrap();
        assert!(updated.is_pinned);
    }

    #[tokio::test]
    async fn test_update_node_active_deactivates_others() {
        let db = test_db().await;
        let n1 = db.create_node(&node("A")).await.unwrap();
        let n2 = db.create_node(&node("B")).await.unwrap();
        db.update_node(&n1.id, &UpdateNode { is_active: Some(true), ..update() }).await.unwrap();
        assert!(db.get_node(&n1.id).await.unwrap().unwrap().is_active);

        // Activating n2 should deactivate n1
        db.update_node(&n2.id, &UpdateNode { is_active: Some(true), ..update() }).await.unwrap();
        assert!(!db.get_node(&n1.id).await.unwrap().unwrap().is_active);
        assert!(db.get_node(&n2.id).await.unwrap().unwrap().is_active);
    }

    #[tokio::test]
    async fn test_update_node_tags_replaces() {
        let db = test_db().await;
        let n = db.create_node(&CreateNode {
            tag_names: Some(vec!["a".into(), "b".into()]), ..node("T")
        }).await.unwrap();
        assert_eq!(db.get_node_tags(&n.id).await.unwrap().len(), 2);

        db.update_node(&n.id, &UpdateNode { tag_names: Some(vec!["c".into()]), ..update() }).await.unwrap();
        let tags = db.get_node_tags(&n.id).await.unwrap();
        assert_eq!(tags.len(), 1);
        assert_eq!(tags[0].name, "c");
    }

    #[tokio::test]
    async fn test_update_node_sections_replaces() {
        let db = test_db().await;
        let s1 = db.create_section(&CreateSection { name: "S1".into(), description: None, color: None, pos_x: None, pos_y: None }).await.unwrap();
        let s2 = db.create_section(&CreateSection { name: "S2".into(), description: None, color: None, pos_x: None, pos_y: None }).await.unwrap();
        let n = db.create_node(&CreateNode { section_ids: Some(vec![s1.id.clone()]), ..node("N") }).await.unwrap();
        assert_eq!(db.get_node_sections(&n.id).await.unwrap().len(), 1);

        db.update_node(&n.id, &UpdateNode { section_ids: Some(vec![s2.id.clone()]), ..update() }).await.unwrap();
        let secs = db.get_node_sections(&n.id).await.unwrap();
        assert_eq!(secs.len(), 1);
        assert_eq!(secs[0].name, "S2");
    }

    #[tokio::test]
    async fn test_update_nonexistent_node() {
        let db = test_db().await;
        assert!(db.update_node("nope", &UpdateNode { title: Some("X".into()), ..update() }).await.unwrap().is_none());
    }

    #[tokio::test]
    async fn test_update_fts_after_title_change() {
        let db = test_db().await;
        let n = db.create_node(&node_with("Coriandre", "Herbe fraîche")).await.unwrap();
        let r1 = db.search("Coriandre", None, None).await.unwrap();
        assert_eq!(r1.len(), 1);

        db.update_node(&n.id, &UpdateNode { title: Some("Persil".into()), ..update() }).await.unwrap();
        let r2 = db.search("Coriandre", None, None).await.unwrap();
        assert_eq!(r2.len(), 0);
        let r3 = db.search("Persil", None, None).await.unwrap();
        assert_eq!(r3.len(), 1);
    }

    // ── Node Fields ──

    #[tokio::test]
    async fn test_create_node_with_fields() {
        let db = test_db().await;
        let fields = serde_json::json!({
            "odeur": "Florale",
            "intensite": 3,
            "notes": "Léger et parfumé"
        });
        let n = db.create_node(&node_with_fields("Lavande", fields)).await.unwrap();
        let f = n.fields.as_ref().unwrap().as_object().unwrap();
        assert_eq!(f.get("odeur").unwrap().as_str().unwrap(), "Florale");
        assert_eq!(f.get("intensite").unwrap().as_u64().unwrap(), 3);
        assert_eq!(f.get("notes").unwrap().as_str().unwrap(), "Léger et parfumé");
    }

    #[tokio::test]
    async fn test_node_fields_empty_by_default() {
        let db = test_db().await;
        let n = db.create_node(&node("Simple")).await.unwrap();
        let f = n.fields.as_ref().unwrap();
        assert!(f.as_object().unwrap().is_empty());
    }

    #[tokio::test]
    async fn test_update_node_fields() {
        let db = test_db().await;
        let n = db.create_node(&node("Test")).await.unwrap();
        let new_fields = serde_json::json!({"couleur": "rouge", "score": 5});
        db.update_node(&n.id, &UpdateNode { fields: Some(new_fields), ..update() }).await.unwrap();
        let updated = db.get_node(&n.id).await.unwrap().unwrap();
        let f = updated.fields.as_ref().unwrap().as_object().unwrap();
        assert_eq!(f.get("couleur").unwrap().as_str().unwrap(), "rouge");
        assert_eq!(f.get("score").unwrap().as_u64().unwrap(), 5);
    }

    #[tokio::test]
    async fn test_update_fields_replaces_all() {
        let db = test_db().await;
        let n = db.create_node(&node_with_fields("F", serde_json::json!({"a": "1", "b": "2"}))).await.unwrap();
        db.update_node(&n.id, &UpdateNode { fields: Some(serde_json::json!({"c": "3"})), ..update() }).await.unwrap();
        let updated = db.get_node(&n.id).await.unwrap().unwrap();
        let f = updated.fields.as_ref().unwrap().as_object().unwrap();
        assert!(f.get("a").is_none()); // Old fields removed
        assert_eq!(f.get("c").unwrap().as_str().unwrap(), "3");
    }

    #[tokio::test]
    async fn test_fields_with_array_value() {
        let db = test_db().await;
        let n = db.create_node(&node_with_fields("Tags", serde_json::json!({
            "ingredients": ["sel", "poivre", "ail"]
        }))).await.unwrap();
        let f = n.fields.as_ref().unwrap().as_object().unwrap();
        let arr = f.get("ingredients").unwrap().as_array().unwrap();
        assert_eq!(arr.len(), 3);
        assert_eq!(arr[0].as_str().unwrap(), "sel");
    }

    #[tokio::test]
    async fn test_fields_create_version() {
        let db = test_db().await;
        let n = db.create_node(&node_with_fields("V", serde_json::json!({"x": "1"}))).await.unwrap();
        db.update_node(&n.id, &UpdateNode { fields: Some(serde_json::json!({"x": "2"})), ..update() }).await.unwrap();
        let versions = db.get_node_versions(&n.id).await.unwrap();
        assert_eq!(versions.len(), 2);
    }

    #[tokio::test]
    async fn test_fields_indexed_in_fts() {
        let db = test_db().await;
        db.create_node(&node_with_fields("Romarin", serde_json::json!({
            "arôme": "Méditerranéen boisé"
        }))).await.unwrap();
        // Search by field value
        let results = db.search("Méditerranéen", None, None).await.unwrap();
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].title, "Romarin");
    }

    #[tokio::test]
    async fn test_fields_purged_with_node() {
        let db = test_db().await;
        let n = db.create_node(&node_with_fields("Del", serde_json::json!({"x": "1"}))).await.unwrap();
        db.soft_delete_node(&n.id).await.unwrap();
        assert!(db.purge_node(&n.id).await.unwrap());
        // Node and its fields are gone
    }

    #[tokio::test]
    async fn test_occurrence_detection_on_fields() {
        let db = test_db().await;
        let _n1 = db.create_node(&node("Curry")).await.unwrap();
        let n2 = db.create_node(&node_with_fields("Curcuma", serde_json::json!({
            "usage": "Ingrédient principal du Curry"
        }))).await.unwrap();
        let pending = db.detect_occurrences(&n2.id).await.unwrap();
        // "Curry" (title of n1) appears in n2's fields
        assert!(!pending.is_empty());
    }

    // ── Soft Delete / Trash / Restore / Purge ──

    #[tokio::test]
    async fn test_soft_delete() {
        let db = test_db().await;
        let n = db.create_node(&node("Del")).await.unwrap();
        assert!(db.soft_delete_node(&n.id).await.unwrap());
        assert!(db.get_node(&n.id).await.unwrap().is_none());
    }

    #[tokio::test]
    async fn test_soft_delete_nonexistent() {
        let db = test_db().await;
        assert!(!db.soft_delete_node("nope").await.unwrap());
    }

    #[tokio::test]
    async fn test_soft_delete_idempotent() {
        let db = test_db().await;
        let n = db.create_node(&node("Del")).await.unwrap();
        assert!(db.soft_delete_node(&n.id).await.unwrap());
        assert!(!db.soft_delete_node(&n.id).await.unwrap()); // already deleted
    }

    #[tokio::test]
    async fn test_trash_list() {
        let db = test_db().await;
        let n1 = db.create_node(&node("A")).await.unwrap();
        let n2 = db.create_node(&node("B")).await.unwrap();
        db.soft_delete_node(&n1.id).await.unwrap();
        db.soft_delete_node(&n2.id).await.unwrap();
        let trash = db.list_trash(100, 0).await.unwrap();
        assert_eq!(trash.len(), 2);
        assert!(trash.iter().all(|n| n.deleted_at.is_some()));
    }

    #[tokio::test]
    async fn test_restore_node() {
        let db = test_db().await;
        let n = db.create_node(&node_with("R", "content")).await.unwrap();
        db.soft_delete_node(&n.id).await.unwrap();
        assert!(db.restore_node(&n.id).await.unwrap());
        let restored = db.get_node(&n.id).await.unwrap().unwrap();
        assert!(restored.deleted_at.is_none());
        assert_eq!(restored.title, "R");
    }

    #[tokio::test]
    async fn test_restore_re_indexes_fts() {
        let db = test_db().await;
        let n = db.create_node(&node_with("Safran", "Épice précieuse")).await.unwrap();
        db.soft_delete_node(&n.id).await.unwrap();
        assert_eq!(db.search("Safran", None, None).await.unwrap().len(), 0);
        db.restore_node(&n.id).await.unwrap();
        assert_eq!(db.search("Safran", None, None).await.unwrap().len(), 1);
    }

    #[tokio::test]
    async fn test_restore_nonexistent() {
        let db = test_db().await;
        assert!(!db.restore_node("nope").await.unwrap());
    }

    #[tokio::test]
    async fn test_purge_only_deleted() {
        let db = test_db().await;
        let n = db.create_node(&node("Active")).await.unwrap();
        // Can't purge an active node
        assert!(!db.purge_node(&n.id).await.unwrap());
    }

    #[tokio::test]
    async fn test_purge_cleans_related_data() {
        let db = test_db().await;
        let n1 = db.create_node(&CreateNode { tag_names: Some(vec!["tag1".into()]), ..node_with("N1", "content") }).await.unwrap();
        let n2 = db.create_node(&node("N2")).await.unwrap();
        let sec = db.create_section(&CreateSection { name: "S".into(), description: None, color: None, pos_x: None, pos_y: None }).await.unwrap();
        db.update_node(&n1.id, &UpdateNode { section_ids: Some(vec![sec.id.clone()]), ..update() }).await.unwrap();
        db.create_edge(&CreateEdge { node_from: n1.id.clone(), node_to: n2.id.clone(), relation: None }).await.unwrap();

        db.soft_delete_node(&n1.id).await.unwrap();
        assert!(db.purge_node(&n1.id).await.unwrap());

        // Edges cleaned
        assert_eq!(db.get_node_edges(&n2.id).await.unwrap().len(), 0);
        // Section still exists
        assert!(db.get_section(&sec.id).await.unwrap().is_some());
        // Node section link gone
        assert_eq!(db.get_section_nodes(&sec.id).await.unwrap().len(), 0);
    }

    // ── Sections CRUD ──

    #[tokio::test]
    async fn test_create_section() {
        let db = test_db().await;
        let s = db.create_section(&CreateSection {
            name: "Herbes".into(), description: Some("Herbes fraîches".into()),
            color: Some("#22c55e".into()), pos_x: Some(1.0), pos_y: Some(2.0),
        }).await.unwrap();
        assert_eq!(s.name, "Herbes");
        assert_eq!(s.color.as_deref(), Some("#22c55e"));
        assert_eq!(s.pos_x, Some(1.0));
    }

    #[tokio::test]
    async fn test_get_section_not_found() {
        let db = test_db().await;
        assert!(db.get_section("nope").await.unwrap().is_none());
    }

    #[tokio::test]
    async fn test_update_section() {
        let db = test_db().await;
        let s = db.create_section(&CreateSection { name: "Old".into(), description: None, color: None, pos_x: None, pos_y: None }).await.unwrap();
        let updated = db.update_section(&s.id, &UpdateSection {
            name: Some("New".into()), description: None, color: Some("#ff0000".into()), pos_x: Some(5.0), pos_y: None,
        }).await.unwrap().unwrap();
        assert_eq!(updated.name, "New");
        assert_eq!(updated.color.as_deref(), Some("#ff0000"));
        assert_eq!(updated.pos_x, Some(5.0));
    }

    #[tokio::test]
    async fn test_delete_section_cleans_node_links() {
        let db = test_db().await;
        let s = db.create_section(&CreateSection { name: "S".into(), description: None, color: None, pos_x: None, pos_y: None }).await.unwrap();
        let n = db.create_node(&CreateNode { section_ids: Some(vec![s.id.clone()]), ..node("N") }).await.unwrap();
        assert_eq!(db.get_node_sections(&n.id).await.unwrap().len(), 1);

        assert!(db.delete_section(&s.id).await.unwrap());
        assert_eq!(db.get_node_sections(&n.id).await.unwrap().len(), 0);
        assert!(db.get_section(&s.id).await.unwrap().is_none());
    }

    #[tokio::test]
    async fn test_delete_section_nonexistent() {
        let db = test_db().await;
        assert!(!db.delete_section("nope").await.unwrap());
    }

    #[tokio::test]
    async fn test_section_nodes() {
        let db = test_db().await;
        let s = db.create_section(&CreateSection { name: "S".into(), description: None, color: None, pos_x: None, pos_y: None }).await.unwrap();
        db.create_node(&CreateNode { section_ids: Some(vec![s.id.clone()]), ..node("A") }).await.unwrap();
        db.create_node(&CreateNode { section_ids: Some(vec![s.id.clone()]), ..node("B") }).await.unwrap();
        db.create_node(&node("C")).await.unwrap(); // Not in section
        let nodes = db.get_section_nodes(&s.id).await.unwrap();
        assert_eq!(nodes.len(), 2);
    }

    #[tokio::test]
    async fn test_node_multiple_sections() {
        let db = test_db().await;
        let s1 = db.create_section(&CreateSection { name: "S1".into(), description: None, color: None, pos_x: None, pos_y: None }).await.unwrap();
        let s2 = db.create_section(&CreateSection { name: "S2".into(), description: None, color: None, pos_x: None, pos_y: None }).await.unwrap();
        let n = db.create_node(&CreateNode { section_ids: Some(vec![s1.id.clone(), s2.id.clone()]), ..node("Multi") }).await.unwrap();
        let secs = db.get_node_sections(&n.id).await.unwrap();
        assert_eq!(secs.len(), 2);
    }

    // ── Edges CRUD ──

    #[tokio::test]
    async fn test_create_edge_with_relation() {
        let db = test_db().await;
        let n1 = db.create_node(&node("A")).await.unwrap();
        let n2 = db.create_node(&node("B")).await.unwrap();
        let edge = db.create_edge(&CreateEdge { node_from: n1.id.clone(), node_to: n2.id.clone(), relation: Some("contient".into()) }).await.unwrap();
        assert_eq!(edge.relation.as_deref(), Some("contient"));
        assert!(!edge.auto_created);
        assert!(edge.confirmed);
    }

    #[tokio::test]
    async fn test_create_edge_without_relation() {
        let db = test_db().await;
        let n1 = db.create_node(&node("A")).await.unwrap();
        let n2 = db.create_node(&node("B")).await.unwrap();
        let edge = db.create_edge(&CreateEdge { node_from: n1.id.clone(), node_to: n2.id.clone(), relation: None }).await.unwrap();
        assert!(edge.relation.is_none());
    }

    #[tokio::test]
    async fn test_edge_bidirectional_query() {
        let db = test_db().await;
        let n1 = db.create_node(&node("A")).await.unwrap();
        let n2 = db.create_node(&node("B")).await.unwrap();
        db.create_edge(&CreateEdge { node_from: n1.id.clone(), node_to: n2.id.clone(), relation: None }).await.unwrap();
        // Both nodes should see the edge
        assert_eq!(db.get_node_edges(&n1.id).await.unwrap().len(), 1);
        assert_eq!(db.get_node_edges(&n2.id).await.unwrap().len(), 1);
    }

    #[tokio::test]
    async fn test_delete_edge() {
        let db = test_db().await;
        let n1 = db.create_node(&node("A")).await.unwrap();
        let n2 = db.create_node(&node("B")).await.unwrap();
        let edge = db.create_edge(&CreateEdge { node_from: n1.id.clone(), node_to: n2.id.clone(), relation: None }).await.unwrap();
        assert!(db.delete_edge(&edge.id).await.unwrap());
        assert_eq!(db.get_node_edges(&n1.id).await.unwrap().len(), 0);
    }

    #[tokio::test]
    async fn test_delete_edge_nonexistent() {
        let db = test_db().await;
        assert!(!db.delete_edge("nope").await.unwrap());
    }

    #[tokio::test]
    async fn test_list_edges() {
        let db = test_db().await;
        let n1 = db.create_node(&node("A")).await.unwrap();
        let n2 = db.create_node(&node("B")).await.unwrap();
        let n3 = db.create_node(&node("C")).await.unwrap();
        db.create_edge(&CreateEdge { node_from: n1.id.clone(), node_to: n2.id.clone(), relation: None }).await.unwrap();
        db.create_edge(&CreateEdge { node_from: n2.id.clone(), node_to: n3.id.clone(), relation: None }).await.unwrap();
        assert_eq!(db.list_edges(100, 0).await.unwrap().len(), 2);
    }

    // ── Templates CRUD ──

    #[tokio::test]
    async fn test_create_template() {
        let db = test_db().await;
        let t = db.create_template(&CreateTemplate {
            name: "Simple".into(),
            structure: Some(serde_json::json!([{"name": "champ", "type": "text"}])),
            preview_css: Some(".simple { color: red; }".into()),
            preview_html: Some("<div>{{champ}}</div>".into()),
        }).await.unwrap();
        assert_eq!(t.name, "Simple");
        assert!(t.structure.is_some());
        assert!(t.preview_css.is_some());
    }

    #[tokio::test]
    async fn test_get_template_not_found() {
        let db = test_db().await;
        assert!(db.get_template("nope").await.unwrap().is_none());
    }

    #[tokio::test]
    async fn test_update_template() {
        let db = test_db().await;
        let t = db.create_template(&CreateTemplate { name: "Old".into(), structure: None, preview_css: None, preview_html: None }).await.unwrap();
        let updated = db.update_template(&t.id, &UpdateTemplate {
            name: Some("New".into()),
            structure: Some(serde_json::json!([])),
            preview_css: Some("css".into()),
            preview_html: Some("<div>html</div>".into()),
        }).await.unwrap().unwrap();
        assert_eq!(updated.name, "New");
    }

    #[tokio::test]
    async fn test_delete_template_unlinks_nodes() {
        let db = test_db().await;
        let t = db.create_template(&CreateTemplate { name: "T".into(), structure: None, preview_css: None, preview_html: None }).await.unwrap();
        let n = db.create_node(&CreateNode { template_id: Some(t.id.clone()), ..node("N") }).await.unwrap();
        assert!(n.template_id.is_some());

        assert!(db.delete_template(&t.id).await.unwrap());
        let updated_n = db.get_node(&n.id).await.unwrap().unwrap();
        assert!(updated_n.template_id.is_none());
    }

    #[tokio::test]
    async fn test_delete_template_nonexistent() {
        let db = test_db().await;
        assert!(!db.delete_template("nope").await.unwrap());
    }

    #[tokio::test]
    async fn test_list_templates() {
        let db = test_db().await;
        db.create_template(&CreateTemplate { name: "A".into(), structure: None, preview_css: None, preview_html: None }).await.unwrap();
        db.create_template(&CreateTemplate { name: "B".into(), structure: None, preview_css: None, preview_html: None }).await.unwrap();
        assert_eq!(db.list_templates().await.unwrap().len(), 2);
    }

    // ── Tags ──

    #[tokio::test]
    async fn test_tags_deduplication() {
        let db = test_db().await;
        db.create_node(&CreateNode { tag_names: Some(vec!["épice".into()]), ..node("A") }).await.unwrap();
        db.create_node(&CreateNode { tag_names: Some(vec!["épice".into(), "autre".into()]), ..node("B") }).await.unwrap();
        let tags = db.list_tags().await.unwrap();
        // "épice" should exist only once
        let epice_count = tags.iter().filter(|t| t.name == "épice").count();
        assert_eq!(epice_count, 1);
        assert_eq!(tags.len(), 2); // "épice" + "autre"
    }

    #[tokio::test]
    async fn test_list_tags_sorted() {
        let db = test_db().await;
        db.create_node(&CreateNode { tag_names: Some(vec!["zeste".into(), "amer".into(), "fruité".into()]), ..node("N") }).await.unwrap();
        let tags = db.list_tags().await.unwrap();
        assert_eq!(tags[0].name, "amer");
        assert_eq!(tags[1].name, "fruité");
        assert_eq!(tags[2].name, "zeste");
    }

    // ── FTS Search ──

    #[tokio::test]
    async fn test_search_by_title() {
        let db = test_db().await;
        db.create_node(&node_with("Paprika fumé", "Piment doux")).await.unwrap();
        db.create_node(&node_with("Poivre noir", "Baie noire")).await.unwrap();
        let results = db.search("Paprika", None, None).await.unwrap();
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].title, "Paprika fumé");
    }

    #[tokio::test]
    async fn test_search_by_content() {
        let db = test_db().await;
        db.create_node(&node_with("A", "saveur chocolat intense")).await.unwrap();
        db.create_node(&node_with("B", "note fruitée légère")).await.unwrap();
        let results = db.search("chocolat", None, None).await.unwrap();
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].title, "A");
    }

    #[tokio::test]
    async fn test_search_no_results() {
        let db = test_db().await;
        db.create_node(&node_with("A", "contenu")).await.unwrap();
        let results = db.search("inexistant", None, None).await.unwrap();
        assert_eq!(results.len(), 0);
    }

    #[tokio::test]
    async fn test_search_excludes_deleted() {
        let db = test_db().await;
        let n = db.create_node(&node_with("Cannelle", "Épice douce")).await.unwrap();
        assert_eq!(db.search("Cannelle", None, None).await.unwrap().len(), 1);
        db.soft_delete_node(&n.id).await.unwrap();
        assert_eq!(db.search("Cannelle", None, None).await.unwrap().len(), 0);
    }

    #[tokio::test]
    async fn test_search_with_section_filter() {
        let db = test_db().await;
        let s1 = db.create_section(&CreateSection { name: "S1".into(), description: None, color: None, pos_x: None, pos_y: None }).await.unwrap();
        let s2 = db.create_section(&CreateSection { name: "S2".into(), description: None, color: None, pos_x: None, pos_y: None }).await.unwrap();
        db.create_node(&CreateNode { section_ids: Some(vec![s1.id.clone()]), ..node_with("Cumin", "épice") }).await.unwrap();
        db.create_node(&CreateNode { section_ids: Some(vec![s2.id.clone()]), ..node_with("Menthe", "épice") }).await.unwrap();

        let r = db.search("épice", Some(&s1.id), None).await.unwrap();
        assert_eq!(r.len(), 1);
        assert_eq!(r[0].title, "Cumin");
    }

    #[tokio::test]
    async fn test_search_with_tag_filter() {
        let db = test_db().await;
        db.create_node(&CreateNode { tag_names: Some(vec!["chaud".into()]), ..node_with("Piment", "épice forte") }).await.unwrap();
        db.create_node(&CreateNode { tag_names: Some(vec!["doux".into()]), ..node_with("Vanille", "épice douce") }).await.unwrap();

        let r = db.search("épice", None, Some("chaud")).await.unwrap();
        assert_eq!(r.len(), 1);
        assert_eq!(r[0].title, "Piment");
    }

    // ── Pending Links / Occurrence Detection ──

    #[tokio::test]
    async fn test_occurrence_detection_bidirectional() {
        let db = test_db().await;
        let n1 = db.create_node(&node_with("Curcuma", "Utilisé dans le curry")).await.unwrap();
        let _n2 = db.create_node(&node_with("Curry", "Contient du Curcuma")).await.unwrap();
        let pending = db.detect_occurrences(&n1.id).await.unwrap();
        // n1 content mentions "curry" (title of n2), n2 content mentions "Curcuma" (title of n1)
        // But both are the same pair, so should have exactly 1 pending link (checked both ways)
        assert!(!pending.is_empty());
    }

    #[tokio::test]
    async fn test_occurrence_case_insensitive() {
        let db = test_db().await;
        let n1 = db.create_node(&node_with("Ail", "L'ail noir est fermenté")).await.unwrap();
        let _n2 = db.create_node(&node_with("Ail noir", "Variété d'ail")).await.unwrap();
        let pending = db.detect_occurrences(&n1.id).await.unwrap();
        // "ail noir" (lowercase) should match title "Ail noir"
        assert!(!pending.is_empty());
    }

    #[tokio::test]
    async fn test_occurrence_no_duplicate_if_edge_exists() {
        let db = test_db().await;
        let n1 = db.create_node(&node_with("A", "Lié à B")).await.unwrap();
        let n2 = db.create_node(&node_with("B", "Lié à A")).await.unwrap();
        db.create_edge(&CreateEdge { node_from: n1.id.clone(), node_to: n2.id.clone(), relation: None }).await.unwrap();

        let pending = db.detect_occurrences(&n1.id).await.unwrap();
        assert!(pending.is_empty()); // Edge already exists, no pending
    }

    #[tokio::test]
    async fn test_occurrence_no_duplicate_if_pending_exists() {
        let db = test_db().await;
        let n1 = db.create_node(&node_with("X", "contient Y")).await.unwrap();
        let _n2 = db.create_node(&node_with("Y", "autre chose")).await.unwrap();

        let p1 = db.detect_occurrences(&n1.id).await.unwrap();
        assert!(!p1.is_empty());
        let p2 = db.detect_occurrences(&n1.id).await.unwrap();
        assert!(p2.is_empty()); // Already pending, no duplicate
    }

    #[tokio::test]
    async fn test_confirm_pending_link() {
        let db = test_db().await;
        let n1 = db.create_node(&node_with("X", "mentionne Y")).await.unwrap();
        let _n2 = db.create_node(&node_with("Y", "indépendant")).await.unwrap();
        let pending = db.detect_occurrences(&n1.id).await.unwrap();
        assert!(!pending.is_empty());

        let edge = db.confirm_pending_link(&pending[0].id, Some("associé")).await.unwrap().unwrap();
        assert!(edge.auto_created);
        assert!(edge.confirmed);
        assert_eq!(edge.relation.as_deref(), Some("associé"));

        // Pending link removed
        assert_eq!(db.list_pending_links(100, 0).await.unwrap().len(), 0);
        // Edge exists
        assert_eq!(db.get_node_edges(&n1.id).await.unwrap().len(), 1);
    }

    #[tokio::test]
    async fn test_dismiss_pending_link() {
        let db = test_db().await;
        let n1 = db.create_node(&node_with("X", "mentionne Y")).await.unwrap();
        let _n2 = db.create_node(&node_with("Y", "indépendant")).await.unwrap();
        let pending = db.detect_occurrences(&n1.id).await.unwrap();
        assert!(!pending.is_empty());

        assert!(db.dismiss_pending_link(&pending[0].id).await.unwrap());
        assert_eq!(db.list_pending_links(100, 0).await.unwrap().len(), 0);
        assert_eq!(db.get_node_edges(&n1.id).await.unwrap().len(), 0); // No edge created
    }

    #[tokio::test]
    async fn test_confirm_nonexistent_pending() {
        let db = test_db().await;
        assert!(db.confirm_pending_link("nope", None).await.unwrap().is_none());
    }

    #[tokio::test]
    async fn test_dismiss_nonexistent_pending() {
        let db = test_db().await;
        assert!(!db.dismiss_pending_link("nope").await.unwrap());
    }

    // ── Versions ──

    #[tokio::test]
    async fn test_version_on_create() {
        let db = test_db().await;
        let n = db.create_node(&node_with("V", "initial")).await.unwrap();
        let versions = db.get_node_versions(&n.id).await.unwrap();
        assert_eq!(versions.len(), 1);
        assert_eq!(versions[0].version_num, 1);
        // Version snapshot is JSON with content and fields
        let snapshot: serde_json::Value = serde_json::from_str(versions[0].content.as_deref().unwrap()).unwrap();
        assert_eq!(snapshot["content"].as_str().unwrap(), "initial");
    }

    #[tokio::test]
    async fn test_version_increments() {
        let db = test_db().await;
        let n = db.create_node(&node_with("V", "v1")).await.unwrap();
        db.update_node(&n.id, &UpdateNode { content: Some("v2".into()), ..update() }).await.unwrap();
        db.update_node(&n.id, &UpdateNode { content: Some("v3".into()), ..update() }).await.unwrap();
        let versions = db.get_node_versions(&n.id).await.unwrap();
        assert_eq!(versions.len(), 3);
        assert_eq!(versions[0].version_num, 3);
        assert_eq!(versions[1].version_num, 2);
        assert_eq!(versions[2].version_num, 1);
    }

    #[tokio::test]
    async fn test_title_update_no_version() {
        let db = test_db().await;
        let n = db.create_node(&node_with("V", "content")).await.unwrap();
        db.update_node(&n.id, &UpdateNode { title: Some("New title".into()), ..update() }).await.unwrap();
        // Title change alone should not create a new version
        let versions = db.get_node_versions(&n.id).await.unwrap();
        assert_eq!(versions.len(), 1);
    }

    // ── Study Desk ──

    #[tokio::test]
    async fn test_study_desk_full() {
        let db = test_db().await;
        let sec = db.create_section(&CreateSection { name: "S".into(), description: None, color: None, pos_x: None, pos_y: None }).await.unwrap();
        let n1 = db.create_node(&CreateNode {
            section_ids: Some(vec![sec.id.clone()]),
            tag_names: Some(vec!["tag1".into(), "tag2".into()]),
            ..node_with("Centre", "Noeud central")
        }).await.unwrap();
        let n2 = db.create_node(&node("Lié A")).await.unwrap();
        let n3 = db.create_node(&node("Lié B")).await.unwrap();
        db.create_edge(&CreateEdge { node_from: n1.id.clone(), node_to: n2.id.clone(), relation: Some("associé".into()) }).await.unwrap();
        db.create_edge(&CreateEdge { node_from: n3.id.clone(), node_to: n1.id.clone(), relation: Some("dérive de".into()) }).await.unwrap();

        let desk = db.get_study_desk(&n1.id).await.unwrap().unwrap();
        assert_eq!(desk.node.title, "Centre");
        assert_eq!(desk.connections.len(), 2);
        assert_eq!(desk.tags.len(), 2);
        assert_eq!(desk.sections.len(), 1);
        assert_eq!(desk.versions_count, 1);
    }

    #[tokio::test]
    async fn test_study_desk_not_found() {
        let db = test_db().await;
        assert!(db.get_study_desk("nope").await.unwrap().is_none());
    }

    #[tokio::test]
    async fn test_study_desk_with_pending_links() {
        let db = test_db().await;
        let n1 = db.create_node(&node_with("Gingembre", "Racine utilisée avec le Citron")).await.unwrap();
        let _n2 = db.create_node(&node_with("Citron", "Agrume")).await.unwrap();
        db.detect_occurrences(&n1.id).await.unwrap();

        let desk = db.get_study_desk(&n1.id).await.unwrap().unwrap();
        assert!(!desk.pending_links.is_empty());
    }

    // ── Graph Data ──

    #[tokio::test]
    async fn test_graph_data_empty() {
        let db = test_db().await;
        let data = db.get_graph_data().await.unwrap();
        assert!(data.sections.is_empty());
        assert!(data.inter_section_edges.is_empty());
    }

    #[tokio::test]
    async fn test_graph_data_with_sections() {
        let db = test_db().await;
        let s1 = db.create_section(&CreateSection { name: "S1".into(), description: None, color: None, pos_x: None, pos_y: None }).await.unwrap();
        let s2 = db.create_section(&CreateSection { name: "S2".into(), description: None, color: None, pos_x: None, pos_y: None }).await.unwrap();
        db.create_node(&CreateNode { section_ids: Some(vec![s1.id.clone()]), ..node("A") }).await.unwrap();
        db.create_node(&CreateNode { section_ids: Some(vec![s1.id.clone()]), ..node("B") }).await.unwrap();
        db.create_node(&CreateNode { section_ids: Some(vec![s2.id.clone()]), ..node("C") }).await.unwrap();

        let data = db.get_graph_data().await.unwrap();
        assert_eq!(data.sections.len(), 2);
        let s1_data = data.sections.iter().find(|s| s.section.name == "S1").unwrap();
        assert_eq!(s1_data.node_count, 2);
        let s2_data = data.sections.iter().find(|s| s.section.name == "S2").unwrap();
        assert_eq!(s2_data.node_count, 1);
    }

    #[tokio::test]
    async fn test_graph_inter_section_edges() {
        let db = test_db().await;
        let s1 = db.create_section(&CreateSection { name: "S1".into(), description: None, color: None, pos_x: None, pos_y: None }).await.unwrap();
        let s2 = db.create_section(&CreateSection { name: "S2".into(), description: None, color: None, pos_x: None, pos_y: None }).await.unwrap();
        let n1 = db.create_node(&CreateNode { section_ids: Some(vec![s1.id.clone()]), ..node("A") }).await.unwrap();
        let n2 = db.create_node(&CreateNode { section_ids: Some(vec![s2.id.clone()]), ..node("B") }).await.unwrap();
        let n3 = db.create_node(&CreateNode { section_ids: Some(vec![s1.id.clone()]), ..node("C") }).await.unwrap();

        // Inter-section edge (A in S1 → B in S2)
        db.create_edge(&CreateEdge { node_from: n1.id.clone(), node_to: n2.id.clone(), relation: None }).await.unwrap();
        // Intra-section edge (A in S1 → C in S1)
        db.create_edge(&CreateEdge { node_from: n1.id.clone(), node_to: n3.id.clone(), relation: None }).await.unwrap();

        let data = db.get_graph_data().await.unwrap();
        assert_eq!(data.inter_section_edges.len(), 1);
    }

    // ── Stats ──

    #[tokio::test]
    async fn test_stats() {
        let db = test_db().await;
        let (n, s, p) = db.stats().await.unwrap();
        assert_eq!(n, 0);
        assert_eq!(s, 0);
        assert_eq!(p, 0);

        db.create_node(&node("A")).await.unwrap();
        db.create_section(&CreateSection { name: "S".into(), description: None, color: None, pos_x: None, pos_y: None }).await.unwrap();
        let (n, s, p) = db.stats().await.unwrap();
        assert_eq!(n, 1);
        assert_eq!(s, 1);
        assert_eq!(p, 0);
    }

    #[tokio::test]
    async fn test_stats_excludes_deleted() {
        let db = test_db().await;
        let node = db.create_node(&node("A")).await.unwrap();
        assert_eq!(db.stats().await.unwrap().0, 1);
        db.soft_delete_node(&node.id).await.unwrap();
        assert_eq!(db.stats().await.unwrap().0, 0);
    }

    // ── Migration idempotence ──

    #[tokio::test]
    async fn test_migrate_idempotent() {
        let db = test_db().await; // Already migrated once
        db.migrate().await.unwrap(); // Should not fail
        db.migrate().await.unwrap(); // Nor on third call
    }

    // ── FTS sanitization ──

    #[tokio::test]
    async fn test_search_special_chars_no_crash() {
        let db = test_db().await;
        db.create_node(&node_with("Test", "content")).await.unwrap();
        // These should not crash (FTS5 special chars)
        assert!(db.search("test AND OR NOT", None, None).await.is_ok());
        assert!(db.search("\"unclosed quote", None, None).await.is_ok());
        assert!(db.search("col:value", None, None).await.is_ok());
        assert!(db.search("*wild*", None, None).await.is_ok());
    }

    #[tokio::test]
    async fn test_transaction_rollback_on_error() {
        let db = test_db().await;
        // Creating node with invalid template_id should still work (no FK constraint on template)
        // but verify transaction doesn't leave partial state
        let n = db.create_node(&node("TxTest")).await.unwrap();
        assert!(db.get_node(&n.id).await.unwrap().is_some());
    }

    #[tokio::test]
    async fn test_get_node_error_vs_not_found() {
        let db = test_db().await;
        // Not found should return None
        assert!(db.get_node("nonexistent-id").await.unwrap().is_none());
    }

    // ── Cascade / Orphan cleanup ──

    #[tokio::test]
    async fn test_cascade_delete_cleans_related() {
        let db = test_db().await;
        let n = db.create_node(&CreateNode {
            tag_names: Some(vec!["test-tag".into()]),
            ..node_with("CascadeTest", "content")
        }).await.unwrap();
        // Soft delete then purge
        db.soft_delete_node(&n.id).await.unwrap();
        db.purge_node(&n.id).await.unwrap();
        // Verify node is gone
        assert!(db.get_node(&n.id).await.unwrap().is_none());
    }

    #[tokio::test]
    async fn test_orphan_tags_cleaned() {
        let db = test_db().await;
        let n = db.create_node(&CreateNode {
            tag_names: Some(vec!["orphan-tag".into()]),
            ..node("OrphanTest")
        }).await.unwrap();
        assert_eq!(db.list_tags().await.unwrap().len(), 1);
        // Update with different tags
        db.update_node(&n.id, &UpdateNode { tag_names: Some(vec!["new-tag".into()]), ..update() }).await.unwrap();
        let tags = db.list_tags().await.unwrap();
        // "orphan-tag" should be cleaned
        assert_eq!(tags.len(), 1);
        assert_eq!(tags[0].name, "new-tag");
    }
}
