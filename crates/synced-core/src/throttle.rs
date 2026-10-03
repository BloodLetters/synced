use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::Mutex;

/// Rate limiter for constraining download throughput.
#[derive(Clone)]
pub struct BandwidthLimiter {
    inner: Option<Arc<Mutex<LimiterInner>>>,
}

struct LimiterInner {
    bytes_per_sec: u64,
    tokens: f64,
    last_update: Instant,
}

impl BandwidthLimiter {
    /// Creates a new limiter with an optional bytes-per-second cap.
    pub fn new(bytes_per_sec: Option<u64>) -> Self {
        let inner = bytes_per_sec.filter(|&rate| rate > 0).map(|rate| {
            Arc::new(Mutex::new(LimiterInner {
                bytes_per_sec: rate,
                tokens: rate as f64,
                last_update: Instant::now(),
            }))
        });

        Self { inner }
    }

    /// Suspends execution if consumption exceeds the configured throughput rate.
    pub async fn acquire(&self, bytes: usize) {
        let inner_arc = match &self.inner {
            Some(arc) => arc.clone(),
            None => return,
        };

        let mut inner = inner_arc.lock().await;
        let now = Instant::now();
        let elapsed = now.duration_since(inner.last_update).as_secs_f64();
        inner.last_update = now;

        inner.tokens = (inner.tokens + elapsed * inner.bytes_per_sec as f64).min(inner.bytes_per_sec as f64);

        if inner.tokens < bytes as f64 {
            let needed = bytes as f64 - inner.tokens;
            let wait_secs = needed / inner.bytes_per_sec as f64;
            inner.tokens = 0.0;
            drop(inner);
            tokio::time::sleep(Duration::from_secs_f64(wait_secs)).await;
        } else {
            inner.tokens -= bytes as f64;
        }
    }
}

/// Parses human-readable rate strings like "10M", "500K", or raw byte counts into bytes per second.
pub fn parse_speed_limit(input: &str) -> Result<u64, &'static str> {
    let trimmed = input.trim();
    if trimmed.is_empty() {
        return Err("Empty speed limit string");
    }

    let upper = trimmed.to_uppercase();
    let (num_part, multiplier) = if upper.ends_with("GB") || upper.ends_with('G') {
        let suffix_len = if upper.ends_with("GB") { 2 } else { 1 };
        (&upper[..upper.len() - suffix_len], 1024 * 1024 * 1024)
    } else if upper.ends_with("MB") || upper.ends_with('M') {
        let suffix_len = if upper.ends_with("MB") { 2 } else { 1 };
        (&upper[..upper.len() - suffix_len], 1024 * 1024)
    } else if upper.ends_with("KB") || upper.ends_with('K') {
        let suffix_len = if upper.ends_with("KB") { 2 } else { 1 };
        (&upper[..upper.len() - suffix_len], 1024)
    } else {
        (upper.as_str(), 1)
    };

    let base: f64 = num_part.trim().parse().map_err(|_| "Invalid number in speed limit")?;
    Ok((base * multiplier as f64) as u64)
}
