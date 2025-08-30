/**
 * PLUGIN MANAGER - Gestionnaire de plugins à chaud pour Symbion
 * 
 * RÔLE :
 * Ce module gère le cycle de vie complet des plugins : chargement, déchargement,
 * monitoring, sandbox et récupération d'erreurs.
 * 
 * FONCTIONNEMENT :
 * - Plugins = processus séparés qui communiquent via MQTT
 * - Hot loading : chargement/déchargement sans redémarrer le kernel
 * - Sandbox : isolation processus + monitoring santé
 * - Manifest JSON : métadonnées et contrats de chaque plugin
 * 
 * UTILITÉ DANS SYMBION :
 * 🎯 Extensibilité : ajouter fonctionnalités sans modifier le kernel
 * 🎯 Stabilité : crash plugin n'affecte pas le kernel principal
 * 🎯 Hot reload : déploiement sans interruption service
 * 🎯 Observabilité : monitoring détaillé de chaque plugin
 * 
 * EXEMPLE PLUGIN MANIFEST :
 * ```json
 * {
 *   "name": "hosts-monitor",
 *   "version": "1.0.0",
 *   "binary": "./symbion-plugin-hosts",
 *   "contracts": ["heartbeat@v2", "wake@v1"],
 *   "auto_start": true,
 *   "restart_on_failure": true
 * }
 * ```
 */

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::process::{Child, Command, Stdio};
use std::path::{Path, PathBuf};
use tokio::fs;
use time::OffsetDateTime;
use uuid::Uuid;
use crate::state::Shared;
use tokio::task;

