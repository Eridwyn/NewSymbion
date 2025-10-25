use serde::{Deserialize, Serialize};
use std::sync::{Arc, Mutex};
use crate::agents::{Agent, SharedAgentRegistry};
use rumqttc::AsyncClient;
use time::OffsetDateTime;

/// Mode contextuel de Symbion
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Mode {
    /// Mode professionnel (bureau, travail)
    Cravate,
    /// Mode domestique (maison, détente)
    Intime,
    /// Mode surveillance (économie énergie, maintenance)
    Neutre,
}

impl Mode {
    pub fn icon(&self) -> &'static str {
        match self {
            Mode::Cravate => "👔",
            Mode::Intime => "🏡",
            Mode::Neutre => "🌱",
        }
    }

    pub fn name(&self) -> &'static str {
        match self {
            Mode::Cravate => "Focus Pro",
            Mode::Intime => "Maison",
            Mode::Neutre => "Veille",
        }
    }

    pub fn theme(&self) -> Theme {
        match self {
            Mode::Cravate => Theme {
                primary: "#2563eb".to_string(),
                bg: "#f8fafc".to_string(),
                accent: "#1e40af".to_string(),
            },
            Mode::Intime => Theme {
                primary: "#10b981".to_string(),
                bg: "#ecfdf5".to_string(),
                accent: "#059669".to_string(),
            },
            Mode::Neutre => Theme {
                primary: "#6b7280".to_string(),
                bg: "#f9fafb".to_string(),
                accent: "#4b5563".to_string(),
            },
        }
    }
}

/// Thème visuel associé à un mode
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Theme {
    pub primary: String,
    pub bg: String,
    pub accent: String,
}

/// État du contexte actuel
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContextState {
    pub mode: Mode,
    #[serde(with = "time::serde::rfc3339")]
    pub changed_at: OffsetDateTime,
    pub reason: String,
    pub confidence: f32,
    pub theme: Theme,
    pub manual_override: Option<ManualOverride>,
}

/// Override manuel temporaire du mode
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ManualOverride {
    pub mode: Mode,
    #[serde(with = "time::serde::rfc3339")]
    pub until: OffsetDateTime,
    pub reason: String,
}

/// Moteur de détection contextuelle
pub struct ContextEngine {
    state: Arc<Mutex<ContextState>>,
}

impl ContextEngine {
    pub fn new() -> Self {
        let initial_state = ContextState {
            mode: Mode::Neutre,
            changed_at: OffsetDateTime::now_utc(),
            reason: "Initialisation système".to_string(),
            confidence: 1.0,
            theme: Mode::Neutre.theme(),
            manual_override: None,
        };

        Self {
            state: Arc::new(Mutex::new(initial_state)),
        }
    }

    /// Détecte le mode contextuel basé sur les données agent
    pub fn detect_mode(&self, agents: &[Agent]) -> Option<(Mode, String, f32)> {
        let now = OffsetDateTime::now_utc();
        let hour = now.hour();
        let weekday = now.weekday();

        // Pas d'agents = mode neutre
        if agents.is_empty() {
            return Some((Mode::Neutre, "Aucun agent actif".to_string(), 1.0));
        }

        // Règle 1: Nuit (23h-7h) = Mode Neutre
        if hour >= 23 || hour < 7 {
            return Some((
                Mode::Neutre,
                format!("Heure nocturne ({}h)", hour),
                0.95,
            ));
        }

        // Règle 2: Week-end (samedi/dimanche) = Mode Intime
        use time::Weekday;
        if weekday == Weekday::Saturday || weekday == Weekday::Sunday {
            return Some((
                Mode::Intime,
                format!("Week-end ({})", match weekday {
                    Weekday::Saturday => "samedi",
                    Weekday::Sunday => "dimanche",
                    _ => "week-end"
                }),
                0.90,
            ));
        }

        // Règle 3: Journée en semaine = Mode Intime par défaut
        // (utiliser override manuel pour passer en mode Cravate si besoin)
        Some((
            Mode::Intime,
            format!("Journée à la maison ({}h)", hour),
            0.80,
        ))
    }

