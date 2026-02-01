/**
 * ENVIRONMENT ALERT MONITOR - Surveillance alertes environnement
 *
 * RÔLE : Surveille périodiquement les états environnement et dispatch les événements
 *        vers le système d'automations (qui gère les notifications avec conditions de mode)
 *
 * ARCHITECTURE : Task async qui vérifie toutes les 30s, track le dernier niveau par room
 * NOTIFICATIONS : Gérées UNIQUEMENT par les automations (respecte les conditions de mode)
 */

use crate::dew_point_alerts::{DewPointCalculator, DewPointAlertLevel};
use crate::sensors::SharedSensorRegistry;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use tokio::time::{interval, Duration};

/// Moniteur d'alertes environnement
pub struct EnvironmentAlertMonitor {
    sensors: SharedSensorRegistry,
    /// Track le dernier niveau d'alerte par room pour éviter le spam d'événements
    last_alert_levels: Arc<RwLock<HashMap<String, DewPointAlertLevel>>>,
    calculator: DewPointCalculator,
    /// Dispatcher pour événements automations
    automation_dispatcher: Option<crate::automations::EventDispatcher>,
}

impl EnvironmentAlertMonitor {
    pub fn new(
        sensors: SharedSensorRegistry,
        _notifications_manager: crate::notifications::SharedNotificationManager,
        _notification_config: crate::notification_config::SharedNotificationConfigManager,
        automation_dispatcher: Option<crate::automations::EventDispatcher>,
    ) -> Self {
        // notifications_manager et notification_config ne sont plus utilisés
        // Les notifications sont gérées par les automations
        Self {
            sensors,
            last_alert_levels: Arc::new(RwLock::new(HashMap::new())),
            calculator: DewPointCalculator::default(),
            automation_dispatcher,
        }
    }

    /// Démarre la surveillance périodique (toutes les 30s)
    pub fn spawn_monitor(self) {
        tokio::spawn(async move {
            let mut check_interval = interval(Duration::from_secs(30));

            println!("[env-alerts] Monitor started (checking every 30s, notifications via automations only)");

            loop {
                check_interval.tick().await;
                self.check_all_environments().await;
            }
        });
    }

    /// Vérifie tous les environnements et dispatch les événements vers les automations
    async fn check_all_environments(&self) {
        // Récupérer tous les environnements
        let environments = self.sensors.list_environments();

        for (room_id, env_state) in environments {
            // Évaluer le niveau d'alerte
            let evaluation = self.calculator.evaluate(&env_state);
            let current_level = evaluation.level;

            // Récupérer le dernier niveau connu
            let last_level = {
                let levels = self.last_alert_levels.read().await;
                levels.get(&room_id).copied().unwrap_or(DewPointAlertLevel::Safe)
            };

            // Dispatcher événement si le niveau a changé
            if current_level != last_level {
                // Mettre à jour le dernier niveau
                {
                    let mut levels = self.last_alert_levels.write().await;
                    levels.insert(room_id.clone(), current_level);
                }

                // Dispatcher événement pour automations
                // Les automations décident si elles envoient une notification selon leurs conditions (ex: mode != veille)
                if let Some(ref dispatcher) = self.automation_dispatcher {
                    let alert_level_str = format!("{:?}", current_level).to_lowercase();
                    let previous_level_str = format!("{:?}", last_level).to_lowercase();

                    dispatcher.dispatch_sensor_alert(
                        &env_state.room_id,
                        &room_id,
                        &alert_level_str,
                        Some(&previous_level_str),
                        env_state.current.temperature_c,
                        env_state.current.humidity_pct,
                    );

                    println!(
                        "[env-alerts] Dispatched event: {} level changed {} → {}",
                        room_id, previous_level_str, alert_level_str
                    );
                }
            }
        }
    }
}
