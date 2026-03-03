//! Temperature and battery metrics

use serde::Serialize;
use sysinfo::Components;

/// Temperature sensor readings
#[derive(Debug, Clone, Serialize)]
pub struct TemperatureMetrics {
    pub cpu_celsius: Option<f32>,
    pub sensors: Vec<TemperatureSensor>,
}

/// Individual temperature sensor
#[derive(Debug, Clone, Serialize)]
pub struct TemperatureSensor {
    pub name: String,
    pub value: f32,
    pub unit: String,
    pub critical: Option<f32>,
}

impl TemperatureMetrics {
    pub fn collect() -> Option<Self> {
        let components = Components::new_with_refreshed_list();
        if components.is_empty() {
            return None;
        }

        let mut cpu_celsius: Option<f32> = None;
        let mut sensors = Vec::new();

        for component in components.iter() {
            let label = component.label().to_string();
            let temp = component.temperature();
            let critical = component.critical();

            if cpu_celsius.is_none() {
                let lower = label.to_lowercase();
                if lower.contains("cpu") || lower.contains("core") || lower.contains("package") || lower.contains("tctl") {
                    cpu_celsius = Some(temp);
                }
            }

            sensors.push(TemperatureSensor {
                name: label,
                value: temp,
                unit: "\u{00b0}C".to_string(),
                critical,
            });
        }

        Some(TemperatureMetrics {
            cpu_celsius,
            sensors,
        })
    }
}

/// Battery status (laptops / UPS)
#[derive(Debug, Clone, Serialize)]
pub struct BatteryMetrics {
    pub percent: f32,
    pub charging: bool,
    pub power_source: String,
}

impl BatteryMetrics {
    pub async fn collect() -> Option<Self> {
        #[cfg(target_os = "linux")]
        {
            Self::collect_linux().await
        }
        #[cfg(not(target_os = "linux"))]
        {
            None
        }
    }

    #[cfg(target_os = "linux")]
    async fn collect_linux() -> Option<Self> {
        use tokio::fs;

        let power_dir = "/sys/class/power_supply";
        let mut entries = fs::read_dir(power_dir).await.ok()?;

        while let Ok(Some(entry)) = entries.next_entry().await {
            let name = entry.file_name().to_string_lossy().to_string();
            if !name.starts_with("BAT") {
                continue;
            }

            let base = format!("{}/{}", power_dir, name);

            let capacity = fs::read_to_string(format!("{}/capacity", base))
                .await.ok()
                .and_then(|s| s.trim().parse::<f32>().ok())?;

            let status = fs::read_to_string(format!("{}/status", base))
                .await.ok()
                .map(|s| s.trim().to_string())
                .unwrap_or_default();

            let charging = status == "Charging" || status == "Full";
            let power_source = if charging { "AC" } else { "Battery" }.to_string();

            return Some(BatteryMetrics {
                percent: capacity,
                charging,
                power_source,
            });
        }

        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_temperature_metrics() {
        let temp = TemperatureMetrics::collect();
        if let Some(temp) = temp {
            assert!(!temp.sensors.is_empty());
            for sensor in &temp.sensors {
                assert!(sensor.value > -50.0 && sensor.value < 200.0);
            }
        }
    }
}
