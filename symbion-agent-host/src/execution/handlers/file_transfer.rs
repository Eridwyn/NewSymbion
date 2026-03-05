//! File transfer command handler
//!
//! Handles: list_files, file_download, file_pull, file_push, delete_file

use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;

use serde_json::Value;
use tracing::{info, warn};

use crate::execution::handler::{CommandHandler, CommandResult};
use crate::file_transfer::FileTransferManager;
use crate::kernel_client::KernelClient;

/// Handler for file transfer commands
pub struct FileTransferHandler {
    manager: Arc<FileTransferManager>,
    kernel_client: Arc<KernelClient>,
}

impl FileTransferHandler {
    pub fn new(manager: Arc<FileTransferManager>, kernel_client: Arc<KernelClient>) -> Self {
        Self { manager, kernel_client }
    }
}

impl CommandHandler for FileTransferHandler {
    fn command_types(&self) -> &[&str] {
        &["list_files", "file_download", "file_pull", "file_push", "delete_file"]
    }

    fn execute<'a>(
        &'a self,
        command_type: &'a str,
        params: Option<&'a Value>,
    ) -> Pin<Box<dyn Future<Output = CommandResult> + Send + 'a>> {
        Box::pin(async move {
            match command_type {
                "list_files" => self.handle_list().await,
                "file_download" => self.handle_download(params).await,
                "file_pull" => self.handle_pull(params).await,
                "file_push" => self.handle_push(params).await,
                "delete_file" => self.handle_delete(params).await,
                _ => CommandResult::error("UNKNOWN_COMMAND", "Unknown file transfer command"),
            }
        })
    }
}

impl FileTransferHandler {
    async fn handle_list(&self) -> CommandResult {
        match self.manager.list_files().await {
            Ok(files) => CommandResult::success(serde_json::json!({
                "files": files,
                "count": files.len(),
                "transfer_dir": self.manager.transfer_dir().to_string_lossy(),
            })),
            Err(e) => CommandResult::error("LIST_FAILED", e.to_string()),
        }
    }

    async fn handle_download(&self, params: Option<&Value>) -> CommandResult {
        let filename = match params.and_then(|p| p.get("filename")).and_then(|v| v.as_str()) {
            Some(f) => f,
            None => return CommandResult::error("MISSING_FILENAME", "file_download requires 'filename'"),
        };

        match self.manager.prepare_download(filename).await {
            Ok((request, chunks)) => CommandResult::success(serde_json::json!({
                "transfer": request,
                "chunks": chunks,
            })),
            Err(e) => CommandResult::error("DOWNLOAD_FAILED", e.to_string()),
        }
    }

    /// Pull a file FROM the kernel (upload flow: PWA → kernel → agent pulls via HTTPS)
    async fn handle_pull(&self, params: Option<&Value>) -> CommandResult {
        let transfer_id = match params.and_then(|p| p.get("transfer_id")).and_then(|v| v.as_str()) {
            Some(v) => v,
            None => return CommandResult::error("MISSING_PARAM", "file_pull requires 'transfer_id'"),
        };
        let token = match params.and_then(|p| p.get("token")).and_then(|v| v.as_str()) {
            Some(v) => v,
            None => return CommandResult::error("MISSING_PARAM", "file_pull requires 'token'"),
        };
        let filename = match params.and_then(|p| p.get("filename")).and_then(|v| v.as_str()) {
            Some(v) => v,
            None => return CommandResult::error("MISSING_PARAM", "file_pull requires 'filename'"),
        };
        let kernel_port = params.and_then(|p| p.get("kernel_port")).and_then(|v| v.as_u64()).map(|v| v as u16);

        // Validate filename before accepting
        if let Err(e) = FileTransferManager::validate_filename(filename) {
            warn!("[file_pull] Rejected filename '{}': {}", filename, e);
            return CommandResult::error("INVALID_FILENAME", e.to_string());
        }

        let dest = self.manager.transfer_dir().join(filename);

        info!("[file_pull] Pulling {} from kernel (transfer={})", filename, transfer_id);

        match self.kernel_client.pull_file(transfer_id, token, &dest, kernel_port).await {
            Ok(bytes) => {
                info!("[file_pull] Successfully pulled {} ({} bytes)", filename, bytes);
                CommandResult::success(serde_json::json!({
                    "message": "File pulled from kernel",
                    "filename": filename,
                    "bytes": bytes,
                    "path": dest.to_string_lossy(),
                }))
            }
            Err(e) => {
                warn!("[file_pull] Failed: {}", e);
                CommandResult::error("PULL_FAILED", e.to_string())
            }
        }
    }

