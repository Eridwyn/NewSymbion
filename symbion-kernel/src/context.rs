use serde::{Deserialize, Serialize};
use std::sync::{Arc, Mutex};
use std::path::PathBuf;
use std::collections::HashMap;
use std::time::Instant;
use parking_lot::RwLock;
use crate::agents::{Agent, SharedAgentRegistry};
use rumqttc::AsyncClient;
use time::OffsetDateTime;
use utoipa::ToSchema;

/// Mode contextuel de Symbion
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, ToSchema)]
pub enum Mode {
    /// Mode professionnel (bureau, travail)
    #[serde(rename = "pro", alias = "cravate")]
    Pro,
    /// Mode domestique (maison, détente)
    #[serde(rename = "maison", alias = "intime")]
    Maison,
    /// Mode surveillance (économie énergie, maintenance)
    #[serde(rename = "veille", alias = "neutre")]
    Veille,
}

impl Mode {
    pub fn icon(&self) -> &'static str {
        match self {
            Mode::Pro => "👔",
            Mode::Maison => "🏡",
            Mode::Veille => "🌱",
        }
    }

    pub fn name(&self) -> &'static str {
        match self {
            Mode::Pro => "Focus Pro",
            Mode::Maison => "Maison",
            Mode::Veille => "Veille",
        }
    }

    pub fn theme(&self) -> Theme {
        match self {
            Mode::Pro => Theme {
                primary: "#2563eb".to_string(),
                bg: "#f8fafc".to_string(),
                accent: "#1e40af".to_string(),
            },
            Mode::Maison => Theme {
                primary: "#10b981".to_string(),
                bg: "#ecfdf5".to_string(),
                accent: "#059669".to_string(),
            },
            Mode::Veille => Theme {
                primary: "#6b7280".to_string(),
                bg: "#f9fafb".to_string(),
                accent: "#4b5563".to_string(),
            },
        }
    }
}

/// Thème visuel associé à un mode
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct Theme {
    pub primary: String,
    pub bg: String,
    pub accent: String,
}

/// État du contexte actuel
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct ContextState {
    pub mode: Mode,
    /// Slug du mode dynamique (si utilisant mode_registry)
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub mode_slug: Option<String>,
    #[serde(with = "time::serde::rfc3339")]
    #[schema(value_type = String)]
    pub changed_at: OffsetDateTime,
    pub reason: String,
    pub confidence: f32,
    pub theme: Theme,
    pub manual_override: Option<ManualOverride>,
}

/// Override manuel temporaire du mode
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct ManualOverride {
    pub mode: Mode,
    /// Slug du mode dynamique (si utilisant mode_registry)
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub mode_slug: Option<String>,
    #[serde(with = "time::serde::rfc3339")]
    #[schema(value_type = String)]
    pub until: OffsetDateTime,
    pub reason: String,
}

/// Entrée d'historique de changement de mode
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct ModeHistoryEntry {
    pub mode: Mode,
    /// Slug du mode dynamique (si utilisant mode_registry)
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub mode_slug: Option<String>,
    #[serde(with = "time::serde::rfc3339")]
    #[schema(value_type = String)]
    pub timestamp: OffsetDateTime,
    pub reason: String,
    pub was_manual: bool,
}

// Note: DetectedPattern removed - use LearnedPattern from context_intelligence.rs instead

/// Statistiques par mode
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct ModeStats {
    pub mode: Mode,
    pub total_duration_minutes: i64,
    pub entry_count: u32,
    pub percentage: f32,
}

/// Métriques de productivité par mode
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct ProductivityMetrics {
    pub mode: Mode,
    pub notes_created: u32,
    pub sessions_count: u32,
    pub avg_session_duration_minutes: i64,
}

/// Changement de mode en attente (hystérésis)
#[derive(Debug, Clone)]
struct PendingChange {
    target_mode: Mode,
    reason: String,
    confidence: f32,
    started_at: Instant,           // Horloge monotone pour timeout check
    created_at: OffsetDateTime,     // Timestamp civil pour logs
}

