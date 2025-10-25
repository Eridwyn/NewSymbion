use serde::{Deserialize, Serialize};
use std::sync::{Arc, Mutex};
use std::path::PathBuf;
use std::collections::HashMap;
use crate::agents::{Agent, SharedAgentRegistry};
use rumqttc::AsyncClient;
use time::OffsetDateTime;

/// Mode contextuel de Symbion
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
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

/// Entrée d'historique de changement de mode
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModeHistoryEntry {
    pub mode: Mode,
    #[serde(with = "time::serde::rfc3339")]
    pub timestamp: OffsetDateTime,
    pub reason: String,
    pub was_manual: bool,
}

/// Pattern détecté dans les changements de mode
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DetectedPattern {
    pub mode: Mode,
    pub day_of_week: u8, // 1=lundi, 7=dimanche
    pub hour: u8,
    pub occurrences: u32,
    pub confidence: f32,
    pub last_seen: String,
}

/// Statistiques par mode
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModeStats {
    pub mode: Mode,
    pub total_duration_minutes: i64,
    pub entry_count: u32,
    pub percentage: f32,
}

/// Métriques de productivité par mode
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProductivityMetrics {
    pub mode: Mode,
    pub notes_created: u32,
    pub sessions_count: u32,
    pub avg_session_duration_minutes: i64,
}

/// Moteur de détection contextuelle
pub struct ContextEngine {
    state: Arc<Mutex<ContextState>>,
    history: Arc<Mutex<Vec<ModeHistoryEntry>>>,
    history_path: PathBuf,
}

