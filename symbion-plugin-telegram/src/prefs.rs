//! Préférences de notification Telegram : toggles par catégorie.
//!
//! Le kernel publie toutes les notifs sur `symbion/notifications/sent@v1`.
//! Le plugin telegram filtre selon ces prefs avant de forward.
//!
//! Règle : P0 (urgent) est TOUJOURS envoyé, peu importe les toggles.
//!
//! Persistance : un fichier JSON local (volatile-friendly), default
//! `data/telegram_prefs.json` dans le WORKDIR du plugin.

use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

/// Catégories de notifications proposées à l'utilisateur.
/// Ordre = ordre d'affichage dans la PWA.
pub const CATEGORIES: &[(&str, &str, &str)] = &[
    // (id, label affiché, icône emoji)
    ("maison", "Maison", "🏠"),
    ("systeme", "Système", "⚙️"),
    ("automations", "Automations", "🤖"),
    ("cafe", "Café", "☕"),
    ("intelligence", "Intelligence", "💡"),
];

/// Mappe le champ `source` d'une notification vers un id de catégorie.
/// Toute source inconnue tombe dans "systeme" (catch-all).
pub fn categorize(source: &str) -> &'static str {
    let s = source.to_lowercase();
    if s.contains("coffee") {
        "cafe"
    } else if s.contains("sensor") || s.contains("environment") {
        "maison"
    } else if s.contains("automation") {
        "automations"
    } else if s.contains("intelligence") || s.contains("context") {
        "intelligence"
    } else {
        "systeme"
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NotifPrefs {
    /// Map catégorie_id → enabled. Si une catégorie est absente : default true.
    pub categories: BTreeMap<String, bool>,
    /// RFC3339 dernière modif.
    #[serde(default)]
    pub updated_at: String,
}

impl Default for NotifPrefs {
    fn default() -> Self {
        let mut categories = BTreeMap::new();
        for (id, _, _) in CATEGORIES {
            categories.insert((*id).to_string(), true);
        }
        Self {
            categories,
            updated_at: now_rfc3339(),
        }
    }
}

impl NotifPrefs {
    /// True si la catégorie est activée. Une catégorie absente = activée par défaut.
    pub fn is_enabled(&self, category: &str) -> bool {
        self.categories.get(category).copied().unwrap_or(true)
    }

    /// Renvoie la liste complète avec les flags actuels (utile pour réponse API).
    pub fn full_view(&self) -> Vec<CategoryView> {
        CATEGORIES
            .iter()
            .map(|(id, label, icon)| CategoryView {
                id: (*id).into(),
                label: (*label).into(),
                icon: (*icon).into(),
                enabled: self.is_enabled(id),
            })
            .collect()
    }
}

#[derive(Debug, Serialize)]
pub struct CategoryView {
    pub id: String,
    pub label: String,
    pub icon: String,
    pub enabled: bool,
}

#[derive(Debug, Deserialize)]
pub struct PrefsUpdate {
    /// Map des nouveaux flags (clés inconnues ignorées).
    pub categories: BTreeMap<String, bool>,
}

/// Charge depuis le fichier. En cas d'erreur (absent, JSON corrompu), retourne Default.
pub fn load(path: &Path) -> NotifPrefs {
    match std::fs::read_to_string(path) {
        Ok(content) => match serde_json::from_str::<NotifPrefs>(&content) {
            Ok(p) => p,
            Err(e) => {
                eprintln!("[telegram-prefs] JSON invalide à {:?}: {} — fallback default", path, e);
                NotifPrefs::default()
            }
        },
        Err(_) => NotifPrefs::default(),
    }
}

/// Sauve atomiquement (write to .tmp puis rename).
pub fn save(path: &Path, prefs: &NotifPrefs) -> Result<(), String> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)
            .map_err(|e| format!("create dir {:?}: {}", parent, e))?;
    }
    let tmp = path.with_extension("json.tmp");
    let json = serde_json::to_string_pretty(prefs)
        .map_err(|e| format!("serialize: {}", e))?;
    std::fs::write(&tmp, json).map_err(|e| format!("write tmp: {}", e))?;
    std::fs::rename(&tmp, path).map_err(|e| format!("rename: {}", e))?;
    Ok(())
}

/// Default location si pas d'override env.
pub fn default_path(workdir: &Path) -> PathBuf {
    workdir.join("data").join("telegram_prefs.json")
}

fn now_rfc3339() -> String {
    use time::format_description::well_known::Rfc3339;
    use time::OffsetDateTime;
    OffsetDateTime::now_utc().format(&Rfc3339).unwrap_or_else(|_| "unknown".into())
}

// ── HTTP handlers (mounted by main.rs on Unix socket) ──

use axum::extract::State;
use axum::http::StatusCode;
use axum::response::Json as AxumJson;
use serde_json::{json, Value};