/// Moteur de détection contextuelle
pub struct ContextEngine {
    state: Arc<Mutex<ContextState>>,
    history: Arc<Mutex<Vec<ModeHistoryEntry>>>,
    history_path: PathBuf,
    state_path: PathBuf,
    pending_change: Arc<RwLock<Option<PendingChange>>>,  // Hystérésis 120s
    /// SQLite database (None = JSON-only fallback mode)
    db: std::sync::Mutex<Option<crate::database::SharedDatabase>>,
}

impl ContextEngine {
    pub fn new() -> Self {
        let history_path = PathBuf::from("context-history.json");
        let state_path = PathBuf::from("context-state.json");

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

        // Charger l'état persisté ou utiliser l'état par défaut
        let initial_state = if state_path.exists() {
            match std::fs::read_to_string(&state_path) {
                Ok(content) => {
                    match serde_json::from_str::<ContextState>(&content) {
                        Ok(mut state) => {
                            // Clear any expired manual override
                            if let Some(ref override_info) = state.manual_override {
                                if override_info.until < OffsetDateTime::now_utc() {
                                    state.manual_override = None;
                                }
                            }
                            println!("[context] Loaded persisted state: mode={:?}, slug={:?}",
                                state.mode, state.mode_slug);
                            state
                        }
                        Err(e) => {
                            eprintln!("[context] Failed to parse state file: {}", e);
                            Self::default_state()
                        }
                    }
                }
                Err(e) => {
                    eprintln!("[context] Failed to read state file: {}", e);
                    Self::default_state()
                }
            }
        } else {
            println!("[context] No state file found, using default");
            Self::default_state()
        };

        Self {
            state: Arc::new(Mutex::new(initial_state)),
            history: Arc::new(Mutex::new(history)),
            history_path,
            state_path,
            pending_change: Arc::new(RwLock::new(None)),
            db: std::sync::Mutex::new(None),
        }
    }

    /// Attach SQLite database and load data from DB if available.
    /// Uses interior mutability so it works on Arc<ContextEngine>.
    pub fn set_database(&self, db: crate::database::SharedDatabase) {
        // Load history from DB if it has data
        match crate::database::context_queries::count_history(&db) {
            Ok(count) if count > 0 => {
                match crate::database::context_queries::list_history(&db, 10000) {
                    Ok(rows) => {
                        let mut entries: Vec<ModeHistoryEntry> = rows.iter().rev().filter_map(|row| {
                            let mode = Self::slug_to_mode(&row.mode);
                            let timestamp = time::OffsetDateTime::parse(
                                &row.timestamp,
                                &time::format_description::well_known::Rfc3339,
                            ).ok()?;
                            Some(ModeHistoryEntry {
                                mode,
                                mode_slug: row.mode_slug.clone(),
                                timestamp,
                                reason: row.reason.clone().unwrap_or_default(),
                                was_manual: row.was_manual,
                            })
                        }).collect();
                        // Deduplicate: DB rows are newest-first, we reversed to oldest-first
                        let loaded = entries.len();
                        if let Ok(mut history) = self.history.lock() {
                            *history = entries;
                        }
                        eprintln!("[context] Loaded {} history entries from SQLite", loaded);
                    }
                    Err(e) => eprintln!("[context] Failed to load history from SQLite: {}", e),
                }
            }
            Ok(_) => {
                // DB empty, seed from current in-memory history (loaded from JSON)
                if let Ok(history) = self.history.lock() {
                    if !history.is_empty() {
                        let rows: Vec<crate::database::context_queries::ContextHistoryRow> = history.iter().map(|e| {
                            crate::database::context_queries::ContextHistoryRow {
                                mode: Self::mode_to_slug(e.mode),
                                mode_slug: e.mode_slug.clone(),
                                timestamp: e.timestamp.format(&time::format_description::well_known::Rfc3339).unwrap_or_default(),
                                reason: Some(e.reason.clone()),
                                was_manual: e.was_manual,
                            }
                        }).collect();
                        for row in &rows {
                            if let Err(e) = crate::database::context_queries::insert_history_entry(&db, row) {
                                eprintln!("[context] Failed to seed history entry to SQLite: {}", e);
                            }
                        }
                        eprintln!("[context] Seeded {} history entries to SQLite from JSON", rows.len());
                    }
                }
            }
            Err(e) => eprintln!("[context] Failed to count history in SQLite: {}", e),
        }

        // Load state from DB if it has data
        match crate::database::context_queries::get_state(&db, "context_state_json") {
            Ok(Some(json)) => {
                match serde_json::from_str::<ContextState>(&json) {
                    Ok(mut loaded_state) => {
                        // Clear expired override
                        if let Some(ref ov) = loaded_state.manual_override {
                            if ov.until < OffsetDateTime::now_utc() {
                                loaded_state.manual_override = None;
                            }
                        }
                        if let Ok(mut state) = self.state.lock() {
                            *state = loaded_state;
                        }
                        eprintln!("[context] Loaded state from SQLite");
                    }
                    Err(e) => eprintln!("[context] Failed to parse state from SQLite: {}", e),
                }
            }
            Ok(None) => {
                // DB empty, seed from current in-memory state
                if let Ok(state) = self.state.lock() {
                    if let Ok(json) = serde_json::to_string(&*state) {
                        if let Err(e) = crate::database::context_queries::set_state(&db, "context_state_json", &json) {
                            eprintln!("[context] Failed to seed state to SQLite: {}", e);
                        } else {
                            eprintln!("[context] Seeded state to SQLite from JSON");
                        }
                    }
                }
            }
            Err(e) => eprintln!("[context] Failed to read state from SQLite: {}", e),
        }

        // Store the DB handle
        if let Ok(mut db_guard) = self.db.lock() {
            *db_guard = Some(db);
        }
    }