impl ContextEngine {
    pub fn new() -> Self {
        let history_path = PathBuf::from("context-history.json");

        // Charger l'historique depuis le fichier s'il existe
        let history = if history_path.exists() {
            match std::fs::read_to_string(&history_path) {
                Ok(content) => {
                    match serde_json::from_str::<Vec<ModeHistoryEntry>>(&content) {
                        Ok(h) => {
                            println!("[context] Loaded {} history entries from file", h.len());
                            h
                        }
                        Err(e) => {
                            eprintln!("[context] Failed to parse history file: {}", e);
                            Vec::new()
                        }
                    }
                }
                Err(e) => {
                    eprintln!("[context] Failed to read history file: {}", e);
                    Vec::new()
                }
            }
        } else {
            println!("[context] No history file found, starting fresh");
            Vec::new()
        };

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
            history: Arc::new(Mutex::new(history)),
            history_path,
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
            state.reason = reason.clone();
            state.confidence = confidence;
            state.theme = new_mode.theme();

            let result = state.clone();

            // Libérer le lock avant d'appeler add_to_history
            drop(state);

            // Ajouter à l'historique
            self.add_to_history(new_mode, reason, false);

            Some(result)
        } else {
            None
        }
    }

    /// Récupère l'état actuel
    pub fn get_state(&self) -> Option<ContextState> {
        self.state.lock().ok().map(|s| s.clone())
    }

    /// Ajoute une entrée à l'historique et sauvegarde
    fn add_to_history(&self, mode: Mode, reason: String, was_manual: bool) {
        if let Ok(mut history) = self.history.lock() {
            let entry = ModeHistoryEntry {
                mode,
                timestamp: OffsetDateTime::now_utc(),
                reason,
                was_manual,
            };

            history.push(entry);

            // Sauvegarder l'historique sur disque
            if let Ok(json) = serde_json::to_string_pretty(&*history) {
                if let Err(e) = std::fs::write(&self.history_path, json) {
                    eprintln!("[context] Failed to save history: {}", e);
                }
            }
        }
    }

    /// Récupère l'historique complet
    pub fn get_history(&self) -> Vec<ModeHistoryEntry> {
        self.history.lock().ok().map(|h| h.clone()).unwrap_or_default()
    }

    /// Force un mode manuellement (override temporaire)
    pub fn set_override(&self, mode: Mode, duration_minutes: i64, reason: String) -> Option<ContextState> {
        let mut state = self.state.lock().ok()?;

        let until = OffsetDateTime::now_utc() + time::Duration::minutes(duration_minutes);

        println!("[context] Override manuel: {:?} pendant {} minutes (raison: {})",
            mode, duration_minutes, reason);

        state.mode = mode;
        state.changed_at = OffsetDateTime::now_utc();
        let full_reason = format!("Override manuel: {}", reason);
        state.reason = full_reason.clone();
        state.confidence = 1.0;
        state.theme = mode.theme();
        state.manual_override = Some(ManualOverride {
            mode,
            until,
            reason,
        });

        let result = state.clone();

        // Libérer le lock avant d'appeler add_to_history
        drop(state);

        // Ajouter à l'historique (marqué comme manuel)
        self.add_to_history(mode, full_reason, true);

        Some(result)
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

    /// Calcule les statistiques d'utilisation par mode
    pub fn calculate_stats(&self) -> Vec<ModeStats> {
        let history = self.get_history();
        if history.is_empty() {
            return Vec::new();
        }

        // Calculer le temps total par mode
        let mut mode_durations: HashMap<Mode, i64> = HashMap::new();
        let mut mode_counts: HashMap<Mode, u32> = HashMap::new();

        // Parcourir l'historique par paires pour calculer les durées
        for i in 0..history.len() {
            let entry = &history[i];
            let next_timestamp = if i + 1 < history.len() {
                history[i + 1].timestamp
            } else {
                OffsetDateTime::now_utc()
            };

            let duration = (next_timestamp - entry.timestamp).whole_minutes();
            *mode_durations.entry(entry.mode).or_insert(0) += duration;
            *mode_counts.entry(entry.mode).or_insert(0) += 1;
        }

        let total_duration: i64 = mode_durations.values().sum();

        // Créer les stats pour chaque mode
        let mut stats: Vec<ModeStats> = mode_durations
            .iter()
            .map(|(mode, duration)| ModeStats {
                mode: *mode,
                total_duration_minutes: *duration,
                entry_count: *mode_counts.get(mode).unwrap_or(&0),
                percentage: if total_duration > 0 {
                    (*duration as f32 / total_duration as f32) * 100.0
                } else {
                    0.0
                },
            })
            .collect();

        stats.sort_by(|a, b| b.total_duration_minutes.cmp(&a.total_duration_minutes));
        stats
    }

    /// Détecte les patterns récurrents dans les changements de mode manuels
    pub fn detect_patterns(&self) -> Vec<DetectedPattern> {
        let history = self.get_history();

        // Ne considérer que les changements manuels
        let manual_changes: Vec<&ModeHistoryEntry> = history
            .iter()
            .filter(|entry| entry.was_manual)
            .collect();

        if manual_changes.len() < 2 {
            return Vec::new();
        }

        // Grouper par (mode, jour de la semaine, heure)
        let mut pattern_map: HashMap<(Mode, u8, u8), Vec<String>> = HashMap::new();

        for entry in manual_changes {
            let weekday = entry.timestamp.weekday().number_from_monday();
            let hour = entry.timestamp.hour();
            let key = (entry.mode, weekday, hour);

            pattern_map
                .entry(key)
                .or_insert_with(Vec::new)
                .push(entry.timestamp.to_string());
        }

        // Créer les patterns détectés (au moins 2 occurrences)
        let mut patterns: Vec<DetectedPattern> = pattern_map
            .iter()
            .filter(|(_, occurrences)| occurrences.len() >= 2)
            .map(|((mode, day, hour), timestamps)| {
                let count = timestamps.len() as u32;
                // Confiance basée sur le nombre d'occurrences (max 1.0)
                let confidence = (count as f32 / 10.0).min(1.0);

                DetectedPattern {
                    mode: *mode,
                    day_of_week: *day,
                    hour: *hour,
                    occurrences: count,
                    confidence,
                    last_seen: timestamps.last().cloned().unwrap_or_default(),
                }
            })
            .collect();

        // Trier par confiance décroissante
        patterns.sort_by(|a, b| {
            b.confidence
                .partial_cmp(&a.confidence)
                .unwrap_or(std::cmp::Ordering::Equal)
        });

        patterns
    }

    /// Calcule les métriques de productivité par mode
    pub fn calculate_productivity(&self) -> Vec<ProductivityMetrics> {
        let history = self.get_history();
        if history.is_empty() {
            return Vec::new();
        }

        // Lire les notes depuis le fichier JSON
        let notes_path = PathBuf::from("notes.json");
        let notes: Vec<serde_json::Value> = if notes_path.exists() {
            match std::fs::read_to_string(&notes_path) {
                Ok(content) => serde_json::from_str(&content).unwrap_or_default(),
                Err(_) => Vec::new(),
            }
        } else {
            Vec::new()
        };

        // Mapper contexte → mode (case-insensitive)
        let context_to_mode = |context: &str| -> Option<Mode> {
            let ctx = context.to_lowercase();
            if ctx.contains("cravate") || ctx.contains("bureau") || ctx.contains("travail") || ctx.contains("pro") {
                Some(Mode::Cravate)
            } else if ctx.contains("intime") || ctx.contains("maison") || ctx.contains("home") {
                Some(Mode::Intime)
            } else if ctx.contains("neutre") || ctx.contains("veille") || ctx.contains("neutral") {
                Some(Mode::Neutre)
            } else {
                None
            }
        };

        // Compter les notes par mode
        let mut notes_by_mode: HashMap<Mode, u32> = HashMap::new();
        for note in &notes {
            if let Some(context) = note.get("data")
                .and_then(|d| d.get("context"))
                .and_then(|c| c.as_str()) {
                if let Some(mode) = context_to_mode(context) {
                    *notes_by_mode.entry(mode).or_insert(0) += 1;
                }
            }
        }

        // Compter les sessions (changements de mode) par mode
        let mut sessions_by_mode: HashMap<Mode, u32> = HashMap::new();
        for entry in &history {
            *sessions_by_mode.entry(entry.mode).or_insert(0) += 1;
        }

        // Calculer durée moyenne par session
        let mut mode_durations: HashMap<Mode, i64> = HashMap::new();
        for i in 0..history.len() {
            let entry = &history[i];
            let next_timestamp = if i + 1 < history.len() {
                history[i + 1].timestamp
            } else {
                OffsetDateTime::now_utc()
            };

            let duration = (next_timestamp - entry.timestamp).whole_minutes();
            *mode_durations.entry(entry.mode).or_insert(0) += duration;
        }

        // Créer les métriques pour chaque mode
        let mut metrics: Vec<ProductivityMetrics> = vec![Mode::Cravate, Mode::Intime, Mode::Neutre]
            .into_iter()
            .map(|mode| {
                let notes_count = *notes_by_mode.get(&mode).unwrap_or(&0);
                let sessions_count = *sessions_by_mode.get(&mode).unwrap_or(&0);
                let total_duration = *mode_durations.get(&mode).unwrap_or(&0);
                let avg_duration = if sessions_count > 0 {
                    total_duration / sessions_count as i64
                } else {
                    0
                };

                ProductivityMetrics {
                    mode,
                    notes_created: notes_count,
                    sessions_count,
                    avg_session_duration_minutes: avg_duration,
                }
            })
            .collect();

        metrics.sort_by(|a, b| b.notes_created.cmp(&a.notes_created));
        metrics
    }

    /// Démarre la tâche périodique de détection contextuelle
    /// Vérifie le contexte toutes les 30 secondes et publie les changements sur MQTT
    pub fn spawn_context_monitor(
        engine: Arc<ContextEngine>,
        agents: SharedAgentRegistry,
        mqtt_client: AsyncClient,
        dashboard_events: crate::dashboard_events::DashboardEventPublisher,
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
                    // Un changement de mode est détecté, publier sur MQTT legacy topic
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

                    // Publier sur dashboard topic
                    if let Err(e) = dashboard_events.publish_context_change(&new_state).await {
                        eprintln!("[context] failed to publish to dashboard: {}", e);
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
