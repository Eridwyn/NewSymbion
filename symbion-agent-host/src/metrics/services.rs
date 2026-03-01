//! System service status monitoring

use anyhow::Result;
use serde::Serialize;

/// System service status
#[derive(Debug, Serialize)]
pub struct ServiceStatus {
    pub name: String,
    pub status: ServiceState,
    pub enabled: Option<bool>,
}

/// Service state enumeration
#[derive(Debug, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum ServiceState {
    Active,
    Inactive,
    Failed,
    Unknown,
}

impl ServiceStatus {
    /// Collect status of critical system services
    pub async fn collect_critical() -> Result<Vec<Self>> {
        let critical_services = if cfg!(target_os = "linux") {
            vec!["ssh", "NetworkManager", "mosquitto", "symbion-kernel"]
        } else if cfg!(target_os = "windows") {
            vec!["Winmgmt", "EventLog", "Mosquitto"]
        } else {
            vec![]
        };

        let mut services = Vec::new();
        for name in critical_services {
            let status = Self::query_service(name).await;
            services.push(status);
        }

        Ok(services)
    }

    async fn query_service(name: &str) -> Self {
        if cfg!(target_os = "linux") {
            Self::query_linux_service(name).await
        } else if cfg!(target_os = "windows") {
            Self::query_windows_service(name).await
        } else {
            ServiceStatus {
                name: name.to_string(),
                status: ServiceState::Unknown,
                enabled: None,
            }
        }
    }

    async fn query_linux_service(name: &str) -> Self {
        use tokio::process::Command;

        let is_active = Command::new("systemctl")
            .args(["is-active", "--quiet", name])
            .output()
            .await
            .map(|o| o.status.success())
            .unwrap_or(false);

        let is_enabled = Command::new("systemctl")
            .args(["is-enabled", "--quiet", name])
            .output()
            .await
            .map(|o| o.status.success())
            .ok();

        let status = if is_active {
            ServiceState::Active
        } else {
            let state_output = Command::new("systemctl")
                .args(["is-failed", "--quiet", name])
                .output()
                .await
                .map(|o| o.status.success())
                .unwrap_or(false);

            if state_output {
                ServiceState::Failed
            } else {
                ServiceState::Inactive
            }
        };

        ServiceStatus {
            name: name.to_string(),
            status,
            enabled: is_enabled,
        }
    }

    async fn query_windows_service(name: &str) -> Self {
        let output = crate::windows_utils::silent_tokio_command("sc")
            .args(["query", name])
            .output()
            .await;

        let (status, enabled) = match output {
            Ok(o) => {
                let stdout = String::from_utf8_lossy(&o.stdout).to_string();
                let state = if stdout.contains("RUNNING") {
                    ServiceState::Active
                } else if stdout.contains("STOPPED") {
                    ServiceState::Inactive
                } else {
                    ServiceState::Unknown
                };
                (state, None)
            }
            Err(_) => (ServiceState::Unknown, None),
        };

        ServiceStatus {
            name: name.to_string(),
            status,
            enabled,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_service_status() {
        let services = ServiceStatus::collect_critical().await.unwrap();
        assert!(!services.is_empty());
    }
}
