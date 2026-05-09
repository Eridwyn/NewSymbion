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

/// Parse `MQTT_BROKER` "host:port" en (host, port). Port défaut 1883 si absent ou invalide.
pub fn parse_mqtt_endpoint(s: &str) -> (String, u16) {
    if let Some((h, p)) = s.rsplit_once(':') {
        (h.to_string(), p.parse().unwrap_or(1883))
    } else {
        (s.to_string(), 1883)
    }
}

/// Parse une liste d'IDs séparés par virgule en HashSet, ignore les invalides.
pub fn parse_allowed_ids(s: &str) -> HashSet<i64> {
    s.split(',').filter_map(|p| p.trim().parse().ok()).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_config(allowed: &[i64]) -> Config {
        Config {
            telegram_bot_token: "test".into(),
            allowed_user_ids: allowed.iter().copied().collect(),
            claude_path: PathBuf::from("/usr/local/bin/claude"),
            claude_timeout_secs: 600,
            claude_workdir: PathBuf::from("/tmp"),
            mqtt_broker_host: "localhost".into(),
            mqtt_broker_port: 1883,
            socket_path: PathBuf::from("/tmp/test.sock"),
            kernel_api_key: "key".into(),
        }
    }

    #[test]
    fn is_allowed_true_for_known_id() {
        let cfg = make_config(&[42, 100]);
        assert!(cfg.is_allowed(42));
        assert!(cfg.is_allowed(100));
    }

    #[test]
    fn is_allowed_false_for_unknown_id() {
        let cfg = make_config(&[42]);
        assert!(!cfg.is_allowed(99));
        assert!(!cfg.is_allowed(0));
        assert!(!cfg.is_allowed(-1));
    }

    #[test]
    fn is_allowed_false_for_empty_set() {
        let cfg = make_config(&[]);
        assert!(!cfg.is_allowed(42));
    }

    #[test]
    fn parse_mqtt_endpoint_host_port() {
        assert_eq!(parse_mqtt_endpoint("127.0.0.1:1883"), ("127.0.0.1".into(), 1883));
        assert_eq!(parse_mqtt_endpoint("broker.example.com:8883"), ("broker.example.com".into(), 8883));
    }

    #[test]
    fn parse_mqtt_endpoint_no_port_falls_back() {
        assert_eq!(parse_mqtt_endpoint("localhost"), ("localhost".into(), 1883));
    }

    #[test]
    fn parse_mqtt_endpoint_invalid_port_falls_back() {
        assert_eq!(parse_mqtt_endpoint("host:abc"), ("host".into(), 1883));
    }

    #[test]
    fn parse_allowed_ids_basic() {
        let s: Vec<i64> = parse_allowed_ids("1,2,3").into_iter().collect();
        let mut sorted = s;
        sorted.sort();
        assert_eq!(sorted, vec![1, 2, 3]);
    }

    #[test]
    fn parse_allowed_ids_with_spaces_and_invalid() {
        let s = parse_allowed_ids(" 42 , bad , 100 ,");
        assert_eq!(s.len(), 2);
        assert!(s.contains(&42));
        assert!(s.contains(&100));
    }

    #[test]
    fn parse_allowed_ids_empty_string() {
        assert!(parse_allowed_ids("").is_empty());
    }
}
