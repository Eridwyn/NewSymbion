//! File Transfer Protocol over MQTT for Symbion Agent Host
//!
//! Supports chunked file transfer with SHA-256 integrity verification.
//! - Files < 500KB: single chunk (base64)
//! - Files > 500KB: chunked at 500KB boundaries
//! - Max file size: 50MB
//! - Sandboxed to configured transfer directory

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::Instant;

use anyhow::{Result, bail};
use base64::Engine;
use base64::engine::general_purpose::STANDARD as BASE64;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use tokio::sync::Mutex;
use tracing::{debug, error, info, warn};

/// Maximum file size for transfer (50 MB)
const MAX_FILE_SIZE: u64 = 50 * 1024 * 1024;
/// Chunk size (500 KB, base64 becomes ~667 KB, under 1MB MQTT limit)
const CHUNK_SIZE: usize = 500 * 1024;
/// Timeout for incomplete transfers (60 seconds)
const TRANSFER_TIMEOUT_SECS: u64 = 60;

/// File transfer request direction
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum TransferDirection {
    Upload,   // kernel → agent (agent receives)
    Download, // agent → kernel (agent sends)
}

/// File transfer request message
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TransferRequest {
    pub transfer_id: String,
    pub direction: TransferDirection,
    pub filename: String,
    pub file_size: u64,
    pub total_chunks: u32,
    pub sha256: String,
    pub agent_id: String,
    pub timestamp: DateTime<Utc>,
}

/// A single file chunk
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileChunk {
    pub transfer_id: String,
    pub chunk_index: u32,
    pub data: String, // base64 encoded
    pub chunk_sha256: String,
}

/// Transfer completion acknowledgement
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TransferComplete {
    pub transfer_id: String,
    pub success: bool,
    pub error: Option<String>,
    pub bytes_transferred: u64,
}

/// Tracks an in-progress upload (receiving chunks)
struct InProgressUpload {
    request: TransferRequest,
    chunks: HashMap<u32, Vec<u8>>,
    received_count: u32,
    started_at: Instant,
    dest_path: PathBuf,
}

/// File transfer manager
pub struct FileTransferManager {
    transfer_dir: PathBuf,
    uploads: Arc<Mutex<HashMap<String, InProgressUpload>>>,
}

