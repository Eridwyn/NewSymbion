//! File Hub — Kernel-side file transfer storage and token management
//!
//! The kernel acts as a secure relay for file transfers between PWA and agents.
//! Files transit through the kernel via HTTPS; MQTT is used only for signaling.

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use chrono::{DateTime, Utc};
use serde::Serialize;
use tokio::sync::Mutex;

/// Maximum file size (200 MB default, configurable via SYMBION_MAX_TRANSFER_SIZE)
const DEFAULT_MAX_FILE_SIZE: u64 = 200 * 1024 * 1024;

/// Transfer token expiry (5 minutes)
const TOKEN_EXPIRY_SECS: i64 = 300;

/// Transfer record expiry for cleanup (30 minutes)
const TRANSFER_EXPIRY_SECS: i64 = 1800;

#[derive(Debug, Clone, Serialize)]
pub enum TransferDirection {
    ToAgent,   // PWA uploads → kernel stores → agent pulls
    FromAgent, // agent pushes → kernel stores → PWA downloads
}

#[derive(Debug, Clone, Serialize, PartialEq)]
pub enum TransferStatus {
    Pending,          // Created, waiting for agent action
    AgentProcessing,  // Agent acknowledged, transfer in progress
    Completed,        // File successfully transferred
    Failed,           // Transfer failed
    Expired,          // Token or transfer expired
}

#[derive(Debug, Clone, Serialize)]
pub struct TransferRecord {
    pub transfer_id: String,
    pub agent_id: String,
    pub filename: String,
    pub file_size: u64,
    pub sha256: Option<String>,
    pub direction: TransferDirection,
    pub status: TransferStatus,
    #[serde(skip_serializing)]
    pub token: String,
    pub created_at: DateTime<Utc>,
    pub completed_at: Option<DateTime<Utc>>,
    pub error: Option<String>,
    /// One-time download token for PWA (generated on completion for FromAgent transfers)
    #[serde(skip_serializing)]
    pub download_token: Option<String>,
}

pub type SharedFileHub = Arc<FileHub>;

pub struct FileHub {
    transfer_dir: PathBuf,
    transfers: Mutex<HashMap<String, TransferRecord>>,
    max_file_size: u64,
}

impl FileHub {
    pub fn new(data_dir: &Path) -> Self {
        let transfer_dir = data_dir.join("transfers");
        std::fs::create_dir_all(&transfer_dir).ok();

        let max_size = std::env::var("SYMBION_MAX_TRANSFER_SIZE")
            .ok()
            .and_then(|s| s.parse::<u64>().ok())
            .unwrap_or(DEFAULT_MAX_FILE_SIZE);

        Self {
            transfer_dir,
            transfers: Mutex::new(HashMap::new()),
            max_file_size: max_size,
        }
    }

    pub fn max_file_size(&self) -> u64 {
        self.max_file_size
    }

    /// Create an upload transfer (PWA → Agent): stores the file, returns record with token for agent
    pub async fn create_upload(
        &self,
        agent_id: &str,
        filename: &str,
        data: bytes::Bytes,
    ) -> Result<TransferRecord, String> {
        Self::validate_filename(filename)?;

        let size = data.len() as u64;
        if size > self.max_file_size {
            return Err(format!(
                "File too large: {} bytes (max {})",
                size, self.max_file_size
            ));
        }

        let transfer_id = uuid::Uuid::new_v4().to_string();
        let token = uuid::Uuid::new_v4().to_string();

        // Store file on disk
        let transfer_path = self.transfer_dir.join(&transfer_id);
        std::fs::create_dir_all(&transfer_path)
            .map_err(|e| format!("Failed to create transfer dir: {}", e))?;

        let file_path = transfer_path.join(filename);
        tokio::fs::write(&file_path, &data)
            .await
            .map_err(|e| format!("Failed to write file: {}", e))?;

        // Calculate SHA-256
        let sha256 = {
            use sha2::{Sha256, Digest};
            let mut hasher = Sha256::new();
            hasher.update(&data);
            format!("{:x}", hasher.finalize())
        };

        let record = TransferRecord {
            transfer_id: transfer_id.clone(),
            agent_id: agent_id.to_string(),
            filename: filename.to_string(),
            file_size: size,
            sha256: Some(sha256),
            direction: TransferDirection::ToAgent,
            status: TransferStatus::Pending,
            token,
            created_at: Utc::now(),
            completed_at: None,
            error: None,
            download_token: None,
        };

        self.transfers
            .lock()
            .await
            .insert(transfer_id, record.clone());

        Ok(record)
    }

