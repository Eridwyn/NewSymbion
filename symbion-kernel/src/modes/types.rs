// Modes Dynamiques - Types et Structures
// Decision Engine v2

use serde::{Deserialize, Serialize};
use time::OffsetDateTime;
use uuid::Uuid;

/// Thème visuel d'un mode
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ModeTheme {
    /// Couleur principale (ex: #2563eb)
    pub primary: String,
    /// Couleur de fond (ex: #f8fafc)
    pub background: String,
    /// Couleur d'accent (ex: #1e40af)
    pub accent: String,
}

impl Default for ModeTheme {
    fn default() -> Self {
        Self {
            primary: "#6b7280".to_string(),
            background: "#f9fafb".to_string(),
            accent: "#4b5563".to_string(),
        }
    }
}

/// Mode dynamique (système ou personnalisé)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DynamicMode {
    /// Identifiant unique (UUID)
    pub id: String,
    /// Nom d'affichage (ex: "Pro", "Focus", "Détente")
    pub name: String,
    /// Slug URL-friendly (ex: "pro", "focus", "detente")
    pub slug: String,
    /// Icône emoji ou classe (ex: "👔", "🎯", "🏡")
    pub icon: String,
    /// Thème de couleurs
    pub theme: ModeTheme,
    /// Mode système (non supprimable)
    pub is_system: bool,
    /// Date de création
    #[serde(with = "time::serde::rfc3339")]
    pub created_at: OffsetDateTime,
    /// Ordre d'affichage
    pub display_order: u32,
}

impl DynamicMode {
    /// Crée un nouveau mode personnalisé
    pub fn new(name: String, icon: String, theme: ModeTheme) -> Self {
        let slug = name.to_lowercase()
            .chars()
            .map(|c| if c.is_alphanumeric() { c } else { '-' })
            .collect::<String>()
            .trim_matches('-')
            .to_string();

        Self {
            id: Uuid::new_v4().to_string(),
            name,
            slug,
            icon,
            theme,
            is_system: false,
            created_at: OffsetDateTime::now_utc(),
            display_order: 100, // Les modes custom sont à la fin
        }
    }

    /// Crée un mode système (prédéfini)
    pub fn system(id: &str, name: &str, slug: &str, icon: &str, theme: ModeTheme, order: u32) -> Self {
        Self {
            id: id.to_string(),
            name: name.to_string(),
            slug: slug.to_string(),
            icon: icon.to_string(),
            theme,
            is_system: true,
            created_at: OffsetDateTime::now_utc(),
            display_order: order,
        }
    }
}

/// Modes système par défaut
pub fn default_system_modes() -> Vec<DynamicMode> {
    vec![
        DynamicMode::system(
            "mode-pro",
            "Pro",
            "pro",
            "👔",
            ModeTheme {
                primary: "#2563eb".to_string(),
                background: "#f8fafc".to_string(),
                accent: "#1e40af".to_string(),
            },
            0,
        ),
        DynamicMode::system(
            "mode-focus",
            "Focus",
            "focus",
            "🎯",
            ModeTheme {
                primary: "#8b5cf6".to_string(),
                background: "#faf5ff".to_string(),
                accent: "#7c3aed".to_string(),
            },
            1,
        ),
        DynamicMode::system(
            "mode-maison",
            "Maison",
            "maison",
            "🏡",
            ModeTheme {
                primary: "#10b981".to_string(),
                background: "#ecfdf5".to_string(),
                accent: "#059669".to_string(),
            },
            2,
        ),
        DynamicMode::system(
            "mode-veille",
            "Veille",
            "veille",
            "🌱",
            ModeTheme {
                primary: "#6b7280".to_string(),
                background: "#f9fafb".to_string(),
                accent: "#4b5563".to_string(),
            },
            3,
        ),
    ]
}

/// Requête de création de mode
#[derive(Debug, Deserialize)]
pub struct CreateModeRequest {
    pub name: String,
    pub icon: String,
    pub theme: ModeTheme,
}

/// Requête de mise à jour de mode
#[derive(Debug, Deserialize)]
pub struct UpdateModeRequest {
    pub name: Option<String>,
    pub icon: Option<String>,
    pub theme: Option<ModeTheme>,
    pub display_order: Option<u32>,
}