/// Erreurs possibles lors des opérations sur les plugins
#[derive(Debug, thiserror::Error)]
pub enum PluginError {
    #[error("Plugin not found: {0}")]
    NotFound(String),
    #[error("Plugin already loaded: {0}")]
    AlreadyLoaded(String),
    #[error("Failed to start plugin: {0}")]
    StartFailed(String),
    #[error("Plugin manifest error: {0}")]
    ManifestError(String),
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
    #[error("JSON error: {0}")]
    Json(#[from] serde_json::Error),
}

/// Manifest décrivant un plugin et ses métadonnées
/// Fichier {plugin}.json dans le dossier plugins/
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PluginManifest {
    /// Nom unique du plugin
    pub name: String,
    /// Version sémantique du plugin
    pub version: String,
    /// Chemin vers l'exécutable du plugin
    pub binary: PathBuf,
    /// Description human-readable
    pub description: Option<String>,
    /// Contrats MQTT que le plugin implémente
    pub contracts: Vec<String>,
    /// Démarrage automatique au boot du kernel
    pub auto_start: bool,
    /// Redémarrage automatique en cas de crash
    pub restart_on_failure: bool,
    /// Timeout maximum pour démarrage (secondes)
    pub startup_timeout_seconds: u64,
    /// Timeout maximum pour arrêt propre (secondes)
    pub shutdown_timeout_seconds: u64,
    /// Variables d'environnement spécifiques au plugin
    pub env: Option<HashMap<String, String>>,
    /// Dépendances requises (autres plugins à démarrer avant)
    pub depends_on: Vec<String>,
    /// Priorité de démarrage (plus petit = démarre en premier)
    pub start_priority: i32,
}

/// État d'exécution d'un plugin à un instant donné
#[derive(Debug, Clone, Serialize)]
pub enum PluginStatus {
    /// Plugin arrêté
    Stopped,
    /// Plugin en attente de ses dépendances
    WaitingDependencies,
    /// Plugin en cours de démarrage
    Starting,
    /// Plugin actif et fonctionnel
    Running,
    /// Plugin en cours d'arrêt propre
    Stopping,
    /// Plugin arrêté de force après timeout
    Killed,
    /// Plugin en erreur (crash, timeout...)
    Failed(String),
    /// Plugin en mode dégradé (safe-mode) après multiples échecs
    SafeMode,
}

/// État du circuit breaker pour éviter les redémarrages en boucle
#[derive(Debug, Clone, Serialize)]
pub enum CircuitState {
    /// Fonctionnement normal
    Normal,
    /// Mode dégradé après plusieurs échecs
    Degraded,
    /// Circuit ouvert, arrêt temporaire des tentatives de redémarrage
    CircuitOpen,
}

/// Instance d'un plugin en cours d'exécution
/// Encapsule le processus, son état et ses métadonnées
#[derive(Debug)]
pub struct PluginInstance {
    /// Métadonnées du plugin depuis son manifest
    pub manifest: PluginManifest,
    /// Processus child si le plugin est démarré
    pub process: Option<Child>,
    /// État actuel du plugin
    pub status: PluginStatus,
    /// Timestamp de démarrage pour calcul uptime
    pub started_at: Option<OffsetDateTime>,
    /// Timestamp dernière activité (heartbeat, message MQTT...)
    pub last_activity: Option<OffsetDateTime>,
    /// Compteur de redémarrages automatiques
    pub restart_count: u32,
    /// ID unique d'instance (pour debugging/logging)
    pub instance_id: String,
    /// Circuit breaker: timestamp dernière tentative de restart
    pub last_restart_attempt: Option<OffsetDateTime>,
    /// Circuit breaker: état (normal, degraded, circuit_open)
    pub circuit_state: CircuitState,
    /// Backup du manifest précédent qui fonctionnait (pour rollback)
    pub last_working_manifest: Option<PluginManifest>,
}

/// Gestionnaire central de tous les plugins Symbion
/// Point d'entrée unique pour lifecycle management
pub struct PluginManager {
    /// Map nom_plugin -> instance active
    plugins: HashMap<String, PluginInstance>,
    /// Chemin du dossier contenant les plugins et manifests
    plugins_dir: PathBuf,
    /// Configuration globale passée aux plugins
    global_env: HashMap<String, String>,
}

impl Default for PluginManifest {
    fn default() -> Self {
        Self {
            name: "unknown".to_string(),
            version: "0.1.0".to_string(),
            binary: PathBuf::from("./plugin"),
            description: None,
            contracts: vec![],
            auto_start: false,
            restart_on_failure: true,
            startup_timeout_seconds: 30,
            shutdown_timeout_seconds: 10,
            env: None,
            depends_on: vec![],
            start_priority: 100,
        }
    }
}

impl PluginInstance {
    /// Crée une nouvelle instance de plugin depuis son manifest
    fn new(manifest: PluginManifest) -> Self {
        Self {
            manifest,
            process: None,
            status: PluginStatus::Stopped,
            started_at: None,
            last_activity: None,
            restart_count: 0,
            instance_id: Uuid::new_v4().to_string(),
            last_restart_attempt: None,
            circuit_state: CircuitState::Normal,
            last_working_manifest: None,
        }
    }

    /// Démarre le processus plugin avec sandbox et monitoring
    fn start(&mut self, global_env: &HashMap<String, String>) -> Result<(), PluginError> {
        if matches!(self.status, PluginStatus::Running | PluginStatus::Starting) {
            return Err(PluginError::AlreadyLoaded(self.manifest.name.clone()));
        }

        self.status = PluginStatus::Starting;
        
        // Préparation environnement
        let mut cmd = Command::new(&self.manifest.binary);
        cmd.stdout(Stdio::piped())
           .stderr(Stdio::piped());

        // Variables globales du kernel
        for (k, v) in global_env {
            cmd.env(k, v);
        }

        // Variables spécifiques au plugin
        if let Some(env) = &self.manifest.env {
            for (k, v) in env {
                cmd.env(k, v);
            }
        }

        // Variable d'identification du plugin
        cmd.env("SYMBION_PLUGIN_NAME", &self.manifest.name);
        cmd.env("SYMBION_PLUGIN_INSTANCE_ID", &self.instance_id);

        // Démarrage processus
        match cmd.spawn() {
            Ok(child) => {
                self.process = Some(child);
                self.status = PluginStatus::Running;
                self.started_at = Some(OffsetDateTime::now_utc());
                self.last_activity = Some(OffsetDateTime::now_utc());
                
                // Sauvegarder le manifest qui fonctionne pour rollback potentiel
                self.last_working_manifest = Some(self.manifest.clone());
                self.circuit_state = CircuitState::Normal;
                
                eprintln!("[plugins] started {} (instance {})", 
                         self.manifest.name, self.instance_id);
                Ok(())
            }
            Err(e) => {
                self.status = PluginStatus::Failed(format!("Start failed: {}", e));
                self.update_circuit_state();
                Err(PluginError::StartFailed(format!("{}: {}", self.manifest.name, e)))
            }
        }
    }

