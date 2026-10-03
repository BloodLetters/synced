/// Snapshot describing initial chunk progress on start or resume.
#[derive(Debug, Clone)]
pub struct ChunkInitInfo {
    pub id: usize,
    pub downloaded_bytes: u64,
    pub total_bytes: u64,
}

/// Progress and lifecycle events emitted during download operations.
#[derive(Debug, Clone)]
pub enum DownloadEvent {
    Probed {
        file_name: String,
        total_size: Option<u64>,
        supports_ranges: bool,
        initial_chunks: Vec<ChunkInitInfo>,
    },
    ChunkCreated {
        chunk_id: usize,
        chunk_total_bytes: u64,
    },
    ChunkProgress {
        chunk_id: usize,
        bytes_read: u64,
        chunk_total_bytes: u64,
    },
    ChunkCompleted {
        chunk_id: usize,
    },
    Paused,
    Finished,
    Error(String),
}
