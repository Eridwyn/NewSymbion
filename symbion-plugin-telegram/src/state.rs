use crate::config::Config;
use dashmap::DashMap;
use rumqttc::AsyncClient;
use std::sync::Arc;
use std::time::Instant;
use teloxide::prelude::*;
use tokio_util::sync::CancellationToken;

const MAX_HISTORY: usize = 50;

/// Per-user Claude session state
#[derive(Debug, Clone)]
pub struct UserSession {
    pub session_id: Option<String>,
    pub model: String,
    pub effort: String,
}

impl Default for UserSession {
    fn default() -> Self {
        Self {
            session_id: None,
            model: "sonnet".into(),
            effort: "low".into(),
        }
    }
}

/// History entry
#[derive(Debug, Clone)]
pub struct HistoryEntry {
    pub prompt: String,
    pub success: bool,
    pub model: String,
}

/// Cached notification for interactive callbacks
#[derive(Debug, Clone)]
pub struct CachedNotification {
    pub data: serde_json::Value,
    pub created_at: Instant,
}

/// Shared application state
#[derive(Clone)]
pub struct AppState {
    pub config: Arc<Config>,
    pub mqtt_client: AsyncClient,
    pub bot: Bot,
    pub active_tasks: Arc<DashMap<i64, CancellationToken>>,
    pub user_sessions: Arc<DashMap<i64, UserSession>>,
    pub start_time: Instant,
    /// Per-user interaction history
    history: Arc<DashMap<i64, Vec<HistoryEntry>>>,
    /// Cached notifications for interactive callbacks (notif_id → data)
    pub pending_notifs: Arc<DashMap<String, CachedNotification>>,
}

impl AppState {
    pub fn new(config: Config, mqtt_client: AsyncClient, bot: Bot) -> Self {
        Self {
            config: Arc::new(config),
            mqtt_client,
            bot,
            active_tasks: Arc::new(DashMap::new()),
            user_sessions: Arc::new(DashMap::new()),
            start_time: Instant::now(),
            history: Arc::new(DashMap::new()),
            pending_notifs: Arc::new(DashMap::new()),
        }
    }

    pub fn get_session(&self, user_id: i64) -> UserSession {
        self.user_sessions
            .get(&user_id)
            .map(|s| s.clone())
            .unwrap_or_default()
    }

    pub fn update_session<F: FnOnce(&mut UserSession)>(&self, user_id: i64, f: F) {
        let mut entry = self.user_sessions.entry(user_id).or_default();
        f(entry.value_mut());
    }

    pub fn is_busy(&self, user_id: i64) -> bool {
        self.active_tasks.contains_key(&user_id)
    }

    pub fn add_history(&self, user_id: i64, prompt: &str, success: bool, model: &str) {
        let mut entry = self.history.entry(user_id).or_insert_with(Vec::new);
        entry.push(HistoryEntry {
            prompt: prompt.to_string(),
            success,
            model: model.to_string(),
        });
        // Keep only last N entries
        if entry.len() > MAX_HISTORY {
            let drain_count = entry.len() - MAX_HISTORY;
            entry.drain(..drain_count);
        }
    }

    pub fn get_history(&self, user_id: i64) -> Vec<HistoryEntry> {
        self.history
            .get(&user_id)
            .map(|h| h.clone())
            .unwrap_or_default()
    }

    /// Store notification for interactive callbacks (auto-expire after 1h)
    pub fn cache_notification(&self, notif_id: &str, data: serde_json::Value) {
        // Cleanup expired entries (>1h)
        let cutoff = Instant::now() - std::time::Duration::from_secs(3600);
        self.pending_notifs.retain(|_, v| v.created_at > cutoff);

        self.pending_notifs.insert(notif_id.to_string(), CachedNotification {
            data,
            created_at: Instant::now(),
        });
    }

    /// Get cached notification data
    pub fn get_cached_notif(&self, notif_id: &str) -> Option<CachedNotification> {
        self.pending_notifs.get(notif_id).map(|v| v.clone())
    }

    /// Remove cached notification after action
    pub fn remove_cached_notif(&self, notif_id: &str) {
        self.pending_notifs.remove(notif_id);
    }
}