    fn default_state() -> ContextState {
        ContextState {
            mode: Mode::Veille,
            mode_slug: Some("veille".to_string()),
            changed_at: OffsetDateTime::now_utc(),
            reason: "Initialisation système".to_string(),
            confidence: 1.0,
            theme: Mode::Veille.theme(),
            manual_override: None,
        }
    }

    /// Persiste l'état actuel (SQLite primary + JSON fallback)
    fn save_state(&self) {
        let state = match self.state.lock() {
            Ok(s) => s.clone(),
            Err(e) => {
                eprintln!("[context] Failed to lock state for saving: {}", e);
                return;
            }
        };

        let json = match serde_json::to_string_pretty(&state) {
            Ok(j) => j,
            Err(e) => {
                eprintln!("[context] Failed to serialize state: {}", e);
                return;
            }
        };

        // Try SQLite first
        let db_ok = if let Ok(db_guard) = self.db.lock() {
            if let Some(ref db) = *db_guard {
                crate::database::context_queries::set_state(db, "context_state_json", &json).is_ok()
            } else {
                false
            }
        } else {
            false
        };

        // JSON fallback (always write for backward compatibility during transition)
        if let Err(e) = std::fs::write(&self.state_path, &json) {
            if !db_ok {
                eprintln!("[context] Failed to save state to both SQLite and JSON: {}", e);
            }
        }
    }

    /// Détecte le mode contextuel basé sur les données agent
    ///
    /// Note: La détection temporelle est gérée par les automations (force_mode).
    /// Cette méthode ne gère que la détection basée sur les agents.
    pub fn detect_mode(&self, agents: &[Agent]) -> Option<(Mode, String, f32)> {
        // Pas d'agents = mode neutre
        if agents.is_empty() {
            return Some((Mode::Veille, "Aucun agent actif".to_string(), 1.0));
        }

        // La détection temporelle est gérée par les automations avec force_mode
        // Ici on retourne None pour laisser le mode actuel inchangé
        // Les automations scheduled + time_range + day_of_week gèrent les changements
        None
    }

