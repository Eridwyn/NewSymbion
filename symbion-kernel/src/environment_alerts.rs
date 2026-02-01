/**
 * ENVIRONMENT ALERT MONITOR - Surveillance alertes environnement avec notifications
 *
 * RÔLE : Surveille périodiquement les états environnement et envoie des notifications
 *        quand le niveau d'alerte change (évite le spam)
 *
 * ARCHITECTURE : Task async qui vérifie toutes les 30s, track le dernier niveau par room
 * UTILITÉ : Alertes moisissure P0/P1 envoyées via NotificationManager (kernel intégré)
 */

use crate::dew_point_alerts::{DewPointCalculator, DewPointAlertLevel};
use crate::notifications::{SharedNotificationManager, Notification, NotificationPriority};
use crate::notification_config::SharedNotificationConfigManager;
use crate::sensors::SharedSensorRegistry;
use std::collections::HashMap;
use std::sync::Arc;
use time::OffsetDateTime;
use tokio::sync::RwLock;
use tokio::time::{interval, Duration};

/// Moniteur d'alertes environnement
pub struct EnvironmentAlertMonitor {
    sensors: SharedSensorRegistry,
    notifications_manager: SharedNotificationManager,
    notification_config: SharedNotificationConfigManager,
    /// Track le dernier niveau d'alerte par room pour éviter le spam
    last_alert_levels: Arc<RwLock<HashMap<String, DewPointAlertLevel>>>,
    /// Track le timestamp de la dernière notification par room (anti-spam)
    last_notification_time: Arc<RwLock<HashMap<String, i64>>>,
    calculator: DewPointCalculator,
    /// Dispatcher pour événements automations
    automation_dispatcher: Option<crate::automations::EventDispatcher>,
}

/// Délai minimum entre deux notifications pour la même room (5 minutes)
const MIN_NOTIFICATION_INTERVAL_SECS: i64 = 300;

impl EnvironmentAlertMonitor {
    pub fn new(
        sensors: SharedSensorRegistry,
        notifications_manager: SharedNotificationManager,
        notification_config: SharedNotificationConfigManager,
        automation_dispatcher: Option<crate::automations::EventDispatcher>,
    ) -> Self {
        Self {
            sensors,
            notifications_manager,
            notification_config,
            last_alert_levels: Arc::new(RwLock::new(HashMap::new())),
            last_notification_time: Arc::new(RwLock::new(HashMap::new())),
            calculator: DewPointCalculator::default(),
            automation_dispatcher,
        }
    }

    /// Démarre la surveillance périodique (toutes les 30s)
    pub fn spawn_monitor(self) {
        tokio::spawn(async move {
            let mut check_interval = interval(Duration::from_secs(30));

            println!("[env-alerts] Monitor started (checking every 30s)");

            loop {
                check_interval.tick().await;
                self.check_all_environments().await;
            }
        });
    }

    /// Vérifie tous les environnements et envoie des notifications si nécessaire
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