    /// Create a download request (Agent → PWA): creates a pending record with token for agent to push
    pub async fn create_download_request(
        &self,
        agent_id: &str,
        filename: &str,
    ) -> Result<TransferRecord, String> {
        Self::validate_filename(filename)?;

        let transfer_id = uuid::Uuid::new_v4().to_string();
        let token = uuid::Uuid::new_v4().to_string();

        // Create transfer directory
        let transfer_path = self.transfer_dir.join(&transfer_id);
        std::fs::create_dir_all(&transfer_path)
            .map_err(|e| format!("Failed to create transfer dir: {}", e))?;

        let record = TransferRecord {
            transfer_id: transfer_id.clone(),
            agent_id: agent_id.to_string(),
            filename: filename.to_string(),
            file_size: 0,
            sha256: None,
            direction: TransferDirection::FromAgent,
            status: TransferStatus::Pending,
            token,
            created_at: Utc::now(),
            completed_at: None,
            error: None,
            download_token: None,
        };

        self.transfers
            .lock()
            .await
            .insert(transfer_id, record.clone());

        Ok(record)
    }

    /// Validate a transfer token (non-consuming — agent may retry on network failure)
    pub async fn validate_token(&self, transfer_id: &str, token: &str) -> Result<(), String> {
        let transfers = self.transfers.lock().await;
        let record = transfers
            .get(transfer_id)
            .ok_or_else(|| "Transfer not found".to_string())?;

        if record.token != token {
            return Err("Invalid token".to_string());
        }

        // Check expiry
        let elapsed = Utc::now()
            .signed_duration_since(record.created_at)
            .num_seconds();
        if elapsed > TOKEN_EXPIRY_SECS {
            return Err("Token expired".to_string());
        }

        if record.status == TransferStatus::Completed || record.status == TransferStatus::Failed {
            return Err(format!("Transfer already {:?}", record.status));
        }

        Ok(())
    }

    /// Get the file path for a transfer (for serving/reading)
    pub fn get_file_path(&self, transfer_id: &str, filename: &str) -> Result<PathBuf, String> {
        Self::validate_filename(filename)?;
        Ok(self.transfer_dir.join(transfer_id).join(filename))
    }

    /// Store a file pushed by the agent (download flow: agent → kernel)
    pub async fn store_agent_file(
        &self,
        transfer_id: &str,
        token: &str,
        data: bytes::Bytes,
    ) -> Result<(), String> {
        self.validate_token(transfer_id, token).await?;

        let size = data.len() as u64;
        if size > self.max_file_size {
            return Err(format!("File too large: {} bytes", size));
        }

        let filename = {
            let transfers = self.transfers.lock().await;
            transfers
                .get(transfer_id)
                .map(|r| r.filename.clone())
                .ok_or_else(|| "Transfer not found".to_string())?
        };

        let file_path = self.get_file_path(transfer_id, &filename)?;
        tokio::fs::write(&file_path, &data)
            .await
            .map_err(|e| format!("Failed to write file: {}", e))?;

        // Calculate SHA-256
        let sha256 = {
            use sha2::{Sha256, Digest};
            let mut hasher = Sha256::new();
            hasher.update(&data);
            format!("{:x}", hasher.finalize())
        };

        // Update record
        let download_token = uuid::Uuid::new_v4().to_string();
        let mut transfers = self.transfers.lock().await;
        if let Some(record) = transfers.get_mut(transfer_id) {
            record.file_size = size;
            record.sha256 = Some(sha256);
            record.status = TransferStatus::Completed;
            record.completed_at = Some(Utc::now());
            record.download_token = Some(download_token);
        }

        Ok(())
    }

    /// Mark transfer as completed (upload flow: agent confirmed pull)
    pub async fn mark_completed(&self, transfer_id: &str) {
        let mut transfers = self.transfers.lock().await;
        if let Some(record) = transfers.get_mut(transfer_id) {
            record.status = TransferStatus::Completed;
            record.completed_at = Some(Utc::now());
        }
    }

    /// Mark transfer as failed
    pub async fn mark_failed(&self, transfer_id: &str, error: &str) {
        let mut transfers = self.transfers.lock().await;
        if let Some(record) = transfers.get_mut(transfer_id) {
            record.status = TransferStatus::Failed;
            record.error = Some(error.to_string());
            record.completed_at = Some(Utc::now());
        }
    }

    /// Get transfer status (for PWA polling)
    pub async fn get_status(&self, transfer_id: &str) -> Option<TransferRecord> {
        self.transfers.lock().await.get(transfer_id).cloned()
    }

    /// Get download token for a completed FromAgent transfer
    pub async fn get_download_info(&self, transfer_id: &str) -> Option<(String, String)> {
        let transfers = self.transfers.lock().await;
        let record = transfers.get(transfer_id)?;
        if record.status != TransferStatus::Completed {
            return None;
        }
        let token = record.download_token.clone()?;
        Some((record.filename.clone(), token))
    }