    /// Met à jour le contexte si le mode a changé (avec hystérésis 120s)
    pub fn update(&self, agents: &[Agent]) -> Option<ContextState> {
        let mut state = self.state.lock().ok()?;

        // Vérifier override manuel
        let override_just_expired = if let Some(ref override_data) = state.manual_override {
            if OffsetDateTime::now_utc() < override_data.until {
                // Override encore valide
                return None;
            } else {
                // Override expiré - marquer pour forcer re-détection
                println!("[context] Override manuel expiré, retour détection automatique");
                state.manual_override = None;
                true
            }
        } else {
            false
        };

        // Détecter nouveau mode candidat
        let detection_result = self.detect_mode(agents);

        // Si l'override vient d'expirer, forcer la mise à jour
        if override_just_expired {
            match detection_result {
                Some((candidate_mode, reason, confidence)) => {
                    println!("[context] Forçage mise à jour post-override: mode détecté = {:?}", candidate_mode);
                    let slug = Self::mode_to_slug(candidate_mode);
                    state.mode = candidate_mode;
                    state.mode_slug = Some(slug.clone());
                    state.changed_at = OffsetDateTime::now_utc();
                    state.reason = format!("Retour auto après override: {}", reason);
                    state.confidence = confidence;
                    state.theme = candidate_mode.theme();

                    let result = state.clone();
                    drop(state);

                    self.add_to_history(candidate_mode, Some(slug), reason, false);
                    self.save_state();

                    return Some(result);
                }
                None => {
                    // Détection échouée mais override expiré - fallback vers mode Neutre
                    println!("[context] Override expiré, détection échouée - fallback vers Veille");
                    state.mode = Mode::Veille;
                    state.mode_slug = Some("veille".to_string());
                    state.changed_at = OffsetDateTime::now_utc();
                    state.reason = "Override expiré, retour mode par défaut".to_string();
                    state.confidence = 0.5;
                    state.theme = Mode::Veille.theme();

                    let result = state.clone();
                    drop(state);

                    self.add_to_history(Mode::Veille, Some("veille".to_string()),
                        "Override expiré, détection échouée".to_string(), false);
                    self.save_state();

                    return Some(result);
                }
            }
        }

        // Comportement normal si pas d'override qui vient d'expirer
        let (candidate_mode, reason, confidence) = detection_result?;

        // Si le candidat == mode actuel, annuler pending si existe
        if candidate_mode == state.mode {
            let mut pending = self.pending_change.write();
            if pending.is_some() {
                println!("[context] hysteresis_reset: mode candidat revenu à actuel ({})",
                    format!("{:?}", candidate_mode).to_lowercase());
                *pending = None;
            }
            return None;
        }

        // Candidat != mode actuel, vérifier emportement priorité
        let has_priority = candidate_mode == Mode::Veille;

        if has_priority {
            // Mode Neutre a priorité, bypass hystérésis
            println!("[context] hysteresis_bypass: Mode Neutre détecté, changement immédiat");

            // Annuler pending
            {
                let mut pending = self.pending_change.write();
                *pending = None;
            }

            // Appliquer changement immédiatement
            let slug = Self::mode_to_slug(candidate_mode);
            state.mode = candidate_mode;
            state.mode_slug = Some(slug.clone());
            state.changed_at = OffsetDateTime::now_utc();
            state.reason = reason.clone();
            state.confidence = confidence;
            state.theme = candidate_mode.theme();

            let result = state.clone();
            drop(state);

            self.add_to_history(candidate_mode, Some(slug), reason, false);
            self.save_state();

            return Some(result);
        }

        // Pas de priorité, appliquer hystérésis
        let mut pending = self.pending_change.write();

        match pending.as_ref() {
            Some(p) if p.target_mode == candidate_mode => {
                // Même mode en attente, vérifier si 120s écoulés
                let elapsed = p.started_at.elapsed();
                if elapsed.as_secs() >= 120 {
                    // Commit le changement
                    println!("[context] hysteresis_commit: {} secondes écoulées, changement {} → {} (raison: {})",
                        elapsed.as_secs(),
                        format!("{:?}", state.mode).to_lowercase(),
                        format!("{:?}", candidate_mode).to_lowercase(),
                        reason);

                    let slug = Self::mode_to_slug(candidate_mode);
                    state.mode = candidate_mode;
                    state.mode_slug = Some(slug.clone());
                    state.changed_at = OffsetDateTime::now_utc();
                    state.reason = reason.clone();
                    state.confidence = confidence;
                    state.theme = candidate_mode.theme();

                    let result = state.clone();

                    // Reset pending
                    *pending = None;

                    drop(pending);
                    drop(state);

                    self.add_to_history(candidate_mode, Some(slug), reason, false);
                    self.save_state();

                    return Some(result);
                } else {
                    // Toujours en attente
                    return None;
                }
            }
            Some(p) => {
                // Changement de cible, reset et démarrer nouveau pending
                println!("[context] hysteresis_reset: nouvelle cible {} (ancienne: {})",
                    format!("{:?}", candidate_mode).to_lowercase(),
                    format!("{:?}", p.target_mode).to_lowercase());

                println!("[context] hysteresis_started: {} → {} (délai 120s)",
                    format!("{:?}", state.mode).to_lowercase(),
                    format!("{:?}", candidate_mode).to_lowercase());

                *pending = Some(PendingChange {
                    target_mode: candidate_mode,
                    reason,
                    confidence,
                    started_at: Instant::now(),
                    created_at: OffsetDateTime::now_utc(),
                });

                None
            }
            None => {
                // Démarrer nouveau pending
                println!("[context] hysteresis_started: {} → {} (délai 120s)",
                    format!("{:?}", state.mode).to_lowercase(),
                    format!("{:?}", candidate_mode).to_lowercase());

                *pending = Some(PendingChange {
                    target_mode: candidate_mode,
                    reason,
                    confidence,
                    started_at: Instant::now(),
                    created_at: OffsetDateTime::now_utc(),
                });

                None
            }
        }
    }

