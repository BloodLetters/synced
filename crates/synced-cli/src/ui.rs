use indicatif::{MultiProgress, ProgressBar, ProgressStyle};
use std::time::Duration;
use synced_core::ChunkInitInfo;

/// Terminal multi-progress bar manager for tracking download state.
pub struct TerminalUi {
    mp: MultiProgress,
    main_bar: Option<ProgressBar>,
    chunk_bars: Vec<ProgressBar>,
}

impl TerminalUi {
    /// Creates a new uninitialized terminal UI manager.
    pub fn new() -> Self {
        Self {
            mp: MultiProgress::new(),
            main_bar: None,
            chunk_bars: Vec::new(),
        }
    }

    /// Initializes progress bars once remote file metadata is resolved.
    pub fn init(
        &mut self,
        file_name: &str,
        total_size: Option<u64>,
        initial_chunks: &[ChunkInitInfo],
    ) {
        let resumed_bytes: u64 = initial_chunks.iter().map(|c| c.downloaded_bytes).sum();

        let main_style = ProgressStyle::default_bar()
            .template("{spinner:.green} [{elapsed_precise}] [{bar:40.cyan/blue}] {bytes}/{total_bytes} ({binary_bytes_per_sec}, {eta}) {msg}")
            .unwrap_or_else(|_| ProgressStyle::default_bar())
            .progress_chars("=>-");

        let main_pb = if let Some(size) = total_size {
            let pb = self.mp.add(ProgressBar::new(size));
            pb.set_style(main_style);
            pb.set_message(file_name.to_string());
            pb.enable_steady_tick(Duration::from_millis(100));
            if resumed_bytes > 0 {
                pb.set_position(resumed_bytes);
                pb.reset_eta();
            }
            pb
        } else {
            let spinner_style = ProgressStyle::default_spinner()
                .template("{spinner:.green} [{elapsed_precise}] {bytes} ({binary_bytes_per_sec}) {msg}")
                .unwrap_or_else(|_| ProgressStyle::default_spinner());
            let pb = self.mp.add(ProgressBar::new_spinner());
            pb.set_style(spinner_style);
            pb.set_message(format!("Streaming {}", file_name));
            pb.enable_steady_tick(Duration::from_millis(100));
            pb
        };

        self.main_bar = Some(main_pb);

        if initial_chunks.len() > 1 {
            let chunk_style = ProgressStyle::default_bar()
                .template("  chunk {prefix:>2} [{bar:30.yellow/white}] {bytes}/{total_bytes} ({binary_bytes_per_sec})")
                .unwrap_or_else(|_| ProgressStyle::default_bar())
                .progress_chars("#>-");

            for chunk in initial_chunks {
                let pb = self.mp.add(ProgressBar::new(chunk.total_bytes));
                pb.set_style(chunk_style.clone());
                pb.set_prefix(chunk.id.to_string());
                if chunk.downloaded_bytes > 0 {
                    pb.set_position(chunk.downloaded_bytes);
                    pb.reset_eta();
                }
                if chunk.downloaded_bytes >= chunk.total_bytes && chunk.total_bytes > 0 {
                    pb.finish_with_message("done");
                }
                self.chunk_bars.push(pb);
            }
        }
    }

    /// Spawns or resizes an additional progress bar for a newly stolen dynamic segment.
    pub fn on_chunk_created(&mut self, chunk_id: usize, chunk_total_bytes: u64) {
        let chunk_style = ProgressStyle::default_bar()
            .template("  chunk {prefix:>2} [{bar:30.yellow/white}] {bytes}/{total_bytes} ({binary_bytes_per_sec})")
            .unwrap_or_else(|_| ProgressStyle::default_bar())
            .progress_chars("#>-");

        while self.chunk_bars.len() <= chunk_id {
            let pb = self.mp.add(ProgressBar::new(0));
            pb.set_style(chunk_style.clone());
            pb.set_prefix(self.chunk_bars.len().to_string());
            self.chunk_bars.push(pb);
        }

        if let Some(chunk_pb) = self.chunk_bars.get(chunk_id) {
            chunk_pb.set_length(chunk_total_bytes);
            chunk_pb.reset_eta();
        }
    }

    /// Updates byte counts for a specific segment and advances overall progress.
    pub fn on_chunk_progress(&mut self, chunk_id: usize, bytes_read: u64, chunk_total: u64) {
        if let Some(chunk_pb) = self.chunk_bars.get(chunk_id) {
            if chunk_pb.length().unwrap_or(0) == 0 && chunk_total > 0 {
                chunk_pb.set_length(chunk_total);
            }
            chunk_pb.inc(bytes_read);
        }

        if let Some(main_pb) = &self.main_bar {
            main_pb.inc(bytes_read);
        }
    }

    /// Marks a specific chunk worker as completed.
    pub fn on_chunk_completed(&mut self, chunk_id: usize) {
        if let Some(chunk_pb) = self.chunk_bars.get(chunk_id) {
            chunk_pb.finish_with_message("done");
        }
    }

    /// Finalizes the progress display when download is paused by user.
    pub fn pause(&mut self) {
        if let Some(main_pb) = &self.main_bar {
            main_pb.abandon_with_message("Paused (state saved)");
        }
        for chunk in &self.chunk_bars {
            chunk.abandon();
        }
    }

    /// Finalizes the overall progress display.
    pub fn finish(&mut self) {
        if let Some(main_pb) = &self.main_bar {
            main_pb.finish_with_message("Download completed");
        }
    }
}