    /// Arrête proprement le plugin avec timeout et graceful shutdown
    fn stop(&mut self) -> Result<(), PluginError> {
        if let Some(mut process) = self.process.take() {
            self.status = PluginStatus::Stopping;
            
            // Phase 1: Tentative arrêt propre (SIGTERM)
            if let Err(e) = process.kill() {
                eprintln!("[plugins] failed to send SIGTERM to {}: {}", self.manifest.name, e);
                self.status = PluginStatus::Failed(format!("SIGTERM failed: {}", e));
                return Err(PluginError::StartFailed(format!("SIGTERM failed: {}", e)));
            }
            
            // Phase 2: Attente arrêt avec timeout
            let timeout = std::time::Duration::from_secs(self.manifest.shutdown_timeout_seconds);
            let start_time = std::time::Instant::now();
            
            loop {
                match process.try_wait() {
                    Ok(Some(status)) => {
                        // Processus arrêté
                        if status.success() {
                            eprintln!("[plugins] {} stopped cleanly", self.manifest.name);
                        } else {
                            eprintln!("[plugins] {} exited with status: {}", 
                                     self.manifest.name, status);
                        }
                        break;
                    }
                    Ok(None) => {
                        // Processus encore actif, vérifier timeout
                        if start_time.elapsed() > timeout {
                            // Phase 3: Arrêt forcé (SIGKILL)
                            eprintln!("[plugins] {} timeout, force killing", self.manifest.name);
                            if let Err(e) = process.kill() {
                                eprintln!("[plugins] force kill failed for {}: {}", self.manifest.name, e);
                            }
                            let _ = process.wait(); // Attend la fin définitive
                            self.status = PluginStatus::Killed;
                            self.started_at = None;
                            return Ok(());
                        }
                        // Petit délai avant réessayer
                        std::thread::sleep(std::time::Duration::from_millis(100));
                    }
                    Err(e) => {
                        eprintln!("[plugins] error waiting for {}: {}", self.manifest.name, e);
                        self.status = PluginStatus::Failed(format!("Wait error: {}", e));
                        return Err(PluginError::StartFailed(format!("Wait error: {}", e)));
                    }
                }
            }
        }
        
        self.status = PluginStatus::Stopped;
        self.started_at = None;
        Ok(())
    }

    /// Vérifie si le processus plugin est encore actif
    fn check_health(&mut self) -> bool {
        if let Some(ref mut process) = self.process {
            match process.try_wait() {
                Ok(Some(status)) => {
                    // Processus terminé
                    let reason = if status.success() {
                        "exited normally".to_string()
                    } else {
                        format!("exited with status: {}", status)
                    };
                    self.status = PluginStatus::Failed(reason);
                    self.process = None;
                    false
                }
                Ok(None) => {
                    // Processus encore actif
                    true
                }
                Err(e) => {
                    // Erreur de vérification
                    self.status = PluginStatus::Failed(format!("Health check error: {}", e));
                    self.process = None;
                    false
                }
            }
        } else {
            false
        }
    }

    /// Met à jour le timestamp de dernière activité (appelé sur réception MQTT)
    fn mark_activity(&mut self) {
        self.last_activity = Some(OffsetDateTime::now_utc());
    }

