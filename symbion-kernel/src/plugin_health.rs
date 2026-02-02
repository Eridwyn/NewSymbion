/**
 * PLUGIN HEALTH MONITORING - Surveillance temps réel des plugins Symbion
 *
 * RÔLE :
 * Ce module assure le monitoring actif des plugins via leurs endpoints /health.
 * Il détecte les pannes et déclenche automatiquement des actions de recovery.
 *
 * FONCTIONNEMENT :
 * - Health check périodique (30s) de tous les plugins enregistrés
 * - Requête HTTP vers /health via Unix socket
 * - Tracking uptime, response time, consecutive failures
 * - Auto-recovery : restart via systemctl après 3 échecs consécutifs
 * - Publication MQTT des métriques de santé
 *
 * UTILITÉ DANS SYMBION :
 * 🎯 Détection proactive des pannes de plugins
 * 🎯 Auto-healing automatique sans intervention manuelle
 * 🎯 Métriques détaillées pour observabilité dashboard
 * 🎯 Réduction downtime via recovery automatique
 *
 * MÉTRIQUES PAR PLUGIN :
 * - status: healthy / unhealthy / unreachable
 * - uptime_seconds: durée depuis démarrage (reportée par plugin)
 * - response_time_ms: latence réponse /health endpoint
 * - consecutive_failures: compteur échecs consécutifs
 * - last_check: timestamp dernière vérification
 * - auto_recovery_count: nombre de restarts automatiques
 */

use serde::{Deserialize, Serialize};
use std::time::{Duration, Instant};
use std::sync::Arc;
use tokio::sync::RwLock;
use std::collections::HashMap;
use std::path::PathBuf;
use http_body_util::BodyExt;
use crate::automations::EventDispatcher;

/// Health status d'un plugin à un instant T
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PluginHealthStatus {
    /// Nom du plugin
    pub plugin_name: String,
    /// État de santé (healthy, unhealthy, unreachable)
    pub status: String,
    /// Uptime en secondes (reporté par le plugin lui-même)
    pub uptime_seconds: u64,
    /// Version du plugin
    pub version: String,
    /// Temps de réponse du health check en ms
    pub response_time_ms: u64,
    /// Nombre d'échecs consécutifs
    pub consecutive_failures: u32,
    /// Timestamp de la dernière vérification
    pub last_check: chrono::DateTime<chrono::Utc>,
    /// Nombre de restarts automatiques effectués
    pub auto_recovery_count: u32,
}

/// Health check response attendue du plugin (Contract v1.0)
#[derive(Debug, Deserialize)]
struct PluginHealthResponse {
    /// Plugin ID (Contract v1.0 uses "plugin_id", legacy uses "plugin")
    #[serde(alias = "plugin")]
    plugin_id: String,
    status: String,
    uptime_seconds: u64,
    /// Spec version (Contract v1.0 uses "spec_version", legacy uses "version")
    #[serde(alias = "version")]
    spec_version: String,
}

/// Moniteur de santé des plugins avec tracking persistant
pub struct PluginHealthMonitor {
    /// État de santé de chaque plugin (thread-safe)
    health_states: Arc<RwLock<HashMap<String, PluginHealthStatus>>>,
}

