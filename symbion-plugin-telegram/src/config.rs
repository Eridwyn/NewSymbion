use std::collections::HashSet;
use std::path::PathBuf;

/// Plugin configuration loaded from environment variables
#[derive(Debug, Clone)]
#[allow(dead_code)]
pub struct Config {
    pub telegram_bot_token: String,
    pub allowed_user_ids: HashSet<i64>,
    pub claude_path: PathBuf,
    pub claude_timeout_secs: u64,
    pub claude_workdir: PathBuf,
    pub mqtt_broker_host: String,
    pub mqtt_broker_port: u16,
    pub socket_path: PathBuf,
    pub kernel_api_key: String,
}

impl Config {
    pub fn from_env() -> Result<Self, String> {
        // Load config.env file if present (same dir as binary or fallback)
        let env_paths = [
            PathBuf::from("scripts/telegram-bridge/config.env"),
            PathBuf::from("/home/eridwyn/RustroverProjects/NewSymbion/scripts/telegram-bridge/config.env"),
        ];
        for path in &env_paths {
            if path.exists() {
                if let Ok(contents) = std::fs::read_to_string(path) {
                    for line in contents.lines() {
                        let line = line.trim();
                        if line.is_empty() || line.starts_with('#') {
                            continue;
                        }
                        if let Some((key, value)) = line.split_once('=') {
                            // Only set if not already in env (env vars take precedence)
                            if std::env::var(key.trim()).is_err() {
                                std::env::set_var(key.trim(), value.trim());
                            }
                        }
                    }
                }
            }
        }

        let telegram_bot_token = std::env::var("TELEGRAM_BOT_TOKEN")
            .map_err(|_| "TELEGRAM_BOT_TOKEN is required")?;

        let allowed_ids_str = std::env::var("ALLOWED_USER_IDS")
            .unwrap_or_default();
        let allowed_user_ids: HashSet<i64> = allowed_ids_str
            .split(',')
            .filter_map(|s| s.trim().parse().ok())
            .collect();

        if allowed_user_ids.is_empty() {
            return Err("ALLOWED_USER_IDS must contain at least one Telegram user ID".into());
        }

        let claude_path = PathBuf::from(
            std::env::var("CLAUDE_PATH").unwrap_or_else(|_| "/usr/local/bin/claude".into()),
        );

        let claude_timeout_secs: u64 = std::env::var("CLAUDE_TIMEOUT")
            .unwrap_or_else(|_| "600".into())
            .parse()
            .unwrap_or(600);

        let claude_workdir = PathBuf::from(
            std::env::var("CLAUDE_WORKDIR")
                .unwrap_or_else(|_| "/home/eridwyn/RustroverProjects/NewSymbion".into()),
        );

        let mqtt_host_port = std::env::var("SYMBION_MQTT_BROKER")
            .unwrap_or_else(|_| "127.0.0.1:1883".into());
        let (mqtt_broker_host, mqtt_broker_port) = if let Some((h, p)) = mqtt_host_port.rsplit_once(':') {
            (h.to_string(), p.parse().unwrap_or(1883))
        } else {
            (mqtt_host_port, 1883)
        };

        let socket_path = PathBuf::from(
            std::env::var("SYMBION_TELEGRAM_SOCKET")
                .unwrap_or_else(|_| "/run/symbion-plugins/telegram.sock".into()),
        );

        let kernel_api_key = std::env::var("SYMBION_API_KEY")
            .unwrap_or_else(|_| "s3cr3t-42".into());

        Ok(Config {
            telegram_bot_token,
            allowed_user_ids,
            claude_path,
            claude_timeout_secs,
            claude_workdir,
            mqtt_broker_host,
            mqtt_broker_port,
            socket_path,
            kernel_api_key,
        })
    }

    pub fn is_allowed(&self, user_id: i64) -> bool {
        self.allowed_user_ids.contains(&user_id)
    }
}
