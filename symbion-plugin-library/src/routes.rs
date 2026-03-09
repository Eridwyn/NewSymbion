use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    response::IntoResponse,
    routing::{delete, get, post, put},
    Json, Router,
};
use std::sync::Arc;

use crate::models::*;
use crate::PluginState;

pub fn build_router(state: Arc<PluginState>) -> Router {
    Router::new()
        // Health
        .route("/health", get(health))
        // Nodes
        .route("/nodes", get(list_nodes))
        .route("/nodes", post(create_node))
        .route("/nodes/:id", get(get_node))
        .route("/nodes/:id", put(update_node))
        .route("/nodes/:id", delete(delete_node))
        .route("/nodes/:id/versions", get(get_node_versions))
        .route("/nodes/:id/desk", get(get_study_desk))
        .route("/nodes/:id/activate", post(activate_node))
        // Sections
        .route("/sections", get(list_sections))
        .route("/sections", post(create_section))
        .route("/sections/:id", get(get_section))
        .route("/sections/:id", put(update_section))
        .route("/sections/:id", delete(delete_section))
        .route("/sections/:id/nodes", get(get_section_nodes))
        // Edges
        .route("/edges", get(list_edges))
        .route("/edges", post(create_edge))
        .route("/edges/:id", delete(delete_edge))
        // Templates
        .route("/templates", get(list_templates))
        .route("/templates", post(create_template))
        .route("/templates/:id", get(get_template))
        .route("/templates/:id", put(update_template))
        .route("/templates/:id", delete(delete_template))
        // Tags
        .route("/tags", get(list_tags))
        // Search
        .route("/search", get(search))
        // Pending links
        .route("/pending-links", get(list_pending_links))
        .route("/pending-links/:id/confirm", post(confirm_pending_link))
        .route("/pending-links/:id/dismiss", post(dismiss_pending_link))
        // Trash
        .route("/trash", get(list_trash))
        .route("/trash/:id/restore", post(restore_node))
        .route("/trash/:id/purge", delete(purge_node))
        // Graph
        .route("/graph", get(get_graph_data))
        .with_state(state)
}

// ── Health ──

async fn health(State(state): State<Arc<PluginState>>) -> Json<serde_json::Value> {
    let (nodes, sections, pending) = state.db.stats().await.unwrap_or((0, 0, 0));
    Json(serde_json::json!({
        "plugin_id": "library",
        "spec_version": "1.0",
        "status": "healthy",
        "uptime_seconds": state.started_at.elapsed().as_secs(),
        "stats": {
            "nodes": nodes,
            "sections": sections,
            "pending_links": pending
        }
    }))
}

// ── Nodes ──

async fn list_nodes(State(state): State<Arc<PluginState>>) -> impl IntoResponse {
    match state.db.list_nodes(false).await {
        Ok(nodes) => Json(serde_json::json!({"nodes": nodes})).into_response(),
        Err(e) => (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()).into_response(),
    }
}

async fn create_node(
    State(state): State<Arc<PluginState>>,
    Json(input): Json<CreateNode>,
) -> impl IntoResponse {
    match state.db.create_node(&input).await {
        Ok(node) => {
            // MQTT event
            let _ = state.mqtt.publish_node_event("created", &node.id, &node.title).await;
            // Trigger occurrence detection
            let db = state.db.clone();
            let mqtt = state.mqtt.clone();
            let node_id = node.id.clone();
            tokio::spawn(async move {
                tokio::time::sleep(tokio::time::Duration::from_secs(1)).await;
                if let Ok(pending) = db.detect_occurrences(&node_id).await {
                    if !pending.is_empty() {
                        let _ = mqtt.publish_pending_links(pending.len()).await;
                    }
                }
            });
            (StatusCode::CREATED, Json(node)).into_response()
        }
        Err(e) => (StatusCode::BAD_REQUEST, e.to_string()).into_response(),
    }
}

