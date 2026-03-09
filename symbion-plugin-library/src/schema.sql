-- Symbion Knowledge Library - Schema v1

CREATE TABLE IF NOT EXISTS templates (
    id           TEXT PRIMARY KEY,
    name         TEXT NOT NULL,
    structure    TEXT,
    preview_css  TEXT,
    preview_html TEXT,
    created_at   DATETIME DEFAULT CURRENT_TIMESTAMP
);

CREATE TABLE IF NOT EXISTS nodes (
    id          TEXT PRIMARY KEY,
    title       TEXT NOT NULL,
    content     TEXT,
    template_id TEXT REFERENCES templates(id),
    created_at  DATETIME DEFAULT CURRENT_TIMESTAMP,
    updated_at  DATETIME DEFAULT CURRENT_TIMESTAMP,
    deleted_at  DATETIME,
    is_pinned   BOOLEAN DEFAULT FALSE,
    is_active   BOOLEAN DEFAULT FALSE
);

CREATE TABLE IF NOT EXISTS node_versions (
    id          TEXT PRIMARY KEY,
    node_id     TEXT REFERENCES nodes(id),
    content     TEXT,
    version_num INTEGER,
    created_at  DATETIME DEFAULT CURRENT_TIMESTAMP
);

CREATE TABLE IF NOT EXISTS sections (
    id          TEXT PRIMARY KEY,
    parent_id   TEXT REFERENCES sections(id) ON DELETE CASCADE,
    name        TEXT NOT NULL,
    description TEXT,
    color       TEXT,
    pos_x       REAL,
    pos_y       REAL,
    created_at  DATETIME DEFAULT CURRENT_TIMESTAMP
);

CREATE TABLE IF NOT EXISTS node_sections (
    node_id     TEXT REFERENCES nodes(id),
    section_id  TEXT REFERENCES sections(id),
    PRIMARY KEY (node_id, section_id)
);

CREATE TABLE IF NOT EXISTS edges (
    id          TEXT PRIMARY KEY,
    node_from   TEXT REFERENCES nodes(id),
    node_to     TEXT REFERENCES nodes(id),
    relation    TEXT,
    auto        BOOLEAN DEFAULT FALSE,
    confirmed   BOOLEAN DEFAULT TRUE,
    created_at  DATETIME DEFAULT CURRENT_TIMESTAMP
);

CREATE TABLE IF NOT EXISTS pending_links (
    id          TEXT PRIMARY KEY,
    node_from   TEXT REFERENCES nodes(id),
    node_to     TEXT REFERENCES nodes(id),
    occurrence  TEXT,
    created_at  DATETIME DEFAULT CURRENT_TIMESTAMP
);

CREATE TABLE IF NOT EXISTS tags (
    id   TEXT PRIMARY KEY,
    name TEXT UNIQUE NOT NULL
);

CREATE TABLE IF NOT EXISTS node_tags (
    node_id TEXT REFERENCES nodes(id),
    tag_id  TEXT REFERENCES tags(id),
    PRIMARY KEY (node_id, tag_id)
);

CREATE TABLE IF NOT EXISTS node_fields (
    id          TEXT PRIMARY KEY,
    node_id     TEXT NOT NULL REFERENCES nodes(id),
    field_name  TEXT NOT NULL,
    field_value TEXT,
    sort_order  INTEGER DEFAULT 0,
    UNIQUE(node_id, field_name)
);

-- Full Text Search (standalone — not external content)
CREATE VIRTUAL TABLE IF NOT EXISTS nodes_fts USING fts5(
    node_id UNINDEXED,
    title,
    content
);

-- Indexes
CREATE INDEX IF NOT EXISTS idx_nodes_deleted ON nodes(deleted_at);
CREATE INDEX IF NOT EXISTS idx_nodes_template ON nodes(template_id);
CREATE INDEX IF NOT EXISTS idx_nodes_active ON nodes(is_active) WHERE is_active = TRUE;
CREATE INDEX IF NOT EXISTS idx_fields_node ON node_fields(node_id);
CREATE INDEX IF NOT EXISTS idx_edges_from ON edges(node_from);
CREATE INDEX IF NOT EXISTS idx_edges_to ON edges(node_to);
CREATE INDEX IF NOT EXISTS idx_versions_node ON node_versions(node_id);
CREATE INDEX IF NOT EXISTS idx_pending_from ON pending_links(node_from);
CREATE INDEX IF NOT EXISTS idx_pending_to ON pending_links(node_to);