    /// Met à jour l'état du circuit breaker selon le nombre d'échecs
    fn update_circuit_state(&mut self) {
        self.last_restart_attempt = Some(OffsetDateTime::now_utc());
        
        match self.restart_count {
            0..=2 => {
                self.circuit_state = CircuitState::Normal;
            }
            3..=5 => {
                self.circuit_state = CircuitState::Degraded;
                eprintln!("[plugins] {} entering degraded mode (restart_count: {})", 
                         self.manifest.name, self.restart_count);
            }
            _ => {
                self.circuit_state = CircuitState::CircuitOpen;
                self.status = PluginStatus::SafeMode;
                eprintln!("[plugins] {} entering safe-mode after {} failures", 
                         self.manifest.name, self.restart_count);
            }
        }
    }

    /// Vérifie si le plugin peut être redémarré selon le circuit breaker
    fn can_restart(&self) -> bool {
        match self.circuit_state {
            CircuitState::Normal => true,
            CircuitState::Degraded => {
                // En mode dégradé, attendre 60s entre les tentatives
                if let Some(last_attempt) = self.last_restart_attempt {
                    let elapsed = OffsetDateTime::now_utc() - last_attempt;
                    elapsed.whole_seconds() >= 60
                } else {
                    true
                }
            }
            CircuitState::CircuitOpen => {
                // Circuit ouvert, attendre 5 minutes avant réessayer
                if let Some(last_attempt) = self.last_restart_attempt {
                    let elapsed = OffsetDateTime::now_utc() - last_attempt;
                    if elapsed.whole_seconds() >= 300 {
                        eprintln!("[plugins] {} circuit breaker timeout, allowing restart attempt", 
                                 self.manifest.name);
                        true
                    } else {
                        false
                    }
                } else {
                    false
                }
            }
        }
    }

    /// Tente un rollback vers le manifest précédent qui fonctionnait
    fn attempt_rollback(&mut self, global_env: &HashMap<String, String>) -> Result<(), PluginError> {
        if let Some(working_manifest) = &self.last_working_manifest {
            eprintln!("[plugins] attempting rollback for {} to version {}", 
                     self.manifest.name, working_manifest.version);
            
            // Sauvegarder le manifest actuel
            let current_manifest = self.manifest.clone();
            
            // Restaurer le manifest qui fonctionnait
            self.manifest = working_manifest.clone();
            
            // Tentative de démarrage avec l'ancienne version
            match self.start(global_env) {
                Ok(()) => {
                    eprintln!("[plugins] rollback successful for {}", self.manifest.name);
                    Ok(())
                }
                Err(e) => {
                    // Échec du rollback, restaurer le manifest actuel
                    self.manifest = current_manifest;
                    eprintln!("[plugins] rollback failed for {}: {}", self.manifest.name, e);
                    Err(e)
                }
            }
        } else {
            Err(PluginError::StartFailed("No working manifest available for rollback".to_string()))
        }
    }
}

impl PluginManager {
    /// Crée un nouveau gestionnaire de plugins
    pub fn new<P: AsRef<Path>>(plugins_dir: P) -> Self {
        let mut global_env = HashMap::new();
        
        // Configuration MQTT pour tous les plugins
        global_env.insert("SYMBION_MQTT_HOST".to_string(), 
                         std::env::var("SYMBION_MQTT_HOST").unwrap_or("localhost".to_string()));
        global_env.insert("SYMBION_MQTT_PORT".to_string(), 
                         std::env::var("SYMBION_MQTT_PORT").unwrap_or("1883".to_string()));

        Self {
            plugins: HashMap::new(),
            plugins_dir: plugins_dir.as_ref().to_path_buf(),
            global_env,
        }
    }

    /// Scanne le dossier plugins/ et charge tous les manifests
    pub async fn discover_plugins(&mut self) -> Result<Vec<String>, PluginError> {
        let mut discovered = Vec::new();
        let mut entries = fs::read_dir(&self.plugins_dir).await?;

        while let Some(entry) = entries.next_entry().await? {
            let path = entry.path();
            if path.extension().and_then(|s| s.to_str()) == Some("json") {
                if let Some(filename) = path.file_stem().and_then(|s| s.to_str()) {
                    match self.load_manifest(&path).await {
                        Ok(manifest) => {
                            let plugin_name = manifest.name.clone();
                            let instance = PluginInstance::new(manifest);
                            self.plugins.insert(plugin_name.clone(), instance);
                            discovered.push(plugin_name.clone());
                            eprintln!("[plugins] discovered: {} (from {})", plugin_name, filename);
                        }
                        Err(e) => {
                            eprintln!("[plugins] failed to load manifest {}: {}", filename, e);
                        }
                    }
                }
            }
        }

        Ok(discovered)
    }

