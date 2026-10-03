use reqwest::Client;
use std::path::{Path, PathBuf};
use std::time::Duration;
use tokio::sync::{mpsc::Sender, watch};
use tokio::task::JoinSet;

use crate::error::CoreError;
use crate::events::DownloadEvent;
use crate::metadata::{
    delete_state, load_state, metadata_path, save_state, DownloadState, SegmentState,
};
use crate::pool::SegmentPool;
use crate::probe::probe_url;
use crate::segment::calculate_segments;
use crate::storage::preallocate_file;
use crate::throttle::BandwidthLimiter;
use crate::worker::run_segment_worker;

/// Configuration options for dispatching a download task.
#[derive(Debug, Clone)]
pub struct DownloadOptions {
    pub url: String,
    pub destination: PathBuf,
    pub concurrency: usize,
    pub speed_limit: Option<u64>,
}

/// Dispatches and supervises a segmented or single-stream download job.
pub async fn download_file(
    options: DownloadOptions,
    tx: Sender<DownloadEvent>,
    cancel_rx: watch::Receiver<bool>,
) -> Result<(), CoreError> {
    let client = Client::builder()
        .user_agent("Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/125.0.0.0 Safari/537.36")
        .redirect(reqwest::redirect::Policy::limited(10))
        .http1_only()
        .tcp_nodelay(true)
        .build()?;

    let probe = probe_url(&client, &options.url).await?;
    let destination = resolve_destination(&options.destination, &probe.file_name);
    let meta_path = metadata_path(&destination);

    let supports_ranges = probe.accept_ranges && probe.content_length.is_some();
    let total_size = probe.content_length.unwrap_or(0);
    let concurrency = if supports_ranges { options.concurrency.max(1) } else { 1 };

    let initial_states = prepare_segment_states(
        &meta_path,
        &probe.url,
        total_size,
        supports_ranges,
        concurrency,
        &destination,
    ).await?;

    let initial_chunks: Vec<crate::events::ChunkInitInfo> = initial_states.iter().map(|s| {
        let total = if s.end >= s.start { (s.end - s.start) + 1 } else { 0 };
        crate::events::ChunkInitInfo {
            id: s.id,
            downloaded_bytes: s.current.saturating_sub(s.start),
            total_bytes: total,
        }
    }).collect();

    let pool = SegmentPool::new(initial_states);
    let limiter = BandwidthLimiter::new(options.speed_limit);

    let _ = tx
        .send(DownloadEvent::Probed {
            file_name: probe.file_name.clone(),
            total_size: probe.content_length,
            supports_ranges,
            initial_chunks,
        })
        .await;

    let mut tasks = JoinSet::new();
    for _ in 0..concurrency {
        let pool = pool.clone();
        let client = client.clone();
        let url = probe.url.clone();
        let dest = destination.clone();
        let limiter = limiter.clone();
        let tx = tx.clone();
        let cancel_rx = cancel_rx.clone();

        tasks.spawn(async move {
            while let Some((segment, is_new)) = pool.claim_work().await {
                if is_new {
                    let total_span = if segment.end.load(std::sync::atomic::Ordering::SeqCst)
                        >= segment.start
                    {
                        (segment.end.load(std::sync::atomic::Ordering::SeqCst) - segment.start) + 1
                    } else {
                        0
                    };
                    let _ = tx
                        .send(DownloadEvent::ChunkCreated {
                            chunk_id: segment.id,
                            chunk_total_bytes: total_span,
                        })
                        .await;
                }

                run_segment_worker(
                    client.clone(),
                    url.clone(),
                    segment,
                    dest.clone(),
                    limiter.clone(),
                    tx.clone(),
                    cancel_rx.clone(),
                )
                .await?;
            }
            Ok::<(), CoreError>(())
        });
    }

    let save_meta_path = meta_path.clone();
    let save_pool = pool.clone();
    let save_url = probe.url.clone();
    let save_file_name = probe.file_name.clone();
    let save_etag = probe.etag.clone();
    let mut save_cancel = cancel_rx.clone();

    let persist_handle = tokio::spawn(async move {
        loop {
            tokio::select! {
                _ = tokio::time::sleep(Duration::from_secs(1)) => {
                    let segs = save_pool.snapshot().await;
                    let state = DownloadState {
                        url: save_url.clone(),
                        file_name: save_file_name.clone(),
                        total_size,
                        etag: save_etag.clone(),
                        segments: segs,
                    };
                    let _ = save_state(&save_meta_path, &state).await;
                }
                _ = save_cancel.changed() => {
                    break;
                }
            }
        }
    });

    let mut download_result = Ok(());
    while let Some(res) = tasks.join_next().await {
        match res {
            Ok(Ok(())) => {}
            Ok(Err(err)) => {
                download_result = Err(err);
                break;
            }
            Err(_) => {
                download_result = Err(CoreError::Cancelled);
                break;
            }
        }
    }

    persist_handle.abort();

    if *cancel_rx.borrow() {
        let segs = pool.snapshot().await;
        let state = DownloadState {
            url: probe.url,
            file_name: probe.file_name,
            total_size,
            etag: probe.etag,
            segments: segs,
        };
        let _ = save_state(&meta_path, &state).await;
        let _ = tx.send(DownloadEvent::Paused).await;
        return Err(CoreError::Cancelled);
    }

    download_result?;

    if pool.is_all_completed().await {
        let _ = delete_state(&meta_path).await;
        let _ = tx.send(DownloadEvent::Finished).await;
    }

    Ok(())
}

/// Prepares segment states from existing metadata or computes new initial splits.
async fn prepare_segment_states(
    meta_path: &Path,
    url: &str,
    total_size: u64,
    supports_ranges: bool,
    concurrency: usize,
    destination: &Path,
) -> Result<Vec<SegmentState>, CoreError> {
    if let Ok(Some(saved)) = load_state(meta_path).await {
        if saved.url == url && saved.total_size == total_size {
            return Ok(saved.segments);
        }
    }

    if supports_ranges && total_size > 0 {
        preallocate_file(destination, total_size).await?;
        let segments = calculate_segments(total_size, concurrency);
        let states = segments
            .into_iter()
            .map(|s| SegmentState {
                id: s.id,
                start: s.start,
                current: s.start,
                end: s.end,
                completed: false,
            })
            .collect();
        return Ok(states);
    }

    Ok(vec![SegmentState {
        id: 0,
        start: 0,
        current: 0,
        end: total_size.saturating_sub(1),
        completed: false,
    }])
}

/// Resolves target directory or path to absolute final file path.
pub fn resolve_destination(target: &Path, default_name: &str) -> PathBuf {
    if target.is_dir() || target.as_os_str().is_empty() {
        target.join(default_name)
    } else {
        target.to_path_buf()
    }
}

/// Returns the platform default download directory, falling back to current directory.
pub fn default_download_dir() -> PathBuf {
    dirs::download_dir().unwrap_or_else(|| PathBuf::from("."))
}