use crate::state::AppState;

/// GET /config — état de la config visible côté PWA.
/// On expose les prefs + un status synthétique. **Jamais le token bot.**
pub async fn get_config_handler(State(state): State<AppState>) -> AxumJson<Value> {
    let prefs = state.prefs.read().await;
    AxumJson(json!({
        "spec_version": "1.0",
        "status": {
            "uptime_seconds": state.start_time.elapsed().as_secs(),
            "allowed_users_count": state.config.allowed_user_ids.len(),
            "active_sessions": state.user_sessions.len(),
        },
        "categories": prefs.full_view(),
        "updated_at": prefs.updated_at,
    }))
}

/// PUT /config — met à jour les prefs (categories on/off).
/// Body : `{ "categories": { "cafe": false, "maison": true } }` — clés inconnues ignorées.
pub async fn put_config_handler(
    State(state): State<AppState>,
    AxumJson(req): AxumJson<PrefsUpdate>,
) -> Result<AxumJson<Value>, (StatusCode, AxumJson<Value>)> {
    let known: std::collections::HashSet<&str> = CATEGORIES.iter().map(|(id, _, _)| *id).collect();

    let mut prefs = state.prefs.write().await;
    let mut updated_keys: Vec<String> = Vec::new();
    for (k, v) in req.categories {
        if known.contains(k.as_str()) {
            prefs.categories.insert(k.clone(), v);
            updated_keys.push(k);
        }
    }
    prefs.updated_at = now_rfc3339();

    let snapshot = prefs.clone();
    drop(prefs); // release lock before IO

    if let Err(e) = save(&state.prefs_path, &snapshot) {
        return Err((
            StatusCode::INTERNAL_SERVER_ERROR,
            AxumJson(json!({ "error": format!("save failed: {}", e) })),
        ));
    }

    Ok(AxumJson(json!({
        "spec_version": "1.0",
        "updated": updated_keys,
        "categories": snapshot.full_view(),
    })))
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn categorize_coffee() {
        assert_eq!(categorize("plugin.coffee"), "cafe");
        assert_eq!(categorize("auto_coffee_water_low"), "cafe");
    }

    #[test]
    fn categorize_sensors_environment() {
        assert_eq!(categorize("plugin.sensors"), "maison");
        assert_eq!(categorize("environment_alert"), "maison");
        assert_eq!(categorize("sensor.kitchen"), "maison");
    }

    #[test]
    fn categorize_automation_sources() {
        assert_eq!(categorize("automation"), "automations");
        assert_eq!(categorize("automation-validation"), "automations");
    }

    #[test]
    fn categorize_intelligence_sources() {
        assert_eq!(categorize("context_intelligence"), "intelligence");
        assert_eq!(categorize("context"), "intelligence");
    }

    #[test]
    fn categorize_fallback_is_systeme() {
        assert_eq!(categorize("unknown_source"), "systeme");
        assert_eq!(categorize("plugin.ssl"), "systeme");
        assert_eq!(categorize(""), "systeme");
    }

    #[test]
    fn default_prefs_all_enabled() {
        let p = NotifPrefs::default();
        for (id, _, _) in CATEGORIES {
            assert!(p.is_enabled(id), "category {} should default enabled", id);
        }
    }

    #[test]
    fn unknown_category_defaults_true() {
        let p = NotifPrefs::default();
        assert!(p.is_enabled("unknown_cat"));
    }

    #[test]
    fn full_view_has_all_categories_with_metadata() {
        let p = NotifPrefs::default();
        let v = p.full_view();
        assert_eq!(v.len(), CATEGORIES.len());
        let ids: Vec<&str> = v.iter().map(|c| c.id.as_str()).collect();
        for (id, _, _) in CATEGORIES {
            assert!(ids.contains(id));
        }
    }

    #[test]
    fn save_then_load_roundtrip() {
        let tmp = TempDir::new().unwrap();
        let path = tmp.path().join("prefs.json");
        let mut p = NotifPrefs::default();
        p.categories.insert("cafe".into(), false);
        p.categories.insert("maison".into(), true);
        save(&path, &p).unwrap();

        let loaded = load(&path);
        assert!(!loaded.is_enabled("cafe"));
        assert!(loaded.is_enabled("maison"));
        assert!(loaded.is_enabled("systeme")); // pas modifié
    }

    #[test]
    fn load_missing_file_returns_default() {
        let tmp = TempDir::new().unwrap();
        let p = load(&tmp.path().join("does_not_exist.json"));
        assert!(p.is_enabled("cafe"));
    }

    #[test]
    fn load_corrupted_json_returns_default() {
        let tmp = TempDir::new().unwrap();
        let path = tmp.path().join("bad.json");
        std::fs::write(&path, "{ not valid json").unwrap();
        let p = load(&path);
        assert!(p.is_enabled("cafe"));
    }
}
