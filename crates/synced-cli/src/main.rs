mod ui;

use std::path::PathBuf;
use clap::Parser;
use synced_core::{download_file, parse_speed_limit, DownloadEvent, DownloadOptions};
use tokio::sync::{mpsc, watch};
use ui::TerminalUi;

/// High-performance multi-part download manager CLI.
#[derive(Parser, Debug)]
#[command(name = "synced", author, version, about = "High-performance segmented downloader")]
struct Args {
    /// Remote URL to download.
    #[arg(required = true)]
    url: String,

    /// Destination file or directory path.
    #[arg(short, long, default_value_os_t = synced_core::default_download_dir())]
    output: PathBuf,

    /// Number of concurrent segments to download.
    #[arg(short, long, default_value_t = 8)]
    concurrency: usize,

    /// Bandwidth rate limit (e.g. 10M, 500K, 2MB).
    #[arg(short, long)]
    limit: Option<String>,
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let args = Args::parse();

    let speed_limit = match args.limit.as_deref() {
        Some(s) => Some(parse_speed_limit(s).map_err(|e| anyhow::anyhow!(e))?),
        None => None,
    };

    let (tx, mut rx) = mpsc::channel::<DownloadEvent>(256);
    let (cancel_tx, cancel_rx) = watch::channel(false);

    let options = DownloadOptions {
        url: args.url,
        destination: args.output,
        concurrency: args.concurrency,
        speed_limit,
    };

    let download_handle = tokio::spawn(async move {
        download_file(options, tx, cancel_rx).await
    });

    let cancel_sender = cancel_tx.clone();
    tokio::spawn(async move {
        if tokio::signal::ctrl_c().await.is_ok() {
            let _ = cancel_sender.send(true);
        }
    });

    let mut ui = TerminalUi::new();

    while let Some(event) = rx.recv().await {
        match event {
            DownloadEvent::Probed {
                file_name,
                total_size,
                initial_chunks,
                ..
            } => {
                ui.init(&file_name, total_size, &initial_chunks);
            }
            DownloadEvent::ChunkCreated {
                chunk_id,
                chunk_total_bytes,
            } => {
                ui.on_chunk_created(chunk_id, chunk_total_bytes);
            }
            DownloadEvent::ChunkProgress {
                chunk_id,
                bytes_read,
                chunk_total_bytes,
            } => {
                ui.on_chunk_progress(chunk_id, bytes_read, chunk_total_bytes);
            }
            DownloadEvent::ChunkCompleted { chunk_id } => {
                ui.on_chunk_completed(chunk_id);
            }
            DownloadEvent::Paused => {
                ui.pause();
            }
            DownloadEvent::Finished => {
                ui.finish();
            }
            DownloadEvent::Error(err) => {
                eprintln!("\nDownload error: {}", err);
            }
        }
    }

    match download_handle.await? {
        Ok(_) => Ok(()),
        Err(synced_core::CoreError::Cancelled) => Ok(()),
        Err(err) => Err(anyhow::anyhow!(err)),
    }
}
