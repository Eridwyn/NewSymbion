use crate::config::Config;
use crate::prefs::NotifPrefs;
use dashmap::DashMap;
use rumqttc::AsyncClient;
use std::path::PathBuf;
use std::sync::Arc;
use std::time::Instant;
use teloxide::prelude::*;
use tokio::sync::RwLock;
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
    /// Préférences notif (toggles par catégorie)
    pub prefs: Arc<RwLock<NotifPrefs>>,
    /// Chemin disque des prefs (utilisé par PUT /config pour sauver)
    pub prefs_path: Arc<PathBuf>,
}

impl AppState {
    pub fn new(
        config: Config,
        mqtt_client: AsyncClient,
        bot: Bot,
        prefs: NotifPrefs,
        prefs_path: PathBuf,
    ) -> Self {
        Self {
            config: Arc::new(config),
            mqtt_client,
            bot,
            active_tasks: Arc::new(DashMap::new()),
            user_sessions: Arc::new(DashMap::new()),
            start_time: Instant::now(),
            history: Arc::new(DashMap::new()),
            pending_notifs: Arc::new(DashMap::new()),
            prefs: Arc::new(RwLock::new(prefs)),
            prefs_path: Arc::new(prefs_path),
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::Config;
    use rumqttc::MqttOptions;
    use std::collections::HashSet;
    use std::path::PathBuf;

    fn make_state() -> AppState {
        let config = Config {
            telegram_bot_token: "test-token".into(),
            allowed_user_ids: HashSet::from([42]),
            claude_path: PathBuf::from("/usr/local/bin/claude"),
            claude_timeout_secs: 600,
            claude_workdir: PathBuf::from("/tmp"),
            mqtt_broker_host: "localhost".into(),
            mqtt_broker_port: 1883,
            socket_path: PathBuf::from("/tmp/test.sock"),
            kernel_api_key: "key".into(),
        };
        let opts = MqttOptions::new("test-client", "127.0.0.1", 1883);
        let (mqtt_client, _eventloop) = AsyncClient::new(opts, 10);
        let bot = Bot::new("123:dummy");
        AppState::new(
            config,
            mqtt_client,
            bot,
            NotifPrefs::default(),
            PathBuf::from("/tmp/test_prefs.json"),
        )
    }

    #[test]
    fn user_session_default_values() {
        let s = UserSession::default();
        assert!(s.session_id.is_none());
        assert_eq!(s.model, "sonnet");
        assert_eq!(s.effort, "low");
    }

    #[test]
    fn get_session_returns_default_when_absent() {
        let st = make_state();
        let s = st.get_session(999);
        assert!(s.session_id.is_none());
        assert_eq!(s.model, "sonnet");
    }

    #[test]
    fn update_session_persists_changes() {
        let st = make_state();
        st.update_session(42, |s| {
            s.model = "opus".into();
            s.session_id = Some("sess-1".into());
        });
        let s = st.get_session(42);
        assert_eq!(s.model, "opus");
        assert_eq!(s.session_id.as_deref(), Some("sess-1"));
    }

    #[test]
    fn is_busy_false_when_no_active_task() {
        let st = make_state();
        assert!(!st.is_busy(42));
    }

    #[test]
    fn is_busy_true_when_token_inserted() {
        let st = make_state();
        st.active_tasks.insert(42, CancellationToken::new());
        assert!(st.is_busy(42));
        assert!(!st.is_busy(43));
    }

    #[test]
    fn add_history_keeps_entries() {
        let st = make_state();
        st.add_history(42, "first prompt", true, "sonnet");
        st.add_history(42, "second prompt", false, "opus");
        let h = st.get_history(42);
        assert_eq!(h.len(), 2);
        assert_eq!(h[0].prompt, "first prompt");
        assert_eq!(h[0].success, true);
        assert_eq!(h[1].model, "opus");
        assert_eq!(h[1].success, false);
    }

    #[test]
    fn add_history_truncates_to_max() {
        let st = make_state();
        for i in 0..(MAX_HISTORY + 10) {
            st.add_history(42, &format!("p{}", i), true, "sonnet");
        }
        let h = st.get_history(42);
        assert_eq!(h.len(), MAX_HISTORY);
        // Les anciennes entrées ont été drainées : la première doit être p10
        assert_eq!(h[0].prompt, "p10");
        assert_eq!(h[MAX_HISTORY - 1].prompt, format!("p{}", MAX_HISTORY + 9));
    }

    #[test]
    fn get_history_empty_for_unknown_user() {
        let st = make_state();
        assert!(st.get_history(999).is_empty());
    }

    #[test]
    fn cache_and_retrieve_notification() {
        let st = make_state();
        let data = serde_json::json!({"title": "Test", "body": "Body"});
        st.cache_notification("notif-1", data.clone());
        let cached = st.get_cached_notif("notif-1").expect("must exist");
        assert_eq!(cached.data, data);
    }

    #[test]
    fn remove_cached_notif_clears_entry() {
        let st = make_state();
        st.cache_notification("notif-1", serde_json::json!({}));
        assert!(st.get_cached_notif("notif-1").is_some());
        st.remove_cached_notif("notif-1");
        assert!(st.get_cached_notif("notif-1").is_none());
    }

    #[test]
    fn get_cached_notif_none_when_absent() {
        let st = make_state();
        assert!(st.get_cached_notif("nope").is_none());
    }
}