            // Envoyer notification si le niveau a changé et n'est pas Safe
            if current_level != last_level {
                // Vérifier le délai anti-spam (5 min minimum entre notifications)
                let now = OffsetDateTime::now_utc().unix_timestamp();
                let can_notify = {
                    let times = self.last_notification_time.read().await;
                    times.get(&room_id)
                        .map(|&last| now - last >= MIN_NOTIFICATION_INTERVAL_SECS)
                        .unwrap_or(true)
                };

                // Mettre à jour le dernier niveau
                {
                    let mut levels = self.last_alert_levels.write().await;
                    levels.insert(room_id.clone(), current_level);
                }

                // Dispatcher événement pour automations (toujours, pas de délai)
                if let Some(ref dispatcher) = self.automation_dispatcher {
                    let alert_level_str = format!("{:?}", current_level).to_lowercase();
                    let previous_level_str = format!("{:?}", last_level).to_lowercase();
                    // room_id est le sensor_id dans ce contexte
                    dispatcher.dispatch_sensor_alert(
                        &env_state.room_id,
                        &room_id,
                        &alert_level_str,
                        Some(&previous_level_str),
                        env_state.current.temperature_c,
                        env_state.current.humidity_pct,
                    );
                }

                // Ne notifier que si délai respecté
                if can_notify {
                    // Ne notifier que si on passe à un niveau d'alerte (pas si on revient à Safe)
                    if current_level != DewPointAlertLevel::Safe {
                        self.send_alert_notification(&room_id, &evaluation).await;
                        // Enregistrer le timestamp
                        let mut times = self.last_notification_time.write().await;
                        times.insert(room_id.clone(), now);
                    } else if last_level >= DewPointAlertLevel::Strong {
                        // Notifier le retour à la normale si on était en alerte forte+
                        self.send_recovery_notification(&room_id, last_level).await;
                        // Enregistrer le timestamp
                        let mut times = self.last_notification_time.write().await;
                        times.insert(room_id.clone(), now);
                    }
                }
            }
        }
    }

    /// Envoie une notification d'alerte
    async fn send_alert_notification(
        &self,
        room_id: &str,
        evaluation: &crate::dew_point_alerts::DewPointEvaluation,
    ) {
        // Déterminer le type_id de la notification selon le niveau
        let type_id = match evaluation.level {
            DewPointAlertLevel::Danger => "environment_alert_danger",
            DewPointAlertLevel::Critical => "environment_alert_critical",
            DewPointAlertLevel::Strong => "environment_alert_strong",
            DewPointAlertLevel::Moderate => "environment_alert_moderate",
            DewPointAlertLevel::Weak => "environment_alert_weak",
            DewPointAlertLevel::Safe => return, // Ne devrait pas arriver
        };

        // Construire les variables pour l'interpolation
        let mut variables = HashMap::new();
        variables.insert("room_id".to_string(), room_id.to_string());
        variables.insert("diagnostics".to_string(), self.format_diagnostics(evaluation));
        variables.insert("suggestion".to_string(), evaluation.level.suggestion().to_string());
        variables.insert("level".to_string(), format!("{:?}", evaluation.level));
        if let Some(temp) = evaluation.air_temp_c {
            variables.insert("temperature".to_string(), format!("{:.1}", temp));
        }
        if let Some(hum) = evaluation.humidity_pct {
            variables.insert("humidity".to_string(), format!("{:.1}", hum));
        }

        // Obtenir le titre et corps depuis la config (ou fallback)
        let (title, body, priority) = match self.notification_config.build_notification(type_id, &variables) {
            Some((t, b, p)) => (t, b, p.into()),
            None => {
                // Config désactivée - ne pas envoyer
                println!("[env-alerts] Notification {} disabled by config", type_id);
                return;
            }
        };

        let notification = Notification {
            id: String::new(), // Will be assigned by manager
            priority,
            title: title.clone(),
            body,
            source: "environment-monitor".to_string(),
            timestamp: OffsetDateTime::now_utc(),
            acknowledged: false,
            acknowledged_at: None,
            actions: vec![],
            data: None,
        };

        match self.notifications_manager.send(notification).await {
            Ok(()) => println!(
                "[env-alerts] Notification envoyée: {} ({:?})",
                room_id, evaluation.level
            ),
            Err(e) => eprintln!("[env-alerts] Erreur notification: {}", e),
        }
    }

    /// Envoie une notification de retour à la normale
    async fn send_recovery_notification(&self, room_id: &str, previous_level: DewPointAlertLevel) {
        let type_id = "environment_alert_recovery";

        let mut variables = HashMap::new();
        variables.insert("room_id".to_string(), room_id.to_string());
        variables.insert("previous_level".to_string(), format!("{:?}", previous_level));

        let (title, body, priority) = match self.notification_config.build_notification(type_id, &variables) {
            Some((t, b, p)) => (t, b, p.into()),
            None => {
                println!("[env-alerts] Notification {} disabled by config", type_id);
                return;
            }
        };

        let notification = Notification {
            id: String::new(),
            priority,
            title,
            body,
            source: "environment-monitor".to_string(),
            timestamp: OffsetDateTime::now_utc(),
            acknowledged: false,
            acknowledged_at: None,
            actions: vec![],
            data: None,
        };

        let _ = self.notifications_manager.send(notification).await;
    }

    /// Formate les diagnostics pour le message
    fn format_diagnostics(&self, evaluation: &crate::dew_point_alerts::DewPointEvaluation) -> String {
        let mut parts = vec![];

        if let Some(humidity) = evaluation.humidity_pct {
            parts.push(format!("HR: {:.1}%", humidity));
        }
        if let Some(temp) = evaluation.air_temp_c {
            parts.push(format!("T: {:.1}°C", temp));
        }
        if let Some(dew) = evaluation.dew_point_c {
            parts.push(format!("Rosée: {:.1}°C", dew));
        }
        if let Some(delta) = evaluation.delta_t {
            parts.push(format!("ΔT: {:.1}°C", delta));
        }

        parts.join(" | ")
    }
}