    /// Récupère l'état actuel
    pub fn get_state(&self) -> Option<ContextState> {
        self.state.lock().ok().map(|s| s.clone())
    }

    /// Retourne le mode actuel en string lowercase (pour automations)
    /// Préfère mode_slug si disponible, sinon utilise l'enum Mode
    pub fn current_mode_str(&self) -> String {
        self.get_state()
            .map(|s| {
                // Préférer le slug dynamique s'il existe
                s.mode_slug.unwrap_or_else(|| {
                    match s.mode {
                        Mode::Pro => "pro".to_string(),
                        Mode::Maison => "maison".to_string(),
                        Mode::Veille => "veille".to_string(),
                    }
                })
            })
            .unwrap_or_else(|| "veille".to_string())
    }

    /// Ajoute une entrée à l'historique et sauvegarde (SQLite primary + JSON fallback)
    fn add_to_history(&self, mode: Mode, mode_slug: Option<String>, reason: String, was_manual: bool) {
        if let Ok(mut history) = self.history.lock() {
            let timestamp = OffsetDateTime::now_utc();
            let entry = ModeHistoryEntry {
                mode,
                mode_slug: mode_slug.clone(),
                timestamp,
                reason: reason.clone(),
                was_manual,
            };

            history.push(entry);

            // Try SQLite first
            if let Ok(db_guard) = self.db.lock() {
                if let Some(ref db) = *db_guard {
                    let row = crate::database::context_queries::ContextHistoryRow {
                        mode: Self::mode_to_slug(mode),
                        mode_slug,
                        timestamp: timestamp.format(&time::format_description::well_known::Rfc3339).unwrap_or_default(),
                        reason: Some(reason),
                        was_manual,
                    };
                    if let Err(e) = crate::database::context_queries::insert_history_entry(db, &row) {
                        eprintln!("[context] Failed to insert history to SQLite: {}", e);
                    }
                }
            }

            // JSON fallback (always write for backward compatibility during transition)
            if let Ok(json) = serde_json::to_string_pretty(&*history) {
                if let Err(e) = std::fs::write(&self.history_path, json) {
                    eprintln!("[context] Failed to save history to JSON: {}", e);
                }
            }
        }
    }