    /// Charge un manifest de plugin depuis un fichier JSON
    async fn load_manifest<P: AsRef<Path>>(&self, path: P) -> Result<PluginManifest, PluginError> {
        let content = fs::read_to_string(path).await?;
        let manifest: PluginManifest = serde_json::from_str(&content)?;
        
        // Validation basique
        if manifest.name.is_empty() {
            return Err(PluginError::ManifestError("name cannot be empty".to_string()));
        }
        if !manifest.binary.exists() {
            return Err(PluginError::ManifestError(
                format!("binary not found: {:?}", manifest.binary)
            ));
        }

        Ok(manifest)
    }

    /// Démarre un plugin par son nom
    pub fn start_plugin(&mut self, name: &str) -> Result<(), PluginError> {
        let plugin = self.plugins.get_mut(name)
            .ok_or_else(|| PluginError::NotFound(name.to_string()))?;
        
        plugin.start(&self.global_env)
    }

    /// Arrête un plugin par son nom
    pub fn stop_plugin(&mut self, name: &str) -> Result<(), PluginError> {
        let plugin = self.plugins.get_mut(name)
            .ok_or_else(|| PluginError::NotFound(name.to_string()))?;
        
        plugin.stop()
    }

    /// Redémarre un plugin (stop puis start)
    pub fn restart_plugin(&mut self, name: &str) -> Result<(), PluginError> {
        if let Err(e) = self.stop_plugin(name) {
            eprintln!("[plugins] stop failed during restart of {}: {}", name, e);
        }
        
        // Petit délai pour laisser le processus se terminer proprement
        std::thread::sleep(std::time::Duration::from_millis(100));
        
        let plugin = self.plugins.get_mut(name).unwrap();
        plugin.restart_count += 1;
        
        self.start_plugin(name)
    }

    /// Lance le healthcheck de tous les plugins avec circuit breaker et rollback
    pub fn health_check_all(&mut self) {
        let mut to_restart = Vec::new();
        let mut to_rollback = Vec::new();

        for (name, plugin) in &mut self.plugins {
            if !plugin.check_health() {
                // Plugin défaillant
                plugin.update_circuit_state();
                
                if !plugin.manifest.restart_on_failure {
                    eprintln!("[plugins] {} failed, restart disabled", name);
                    continue;
                }

                if !plugin.can_restart() {
                    eprintln!("[plugins] {} failed, but circuit breaker prevents restart", name);
                    continue;
                }

                match plugin.circuit_state {
                    CircuitState::Normal => {
                        eprintln!("[plugins] {} failed, scheduling normal restart", name);
                        to_restart.push(name.clone());
                    }
                    CircuitState::Degraded => {
                        if plugin.restart_count >= 3 && plugin.last_working_manifest.is_some() {
                            eprintln!("[plugins] {} failed repeatedly, attempting rollback", name);
                            to_rollback.push(name.clone());
                        } else {
                            eprintln!("[plugins] {} failed, scheduling degraded restart", name);
                            to_restart.push(name.clone());
                        }
                    }
                    CircuitState::CircuitOpen => {
                        // En safe-mode, on ne fait rien automatiquement
                        eprintln!("[plugins] {} in safe-mode, manual intervention required", name);
                    }
                }
            }
        }

        // Tentatives de rollback en priorité
        for name in to_rollback {
            if let Some(plugin) = self.plugins.get_mut(&name) {
                match plugin.attempt_rollback(&self.global_env) {
                    Ok(()) => {
                        eprintln!("[plugins] rollback successful for {}", name);
                    }
                    Err(e) => {
                        eprintln!("[plugins] rollback failed for {}: {}", name, e);
                        // Si le rollback échoue aussi, passer en safe-mode
                        plugin.status = PluginStatus::SafeMode;
                        plugin.circuit_state = CircuitState::CircuitOpen;
                    }
                }
            }
        }

        // Redémarrages normaux
        for name in to_restart {
            if let Err(e) = self.restart_plugin(&name) {
                eprintln!("[plugins] restart failed for {}: {}", name, e);
            }
        }
    }

