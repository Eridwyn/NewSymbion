//! Screenshot capture handler for Symbion Agent Host
//!
//! Privacy-first: requires explicit `allow_screenshots` config.
//! Optionally sends a notification before capture.
//! Cross-platform: gnome-screenshot/scrot/grim on Linux, PowerShell on Windows.

use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;

use serde_json::Value;
use tracing::{info, warn};

use crate::execution::handler::{CommandHandler, CommandResult};
use crate::file_transfer::FileTransferManager;

/// Handler for screenshot capture
pub struct ScreenshotHandler {
    file_manager: Arc<FileTransferManager>,
}

impl ScreenshotHandler {
    pub fn new(file_manager: Arc<FileTransferManager>) -> Self {
        Self { file_manager }
    }
}

impl CommandHandler for ScreenshotHandler {
    fn command_types(&self) -> &[&str] {
        &["screenshot"]
    }

    fn execute<'a>(
        &'a self,
        _command_type: &'a str,
        params: Option<&'a Value>,
    ) -> Pin<Box<dyn Future<Output = CommandResult> + Send + 'a>> {
        Box::pin(async move {
            // Check privacy setting
            let allow = params
                .and_then(|p| p.get("allow_screenshots"))
                .and_then(|v| v.as_bool())
                .unwrap_or(false);

            if !allow {
                return CommandResult::error(
                    "SCREENSHOTS_DISABLED",
                    "Screenshots require 'allow_screenshots: true' in parameters",
                );
            }

            // Optional notification before capture
            let notify_before = params
                .and_then(|p| p.get("notify_before"))
                .and_then(|v| v.as_bool())
                .unwrap_or(true);

            if notify_before {
                #[cfg(feature = "notifications")]
                {
                    let _ = notify_rust::Notification::new()
                        .summary("Symbion")
                        .body("Screenshot capture in 2 seconds...")
                        .timeout(notify_rust::Timeout::Milliseconds(2000))
                        .show();
                    tokio::time::sleep(std::time::Duration::from_secs(2)).await;
                }
            }

            // Capture screenshot
            let filename = format!("screenshot_{}.png", chrono::Utc::now().format("%Y%m%d_%H%M%S"));
            let dest = self.file_manager.transfer_dir().join(&filename);

            match capture_screenshot(&dest).await {
                Ok(()) => {
                    info!("[screenshot] Captured: {}", filename);
                    CommandResult::success(serde_json::json!({
                        "message": "Screenshot captured",
                        "filename": filename,
                        "path": dest.to_string_lossy(),
                    }))
                }
                Err(e) => {
                    warn!("[screenshot] Capture failed: {}", e);
                    CommandResult::error("CAPTURE_FAILED", e.to_string())
                }
            }
        })
    }
}

/// Cross-platform screenshot capture
async fn capture_screenshot(dest: &std::path::Path) -> anyhow::Result<()> {
    let dest_str = dest.to_string_lossy().to_string();

    #[cfg(target_os = "linux")]
    {
        // Try gnome-screenshot first, then scrot, then grim (Wayland)
        let tools = [
            ("gnome-screenshot", vec!["-f", &dest_str]),
            ("scrot", vec![dest_str.clone().leak() as &str]),
            ("grim", vec![dest_str.clone().leak() as &str]),
        ];

        for (tool, args) in &tools {
            if let Ok(output) = tokio::process::Command::new(tool)
                .args(args)
                .output()
                .await
            {
                if output.status.success() {
                    return Ok(());
                }
            }
        }

        anyhow::bail!("No screenshot tool found (tried gnome-screenshot, scrot, grim)");
    }

    #[cfg(target_os = "windows")]
    {
        let script = format!(
            "Add-Type -AssemblyName System.Windows.Forms; \
             [System.Windows.Forms.Screen]::PrimaryScreen | ForEach-Object {{ \
               $bmp = New-Object System.Drawing.Bitmap($_.Bounds.Width, $_.Bounds.Height); \
               $g = [System.Drawing.Graphics]::FromImage($bmp); \
               $g.CopyFromScreen($_.Bounds.Location, [System.Drawing.Point]::Empty, $_.Bounds.Size); \
               $bmp.Save('{}'); \
             }}",
            dest_str.replace('\\', "\\\\")
        );

        let output = tokio::process::Command::new("powershell")
            .args(["-Command", &script])
            .output()
            .await?;

        if output.status.success() {
            return Ok(());
        }

        anyhow::bail!("PowerShell screenshot failed: {}", String::from_utf8_lossy(&output.stderr));
    }

    #[cfg(not(any(target_os = "linux", target_os = "windows")))]
    {
        let _ = dest_str;
        anyhow::bail!("Screenshot not supported on this platform");
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_handler() -> ScreenshotHandler {
        let dir = std::env::temp_dir().join(format!("symbion-ss-test-{}", uuid::Uuid::new_v4()));
        std::fs::create_dir_all(&dir).unwrap();
        ScreenshotHandler::new(Arc::new(FileTransferManager::new(dir)))
    }

    #[tokio::test]
    async fn test_screenshot_disabled_by_default() {
        let handler = make_handler();
        let result = handler.execute("screenshot", None).await;
        assert_eq!(result.status, "error");
        assert_eq!(result.error.unwrap().code, "SCREENSHOTS_DISABLED");
    }

    #[tokio::test]
    async fn test_screenshot_explicit_deny() {
        let handler = make_handler();
        let params = serde_json::json!({"allow_screenshots": false});
        let result = handler.execute("screenshot", Some(&params)).await;
        assert_eq!(result.status, "error");
        assert_eq!(result.error.unwrap().code, "SCREENSHOTS_DISABLED");
    }

    #[test]
    fn test_handler_command_types() {
        let handler = make_handler();
        assert_eq!(handler.command_types(), &["screenshot"]);
    }

    #[test]
    fn test_handler_is_send_sync() {
        fn assert_send_sync<T: Send + Sync>() {}
        assert_send_sync::<ScreenshotHandler>();
    }
}