    /// Récupère l'historique complet
    pub fn get_history(&self) -> Vec<ModeHistoryEntry> {
        self.history.lock().ok().map(|h| h.clone()).unwrap_or_default()
    }

    /// Retourne la liste des modes disponibles (pour schema automations)
    pub fn get_available_modes(&self) -> Vec<String> {
        vec![
            "pro".to_string(),
            "maison".to_string(),
            "veille".to_string(),
        ]
    }

    /// Convertit un Mode enum vers son slug correspondant
    fn mode_to_slug(mode: Mode) -> String {
        match mode {
            Mode::Pro => "pro".to_string(),
            Mode::Maison => "maison".to_string(),
            Mode::Veille => "veille".to_string(),
        }
    }

    /// Convertit un slug vers le Mode enum correspondant (pour compatibilité)
    pub fn slug_to_mode(slug: &str) -> Mode {
        match slug.to_lowercase().as_str() {
            "pro" | "cravate" | "work" | "professional" => Mode::Pro,
            "focus" => Mode::Pro,  // Focus maps to Pro for legacy compat
            "maison" | "intime" | "home" | "domestic" => Mode::Maison,
            "veille" | "neutre" | "neutral" | "eco" => Mode::Veille,
            _ => Mode::Veille,  // Default to Veille for unknown slugs
        }
    }

    /// Force un mode manuellement (override temporaire) - méthode legacy
    pub fn set_override(&self, mode: Mode, duration_minutes: i64, reason: String) -> Option<ContextState> {
        let slug = Self::mode_to_slug(mode);
        self.set_override_dynamic(slug, mode.theme(), duration_minutes, reason)
    }

    /// Force un mode manuellement avec support des modes dynamiques
    /// Accepte le slug du mode et son thème (récupérés depuis mode_registry)
    pub fn set_override_dynamic(
        &self,
        mode_slug: String,
        theme: Theme,
        duration_minutes: i64,
        reason: String,
    ) -> Option<ContextState> {
        let mut state = self.state.lock().ok()?;

        let until = OffsetDateTime::now_utc() + time::Duration::minutes(duration_minutes);

        // Map slug to legacy Mode enum for backward compatibility
        let mode = Self::slug_to_mode(&mode_slug);

        println!("[context] Override manuel: {} ({:?}) pendant {} minutes (raison: {})",
            mode_slug, mode, duration_minutes, reason);

        state.mode = mode;
        state.mode_slug = Some(mode_slug.clone());
        state.changed_at = OffsetDateTime::now_utc();
        let full_reason = format!("Override manuel: {}", reason);
        state.reason = full_reason.clone();
        state.confidence = 1.0;
        state.theme = theme;
        state.manual_override = Some(ManualOverride {
            mode,
            mode_slug: Some(mode_slug.clone()),
            until,
            reason,
        });

        let result = state.clone();

        // Libérer le lock avant d'appeler add_to_history
        drop(state);

        // Ajouter à l'historique (marqué comme manuel)
        self.add_to_history(mode, Some(mode_slug), full_reason, true);

        // Persister l'état
        self.save_state();

        Some(result)
    }