    /// Démarre automatiquement tous les plugins marqués auto_start avec gestion des dépendances
    pub fn auto_start_plugins(&mut self) {
        let auto_start_plugins: Vec<String> = self.plugins
            .values()
            .filter(|p| p.manifest.auto_start)
            .map(|p| p.manifest.name.clone())
            .collect();

        // Démarrage ordonné selon les dépendances et priorités
        match self.start_plugins_ordered(&auto_start_plugins) {
            Ok(started) => {
                eprintln!("[plugins] auto-started {} plugins: [{}]", 
                         started.len(), started.join(", "));
            }
            Err(e) => {
                eprintln!("[plugins] auto-start failed: {}", e);
            }
        }
    }

    /// Démarre une liste de plugins dans l'ordre des dépendances
    pub fn start_plugins_ordered(&mut self, plugin_names: &[String]) -> Result<Vec<String>, PluginError> {
        let mut started = Vec::new();
        let mut remaining: Vec<String> = plugin_names.to_vec();
        let max_iterations = remaining.len() + 5; // Éviter boucles infinies
        let mut iterations = 0;

        while !remaining.is_empty() && iterations < max_iterations {
            let mut progress = false;
            iterations += 1;

            // Trier par priorité de démarrage
            remaining.sort_by_key(|name| {
                self.plugins.get(name)
                    .map(|p| p.manifest.start_priority)
                    .unwrap_or(999)
            });

            let mut i = 0;
            while i < remaining.len() {
                let name = &remaining[i];
                
                if self.can_start_plugin(name) {
                    // Toutes les dépendances sont satisfaites
                    match self.start_plugin(name) {
                        Ok(()) => {
                            started.push(name.clone());
                            remaining.remove(i);
                            progress = true;
                            // Ne pas incrémenter i car on a supprimé un élément
                        }
                        Err(e) => {
                            eprintln!("[plugins] failed to start {}: {}", name, e);
                            // Marquer le plugin en erreur mais continuer
                            if let Some(plugin) = self.plugins.get_mut(name) {
                                plugin.status = PluginStatus::Failed(format!("Start failed: {}", e));
                            }
                            remaining.remove(i);
                            // Ne pas incrémenter i
                        }
                    }
                } else {
                    // Attendre les dépendances
                    if let Some(plugin) = self.plugins.get_mut(name) {
                        plugin.status = PluginStatus::WaitingDependencies;
                    }
                    i += 1;
                }
            }

            if !progress {
                // Aucun progrès dans cette itération
                let unresolved: Vec<String> = remaining.iter()
                    .map(|name| format!("{} (depends on: [{}])", 
                         name, 
                         self.plugins.get(name)
                             .map(|p| p.manifest.depends_on.join(", "))
                             .unwrap_or_default()))
                    .collect();
                
                return Err(PluginError::StartFailed(
                    format!("Circular dependencies or missing dependencies: [{}]", 
                           unresolved.join(", "))));
            }
        }

        if !remaining.is_empty() {
            return Err(PluginError::StartFailed(
                format!("Max iterations reached, remaining plugins: [{}]", 
                       remaining.join(", "))));
        }

        Ok(started)
    }

    /// Vérifie si un plugin peut être démarré (dépendances satisfaites)
    fn can_start_plugin(&self, plugin_name: &str) -> bool {
        let Some(plugin) = self.plugins.get(plugin_name) else {
            return false;
        };

        // Vérifier que toutes les dépendances sont démarrées
        for dep_name in &plugin.manifest.depends_on {
            if let Some(dep_plugin) = self.plugins.get(dep_name) {
                if !matches!(dep_plugin.status, PluginStatus::Running) {
                    return false;
                }
            } else {
                // Dépendance introuvable
                eprintln!("[plugins] {} depends on unknown plugin: {}", plugin_name, dep_name);
                return false;
            }
        }

        true
    }

