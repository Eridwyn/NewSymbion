/**
 * ENVIRONMENT ALERT MONITOR - Surveillance alertes environnement avec notifications
 *
 * RÔLE : Surveille périodiquement les états environnement et envoie des notifications
 *        quand le niveau d'alerte change (évite le spam)
 *
 * ARCHITECTURE : Task async qui vérifie toutes les 30s, track le dernier niveau par room
 * UTILITÉ : Alertes moisissure P0/P1 envoyées au plugin notifications si dispo
 */

use crate::dew_point_alerts::{DewPointCalculator, DewPointAlertLevel};
use crate::notification_client::{NotificationClient, NotificationPayload, NotificationPriority};
use crate::sensors::SharedSensorRegistry;
use std::collections::HashMap;
use std::sync::Arc;
use time::OffsetDateTime;
use tokio::sync::RwLock;
use tokio::time::{interval, Duration};

/// Moniteur d'alertes environnement
pub struct EnvironmentAlertMonitor {
    sensors: SharedSensorRegistry,
    notification_client: NotificationClient,
    /// Track le dernier niveau d'alerte par room pour éviter le spam
    last_alert_levels: Arc<RwLock<HashMap<String, DewPointAlertLevel>>>,
    /// Track le timestamp de la dernière notification par room (anti-spam)
    last_notification_time: Arc<RwLock<HashMap<String, i64>>>,
    calculator: DewPointCalculator,
}

/// Délai minimum entre deux notifications pour la même room (5 minutes)
const MIN_NOTIFICATION_INTERVAL_SECS: i64 = 300;

impl EnvironmentAlertMonitor {
    pub fn new(sensors: SharedSensorRegistry, notification_client: NotificationClient) -> Self {
        Self {
            sensors,
            notification_client,
            last_alert_levels: Arc::new(RwLock::new(HashMap::new())),
            last_notification_time: Arc::new(RwLock::new(HashMap::new())),
            calculator: DewPointCalculator::default(),
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
        let (priority, title, body) = match evaluation.level {
            DewPointAlertLevel::Danger => (
                NotificationPriority::P0,
                format!("🚨 DANGER - {}", room_id),
                format!(
                    "Condensation certaine! {}\nAction: {}",
                    self.format_diagnostics(evaluation),
                    evaluation.level.suggestion()
                ),
            ),
            DewPointAlertLevel::Critical => (
                NotificationPriority::P0,
                format!("⚠️ CRITIQUE - {}", room_id),
                format!(
                    "Condensation très probable! {}\nAction: {}",
                    self.format_diagnostics(evaluation),
                    evaluation.level.suggestion()
                ),
            ),
            DewPointAlertLevel::Strong => (
                NotificationPriority::P1,
                format!("🟠 Risque condensation - {}", room_id),
                format!(
                    "Risque de condensation détecté. {}\nAction: {}",
                    self.format_diagnostics(evaluation),
                    evaluation.level.suggestion()
                ),
            ),
            DewPointAlertLevel::Moderate => (
                NotificationPriority::P1,
                format!("🟡 Humidité excessive - {}", room_id),
                format!(
                    "Humidité excessive prolongée. {}\nAction: {}",
                    self.format_diagnostics(evaluation),
                    evaluation.level.suggestion()
                ),
            ),
            DewPointAlertLevel::Weak => (
                NotificationPriority::P2,
                format!("💧 Humidité haute - {}", room_id),
                format!(
                    "Humidité en tendance haute. {}\nAction: {}",
                    self.format_diagnostics(evaluation),
                    evaluation.level.suggestion()
                ),
            ),
            DewPointAlertLevel::Safe => return, // Ne devrait pas arriver
        };

        let notification = NotificationPayload::new(priority, title, body, "environment-monitor");

        match self.notification_client.send(notification).await {
            Ok(true) => println!(
                "[env-alerts] Notification envoyée: {} ({:?})",
                room_id, evaluation.level
            ),
            Ok(false) => println!(
                "[env-alerts] Plugin indisponible, alerte ignorée: {} ({:?})",
                room_id, evaluation.level
            ),
            Err(e) => eprintln!("[env-alerts] Erreur notification: {}", e),
        }
    }

    /// Envoie une notification de retour à la normale
    async fn send_recovery_notification(&self, room_id: &str, previous_level: DewPointAlertLevel) {
        let notification = NotificationPayload::new(
            NotificationPriority::P2,
            format!("✅ Retour normal - {}", room_id),
            format!(
                "Les conditions sont revenues à la normale (était: {:?})",
                previous_level
            ),
            "environment-monitor",
        );

        let _ = self.notification_client.send(notification).await;
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