async fn get_node(
    State(state): State<Arc<PluginState>>,
    Path(id): Path<String>,
) -> impl IntoResponse {
    match state.db.get_node(&id).await {
        Ok(Some(node)) => Json(node).into_response(),
        Ok(None) => StatusCode::NOT_FOUND.into_response(),
        Err(e) => (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()).into_response(),
    }
}

async fn update_node(
    State(state): State<Arc<PluginState>>,
    Path(id): Path<String>,
    Json(input): Json<UpdateNode>,
) -> impl IntoResponse {
    match state.db.update_node(&id, &input).await {
        Ok(Some(node)) => {
            let _ = state.mqtt.publish_node_event("updated", &node.id, &node.title).await;
            // Trigger occurrence detection on content/fields/title change
            if input.content.is_some() || input.title.is_some() || input.fields.is_some() {
                let db = state.db.clone();
                let mqtt = state.mqtt.clone();
                let node_id = node.id.clone();
                tokio::spawn(async move {
                    tokio::time::sleep(tokio::time::Duration::from_secs(1)).await;
                    if let Ok(pending) = db.detect_occurrences(&node_id).await {
                        if !pending.is_empty() {
                            let _ = mqtt.publish_pending_links(pending.len()).await;
                        }
                    }
                });
            }
            Json(node).into_response()
        }
        Ok(None) => StatusCode::NOT_FOUND.into_response(),
        Err(e) => (StatusCode::BAD_REQUEST, e.to_string()).into_response(),
    }
}

async fn delete_node(
    State(state): State<Arc<PluginState>>,
    Path(id): Path<String>,
) -> impl IntoResponse {
    match state.db.soft_delete_node(&id).await {
        Ok(true) => StatusCode::NO_CONTENT.into_response(),
        Ok(false) => StatusCode::NOT_FOUND.into_response(),
        Err(e) => (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()).into_response(),
    }
}

async fn get_node_versions(
    State(state): State<Arc<PluginState>>,
    Path(id): Path<String>,
) -> impl IntoResponse {
    match state.db.get_node_versions(&id).await {
        Ok(versions) => Json(serde_json::json!({"versions": versions})).into_response(),
        Err(e) => (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()).into_response(),
    }
}

async fn get_study_desk(
    State(state): State<Arc<PluginState>>,
    Path(id): Path<String>,
) -> impl IntoResponse {
    match state.db.get_study_desk(&id).await {
        Ok(Some(desk)) => Json(desk).into_response(),
        Ok(None) => StatusCode::NOT_FOUND.into_response(),
        Err(e) => (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()).into_response(),
    }
}

async fn activate_node(
    State(state): State<Arc<PluginState>>,
    Path(id): Path<String>,
) -> impl IntoResponse {
    let input = UpdateNode {
        title: None, content: None, fields: None, template_id: None,
        is_pinned: None, is_active: Some(true),
        section_ids: None, tag_names: None,
    };
    match state.db.update_node(&id, &input).await {
        Ok(Some(node)) => Json(node).into_response(),
        Ok(None) => StatusCode::NOT_FOUND.into_response(),
        Err(e) => (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()).into_response(),
    }
}

// ── Sections ──

async fn list_sections(State(state): State<Arc<PluginState>>) -> impl IntoResponse {
    match state.db.list_sections().await {
        Ok(sections) => Json(serde_json::json!({"sections": sections})).into_response(),
        Err(e) => (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()).into_response(),
    }
}

async fn create_section(
    State(state): State<Arc<PluginState>>,
    Json(input): Json<CreateSection>,
) -> impl IntoResponse {
    match state.db.create_section(&input).await {
        Ok(section) => (StatusCode::CREATED, Json(section)).into_response(),
        Err(e) => (StatusCode::BAD_REQUEST, e.to_string()).into_response(),
    }
}