impl PluginHealthMonitor {
    pub fn new() -> Self {
        Self {
            health_states: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// Récupère l'état de santé actuel de tous les plugins
    pub async fn get_all_health(&self) -> Vec<PluginHealthStatus> {
        let states = self.health_states.read().await;
        states.values().cloned().collect()
    }

    /// Récupère l'état de santé d'un plugin spécifique
    pub async fn get_plugin_health(&self, plugin_name: &str) -> Option<PluginHealthStatus> {
        let states = self.health_states.read().await;
        states.get(plugin_name).cloned()
    }

    /// Effectue un health check sur un plugin spécifique
    async fn check_plugin_health(&self, plugin_name: &str, socket_path: &PathBuf) -> Result<PluginHealthStatus, String> {
        let start = Instant::now();

        // Connexion au Unix socket du plugin
        let stream = tokio::net::UnixStream::connect(socket_path)
            .await
            .map_err(|e| format!("Failed to connect to socket: {}", e))?;

        // Construction requête HTTP GET /health
        let request = hyper::Request::builder()
            .method("GET")
            .uri("/health")
            .body(http_body_util::Full::new(hyper::body::Bytes::new()))
            .map_err(|e| format!("Failed to build request: {}", e))?;

        // Handshake HTTP/1.1 via Unix socket
        let io = hyper_util::rt::TokioIo::new(stream);
        let (mut sender, conn) = hyper::client::conn::http1::handshake(io)
            .await
            .map_err(|e| format!("HTTP handshake failed: {}", e))?;

        // Spawn connection handler
        tokio::spawn(async move {
            if let Err(e) = conn.await {
                eprintln!("[plugin-health] Connection error: {}", e);
            }
        });

        // Envoyer requête avec timeout de 5 secondes
        let response = tokio::time::timeout(
            Duration::from_secs(5),
            sender.send_request(request)
        )
        .await
        .map_err(|_| "Health check timeout (5s)".to_string())?
        .map_err(|e| format!("Failed to send request: {}", e))?;

        let response_time_ms = start.elapsed().as_millis() as u64;

        // Lire le body de la réponse
        let body_bytes = response
            .into_body()
            .collect()
            .await
            .map_err(|e| format!("Failed to read response body: {}", e))?
            .to_bytes();

        // Parser la réponse JSON
        let health_response: PluginHealthResponse = serde_json::from_slice(&body_bytes)
            .map_err(|e| format!("Failed to parse health response: {}", e))?;

        // Construire le status
        Ok(PluginHealthStatus {
            plugin_name: plugin_name.to_string(),
            status: health_response.status,
            uptime_seconds: health_response.uptime_seconds,
            version: health_response.spec_version,
            response_time_ms,
            consecutive_failures: 0,
            last_check: chrono::Utc::now(),
            auto_recovery_count: 0,
        })
    }

    /// Tente un auto-recovery d'un plugin via systemctl restart
    async fn attempt_recovery(&self, plugin_name: &str) -> Result<(), String> {
        let service_name = format!("symbion-plugin-{}", plugin_name);

        println!("[plugin-health] 🔄 Attempting auto-recovery for plugin '{}'...", plugin_name);

        let output = tokio::process::Command::new("sudo")
            .args(&["systemctl", "restart", &service_name])
            .output()
            .await
            .map_err(|e| format!("Failed to execute systemctl: {}", e))?;

        if output.status.success() {
            println!("[plugin-health] ✅ Auto-recovery succeeded for plugin '{}'", plugin_name);
            Ok(())
        } else {
            let stderr = String::from_utf8_lossy(&output.stderr);
            Err(format!("systemctl restart failed: {}", stderr))
        }
    }

    /// Démarre le monitoring périodique des plugins
    pub fn spawn_health_monitor(
        self,
        plugin_registry: crate::plugin_proxy::PluginRegistry,
        automation_dispatcher: EventDispatcher,
    ) {
        let monitor = Arc::new(self);
        let dispatcher = automation_dispatcher;

        tokio::spawn(async move {
            let mut interval = tokio::time::interval(Duration::from_secs(30));

            loop {
                interval.tick().await;

                // Récupérer la liste des plugins enregistrés
                let plugins = plugin_registry.list_plugins().await;

                if plugins.is_empty() {
                    continue;
                }

                println!("[plugin-health] 🏥 Running health checks on {} plugins...", plugins.len());

                for plugin_info in plugins {
                    let plugin_name = plugin_info.name.clone();
                    let socket_path = plugin_info.socket_path.clone();

                    // Vérifier si socket existe toujours
                    if !socket_path.exists() {
                        println!("[plugin-health] ⚠️  Plugin '{}' socket missing: {}",
                                plugin_name, socket_path.display());

                        // Marquer comme unreachable
                        let mut states = monitor.health_states.write().await;
                        let entry = states.entry(plugin_name.clone()).or_insert(PluginHealthStatus {
                            plugin_name: plugin_name.clone(),
                            status: "unreachable".to_string(),
                            uptime_seconds: 0,
                            version: "unknown".to_string(),
                            response_time_ms: 0,
                            consecutive_failures: 0,
                            last_check: chrono::Utc::now(),
                            auto_recovery_count: 0,
                        });
                        entry.consecutive_failures += 1;
                        entry.status = "unreachable".to_string();
                        entry.last_check = chrono::Utc::now();

                        let failures = entry.consecutive_failures;
                        let recovery_count = entry.auto_recovery_count;
                        drop(states); // Release lock avant recovery

                        // Dispatch unhealthy event (triggers automations)
                        dispatcher.dispatch_plugin_health(&plugin_name, "unhealthy", None);

                        // Tenter recovery après 3 échecs (et max 3 tentatives de recovery)
                        if failures >= 3 && recovery_count < 3 {
                            println!("[plugin-health] 🚨 Plugin '{}' has {} consecutive failures, attempting recovery...",
                                    plugin_name, failures);

                            // Dispatch recovery attempt event
                            dispatcher.dispatch_plugin_health(&plugin_name, "recovery_attempt", Some("unhealthy"));

                            if let Err(e) = monitor.attempt_recovery(&plugin_name).await {
                                eprintln!("[plugin-health] ❌ Recovery failed for '{}': {}", plugin_name, e);

                                // Dispatch recovery failed event
                                dispatcher.dispatch_plugin_health(&plugin_name, "recovery_failed", Some("recovery_attempt"));
                            } else {
                                // Reset compteur échecs et incrémenter recovery count
                                let mut states = monitor.health_states.write().await;
                                if let Some(entry) = states.get_mut(&plugin_name) {
                                    entry.consecutive_failures = 0;
                                    entry.auto_recovery_count += 1;
                                }

                                // Dispatch recovery success event
                                dispatcher.dispatch_plugin_health(&plugin_name, "recovery_success", Some("recovery_attempt"));
                            }
                        }

                        continue;
                    }

                    // Effectuer le health check
                    match monitor.check_plugin_health(&plugin_name, &socket_path).await {
                        Ok(mut health_status) => {
                            // Health check réussi
                            println!("[plugin-health] ✅ Plugin '{}' healthy (uptime: {}s, response: {}ms)",
                                    plugin_name, health_status.uptime_seconds, health_status.response_time_ms);

                            // Préserver le compteur de recovery
                            let mut states = monitor.health_states.write().await;
                            if let Some(old_state) = states.get(&plugin_name) {
                                health_status.auto_recovery_count = old_state.auto_recovery_count;
                            }
                            states.insert(plugin_name.clone(), health_status);
                        }
                        Err(e) => {
                            // Health check échoué
                            eprintln!("[plugin-health] ❌ Health check failed for '{}': {}", plugin_name, e);

                            let mut states = monitor.health_states.write().await;
                            let entry = states.entry(plugin_name.clone()).or_insert(PluginHealthStatus {
                                plugin_name: plugin_name.clone(),
                                status: "unhealthy".to_string(),
                                uptime_seconds: 0,
                                version: "unknown".to_string(),
                                response_time_ms: 0,
                                consecutive_failures: 0,
                                last_check: chrono::Utc::now(),
                                auto_recovery_count: 0,
                            });
                            entry.consecutive_failures += 1;
                            entry.status = "unhealthy".to_string();
                            entry.last_check = chrono::Utc::now();

                            let failures = entry.consecutive_failures;
                            let recovery_count = entry.auto_recovery_count;
                            drop(states); // Release lock avant recovery

                            // Dispatch unhealthy event (triggers automations)
                            dispatcher.dispatch_plugin_health(&plugin_name, "unhealthy", None);

                            // Tenter recovery après 3 échecs (et max 3 tentatives de recovery)
                            if failures >= 3 && recovery_count < 3 {
                                println!("[plugin-health] 🚨 Plugin '{}' has {} consecutive failures, attempting recovery...",
                                        plugin_name, failures);

                                // Dispatch recovery attempt event
                                dispatcher.dispatch_plugin_health(&plugin_name, "recovery_attempt", Some("unhealthy"));

                                if let Err(e) = monitor.attempt_recovery(&plugin_name).await {
                                    eprintln!("[plugin-health] ❌ Recovery failed for '{}': {}", plugin_name, e);

                                    // Dispatch recovery failed event
                                    dispatcher.dispatch_plugin_health(&plugin_name, "recovery_failed", Some("recovery_attempt"));
                                } else {
                                    // Reset compteur échecs et incrémenter recovery count
                                    let mut states = monitor.health_states.write().await;
                                    if let Some(entry) = states.get_mut(&plugin_name) {
                                        entry.consecutive_failures = 0;
                                        entry.auto_recovery_count += 1;
                                    }

                                    // Dispatch recovery success event
                                    dispatcher.dispatch_plugin_health(&plugin_name, "recovery_success", Some("recovery_attempt"));
                                }
                            }
                        }
                    }
                }

                println!("[plugin-health] 🏁 Health check cycle completed");
            }
        });

        println!("[plugin-health] 🚀 Plugin health monitoring started (30s interval, auto-recovery after 3 failures)");
    }
}
