use std::path::PathBuf;
use std::time::Instant;
use synced_core::{default_download_dir, download_file, parse_speed_limit, DownloadEvent, DownloadOptions};
use tokio::sync::{mpsc, watch};

/// Live snapshot of an individual segment inside the GUI.
#[derive(Debug, Clone)]
pub struct ChunkModel {
    pub id: usize,
    pub downloaded: u64,
    pub total: u64,
    pub completed: bool,
    pub speed: f64,
    pub bytes_tick: u64,
}

/// Lifecycle status of the active download task.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DownloadStatus {
    Idle,
    Probing,
    Downloading,
    Paused,
    Completed,
    Error(String),
}

/// Core application state bound to the desktop window.
pub struct GuiState {
    pub url: String,
    pub destination: String,
    pub concurrency: usize,
    pub limit_str: String,
    pub file_name: String,
    pub total_size: Option<u64>,
    pub downloaded_bytes: u64,
    pub status: DownloadStatus,
    pub chunks: Vec<ChunkModel>,
    pub current_speed: f64,
    pub peak_speed: f64,
    pub avg_speed: f64,
    pub speed_history: Vec<f64>,
    pub last_speed_update: Instant,
    pub start_time: Option<Instant>,
    pub bytes_since_last_tick: u64,
    pub event_rx: Option<mpsc::Receiver<DownloadEvent>>,
    pub cancel_tx: Option<watch::Sender<bool>>,
}

impl GuiState {
    /// Creates a default initialized GUI state instance.
    pub fn new() -> Self {
        Self {
            url: String::new(),
            destination: default_download_dir().to_string_lossy().to_string(),
            concurrency: 8,
            limit_str: String::new(),
            file_name: "Ready".to_string(),
            total_size: None,
            downloaded_bytes: 0,
            status: DownloadStatus::Idle,
            chunks: Vec::new(),
            current_speed: 0.0,
            peak_speed: 0.0,
            avg_speed: 0.0,
            speed_history: Vec::new(),
            last_speed_update: Instant::now(),
            start_time: None,
            bytes_since_last_tick: 0,
            event_rx: None,
            cancel_tx: None,
        }
    }

    /// Spawns a background task running the synced-core downloader.
    pub fn start_download(&mut self) {
        if self.url.trim().is_empty() {
            return;
        }

        let speed_limit = parse_speed_limit(&self.limit_str).ok();
        let (tx, rx) = mpsc::channel(256);
        let (cancel_tx, cancel_rx) = watch::channel(false);

        let options = DownloadOptions {
            url: self.url.trim().to_string(),
            destination: PathBuf::from(self.destination.trim()),
            concurrency: self.concurrency,
            speed_limit,
        };

        self.status = DownloadStatus::Probing;
        self.start_time = Some(Instant::now());
        self.event_rx = Some(rx);
        self.cancel_tx = Some(cancel_tx);

        tokio::spawn(async move {
            let _ = download_file(options, tx, cancel_rx).await;
        });
    }

    /// Sends cancellation signal to pause active background workers.
    pub fn pause_download(&mut self) {
        if let Some(cancel) = &self.cancel_tx {
            let _ = cancel.send(true);
        }
    }

    /// Cancels active download task and resets state.
    pub fn cancel_download(&mut self) {
        self.pause_download();
        self.status = DownloadStatus::Idle;
        self.current_speed = 0.0;
        self.chunks.clear();
        self.downloaded_bytes = 0;
        self.total_size = None;
        self.start_time = None;
        self.speed_history.clear();
    }