async fn get_section(
    State(state): State<Arc<PluginState>>,
    Path(id): Path<String>,
) -> impl IntoResponse {
    match state.db.get_section(&id).await {
        Ok(Some(s)) => Json(s).into_response(),
        Ok(None) => StatusCode::NOT_FOUND.into_response(),
        Err(e) => (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()).into_response(),
    }
}

async fn update_section(
    State(state): State<Arc<PluginState>>,
    Path(id): Path<String>,
    Json(input): Json<UpdateSection>,
) -> impl IntoResponse {
    match state.db.update_section(&id, &input).await {
        Ok(Some(s)) => Json(s).into_response(),
        Ok(None) => StatusCode::NOT_FOUND.into_response(),
        Err(e) => (StatusCode::BAD_REQUEST, e.to_string()).into_response(),
    }
}

async fn delete_section(
    State(state): State<Arc<PluginState>>,
    Path(id): Path<String>,
) -> impl IntoResponse {
    match state.db.delete_section(&id).await {
        Ok(true) => StatusCode::NO_CONTENT.into_response(),
        Ok(false) => StatusCode::NOT_FOUND.into_response(),
        Err(e) => (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()).into_response(),
    }
}

async fn get_section_nodes(
    State(state): State<Arc<PluginState>>,
    Path(id): Path<String>,
) -> impl IntoResponse {
    match state.db.get_section_nodes(&id).await {
        Ok(nodes) => Json(serde_json::json!({"nodes": nodes})).into_response(),
        Err(e) => (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()).into_response(),
    }
}

// ── Edges ──

async fn list_edges(State(state): State<Arc<PluginState>>) -> impl IntoResponse {
    match state.db.list_edges().await {
        Ok(edges) => Json(serde_json::json!({"edges": edges})).into_response(),
        Err(e) => (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()).into_response(),
    }
}

async fn create_edge(
    State(state): State<Arc<PluginState>>,
    Json(input): Json<CreateEdge>,
) -> impl IntoResponse {
    match state.db.create_edge(&input).await {
        Ok(edge) => (StatusCode::CREATED, Json(edge)).into_response(),
        Err(e) => (StatusCode::BAD_REQUEST, e.to_string()).into_response(),
    }
}

async fn delete_edge(
    State(state): State<Arc<PluginState>>,
    Path(id): Path<String>,
) -> impl IntoResponse {
    match state.db.delete_edge(&id).await {
        Ok(true) => StatusCode::NO_CONTENT.into_response(),
        Ok(false) => StatusCode::NOT_FOUND.into_response(),
        Err(e) => (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()).into_response(),
    }
}

// ── Templates ──

async fn list_templates(State(state): State<Arc<PluginState>>) -> impl IntoResponse {
    match state.db.list_templates().await {
        Ok(templates) => Json(serde_json::json!({"templates": templates})).into_response(),
        Err(e) => (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()).into_response(),
    }
}

async fn create_template(
    State(state): State<Arc<PluginState>>,
    Json(input): Json<CreateTemplate>,
) -> impl IntoResponse {
    match state.db.create_template(&input).await {
        Ok(t) => (StatusCode::CREATED, Json(t)).into_response(),
        Err(e) => (StatusCode::BAD_REQUEST, e.to_string()).into_response(),
    }
}

async fn get_template(
    State(state): State<Arc<PluginState>>,
    Path(id): Path<String>,
) -> impl IntoResponse {
    match state.db.get_template(&id).await {
        Ok(Some(t)) => Json(t).into_response(),
        Ok(None) => StatusCode::NOT_FOUND.into_response(),
        Err(e) => (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()).into_response(),
    }
}

async fn update_template(
    State(state): State<Arc<PluginState>>,
    Path(id): Path<String>,
    Json(input): Json<UpdateTemplate>,
) -> impl IntoResponse {
    match state.db.update_template(&id, &input).await {
        Ok(Some(t)) => Json(t).into_response(),
        Ok(None) => StatusCode::NOT_FOUND.into_response(),
        Err(e) => (StatusCode::BAD_REQUEST, e.to_string()).into_response(),
    }
}