    /// Liste tous les plugins avec leur état
    pub fn list_plugins(&self) -> Vec<PluginInfo> {
        self.plugins.values().map(|p| PluginInfo {
            name: p.manifest.name.clone(),
            version: p.manifest.version.clone(),
            status: p.status.clone(),
            uptime_seconds: p.started_at.map(|start| {
                (OffsetDateTime::now_utc() - start).whole_seconds() as u64
            }),
            restart_count: p.restart_count,
            contracts: p.manifest.contracts.clone(),
        }).collect()
    }

    /// Met à jour l'activité d'un plugin (appelé sur réception MQTT)
    pub fn mark_plugin_activity(&mut self, plugin_name: &str) {
        if let Some(plugin) = self.plugins.get_mut(plugin_name) {
            plugin.mark_activity();
        }
    }

    /// Réinitialise le circuit breaker d'un plugin pour permettre sa récupération manuelle
    pub fn reset_plugin_circuit(&mut self, plugin_name: &str) -> Result<(), PluginError> {
        let plugin = self.plugins.get_mut(plugin_name)
            .ok_or_else(|| PluginError::NotFound(plugin_name.to_string()))?;

        plugin.circuit_state = CircuitState::Normal;
        plugin.restart_count = 0;
        plugin.last_restart_attempt = None;
        
        if matches!(plugin.status, PluginStatus::SafeMode) {
            plugin.status = PluginStatus::Stopped;
        }

        eprintln!("[plugins] circuit breaker reset for {}, ready for manual restart", plugin_name);
        Ok(())
    }

    /// Force le rollback d'un plugin vers sa dernière version fonctionnelle
    pub fn force_plugin_rollback(&mut self, plugin_name: &str) -> Result<(), PluginError> {
        let plugin = self.plugins.get_mut(plugin_name)
            .ok_or_else(|| PluginError::NotFound(plugin_name.to_string()))?;

        // Arrêter le plugin s'il tourne
        if matches!(plugin.status, PluginStatus::Running) {
            let _ = plugin.stop();
        }

        // Réinitialiser le circuit breaker
        plugin.circuit_state = CircuitState::Normal;
        plugin.restart_count = 0;

        // Tenter le rollback
        plugin.attempt_rollback(&self.global_env)
    }

    /// Récupère les statistiques détaillées d'un plugin pour debugging
    pub fn get_plugin_debug_info(&self, plugin_name: &str) -> Option<PluginDebugInfo> {
        self.plugins.get(plugin_name).map(|p| PluginDebugInfo {
            name: p.manifest.name.clone(),
            status: p.status.clone(),
            circuit_state: p.circuit_state.clone(),
            restart_count: p.restart_count,
            uptime_seconds: p.started_at.map(|start| {
                (OffsetDateTime::now_utc() - start).whole_seconds() as u64
            }),
            last_activity_ago_seconds: p.last_activity.map(|last| {
                (OffsetDateTime::now_utc() - last).whole_seconds() as u64
            }),
            has_rollback_available: p.last_working_manifest.is_some(),
            manifest_version: p.manifest.version.clone(),
            rollback_version: p.last_working_manifest.as_ref().map(|m| m.version.clone()),
        })
    }

    /// Arrête proprement tous les plugins dans l'ordre inverse des dépendances
    pub fn shutdown_all(&mut self) {
        eprintln!("[plugins] shutting down all plugins...");
        
        // Récupérer tous les plugins actifs
        let running_plugins: Vec<String> = self.plugins
            .values()
            .filter(|p| matches!(p.status, PluginStatus::Running | PluginStatus::Starting))
            .map(|p| p.manifest.name.clone())
            .collect();
        
        match self.stop_plugins_ordered(&running_plugins) {
            Ok(stopped) => {
                eprintln!("[plugins] shutdown complete, stopped {} plugins: [{}]", 
                         stopped.len(), stopped.join(", "));
            }
            Err(e) => {
                eprintln!("[plugins] shutdown error: {}", e);
            }
        }
    }

