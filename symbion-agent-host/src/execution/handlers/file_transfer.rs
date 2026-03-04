//! File transfer command handler
//!
//! Handles: file_upload_start, file_download, list_files

use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;

use serde_json::Value;

use crate::execution::handler::{CommandHandler, CommandResult};
use crate::file_transfer::FileTransferManager;

/// Handler for file transfer commands
pub struct FileTransferHandler {
    manager: Arc<FileTransferManager>,
}

impl FileTransferHandler {
    pub fn new(manager: Arc<FileTransferManager>) -> Self {
        Self { manager }
    }
}

impl CommandHandler for FileTransferHandler {
    fn command_types(&self) -> &[&str] {
        &["list_files", "file_download"]
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
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_handler() -> FileTransferHandler {
        let dir = std::env::temp_dir().join(format!("symbion-fth-test-{}", uuid::Uuid::new_v4()));
        std::fs::create_dir_all(&dir).unwrap();
        FileTransferHandler::new(Arc::new(FileTransferManager::new(dir)))
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

    #[test]
    fn test_handler_command_types() {
        let handler = make_handler();
        let types = handler.command_types();
        assert!(types.contains(&"list_files"));
        assert!(types.contains(&"file_download"));
    }
}
