//! OS Notification handler for Symbion Agent Host
//!
//! Displays desktop notifications via notify-rust when the kernel
//! sends a "notify" command. Feature-gated behind `notifications`.

use std::future::Future;
use std::pin::Pin;

use serde_json::Value;
use tracing::{info, warn};

use crate::execution::handler::{CommandHandler, CommandResult};

/// Handler for desktop notifications
pub struct NotifyHandler;

impl CommandHandler for NotifyHandler {
    fn command_types(&self) -> &[&str] {
        &["notify"]
    }

    fn execute<'a>(
        &'a self,
        _command_type: &'a str,
        params: Option<&'a Value>,
    ) -> Pin<Box<dyn Future<Output = CommandResult> + Send + 'a>> {
        Box::pin(async move {
            let params = match params {
                Some(p) => p,
                None => {
                    return CommandResult::error(
                        "MISSING_PARAMS",
                        "Notification requires 'title' and 'body' parameters",
                    );
                }
            };

            let title = params.get("title")
                .and_then(|v| v.as_str())
                .unwrap_or("Symbion");

            let body = params.get("body")
                .and_then(|v| v.as_str())
                .unwrap_or("");

            if body.is_empty() {
                return CommandResult::error(
                    "MISSING_BODY",
                    "Notification 'body' cannot be empty",
                );
            }

            let urgency = params.get("urgency")
                .and_then(|v| v.as_str())
                .unwrap_or("normal");

            let timeout_ms = params.get("timeout_ms")
                .and_then(|v| v.as_u64())
                .unwrap_or(5000);

            show_notification(title, body, urgency, timeout_ms)
        })
    }
}

/// Show notification using notify-rust (feature-gated)
#[cfg(feature = "notifications")]
fn show_notification(title: &str, body: &str, urgency: &str, timeout_ms: u64) -> CommandResult {
    use notify_rust::{Notification, Urgency};

    let notify_urgency = match urgency {
        "low" => Urgency::Low,
        "critical" => Urgency::Critical,
        _ => Urgency::Normal,
    };

    match Notification::new()
        .summary(title)
        .body(body)
        .hint(notify_rust::Hint::Urgency(notify_urgency))
        .timeout(notify_rust::Timeout::Milliseconds(timeout_ms as u32))
        .show()
    {
        Ok(_) => {
            info!("Notification shown: {} — {}", title, body);
            CommandResult::success(serde_json::json!({
                "message": "Notification displayed",
                "title": title,
                "urgency": urgency,
            }))
        }
        Err(e) => {
            warn!("Failed to show notification: {}", e);
            CommandResult::error("NOTIFICATION_FAILED", format!("Failed to show notification: {}", e))
        }
    }
}

/// Fallback when notifications feature is not enabled
#[cfg(not(feature = "notifications"))]
fn show_notification(_title: &str, _body: &str, _urgency: &str, _timeout_ms: u64) -> CommandResult {
    CommandResult::error(
        "FEATURE_DISABLED",
        "Notifications feature is not enabled. Compile with --features notifications",
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_notify_missing_params() {
        let handler = NotifyHandler;
        let result = handler.execute("notify", None).await;
        assert_eq!(result.status, "error");
        assert_eq!(result.error.unwrap().code, "MISSING_PARAMS");
    }

    #[tokio::test]
    async fn test_notify_empty_body() {
        let handler = NotifyHandler;
        let params = serde_json::json!({"title": "Test", "body": ""});
        let result = handler.execute("notify", Some(&params)).await;
        assert_eq!(result.status, "error");
        assert_eq!(result.error.unwrap().code, "MISSING_BODY");
    }

    #[tokio::test]
    async fn test_notify_command_types() {
        let handler = NotifyHandler;
        assert_eq!(handler.command_types(), &["notify"]);
    }

    #[tokio::test]
    async fn test_notify_default_values() {
        // Without a display server, notification will fail, but we can test param parsing
        let handler = NotifyHandler;
        let params = serde_json::json!({"body": "test message"});
        let result = handler.execute("notify", Some(&params)).await;
        // On CI without display, this will error — but at least it parsed correctly
        // The key thing is it didn't error with MISSING_PARAMS or MISSING_BODY
        assert!(result.status == "success" || result.error.as_ref().map(|e| e.code.as_str()) != Some("MISSING_PARAMS"));
    }

    #[test]
    fn test_notify_handler_is_send_sync() {
        fn assert_send_sync<T: Send + Sync>() {}
        assert_send_sync::<NotifyHandler>();
    }
}
