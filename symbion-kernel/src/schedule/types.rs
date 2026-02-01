// Schedule Types - Structures pour le planning horaire
// Decision Engine v2

use serde::{Deserialize, Serialize};
use time::{OffsetDateTime, Time};
use uuid::Uuid;

/// Règle de planning (créneau horaire → mode)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScheduleRule {
    /// Identifiant unique
    pub id: String,
    /// ID du mode à activer
    pub mode_id: String,
    /// Jours de la semaine (0=Lundi, 6=Dimanche)
    pub days: Vec<u8>,
    /// Heure de début (HH:MM)
    pub start_time: String,
    /// Heure de fin (HH:MM)
    pub end_time: String,
    /// Priorité (plus élevé = prioritaire)
    pub priority: u8,
    /// Règle active ou non
    pub enabled: bool,
    /// Nom descriptif (optionnel)
    pub name: Option<String>,
    /// Date de création
    #[serde(with = "time::serde::rfc3339")]
    pub created_at: OffsetDateTime,
}

impl ScheduleRule {
    /// Crée une nouvelle règle
    pub fn new(
        mode_id: String,
        days: Vec<u8>,
        start_time: String,
        end_time: String,
        priority: u8,
        name: Option<String>,
    ) -> Self {
        Self {
            id: Uuid::new_v4().to_string(),
            mode_id,
            days,
            start_time,
            end_time,
            priority,
            enabled: true,
            name,
            created_at: OffsetDateTime::now_utc(),
        }
    }

    /// Vérifie si la règle est active pour un moment donné
    pub fn is_active_at(&self, time: Time, weekday: u8) -> bool {
        if !self.enabled {
            return false;
        }

        // Vérifier le jour de la semaine
        if !self.days.contains(&weekday) {
            return false;
        }

        // Parser les heures
        let start = self.parse_time(&self.start_time);
        let end = self.parse_time(&self.end_time);

        match (start, end) {
            (Some(s), Some(e)) => {
                if s <= e {
                    // Créneau normal (ex: 8h-18h)
                    time >= s && time < e
                } else {
                    // Créneau qui traverse minuit (ex: 22h-6h)
                    time >= s || time < e
                }
            }
            _ => false,
        }
    }

    fn parse_time(&self, time_str: &str) -> Option<Time> {
        let parts: Vec<&str> = time_str.split(':').collect();
        if parts.len() >= 2 {
            let hour: u8 = parts[0].parse().ok()?;
            let minute: u8 = parts[1].parse().ok()?;
            Time::from_hms(hour, minute, 0).ok()
        } else {
            None
        }
    }
}

/// Planning complet
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Schedule {
    /// Liste des règles
    pub rules: Vec<ScheduleRule>,
    /// Mode par défaut quand aucune règle ne s'applique
    pub default_mode_id: String,
}

impl Default for Schedule {
    fn default() -> Self {
        Self {
            rules: Vec::new(),
            default_mode_id: "mode-veille".to_string(),
        }
    }
}

/// Requête de création de règle
#[derive(Debug, Deserialize)]
pub struct CreateRuleRequest {
    pub mode_id: String,
    pub days: Vec<u8>,
    pub start_time: String,
    pub end_time: String,
    #[serde(default)]
    pub priority: u8,
    pub name: Option<String>,
}

/// Requête de mise à jour de règle
#[derive(Debug, Deserialize)]
pub struct UpdateRuleRequest {
    pub mode_id: Option<String>,
    pub days: Option<Vec<u8>>,
    pub start_time: Option<String>,
    pub end_time: Option<String>,
    pub priority: Option<u8>,
    pub enabled: Option<bool>,
    pub name: Option<String>,
}

/// Requête pour changer le mode par défaut
#[derive(Debug, Deserialize)]
pub struct UpdateDefaultModeRequest {
    pub default_mode_id: String,
}

/// Réponse avec le mode actif actuel
#[derive(Debug, Serialize)]
pub struct CurrentScheduleInfo {
    pub active_rule: Option<ScheduleRule>,
    pub current_mode_id: String,
    pub is_default: bool,
}