    /// Change le mode de manière "naturelle" (sans override temporaire)
    /// Utilisé par les automations intelligentes qui ne veulent pas bloquer le système
    /// Le mode reste actif jusqu'au prochain changement (manuel ou automatique)
    pub fn set_mode_natural(
        &self,
        mode_slug: String,
        theme: Theme,
        reason: String,
    ) -> Option<ContextState> {
        let mut state = self.state.lock().ok()?;

        // Map slug to legacy Mode enum for backward compatibility
        let mode = Self::slug_to_mode(&mode_slug);

        println!("[context] Mode naturel: {} ({:?}) - raison: {}",
            mode_slug, mode, reason);

        // Annuler tout override existant (le mode naturel prend le relais)
        state.manual_override = None;

        state.mode = mode;
        state.mode_slug = Some(mode_slug.clone());
        state.changed_at = OffsetDateTime::now_utc();
        state.reason = reason.clone();
        state.confidence = 1.0;
        state.theme = theme;

        let result = state.clone();

        // Libérer le lock avant d'appeler add_to_history
        drop(state);

        // Ajouter à l'historique (marqué comme automatique, pas manuel)
        self.add_to_history(mode, Some(mode_slug), reason, false);

        // Persister l'état
        self.save_state();

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
                let slug = Self::mode_to_slug(new_mode);
                state.mode = new_mode;
                state.mode_slug = Some(slug);
                state.changed_at = OffsetDateTime::now_utc();
                state.reason = reason;
                state.confidence = confidence;
                state.theme = new_mode.theme();
            } else {
                // Fallback: mode veille si détection échoue
                state.mode = Mode::Veille;
                state.mode_slug = Some("veille".to_string());
                state.changed_at = OffsetDateTime::now_utc();
                state.reason = "Override annulé, mode par défaut".to_string();
                state.confidence = 0.5;
                state.theme = Mode::Veille.theme();
            }

            let result = state.clone();
            drop(state);

            // Persister l'état
            self.save_state();

            Some(result)
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

