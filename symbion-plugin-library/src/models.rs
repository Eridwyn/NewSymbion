use serde::{Deserialize, Serialize};

// ── Nodes ──

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Node {
    pub id: String,
    pub title: String,
    pub content: Option<String>,
    #[serde(skip_deserializing, skip_serializing_if = "Option::is_none")]
    pub fields: Option<serde_json::Value>,
    pub template_id: Option<String>,
    pub created_at: String,
    pub updated_at: String,
    pub deleted_at: Option<String>,
    pub is_pinned: bool,
    pub is_active: bool,
    #[serde(default)]
    pub view_count: i64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub last_viewed: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct CreateNode {
    pub title: String,
    pub content: Option<String>,
    pub fields: Option<serde_json::Value>,
    pub template_id: Option<String>,
    pub section_ids: Option<Vec<String>>,
    pub tag_names: Option<Vec<String>>,
}

#[derive(Debug, Deserialize)]
pub struct UpdateNode {
    pub title: Option<String>,
    pub content: Option<String>,
    pub fields: Option<serde_json::Value>,
    pub template_id: Option<String>,
    pub is_pinned: Option<bool>,
    pub is_active: Option<bool>,
    pub section_ids: Option<Vec<String>>,
    pub tag_names: Option<Vec<String>>,
}

// ── Node Fields ──

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NodeField {
    pub id: String,
    pub node_id: String,
    pub field_name: String,
    pub field_value: Option<String>,
    pub sort_order: i64,
}

// ── Sections ──

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Section {
    pub id: String,
    pub parent_id: Option<String>,
    pub name: String,
    pub description: Option<String>,
    pub color: Option<String>,
    pub pos_x: Option<f64>,
    pub pos_y: Option<f64>,
    pub created_at: String,
}

#[derive(Debug, Deserialize)]
pub struct CreateSection {
    pub name: String,
    pub parent_id: Option<String>,
    pub description: Option<String>,
    pub color: Option<String>,
    pub pos_x: Option<f64>,
    pub pos_y: Option<f64>,
}

#[derive(Debug, Deserialize)]
pub struct UpdateSection {
    pub name: Option<String>,
    pub parent_id: Option<String>,
    pub description: Option<String>,
    pub color: Option<String>,
    pub pos_x: Option<f64>,
    pub pos_y: Option<f64>,
}

// ── Edges ──

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Edge {
    pub id: String,
    pub node_from: String,
    pub node_to: String,
    pub relation: Option<String>,
    pub auto_created: bool,
    pub confirmed: bool,
    pub created_at: String,
}

#[derive(Debug, Deserialize)]
pub struct CreateEdge {
    pub node_from: String,
    pub node_to: String,
    pub relation: Option<String>,
}

// ── Templates ──

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Template {
    pub id: String,
    pub name: String,
    pub structure: Option<serde_json::Value>,
    pub preview_css: Option<String>,
    pub preview_html: Option<String>,
    pub created_at: String,
}

#[derive(Debug, Deserialize)]
pub struct CreateTemplate {
    pub name: String,
    pub structure: Option<serde_json::Value>,
    pub preview_css: Option<String>,
    pub preview_html: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct UpdateTemplate {
    pub name: Option<String>,
    pub structure: Option<serde_json::Value>,
    pub preview_css: Option<String>,
    pub preview_html: Option<String>,
}

// ── Tags ──

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Tag {
    pub id: String,
    pub name: String,
}

// ── Pending Links ──

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PendingLink {
    pub id: String,
    pub node_from: String,
    pub node_to: String,
    pub occurrence: Option<String>,
    pub created_at: String,
}

// ── Node Versions ──

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NodeVersion {
    pub id: String,
    pub node_id: String,
    pub content: Option<String>,
    pub version_num: i64,
    pub created_at: String,
}

// ── Study Desk (bureau d'étude) ──

#[derive(Debug, Serialize)]
pub struct StudyDesk {
    pub node: Node,
    pub fields: Option<serde_json::Value>,
    pub connections: Vec<StudyConnection>,
    pub pending_links: Vec<PendingLink>,
    pub tags: Vec<Tag>,
    pub sections: Vec<Section>,
    pub versions_count: i64,
}

#[derive(Debug, Serialize)]
pub struct StudyConnection {
    pub node: Node,
    pub edge: Edge,
}

// ── Search ──

#[derive(Debug, Deserialize)]
pub struct SearchQuery {
    pub q: String,
    pub section_id: Option<String>,
    pub tag: Option<String>,
    pub template_id: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct SearchResult {
    pub nodes: Vec<Node>,
    pub total: usize,
}

// ── List Params (pagination) ──

#[derive(Debug, Deserialize)]
pub struct ListParams {
    pub limit: Option<i64>,
    pub offset: Option<i64>,
}

impl ListParams {
    pub fn limit(&self) -> i64 {
        self.limit.unwrap_or(100).min(500).max(1)
    }
    pub fn offset(&self) -> i64 {
        self.offset.unwrap_or(0).max(0)
    }
}

// ── Graph data for library view ──

#[derive(Debug, Serialize)]
pub struct GraphData {
    pub sections: Vec<SectionWithNodes>,
    pub inter_section_edges: Vec<Edge>,
}

#[derive(Debug, Serialize)]
pub struct SectionWithNodes {
    pub section: Section,
    pub node_count: usize,
    pub children: Vec<SectionWithNodes>,
}