async fn delete_template(
    State(state): State<Arc<PluginState>>,
    Path(id): Path<String>,
) -> impl IntoResponse {
    match state.db.delete_template(&id).await {
        Ok(true) => StatusCode::NO_CONTENT.into_response(),
        Ok(false) => StatusCode::NOT_FOUND.into_response(),
        Err(e) => (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()).into_response(),
    }
}

// ── Tags ──

async fn list_tags(State(state): State<Arc<PluginState>>) -> impl IntoResponse {
    match state.db.list_tags().await {
        Ok(tags) => Json(serde_json::json!({"tags": tags})).into_response(),
        Err(e) => (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()).into_response(),
    }
}

// ── Search ──

async fn search(
    State(state): State<Arc<PluginState>>,
    Query(params): Query<SearchQuery>,
) -> impl IntoResponse {
    match state.db.search(&params.q, params.section_id.as_deref(), params.tag.as_deref()).await {
        Ok(nodes) => {
            let total = nodes.len();
            Json(SearchResult { nodes, total }).into_response()
        }
        Err(e) => (StatusCode::BAD_REQUEST, format!("Search error: {}", e)).into_response(),
    }
}

// ── Pending Links ──

async fn list_pending_links(State(state): State<Arc<PluginState>>) -> impl IntoResponse {
    match state.db.list_pending_links().await {
        Ok(links) => Json(serde_json::json!({"pending_links": links})).into_response(),
        Err(e) => (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()).into_response(),
    }
}

#[derive(serde::Deserialize)]
struct ConfirmBody {
    relation: Option<String>,
}

async fn confirm_pending_link(
    State(state): State<Arc<PluginState>>,
    Path(id): Path<String>,
    Json(body): Json<ConfirmBody>,
) -> impl IntoResponse {
    match state.db.confirm_pending_link(&id, body.relation.as_deref()).await {
        Ok(Some(edge)) => (StatusCode::CREATED, Json(edge)).into_response(),
        Ok(None) => StatusCode::NOT_FOUND.into_response(),
        Err(e) => (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()).into_response(),
    }
}

async fn dismiss_pending_link(
    State(state): State<Arc<PluginState>>,
    Path(id): Path<String>,
) -> impl IntoResponse {
    match state.db.dismiss_pending_link(&id).await {
        Ok(true) => StatusCode::NO_CONTENT.into_response(),
        Ok(false) => StatusCode::NOT_FOUND.into_response(),
        Err(e) => (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()).into_response(),
    }
}

// ── Trash ──

async fn list_trash(State(state): State<Arc<PluginState>>) -> impl IntoResponse {
    match state.db.list_trash().await {
        Ok(nodes) => Json(serde_json::json!({"trash": nodes})).into_response(),
        Err(e) => (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()).into_response(),
    }
}

async fn restore_node(
    State(state): State<Arc<PluginState>>,
    Path(id): Path<String>,
) -> impl IntoResponse {
    match state.db.restore_node(&id).await {
        Ok(true) => StatusCode::NO_CONTENT.into_response(),
        Ok(false) => StatusCode::NOT_FOUND.into_response(),
        Err(e) => (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()).into_response(),
    }
}

async fn purge_node(
    State(state): State<Arc<PluginState>>,
    Path(id): Path<String>,
) -> impl IntoResponse {
    match state.db.purge_node(&id).await {
        Ok(true) => StatusCode::NO_CONTENT.into_response(),
        Ok(false) => StatusCode::NOT_FOUND.into_response(),
        Err(e) => (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()).into_response(),
    }
}

// ── Graph ──

async fn get_graph_data(State(state): State<Arc<PluginState>>) -> impl IntoResponse {
    match state.db.get_graph_data().await {
        Ok(data) => Json(data).into_response(),
        Err(e) => (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()).into_response(),
    }
}