    /// Met à jour le contexte si le mode a changé
    pub fn update(&self, agents: &[Agent]) -> Option<ContextState> {
        let mut state = self.state.lock().ok()?;

        // Vérifier override manuel
        if let Some(ref override_data) = state.manual_override {
            if OffsetDateTime::now_utc() < override_data.until {
                // Override encore valide
                return None;
            } else {
                // Override expiré
                println!("[context] Override manuel expiré, retour détection automatique");
                state.manual_override = None;
            }
        }

        // Détecter nouveau mode
        let (new_mode, reason, confidence) = self.detect_mode(agents)?;

        // Si le mode a changé, mettre à jour
        if new_mode != state.mode {
            println!("[context] Changement de mode: {:?} → {:?} (raison: {})",
                state.mode, new_mode, reason);

            state.mode = new_mode;
            state.changed_at = OffsetDateTime::now_utc();
            state.reason = reason;
            state.confidence = confidence;
            state.theme = new_mode.theme();

            Some(state.clone())
        } else {
            None
        }
    }

    /// Récupère l'état actuel
    pub fn get_state(&self) -> Option<ContextState> {
        self.state.lock().ok().map(|s| s.clone())
    }

    /// Force un mode manuellement (override temporaire)
    pub fn set_override(&self, mode: Mode, duration_minutes: i64, reason: String) -> Option<ContextState> {
        let mut state = self.state.lock().ok()?;

        let until = OffsetDateTime::now_utc() + time::Duration::minutes(duration_minutes);

        println!("[context] Override manuel: {:?} pendant {} minutes (raison: {})",
            mode, duration_minutes, reason);

        state.mode = mode;
        state.changed_at = OffsetDateTime::now_utc();
        state.reason = format!("Override manuel: {}", reason);
        state.confidence = 1.0;
        state.theme = mode.theme();
        state.manual_override = Some(ManualOverride {
            mode,
            until,
            reason,
        });

        Some(state.clone())
    }

    /// Annule l'override manuel
    pub fn clear_override(&self, agents: &[Agent]) -> Option<ContextState> {
        let mut state = self.state.lock().ok()?;

        if state.manual_override.is_some() {
            println!("[context] Annulation override manuel");
            state.manual_override = None;

            // Re-détecter le mode automatiquement
            if let Some((new_mode, reason, confidence)) = self.detect_mode(agents) {
                state.mode = new_mode;
                state.changed_at = OffsetDateTime::now_utc();
                state.reason = reason;
                state.confidence = confidence;
                state.theme = new_mode.theme();
            }

            Some(state.clone())
        } else {
            None
        }
    }

    /// Démarre la tâche périodique de détection contextuelle
    /// Vérifie le contexte toutes les 30 secondes et publie les changements sur MQTT
    pub fn spawn_context_monitor(
        engine: Arc<ContextEngine>,
        agents: SharedAgentRegistry,
        mqtt_client: AsyncClient,
    ) {
        tokio::spawn(async move {
            let mut interval = tokio::time::interval(tokio::time::Duration::from_secs(30));

            loop {
                interval.tick().await;

                // Récupérer la liste des agents
                let agents_map = agents.list_agents().await;
                let agents_list: Vec<Agent> = agents_map.values().cloned().collect();

                // Mettre à jour le contexte
                if let Some(new_state) = engine.update(&agents_list) {
                    // Un changement de mode est détecté, publier sur MQTT
                    let payload = match serde_json::to_string(&new_state) {
                        Ok(json) => json,
                        Err(e) => {
                            eprintln!("[context] failed to serialize state: {}", e);
                            continue;
                        }
                    };

                    println!("[context] Publishing mode change: {:?} ({})",
                        new_state.mode, new_state.reason);

                    if let Err(e) = mqtt_client.publish(
                        "symbion/context/mode",
                        rumqttc::QoS::AtLeastOnce,
                        false,
                        payload,
                    ).await {
                        eprintln!("[context] failed to publish mode change: {}", e);
                    }
                }
            }
        });

        println!("[context] Context monitor started (checks every 30s)");
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_night_mode() {
        let engine = ContextEngine::new();
        let agents = vec![]; // Pas d'agents

        let (mode, reason, _) = engine.detect_mode(&agents).unwrap();
        assert_eq!(mode, Mode::Neutre);
        assert!(reason.contains("Aucun agent"));
    }

    #[test]
    fn test_mode_icons() {
        assert_eq!(Mode::Cravate.icon(), "👔");
        assert_eq!(Mode::Intime.icon(), "🏡");
        assert_eq!(Mode::Neutre.icon(), "🌱");
    }

    #[test]
    fn test_mode_themes() {
        let theme = Mode::Cravate.theme();
        assert_eq!(theme.primary, "#2563eb");

        let theme = Mode::Intime.theme();
        assert_eq!(theme.primary, "#10b981");
    }
}
