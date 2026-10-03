use std::path::PathBuf;
use std::sync::atomic::Ordering;
use std::time::Duration;
use futures_util::StreamExt;
use reqwest::header::RANGE;
use reqwest::{Client, StatusCode};
use tokio::sync::{mpsc::Sender, watch};

use crate::error::CoreError;
use crate::events::DownloadEvent;
use crate::pool::ActiveSegment;
use crate::storage::ChunkWriter;
use crate::throttle::BandwidthLimiter;

const MAX_WORKER_RETRIES: usize = 5;

/// Downloads an assigned segment with dynamic boundary trimming, rate limiting, and retry backoff.
pub async fn run_segment_worker(
    client: Client,
    url: String,
    segment: ActiveSegment,
    destination: PathBuf,
    limiter: BandwidthLimiter,
    tx: Sender<DownloadEvent>,
    mut cancel_rx: watch::Receiver<bool>,
) -> Result<(), CoreError> {
    let mut retries = 0;

    loop {
        if *cancel_rx.borrow() {
            segment.active.store(false, Ordering::SeqCst);
            return Err(CoreError::Cancelled);
        }

        let current = segment.current.load(Ordering::SeqCst);
        let end = segment.end.load(Ordering::SeqCst);

        if current > end {
            segment.completed.store(true, Ordering::SeqCst);
            segment.active.store(false, Ordering::SeqCst);
            let _ = tx.send(DownloadEvent::ChunkCompleted { chunk_id: segment.id }).await;
            return Ok(());
        }

        match stream_segment_bytes(
            &client,
            &url,
            &segment,
            &destination,
            &limiter,
            &tx,
            &mut cancel_rx,
        ).await {
            Ok(finished) => {
                if finished {
                    segment.completed.store(true, Ordering::SeqCst);
                    segment.active.store(false, Ordering::SeqCst);
                    let _ = tx.send(DownloadEvent::ChunkCompleted { chunk_id: segment.id }).await;
                    return Ok(());
                }
            }
            Err(err) => {
                if *cancel_rx.borrow() {
                    segment.active.store(false, Ordering::SeqCst);
                    return Err(CoreError::Cancelled);
                }

                retries += 1;
                if retries > MAX_WORKER_RETRIES {
                    segment.active.store(false, Ordering::SeqCst);
                    return Err(err);
                }

                let backoff = Duration::from_millis(300 * (1 << retries.min(4)));
                tokio::time::sleep(backoff).await;
            }
        }
    }
}

/// Executes single HTTP stream attempt for remaining segment bytes.
async fn stream_segment_bytes(
    client: &Client,
    url: &str,
    segment: &ActiveSegment,
    destination: &PathBuf,
    limiter: &BandwidthLimiter,
    tx: &Sender<DownloadEvent>,
    cancel_rx: &mut watch::Receiver<bool>,
) -> Result<bool, CoreError> {
    let current = segment.current.load(Ordering::SeqCst);
    let end = segment.end.load(Ordering::SeqCst);

    if current > end {
        return Ok(true);
    }

    let range_header = format!("bytes={}-{}", current, end);
    let response = client
        .get(url)
        .header(RANGE, range_header)
        .send()
        .await?;

    let status = response.status();
    if !status.is_success() && status != StatusCode::PARTIAL_CONTENT {
        return Err(CoreError::HttpStatus(status));
    }

    let mut writer = ChunkWriter::open_at(destination, current).await?;
    let mut stream = response.bytes_stream();

    while let Some(chunk_res) = stream.next().await {
        if *cancel_rx.borrow() {
            writer.flush().await?;
            return Err(CoreError::Cancelled);
        }

        let chunk = chunk_res?;
        let current_offset = segment.current.load(Ordering::SeqCst);
        let dynamic_end = segment.end.load(Ordering::SeqCst);

        if current_offset > dynamic_end {
            writer.flush().await?;
            return Ok(true);
        }

        let remaining = (dynamic_end - current_offset + 1) as usize;
        let bytes_to_write = if chunk.len() > remaining {
            &chunk[..remaining]
        } else {
            &chunk[..]
        };

        limiter.acquire(bytes_to_write.len()).await;
        writer.write_bytes(bytes_to_write).await?;

        let written_len = bytes_to_write.len() as u64;
        segment.current.fetch_add(written_len, Ordering::SeqCst);

        let total_span = if dynamic_end >= segment.start {
            (dynamic_end - segment.start) + 1
        } else {
            0
        };

        let _ = tx
            .send(DownloadEvent::ChunkProgress {
                chunk_id: segment.id,
                bytes_read: written_len,
                chunk_total_bytes: total_span,
            })
            .await;

        if chunk.len() > remaining {
            writer.flush().await?;
            return Ok(true);
        }
    }

    writer.flush().await?;
    let final_current = segment.current.load(Ordering::SeqCst);
    let final_end = segment.end.load(Ordering::SeqCst);
    Ok(final_current > final_end)
}