    /// Push a file TO the kernel (download flow: agent pushes via HTTPS → kernel stores → PWA downloads)
    async fn handle_push(&self, params: Option<&Value>) -> CommandResult {
        let transfer_id = match params.and_then(|p| p.get("transfer_id")).and_then(|v| v.as_str()) {
            Some(v) => v,
            None => return CommandResult::error("MISSING_PARAM", "file_push requires 'transfer_id'"),
        };
        let token = match params.and_then(|p| p.get("token")).and_then(|v| v.as_str()) {
            Some(v) => v,
            None => return CommandResult::error("MISSING_PARAM", "file_push requires 'token'"),
        };
        let filename = match params.and_then(|p| p.get("filename")).and_then(|v| v.as_str()) {
            Some(v) => v,
            None => return CommandResult::error("MISSING_PARAM", "file_push requires 'filename'"),
        };
        let kernel_port = params.and_then(|p| p.get("kernel_port")).and_then(|v| v.as_u64()).map(|v| v as u16);

        if let Err(e) = FileTransferManager::validate_filename(filename) {
            warn!("[file_push] Rejected filename '{}': {}", filename, e);
            return CommandResult::error("INVALID_FILENAME", e.to_string());
        }

        let file_path = self.manager.transfer_dir().join(filename);
        if !file_path.exists() {
            return CommandResult::error("FILE_NOT_FOUND", format!("File '{}' not found in transfer directory", filename));
        }

        info!("[file_push] Pushing {} to kernel (transfer={})", filename, transfer_id);

        match self.kernel_client.push_file(transfer_id, token, &file_path, kernel_port).await {
            Ok(()) => {
                info!("[file_push] Successfully pushed {}", filename);
                CommandResult::success(serde_json::json!({
                    "message": "File pushed to kernel",
                    "filename": filename,
                }))
            }
            Err(e) => {
                warn!("[file_push] Failed: {}", e);
                CommandResult::error("PUSH_FAILED", e.to_string())
            }
        }
    }

    /// Delete a file from the agent's transfer directory
    async fn handle_delete(&self, params: Option<&Value>) -> CommandResult {
        let filename = match params.and_then(|p| p.get("filename")).and_then(|v| v.as_str()) {
            Some(f) => f,
            None => return CommandResult::error("MISSING_FILENAME", "delete_file requires 'filename'"),
        };

        if let Err(e) = FileTransferManager::validate_filename(filename) {
            warn!("[delete_file] Rejected filename '{}': {}", filename, e);
            return CommandResult::error("INVALID_FILENAME", e.to_string());
        }

        let file_path = self.manager.transfer_dir().join(filename);
        if !file_path.exists() {
            return CommandResult::error("FILE_NOT_FOUND", format!("File '{}' not found", filename));
        }

        match tokio::fs::remove_file(&file_path).await {
            Ok(()) => {
                info!("[delete_file] Deleted {}", filename);
                CommandResult::success(serde_json::json!({
                    "message": "File deleted",
                    "filename": filename,
                }))
            }
            Err(e) => {
                warn!("[delete_file] Failed to delete {}: {}", filename, e);
                CommandResult::error("DELETE_FAILED", e.to_string())
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_handler() -> FileTransferHandler {
        let dir = std::env::temp_dir().join(format!("symbion-fth-test-{}", uuid::Uuid::new_v4()));
        std::fs::create_dir_all(&dir).unwrap();
        let kernel_client = Arc::new(KernelClient::new(None, "127.0.0.1", false));
        FileTransferHandler::new(Arc::new(FileTransferManager::new(dir)), kernel_client)
    }

    #[tokio::test]
    async fn test_list_files_empty() {
        let handler = make_handler();
        let result = handler.execute("list_files", None).await;
        assert_eq!(result.status, "success");
        assert_eq!(result.data.as_ref().unwrap()["count"], 0);
    }

    #[tokio::test]
    async fn test_download_missing_filename() {
        let handler = make_handler();
        let result = handler.execute("file_download", None).await;
        assert_eq!(result.status, "error");
        assert_eq!(result.error.unwrap().code, "MISSING_FILENAME");
    }

    #[tokio::test]
    async fn test_download_file_not_found() {
        let handler = make_handler();
        let params = serde_json::json!({"filename": "nonexistent.txt"});
        let result = handler.execute("file_download", Some(&params)).await;
        assert_eq!(result.status, "error");
    }

    #[tokio::test]
    async fn test_pull_missing_params() {
        let handler = make_handler();
        let result = handler.execute("file_pull", None).await;
        assert_eq!(result.status, "error");
        assert_eq!(result.error.unwrap().code, "MISSING_PARAM");
    }

    #[tokio::test]
    async fn test_push_file_not_found() {
        let handler = make_handler();
        let params = serde_json::json!({
            "transfer_id": "test-123",
            "token": "tok-456",
            "filename": "nonexistent.txt"
        });
        let result = handler.execute("file_push", Some(&params)).await;
        assert_eq!(result.status, "error");
        assert_eq!(result.error.unwrap().code, "FILE_NOT_FOUND");
    }

    #[tokio::test]
    async fn test_delete_file_not_found() {
        let handler = make_handler();
        let params = serde_json::json!({"filename": "nonexistent.txt"});
        let result = handler.execute("delete_file", Some(&params)).await;
        assert_eq!(result.status, "error");
        assert_eq!(result.error.unwrap().code, "FILE_NOT_FOUND");
    }

    #[tokio::test]
    async fn test_delete_file_invalid_name() {
        let handler = make_handler();
        let params = serde_json::json!({"filename": "../etc/passwd"});
        let result = handler.execute("delete_file", Some(&params)).await;
        assert_eq!(result.status, "error");
        assert_eq!(result.error.unwrap().code, "INVALID_FILENAME");
    }

    #[test]
    fn test_handler_command_types() {
        let handler = make_handler();
        let types = handler.command_types();
        assert!(types.contains(&"list_files"));
        assert!(types.contains(&"file_download"));
        assert!(types.contains(&"file_pull"));
        assert!(types.contains(&"file_push"));
        assert!(types.contains(&"delete_file"));
    }
}