    /// Clean up expired transfers (called periodically)
    pub async fn cleanup_expired(&self) {
        let now = Utc::now();
        let mut transfers = self.transfers.lock().await;
        let expired: Vec<String> = transfers
            .iter()
            .filter(|(_, r)| {
                now.signed_duration_since(r.created_at).num_seconds() > TRANSFER_EXPIRY_SECS
            })
            .map(|(id, _)| id.clone())
            .collect();

        for id in &expired {
            transfers.remove(id);
            // Clean up files
            let dir = self.transfer_dir.join(id);
            tokio::fs::remove_dir_all(&dir).await.ok();
        }

        if !expired.is_empty() {
            eprintln!("[file-hub] Cleaned up {} expired transfers", expired.len());
        }
    }

    /// Validate filename (no path traversal, no absolute paths, no null bytes)
    fn validate_filename(filename: &str) -> Result<(), String> {
        if filename.is_empty() {
            return Err("Empty filename".to_string());
        }
        if filename.contains("..") {
            return Err("Path traversal detected".to_string());
        }
        if filename.starts_with('/') || filename.starts_with('\\') {
            return Err("Absolute path not allowed".to_string());
        }
        if filename.contains('\0') {
            return Err("Null byte in filename".to_string());
        }
        if filename.contains('/') || filename.contains('\\') {
            return Err("Directory separator not allowed".to_string());
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_validate_filename_valid() {
        assert!(FileHub::validate_filename("report.txt").is_ok());
        assert!(FileHub::validate_filename("my-file_2024.pdf").is_ok());
    }

    #[test]
    fn test_validate_filename_traversal() {
        assert!(FileHub::validate_filename("../etc/passwd").is_err());
        assert!(FileHub::validate_filename("..\\windows\\system32").is_err());
    }

    #[test]
    fn test_validate_filename_absolute() {
        assert!(FileHub::validate_filename("/etc/passwd").is_err());
        assert!(FileHub::validate_filename("\\windows\\system32").is_err());
    }

    #[test]
    fn test_validate_filename_null() {
        assert!(FileHub::validate_filename("file\0.txt").is_err());
    }

    #[test]
    fn test_validate_filename_empty() {
        assert!(FileHub::validate_filename("").is_err());
    }

    #[test]
    fn test_validate_filename_subdirectory() {
        assert!(FileHub::validate_filename("subdir/file.txt").is_err());
    }

    #[tokio::test]
    async fn test_create_upload() {
        let tmp = tempfile::tempdir().unwrap();
        let hub = FileHub::new(tmp.path());
        let data = bytes::Bytes::from("hello world");
        let record = hub.create_upload("agent-1", "test.txt", data).await.unwrap();

        assert_eq!(record.agent_id, "agent-1");
        assert_eq!(record.filename, "test.txt");
        assert_eq!(record.file_size, 11);
        assert!(record.sha256.is_some());
        assert_eq!(record.status, TransferStatus::Pending);
        assert!(!record.token.is_empty());
    }

    #[tokio::test]
    async fn test_create_upload_too_large() {
        let tmp = tempfile::tempdir().unwrap();
        let mut hub = FileHub::new(tmp.path());
        hub.max_file_size = 10; // 10 bytes max

        let data = bytes::Bytes::from("this is more than 10 bytes");
        let result = hub.create_upload("agent-1", "big.txt", data).await;
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("too large"));
    }

    #[tokio::test]
    async fn test_validate_token() {
        let tmp = tempfile::tempdir().unwrap();
        let hub = FileHub::new(tmp.path());
        let data = bytes::Bytes::from("test");
        let record = hub.create_upload("agent-1", "test.txt", data).await.unwrap();

        // Valid token
        assert!(hub.validate_token(&record.transfer_id, &record.token).await.is_ok());

        // Invalid token
        assert!(hub.validate_token(&record.transfer_id, "wrong").await.is_err());

        // Non-existent transfer
        assert!(hub.validate_token("nope", "whatever").await.is_err());
    }

    #[tokio::test]
    async fn test_mark_completed() {
        let tmp = tempfile::tempdir().unwrap();
        let hub = FileHub::new(tmp.path());
        let data = bytes::Bytes::from("test");
        let record = hub.create_upload("agent-1", "test.txt", data).await.unwrap();

        hub.mark_completed(&record.transfer_id).await;
        let status = hub.get_status(&record.transfer_id).await.unwrap();
        assert_eq!(status.status, TransferStatus::Completed);
        assert!(status.completed_at.is_some());
    }

    #[tokio::test]
    async fn test_cleanup_expired() {
        let tmp = tempfile::tempdir().unwrap();
        let hub = FileHub::new(tmp.path());
        let data = bytes::Bytes::from("test");
        let record = hub.create_upload("agent-1", "test.txt", data).await.unwrap();

        // Manually set created_at to 1 hour ago
        {
            let mut transfers = hub.transfers.lock().await;
            if let Some(r) = transfers.get_mut(&record.transfer_id) {
                r.created_at = Utc::now() - chrono::Duration::hours(1);
            }
        }

        hub.cleanup_expired().await;
        assert!(hub.get_status(&record.transfer_id).await.is_none());
    }
}
