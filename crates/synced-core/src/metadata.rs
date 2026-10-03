use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use tokio::fs;

/// Persistent snapshot of an individual download segment.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct SegmentState {
    pub id: usize,
    pub start: u64,
    pub current: u64,
    pub end: u64,
    pub completed: bool,
}

/// Persistent snapshot of overall download progress.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DownloadState {
    pub url: String,
    pub file_name: String,
    pub total_size: u64,
    pub etag: Option<String>,
    pub segments: Vec<SegmentState>,
}

/// Derives the companion metadata file path for a destination target.
pub fn metadata_path(destination: &Path) -> PathBuf {
    let mut path = destination.as_os_str().to_os_string();
    path.push(".synced");
    PathBuf::from(path)
}

/// Writes download snapshot to disk as JSON.
pub async fn save_state(path: &Path, state: &DownloadState) -> Result<(), std::io::Error> {
    let content = serde_json::to_vec_pretty(state)
        .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))?;

    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).await?;
    }

    fs::write(path, content).await
}

/// Loads an existing download snapshot from disk if present.
pub async fn load_state(path: &Path) -> Result<Option<DownloadState>, std::io::Error> {
    if !path.exists() {
        return Ok(None);
    }

    let bytes = fs::read(path).await?;
    let state = serde_json::from_slice::<DownloadState>(&bytes)
        .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))?;

    Ok(Some(state))
}

/// Deletes the companion metadata file once download finishes.
pub async fn delete_state(path: &Path) -> Result<(), std::io::Error> {
    if path.exists() {
        fs::remove_file(path).await?;
    }
    Ok(())
}