    /// Pulls available progress messages from the core event channel.
    pub fn poll_events(&mut self) {
        let mut rx = match self.event_rx.take() {
            Some(receiver) => receiver,
            None => return,
        };

        while let Ok(event) = rx.try_recv() {
            match event {
                DownloadEvent::Probed { file_name, total_size, initial_chunks, .. } => {
                    self.file_name = file_name;
                    self.total_size = total_size;
                    self.status = DownloadStatus::Downloading;
                    self.downloaded_bytes = initial_chunks.iter().map(|c| c.downloaded_bytes).sum();
                    self.chunks = initial_chunks
                        .into_iter()
                        .map(|c| ChunkModel {
                            id: c.id,
                            downloaded: c.downloaded_bytes,
                            total: c.total_bytes,
                            completed: c.downloaded_bytes >= c.total_bytes && c.total_bytes > 0,
                            speed: 0.0,
                            bytes_tick: 0,
                        })
                        .collect();
                }
                DownloadEvent::ChunkCreated { chunk_id, chunk_total_bytes } => {
                    while self.chunks.len() <= chunk_id {
                        let id = self.chunks.len();
                        self.chunks.push(ChunkModel { id, downloaded: 0, total: 0, completed: false, speed: 0.0, bytes_tick: 0 });
                    }
                    if let Some(c) = self.chunks.get_mut(chunk_id) {
                        c.total = chunk_total_bytes;
                    }
                }
                DownloadEvent::ChunkProgress { chunk_id, bytes_read, chunk_total_bytes } => {
                    self.downloaded_bytes += bytes_read;
                    self.bytes_since_last_tick += bytes_read;
                    if let Some(c) = self.chunks.get_mut(chunk_id) {
                        c.downloaded += bytes_read;
                        c.bytes_tick += bytes_read;
                        c.total = chunk_total_bytes;
                    }
                }
                DownloadEvent::ChunkCompleted { chunk_id } => {
                    if let Some(c) = self.chunks.get_mut(chunk_id) {
                        c.completed = true;
                        c.speed = 0.0;
                    }
                }
                DownloadEvent::Paused => { self.status = DownloadStatus::Paused; self.current_speed = 0.0; }
                DownloadEvent::Finished => { self.status = DownloadStatus::Completed; self.current_speed = 0.0; }
                DownloadEvent::Error(err) => { self.status = DownloadStatus::Error(err); self.current_speed = 0.0; }
            }
        }

        self.update_speed();
        self.event_rx = Some(rx);
    }

    /// Recalculates real moving average transfer rate without dummy multipliers.
    fn update_speed(&mut self) {
        let now = Instant::now();
        let elapsed = now.duration_since(self.last_speed_update).as_secs_f64();

        if elapsed >= 0.4 {
            let speed = self.bytes_since_last_tick as f64 / elapsed;
            self.current_speed = speed;
            self.peak_speed = self.peak_speed.max(speed);

            if let Some(start) = self.start_time {
                let total_elapsed = now.duration_since(start).as_secs_f64();
                if total_elapsed > 0.0 {
                    self.avg_speed = self.downloaded_bytes as f64 / total_elapsed;
                }
            }

            for c in &mut self.chunks {
                c.speed = c.bytes_tick as f64 / elapsed;
                c.bytes_tick = 0;
            }

            self.speed_history.push(speed);
            if self.speed_history.len() > 24 {
                self.speed_history.remove(0);
            }

            self.bytes_since_last_tick = 0;
            self.last_speed_update = now;
        }
    }

    /// Extracts uppercase extension from file name.
    pub fn file_extension(&self) -> String {
        self.file_name.rsplit('.').next().unwrap_or("FILE").to_uppercase()
    }

    /// Calculates formatted ETA string based on remaining bytes and current rate.
    pub fn eta_string(&self) -> String {
        let total = match self.total_size {
            Some(t) if t > self.downloaded_bytes => t - self.downloaded_bytes,
            _ => return "calculating".to_string(),
        };
        if self.current_speed <= 0.0 {
            return "paused".to_string();
        }
        let secs = (total as f64 / self.current_speed) as u64;
        let (mins, rem) = (secs / 60, secs % 60);
        if mins > 0 { format!("{}m {}s left", mins, rem) } else { format!("{}s left", rem) }
    }
}

/// Formats byte quantities into readable units (B, KB, MB, GB).
pub fn format_bytes(bytes: u64) -> String {
    let b = bytes as f64;
    if b >= 1073741824.0 { format!("{:.1} GB", b / 1073741824.0) }
    else if b >= 1048576.0 { format!("{:.1} MB", b / 1048576.0) }
    else if b >= 1024.0 { format!("{:.1} KB", b / 1024.0) }
    else { format!("{} B", bytes) }
}

/// Formats throughput rates into string.
pub fn format_speed(bytes_per_sec: f64) -> String {
    format!("{:.1} MB/s", bytes_per_sec / (1024.0 * 1024.0))
}