    /// Arrête une liste de plugins dans l'ordre inverse des dépendances
    pub fn stop_plugins_ordered(&mut self, plugin_names: &[String]) -> Result<Vec<String>, PluginError> {
        let mut stopped = Vec::new();
        let mut remaining: Vec<String> = plugin_names.to_vec();
        let max_iterations = remaining.len() + 5;
        let mut iterations = 0;

        while !remaining.is_empty() && iterations < max_iterations {
            let mut progress = false;
            iterations += 1;

            let mut i = 0;
            while i < remaining.len() {
                let name = &remaining[i];
                
                if self.can_stop_plugin(name, &remaining) {
                    // Aucun autre plugin ne dépend de celui-ci
                    match self.stop_plugin(name) {
                        Ok(()) => {
                            stopped.push(name.clone());
                            remaining.remove(i);
                            progress = true;
                        }
                        Err(e) => {
                            eprintln!("[plugins] failed to stop {}: {}", name, e);
                            remaining.remove(i);
                        }
                    }
                } else {
                    i += 1;
                }
            }

            if !progress {
                // Forcer l'arrêt des plugins restants
                eprintln!("[plugins] forcing stop of remaining plugins due to circular dependencies");
                for name in &remaining {
                    if let Err(e) = self.stop_plugin(name) {
                        eprintln!("[plugins] force stop failed for {}: {}", name, e);
                    } else {
                        stopped.push(name.clone());
                    }
                }
                break;
            }
        }

        Ok(stopped)
    }

    /// Vérifie si un plugin peut être arrêté (aucun autre plugin actif ne dépend de lui)
    fn can_stop_plugin(&self, plugin_name: &str, remaining_plugins: &[String]) -> bool {
        // Chercher si d'autres plugins encore actifs dépendent de celui-ci
        for other_name in remaining_plugins {
            if other_name == plugin_name {
                continue;
            }
            
            if let Some(other_plugin) = self.plugins.get(other_name) {
                if other_plugin.manifest.depends_on.contains(&plugin_name.to_string()) {
                    return false; // Un autre plugin dépend encore de celui-ci
                }
            }
        }
        
        true
    }
}

/// Informations publiques d'un plugin pour les APIs
#[derive(Debug, Serialize)]
pub struct PluginInfo {
    pub name: String,
    pub version: String,
    pub status: PluginStatus,
    pub uptime_seconds: Option<u64>,
    pub restart_count: u32,
    pub contracts: Vec<String>,
}

/// Informations détaillées de debugging d'un plugin
#[derive(Debug, Serialize)]
pub struct PluginDebugInfo {
    pub name: String,
    pub status: PluginStatus,
    pub circuit_state: CircuitState,
    pub restart_count: u32,
    pub uptime_seconds: Option<u64>,
    pub last_activity_ago_seconds: Option<u64>,
    pub has_rollback_available: bool,
    pub manifest_version: String,
    pub rollback_version: Option<String>,
}

impl Drop for PluginManager {
    /// Nettoyage automatique lors de la destruction du PluginManager
    fn drop(&mut self) {
        self.shutdown_all();
    }
}

/// Démarre le monitoring périodique de la santé des plugins
/// Exécute le healthcheck toutes les 30 secondes et redémarre les plugins défaillants
pub fn spawn_plugin_health_monitor(plugins: Shared<PluginManager>) {
    task::spawn(async move {
        let mut interval = tokio::time::interval(std::time::Duration::from_secs(30));
        
        loop {
            interval.tick().await;
            
            {
                // Réduire la durée du verrou en séparant les opérations
                let active_plugins = {
                    let mut manager = plugins.lock();
                    manager.health_check_all();
                    
                    // Récupérer la liste rapidement et relâcher le verrou
                    manager.plugins
                        .values()
                        .filter(|p| matches!(p.status, PluginStatus::Running))
                        .map(|p| p.manifest.name.clone())
                        .collect::<Vec<String>>()
                }; // Verrou relâché ici
                
                // Log en dehors du verrou
                if !active_plugins.is_empty() {
                    println!("[plugins] health check - active: [{}]", active_plugins.join(", "));
                }
            }
        }
    });
}