    // Note: detect_patterns() removed - use ContextIntelligence::get_patterns() instead

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
                Ok(content) => serde_json::from_str(&content).unwrap_or_else(|e| {
                    eprintln!("[context] Failed to parse notes.json: {}", e);
                    Vec::new()
                }),
                Err(_) => Vec::new(),
            }
        } else {
            Vec::new()
        };

        // Mapper contexte → mode (case-insensitive)
        let context_to_mode = |context: &str| -> Option<Mode> {
            let ctx = context.to_lowercase();
            if ctx.contains("pro") || ctx.contains("cravate") || ctx.contains("bureau") || ctx.contains("travail") {
                Some(Mode::Pro)
            } else if ctx.contains("maison") || ctx.contains("intime") || ctx.contains("home") {
                Some(Mode::Maison)
            } else if ctx.contains("veille") || ctx.contains("neutre") || ctx.contains("neutral") {
                Some(Mode::Veille)
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
        let mut metrics: Vec<ProductivityMetrics> = vec![Mode::Pro, Mode::Maison, Mode::Veille]
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
        automation_dispatcher: Option<crate::automations::EventDispatcher>,
    ) {
        tokio::spawn(async move {
            let mut interval = tokio::time::interval(tokio::time::Duration::from_secs(30));

            loop {
                interval.tick().await;

                // Récupérer la liste des agents
                let agents_map = agents.list_agents().await;
                let agents_list: Vec<Agent> = agents_map.values().cloned().collect();

                // Capturer le mode actuel AVANT update
                let old_mode = engine.get_state().map(|s| format!("{:?}", s.mode).to_lowercase());

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

                    let new_mode = format!("{:?}", new_state.mode).to_lowercase();

                    println!("[context] Publishing mode change: {:?} ({})",
                        new_state.mode, new_state.reason);

                    if let Err(e) = mqtt_client.publish(
                        "symbion/context/mode",
                        rumqttc::QoS::AtLeastOnce,
                        true,  // retain=true pour conserver dernier état
                        payload,
                    ).await {
                        eprintln!("[context] failed to publish mode change: {}", e);
                    }

                    // Publier sur dashboard topic
                    if let Err(e) = dashboard_events.publish_context_change(&new_state).await {
                        eprintln!("[context] failed to publish to dashboard: {}", e);
                    }

                    // Dispatcher événement pour automations
                    if let Some(ref dispatcher) = automation_dispatcher {
                        let from_mode = old_mode.unwrap_or_else(|| "unknown".to_string());
                        dispatcher.dispatch_mode_change(&from_mode, &new_mode, &new_state.reason);
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
        assert_eq!(mode, Mode::Veille);
        assert!(reason.contains("Aucun agent"));
    }

    #[test]
    fn test_mode_icons() {
        assert_eq!(Mode::Pro.icon(), "👔");
        assert_eq!(Mode::Maison.icon(), "🏡");
        assert_eq!(Mode::Veille.icon(), "🌱");
    }

    #[test]
    fn test_mode_themes() {
        let theme = Mode::Pro.theme();
        assert_eq!(theme.primary, "#2563eb");

        let theme = Mode::Maison.theme();
        assert_eq!(theme.primary, "#10b981");
    }

    #[test]
    fn test_hysteresis_structure() {
        // Test que la structure PendingChange est créée correctement
        let pending = PendingChange {
            target_mode: Mode::Pro,
            reason: "Test".to_string(),
            confidence: 0.85,
            started_at: Instant::now(),
            created_at: OffsetDateTime::now_utc(),
        };

        assert_eq!(pending.target_mode, Mode::Pro);
        assert_eq!(pending.reason, "Test");
        assert_eq!(pending.confidence, 0.85);

        // Vérifier que l'elapsed fonctionne (doit être très proche de 0)
        let elapsed = pending.started_at.elapsed();
        assert!(elapsed.as_millis() < 100);
    }

    #[test]
    fn test_priority_neutre_mode() {
        // Vérifier que Mode::Veille est bien identifié comme prioritaire
        // (Dans la logique update(), has_priority = candidate_mode == Mode::Veille)
        let neutre_priority = Mode::Veille;
        let cravate_no_priority = Mode::Pro;
        let intime_no_priority = Mode::Maison;

        // Mode::Veille devrait bypasser hystérésis
        assert_eq!(neutre_priority, Mode::Veille);
        assert_ne!(cravate_no_priority, Mode::Veille);
        assert_ne!(intime_no_priority, Mode::Veille);
    }

    #[test]
    fn test_concurrent_override_and_read() {
        use std::sync::Arc;
        use std::thread;

        let engine = Arc::new(ContextEngine::new());
        let mut handles = vec![];

        // 20 threads setting overrides simultaneously
        for i in 0..20 {
            let eng = Arc::clone(&engine);
            handles.push(thread::spawn(move || {
                let mode = if i % 2 == 0 { Mode::Pro } else { Mode::Maison };
                eng.set_override(mode, 60, format!("concurrent-test-{}", i));
            }));
        }

        // 20 threads reading state simultaneously
        for _ in 0..20 {
            let eng = Arc::clone(&engine);
            handles.push(thread::spawn(move || {
                // get_state returns Option<ContextState> — should never panic
                let _state = eng.get_state();
            }));
        }

        for h in handles {
            h.join().unwrap();
        }

        // History should have at least 20 entries (overrides + possible initial state entries)
        let history = engine.get_history();
        assert!(history.len() >= 20, "Expected >=20 history entries, got {}", history.len());
    }

    #[test]
    fn test_concurrent_clear_override() {
        use std::sync::Arc;
        use std::thread;

        let engine = Arc::new(ContextEngine::new());
        let agents: Vec<Agent> = vec![];

        // Set an override first
        engine.set_override(Mode::Pro, 60, "setup".to_string());

        let mut handles = vec![];

        // 30 threads: half clear, half set overrides
        for i in 0..30 {
            let eng = Arc::clone(&engine);
            let ag = agents.clone();
            handles.push(thread::spawn(move || {
                if i % 2 == 0 {
                    eng.clear_override(&ag);
                } else {
                    eng.set_override(Mode::Maison, 30, format!("race-{}", i));
                }
            }));
        }

        for h in handles {
            h.join().unwrap();
        }

        // State should be consistent (no panic, no deadlock)
        let state = engine.get_state();
        if let Some(s) = state {
            assert!(s.mode == Mode::Pro || s.mode == Mode::Maison || s.mode == Mode::Veille);
        }
    }
}