impl FileTransferManager {
    /// Create a new file transfer manager with a sandboxed transfer directory
    pub fn new(transfer_dir: PathBuf) -> Self {
        Self {
            transfer_dir,
            uploads: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    /// Get the transfer directory path
    pub fn transfer_dir(&self) -> &Path {
        &self.transfer_dir
    }

    /// Ensure the transfer directory exists
    pub async fn ensure_dir(&self) -> Result<()> {
        tokio::fs::create_dir_all(&self.transfer_dir).await?;
        Ok(())
    }

    /// Validate a filename: no path traversal, no absolute paths
    pub fn validate_filename(filename: &str) -> Result<()> {
        if filename.contains("..") {
            bail!("Path traversal detected in filename");
        }
        if filename.starts_with('/') || filename.starts_with('\\') {
            bail!("Absolute paths not allowed");
        }
        if filename.contains('\0') {
            bail!("Null bytes not allowed in filename");
        }
        Ok(())
    }

    /// Start receiving an upload (kernel → agent)
    pub async fn start_upload(&self, request: TransferRequest) -> Result<()> {
        Self::validate_filename(&request.filename)?;

        if request.file_size > MAX_FILE_SIZE {
            bail!("File too large: {} bytes (max {})", request.file_size, MAX_FILE_SIZE);
        }

        let dest_path = self.transfer_dir.join(&request.filename);

        let mut uploads = self.uploads.lock().await;
        uploads.insert(request.transfer_id.clone(), InProgressUpload {
            request,
            chunks: HashMap::new(),
            received_count: 0,
            started_at: Instant::now(),
            dest_path,
        });

        Ok(())
    }

    /// Receive a chunk for an in-progress upload.
    /// Returns Some(TransferComplete) when all chunks received and assembled.
    pub async fn receive_chunk(&self, chunk: FileChunk) -> Result<Option<TransferComplete>> {
        let mut uploads = self.uploads.lock().await;

        let upload = match uploads.get_mut(&chunk.transfer_id) {
            Some(u) => u,
            None => bail!("Unknown transfer: {}", chunk.transfer_id),
        };

        // Decode base64
        let data = BASE64.decode(&chunk.data)?;

        // Verify chunk SHA-256
        let mut hasher = Sha256::new();
        hasher.update(&data);
        let chunk_hash = format!("{:x}", hasher.finalize());
        if chunk_hash != chunk.chunk_sha256 {
            bail!("Chunk {} integrity check failed: expected {}, got {}",
                  chunk.chunk_index, chunk.chunk_sha256, chunk_hash);
        }

        upload.chunks.insert(chunk.chunk_index, data);
        upload.received_count += 1;

        debug!("[file_transfer] Received chunk {}/{} for transfer {}",
               upload.received_count, upload.request.total_chunks, chunk.transfer_id);

        // Check if all chunks received
        if upload.received_count >= upload.request.total_chunks {
            let transfer_id = chunk.transfer_id.clone();
            let upload = uploads.remove(&transfer_id).unwrap();
            return Ok(Some(self.assemble_file(upload).await));
        }

        Ok(None)
    }

    /// Prepare a file for download (agent → kernel): read and chunk it
    pub async fn prepare_download(&self, filename: &str) -> Result<(TransferRequest, Vec<FileChunk>)> {
        Self::validate_filename(filename)?;

        let file_path = self.transfer_dir.join(filename);
        if !file_path.exists() {
            bail!("File not found: {}", filename);
        }

        let data = tokio::fs::read(&file_path).await?;
        if data.len() as u64 > MAX_FILE_SIZE {
            bail!("File too large: {} bytes", data.len());
        }

        // Calculate file SHA-256
        let mut file_hasher = Sha256::new();
        file_hasher.update(&data);
        let file_sha256 = format!("{:x}", file_hasher.finalize());

        let total_chunks = ((data.len() + CHUNK_SIZE - 1) / CHUNK_SIZE) as u32;
        let transfer_id = uuid::Uuid::new_v4().to_string();

        let request = TransferRequest {
            transfer_id: transfer_id.clone(),
            direction: TransferDirection::Download,
            filename: filename.to_string(),
            file_size: data.len() as u64,
            total_chunks,
            sha256: file_sha256,
            agent_id: String::new(), // filled by caller
            timestamp: Utc::now(),
        };

        let mut chunks = Vec::new();
        for (i, chunk_data) in data.chunks(CHUNK_SIZE).enumerate() {
            let encoded = BASE64.encode(chunk_data);

            let mut chunk_hasher = Sha256::new();
            chunk_hasher.update(chunk_data);
            let chunk_sha256 = format!("{:x}", chunk_hasher.finalize());

            chunks.push(FileChunk {
                transfer_id: transfer_id.clone(),
                chunk_index: i as u32,
                data: encoded,
                chunk_sha256,
            });
        }

        Ok((request, chunks))
    }

    /// Clean up timed-out transfers
    pub async fn cleanup_stale(&self) {
        let mut uploads = self.uploads.lock().await;
        let before = uploads.len();
        uploads.retain(|_, u| u.started_at.elapsed().as_secs() < TRANSFER_TIMEOUT_SECS);
        let removed = before - uploads.len();
        if removed > 0 {
            warn!("[file_transfer] Cleaned up {} stale transfers", removed);
        }
    }

    /// List files in the transfer directory
    pub async fn list_files(&self) -> Result<Vec<FileInfo>> {
        let mut files = Vec::new();
        let mut entries = tokio::fs::read_dir(&self.transfer_dir).await?;

        while let Some(entry) = entries.next_entry().await? {
            if let Ok(metadata) = entry.metadata().await {
                if metadata.is_file() {
                    files.push(FileInfo {
                        name: entry.file_name().to_string_lossy().to_string(),
                        size: metadata.len(),
                    });
                }
            }
        }

        Ok(files)
    }

    /// Assemble chunks into a file
    async fn assemble_file(&self, upload: InProgressUpload) -> TransferComplete {
        let mut assembled = Vec::with_capacity(upload.request.file_size as usize);

        for i in 0..upload.request.total_chunks {
            match upload.chunks.get(&i) {
                Some(data) => assembled.extend_from_slice(data),
                None => {
                    return TransferComplete {
                        transfer_id: upload.request.transfer_id,
                        success: false,
                        error: Some(format!("Missing chunk {}", i)),
                        bytes_transferred: 0,
                    };
                }
            }
        }

        // Verify file SHA-256
        let mut hasher = Sha256::new();
        hasher.update(&assembled);
        let file_hash = format!("{:x}", hasher.finalize());

        if file_hash != upload.request.sha256 {
            return TransferComplete {
                transfer_id: upload.request.transfer_id,
                success: false,
                error: Some(format!("File integrity check failed: expected {}, got {}",
                                    upload.request.sha256, file_hash)),
                bytes_transferred: 0,
            };
        }

        // Write to disk
        if let Err(e) = tokio::fs::write(&upload.dest_path, &assembled).await {
            return TransferComplete {
                transfer_id: upload.request.transfer_id,
                success: false,
                error: Some(format!("Failed to write file: {}", e)),
                bytes_transferred: 0,
            };
        }

        info!("[file_transfer] File assembled: {} ({} bytes)",
              upload.request.filename, assembled.len());

        TransferComplete {
            transfer_id: upload.request.transfer_id,
            success: true,
            error: None,
            bytes_transferred: assembled.len() as u64,
        }
    }
}

/// Basic file info for listing
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileInfo {
    pub name: String,
    pub size: u64,
}

#[cfg(test)]
mod tests {
    use super::*;

    fn temp_transfer_dir() -> PathBuf {
        let dir = std::env::temp_dir().join(format!("symbion-ft-test-{}", uuid::Uuid::new_v4()));
        std::fs::create_dir_all(&dir).unwrap();
        dir
    }

    #[test]
    fn test_validate_filename_ok() {
        assert!(FileTransferManager::validate_filename("report.txt").is_ok());
        assert!(FileTransferManager::validate_filename("logs/agent.log").is_ok());
    }

    #[test]
    fn test_validate_filename_traversal() {
        assert!(FileTransferManager::validate_filename("../etc/passwd").is_err());
        assert!(FileTransferManager::validate_filename("foo/../../bar").is_err());
    }

    #[test]
    fn test_validate_filename_absolute() {
        assert!(FileTransferManager::validate_filename("/etc/passwd").is_err());
        assert!(FileTransferManager::validate_filename("\\Windows\\System32").is_err());
    }

    #[test]
    fn test_validate_filename_null_bytes() {
        assert!(FileTransferManager::validate_filename("foo\0bar").is_err());
    }

    #[tokio::test]
    async fn test_prepare_download_small_file() {
        let dir = temp_transfer_dir();
        let mgr = FileTransferManager::new(dir.clone());
        tokio::fs::write(dir.join("test.txt"), "hello world").await.unwrap();

        let (req, chunks) = mgr.prepare_download("test.txt").await.unwrap();
        assert_eq!(req.total_chunks, 1);
        assert_eq!(req.file_size, 11);
        assert_eq!(chunks.len(), 1);
        assert_eq!(req.direction, TransferDirection::Download);

        let _ = std::fs::remove_dir_all(&dir);
    }

    #[tokio::test]
    async fn test_upload_single_chunk() {
        let dir = temp_transfer_dir();
        let mgr = FileTransferManager::new(dir.clone());
        mgr.ensure_dir().await.unwrap();

        let data = b"test file content";
        let encoded = BASE64.encode(data);

        let mut hasher = Sha256::new();
        hasher.update(data);
        let data_hash = format!("{:x}", hasher.finalize());

        let request = TransferRequest {
            transfer_id: "t1".to_string(),
            direction: TransferDirection::Upload,
            filename: "uploaded.txt".to_string(),
            file_size: data.len() as u64,
            total_chunks: 1,
            sha256: data_hash.clone(),
            agent_id: "agent-1".to_string(),
            timestamp: Utc::now(),
        };

        mgr.start_upload(request).await.unwrap();

        let chunk = FileChunk {
            transfer_id: "t1".to_string(),
            chunk_index: 0,
            data: encoded,
            chunk_sha256: data_hash,
        };

        let result = mgr.receive_chunk(chunk).await.unwrap().unwrap();
        assert!(result.success);
        assert_eq!(result.bytes_transferred, data.len() as u64);

        // Verify file on disk
        let content = tokio::fs::read(dir.join("uploaded.txt")).await.unwrap();
        assert_eq!(content, data);

        let _ = std::fs::remove_dir_all(&dir);
    }

    #[tokio::test]
    async fn test_upload_bad_checksum() {
        let dir = temp_transfer_dir();
        let mgr = FileTransferManager::new(dir.clone());
        mgr.ensure_dir().await.unwrap();

        let request = TransferRequest {
            transfer_id: "t2".to_string(),
            direction: TransferDirection::Upload,
            filename: "bad.txt".to_string(),
            file_size: 5,
            total_chunks: 1,
            sha256: "aaaa".to_string(),
            agent_id: "agent-1".to_string(),
            timestamp: Utc::now(),
        };

        mgr.start_upload(request).await.unwrap();

        let chunk = FileChunk {
            transfer_id: "t2".to_string(),
            chunk_index: 0,
            data: BASE64.encode(b"hello"),
            chunk_sha256: "wrong_hash".to_string(),
        };

        assert!(mgr.receive_chunk(chunk).await.is_err());

        let _ = std::fs::remove_dir_all(&dir);
    }

    #[tokio::test]
    async fn test_upload_file_too_large() {
        let dir = temp_transfer_dir();
        let mgr = FileTransferManager::new(dir.clone());

        let request = TransferRequest {
            transfer_id: "t3".to_string(),
            direction: TransferDirection::Upload,
            filename: "huge.bin".to_string(),
            file_size: MAX_FILE_SIZE + 1,
            total_chunks: 1,
            sha256: "abc".to_string(),
            agent_id: "agent-1".to_string(),
            timestamp: Utc::now(),
        };

        assert!(mgr.start_upload(request).await.is_err());
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[tokio::test]
    async fn test_upload_path_traversal() {
        let dir = temp_transfer_dir();
        let mgr = FileTransferManager::new(dir.clone());

        let request = TransferRequest {
            transfer_id: "t4".to_string(),
            direction: TransferDirection::Upload,
            filename: "../etc/evil.txt".to_string(),
            file_size: 10,
            total_chunks: 1,
            sha256: "abc".to_string(),
            agent_id: "agent-1".to_string(),
            timestamp: Utc::now(),
        };

        assert!(mgr.start_upload(request).await.is_err());
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[tokio::test]
    async fn test_list_files() {
        let dir = temp_transfer_dir();
        let mgr = FileTransferManager::new(dir.clone());
        mgr.ensure_dir().await.unwrap();

        tokio::fs::write(dir.join("a.txt"), "aaa").await.unwrap();
        tokio::fs::write(dir.join("b.txt"), "bbb").await.unwrap();

        let files = mgr.list_files().await.unwrap();
        assert_eq!(files.len(), 2);

        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn test_transfer_request_serialization() {
        let req = TransferRequest {
            transfer_id: "t1".to_string(),
            direction: TransferDirection::Upload,
            filename: "file.txt".to_string(),
            file_size: 1024,
            total_chunks: 2,
            sha256: "abc123".to_string(),
            agent_id: "agent-1".to_string(),
            timestamp: Utc::now(),
        };
        let json = serde_json::to_string(&req).unwrap();
        assert!(json.contains("upload"));
        let parsed: TransferRequest = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed.transfer_id, "t1");
    }

    #[test]
    fn test_file_chunk_serialization() {
        let chunk = FileChunk {
            transfer_id: "t1".to_string(),
            chunk_index: 0,
            data: "aGVsbG8=".to_string(),
            chunk_sha256: "abc".to_string(),
        };
        let json = serde_json::to_string(&chunk).unwrap();
        let parsed: FileChunk = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed.chunk_index, 0);
    }

    #[tokio::test]
    async fn test_cleanup_stale() {
        let dir = temp_transfer_dir();
        let mgr = FileTransferManager::new(dir.clone());

        // No stale transfers to clean up
        mgr.cleanup_stale().await;
        let _ = std::fs::remove_dir_all(&dir);
    }
}
