//! MQTT Connection Watchdog
//!
//! Détecte les connexions MQTT "half-dead" (pub OK, sub KO) et force la reconnexion.
//!
//! Problème résolu: rumqttc peut avoir une connexion zombie où le kernel publie
//! mais ne reçoit plus les messages des agents. Ce watchdog surveille l'activité
//! de réception et force une reconnexion si nécessaire.
//!
//! Créé: 2026-02-03 suite à incident MQTT half-dead

use std::sync::atomic::{AtomicU64, AtomicBool, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::Notify;

/// Configuration du watchdog MQTT
#[derive(Debug, Clone)]
pub struct MqttWatchdogConfig {
    /// Délai sans message reçu avant alerte (défaut: 5 minutes)
    pub no_message_timeout: Duration,
    /// Intervalle de vérification (défaut: 30 secondes)
    pub check_interval: Duration,
    /// Nombre de checks consécutifs en échec avant action (défaut: 2)
    pub consecutive_failures_threshold: u32,
    /// Force exit(1) après X reconnexions échouées pour que systemd redémarre (défaut: 3)
    /// 0 = désactivé
    pub force_exit_after_reconnects: u32,
}

impl Default for MqttWatchdogConfig {
    fn default() -> Self {
        Self {
            no_message_timeout: Duration::from_secs(5 * 60), // 5 minutes
            check_interval: Duration::from_secs(30),
            consecutive_failures_threshold: 2,
            force_exit_after_reconnects: 1, // Exit après 1 reconnexion demandée (systemd redémarre)
        }
    }
}

/// État partagé du watchdog MQTT
#[derive(Debug)]
pub struct MqttWatchdogState {
    /// Timestamp du dernier message MQTT reçu (epoch millis)
    last_message_received: AtomicU64,
    /// Compteur de messages reçus (pour stats)
    messages_received_total: AtomicU64,
    /// Flag indiquant si le watchdog est actif
    is_active: AtomicBool,
    /// Flag indiquant qu'une reconnexion est demandée
    reconnect_requested: AtomicBool,
    /// Notifier pour signaler une demande de reconnexion
    reconnect_notify: Notify,
}

impl MqttWatchdogState {
    pub fn new() -> Self {
        Self {
            last_message_received: AtomicU64::new(Self::now_millis()),
            messages_received_total: AtomicU64::new(0),
            is_active: AtomicBool::new(true),
            reconnect_requested: AtomicBool::new(false),
            reconnect_notify: Notify::new(),
        }
    }

    /// Appelé à chaque message MQTT reçu pour réinitialiser le timer
    pub fn message_received(&self) {
        self.last_message_received.store(Self::now_millis(), Ordering::SeqCst);
        self.messages_received_total.fetch_add(1, Ordering::Relaxed);
    }

    /// Retourne le temps écoulé depuis le dernier message reçu
    pub fn time_since_last_message(&self) -> Duration {
        let last = self.last_message_received.load(Ordering::SeqCst);
        let now = Self::now_millis();
        Duration::from_millis(now.saturating_sub(last))
    }

    /// Retourne le nombre total de messages reçus
    pub fn total_messages_received(&self) -> u64 {
        self.messages_received_total.load(Ordering::Relaxed)
    }

    /// Vérifie si une reconnexion a été demandée
    pub fn is_reconnect_requested(&self) -> bool {
        self.reconnect_requested.load(Ordering::SeqCst)
    }

    /// Réinitialise le flag de reconnexion (après reconnexion réussie)
    pub fn clear_reconnect_request(&self) {
        self.reconnect_requested.store(false, Ordering::SeqCst);
        // Reset le timer aussi
        self.last_message_received.store(Self::now_millis(), Ordering::SeqCst);
    }

    /// Demande une reconnexion
    fn request_reconnect(&self) {
        self.reconnect_requested.store(true, Ordering::SeqCst);
        self.reconnect_notify.notify_one();
    }

    /// Attend une demande de reconnexion
    pub async fn wait_for_reconnect_request(&self) {
        self.reconnect_notify.notified().await;
    }

    /// Désactive le watchdog (pour shutdown propre)
    pub fn deactivate(&self) {
        self.is_active.store(false, Ordering::SeqCst);
    }

    /// Vérifie si le watchdog est actif
    pub fn is_active(&self) -> bool {
        self.is_active.load(Ordering::SeqCst)
    }

    fn now_millis() -> u64 {
        use std::time::SystemTime;
        SystemTime::now()
            .duration_since(SystemTime::UNIX_EPOCH)
            .unwrap_or_default()
            .as_millis() as u64
    }
}

impl Default for MqttWatchdogState {
    fn default() -> Self {
        Self::new()
    }
}

/// Handle partagé pour le watchdog
pub type SharedMqttWatchdog = Arc<MqttWatchdogState>;

/// Crée un nouveau watchdog partagé
pub fn create_watchdog() -> SharedMqttWatchdog {
    Arc::new(MqttWatchdogState::new())
}

/// Lance la tâche de surveillance du watchdog
///
/// Cette fonction doit être spawn dans un tokio task.
/// Elle surveille l'activité MQTT et demande une reconnexion si nécessaire.
pub async fn run_watchdog(
    state: SharedMqttWatchdog,
    config: MqttWatchdogConfig,
    has_registered_agents: impl Fn() -> bool + Send + 'static,
) {
    println!("[mqtt-watchdog] Starting MQTT connection watchdog");
    println!("[mqtt-watchdog] Config: timeout={}s, check_interval={}s, threshold={}",
        config.no_message_timeout.as_secs(),
        config.check_interval.as_secs(),
        config.consecutive_failures_threshold
    );

    let mut consecutive_failures: u32 = 0;
    let mut reconnection_requests: u32 = 0;
    let mut last_total_messages = state.total_messages_received();

    loop {
        tokio::time::sleep(config.check_interval).await;

        if !state.is_active() {
            println!("[mqtt-watchdog] Watchdog deactivated, stopping");
            break;
        }

        // Vérifie si on a des agents enregistrés
        // Si pas d'agents, pas besoin de surveiller les messages entrants
        if !has_registered_agents() {
            consecutive_failures = 0;
            continue;
        }

        let time_since_last = state.time_since_last_message();
        let current_total = state.total_messages_received();
        let messages_delta = current_total - last_total_messages;
        last_total_messages = current_total;

        // Log de debug périodique
        if time_since_last.as_secs() > 60 {
            println!("[mqtt-watchdog] ⚠️  No MQTT message received for {}s (delta: {} msgs)",
                time_since_last.as_secs(), messages_delta);
        }

        // Vérifie si timeout dépassé
        if time_since_last > config.no_message_timeout {
            consecutive_failures += 1;

            eprintln!("[mqtt-watchdog] ❌ MQTT subscription appears dead! No message for {}s (failure {}/{})",
                time_since_last.as_secs(),
                consecutive_failures,
                config.consecutive_failures_threshold
            );

            if consecutive_failures >= config.consecutive_failures_threshold {
                reconnection_requests += 1;

                eprintln!("[mqtt-watchdog] 🔄 Requesting MQTT reconnection... (attempt {}/{})",
                    reconnection_requests,
                    config.force_exit_after_reconnects
                );

                // Vérifier si on doit forcer un exit pour systemd restart
                if config.force_exit_after_reconnects > 0
                    && reconnection_requests >= config.force_exit_after_reconnects
                {
                    eprintln!("[mqtt-watchdog] 💀 MQTT connection unrecoverable after {} reconnection attempts!",
                        reconnection_requests);
                    eprintln!("[mqtt-watchdog] 💀 Forcing exit(1) for systemd restart...");
                    std::process::exit(1);
                }

                state.request_reconnect();

                // Reset le compteur de failures après avoir demandé la reconnexion
                consecutive_failures = 0;

                // Attendre un peu avant de reprendre la surveillance
                // pour laisser le temps à la reconnexion de se faire
                tokio::time::sleep(Duration::from_secs(10)).await;
            }
        } else {
            // Reset si on reçoit des messages
            if consecutive_failures > 0 && messages_delta > 0 {
                println!("[mqtt-watchdog] ✅ MQTT connection recovered ({} messages received)", messages_delta);
                reconnection_requests = 0; // Reset aussi le compteur de reconnexions
            }
            consecutive_failures = 0;
        }
    }
}

/// Statistiques du watchdog pour l'endpoint /system/health
#[derive(Debug, Clone, serde::Serialize)]
pub struct MqttWatchdogStats {
    pub last_message_received_ago_secs: u64,
    pub total_messages_received: u64,
    pub reconnect_requested: bool,
    pub watchdog_active: bool,
}

impl MqttWatchdogStats {
    pub fn from_state(state: &MqttWatchdogState) -> Self {
        Self {
            last_message_received_ago_secs: state.time_since_last_message().as_secs(),
            total_messages_received: state.total_messages_received(),
            reconnect_requested: state.is_reconnect_requested(),
            watchdog_active: state.is_active(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_watchdog_state_creation() {
        let state = MqttWatchdogState::new();
        assert!(state.is_active());
        assert!(!state.is_reconnect_requested());
        assert_eq!(state.total_messages_received(), 0);
    }

    #[test]
    fn test_message_received_updates_timestamp() {
        let state = MqttWatchdogState::new();

        // Attend un peu
        std::thread::sleep(Duration::from_millis(50));

        let before = state.time_since_last_message();
        state.message_received();
        let after = state.time_since_last_message();

        assert!(after < before);
        assert_eq!(state.total_messages_received(), 1);
    }

    #[test]
    fn test_reconnect_request() {
        let state = MqttWatchdogState::new();

        assert!(!state.is_reconnect_requested());
        state.request_reconnect();
        assert!(state.is_reconnect_requested());

        state.clear_reconnect_request();
        assert!(!state.is_reconnect_requested());
    }

    #[test]
    fn test_deactivation() {
        let state = MqttWatchdogState::new();

        assert!(state.is_active());
        state.deactivate();
        assert!(!state.is_active());
    }

    #[tokio::test]
    async fn test_watchdog_no_agents_no_failure() {
        let state = create_watchdog();
        let config = MqttWatchdogConfig {
            no_message_timeout: Duration::from_millis(10),
            check_interval: Duration::from_millis(5),
            consecutive_failures_threshold: 1,
            force_exit_after_reconnects: 0,
        };

        let state_clone = state.clone();
        let handle = tokio::spawn(async move {
            run_watchdog(state_clone, config, || false).await; // Pas d'agents
        });

        // Attendre un peu
        tokio::time::sleep(Duration::from_millis(50)).await;

        // Pas de reconnexion demandée car pas d'agents
        assert!(!state.is_reconnect_requested());

        state.deactivate();
        let _ = tokio::time::timeout(Duration::from_millis(100), handle).await;
    }
}
