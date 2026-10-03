use std::sync::atomic::{AtomicBool, AtomicU64, AtomicUsize, Ordering};
use std::sync::Arc;
use tokio::sync::Mutex;

use crate::metadata::SegmentState;

/// Minimum remaining byte threshold required before a segment can be split dynamically.
pub const MIN_SPLIT_THRESHOLD: u64 = 4 * 1024 * 1024;

/// In-memory representation of a segmented download slice.
#[derive(Clone)]
pub struct ActiveSegment {
    pub id: usize,
    pub start: u64,
    pub current: Arc<AtomicU64>,
    pub end: Arc<AtomicU64>,
    pub completed: Arc<AtomicBool>,
    pub active: Arc<AtomicBool>,
}

impl ActiveSegment {
    /// Constructs a new active segment descriptor.
    pub fn new(id: usize, start: u64, current: u64, end: u64, completed: bool) -> Self {
        Self {
            id,
            start,
            current: Arc::new(AtomicU64::new(current)),
            end: Arc::new(AtomicU64::new(end)),
            completed: Arc::new(AtomicBool::new(completed)),
            active: Arc::new(AtomicBool::new(false)),
        }
    }

    /// Converts active segment atomic state to persistent segment snapshot.
    pub fn to_state(&self) -> SegmentState {
        SegmentState {
            id: self.id,
            start: self.start,
            current: self.current.load(Ordering::SeqCst),
            end: self.end.load(Ordering::SeqCst),
            completed: self.completed.load(Ordering::SeqCst),
        }
    }
}

/// Dynamic work-stealing pool managing concurrent download segments.
#[derive(Clone)]
pub struct SegmentPool {
    segments: Arc<Mutex<Vec<ActiveSegment>>>,
    next_id: Arc<AtomicUsize>,
}

impl SegmentPool {
    /// Creates a pool initialized from existing or newly calculated segment states.
    pub fn new(initial: Vec<SegmentState>) -> Self {
        let max_id = initial.iter().map(|s| s.id).max().unwrap_or(0);
        let segments = initial
            .into_iter()
            .map(|s| ActiveSegment::new(s.id, s.start, s.current, s.end, s.completed))
            .collect();

        Self {
            segments: Arc::new(Mutex::new(segments)),
            next_id: Arc::new(AtomicUsize::new(max_id + 1)),
        }
    }

    /// Acquires an idle segment or steals work by splitting an active segment.
    pub async fn claim_work(&self) -> Option<(ActiveSegment, bool)> {
        let mut segments = self.segments.lock().await;

        for seg in segments.iter() {
            if !seg.completed.load(Ordering::SeqCst) && !seg.active.load(Ordering::SeqCst) {
                seg.active.store(true, Ordering::SeqCst);
                return Some((seg.clone(), false));
            }
        }

        let mut candidate_idx = None;
        let mut max_remaining = 0u64;

        for (idx, seg) in segments.iter().enumerate() {
            if !seg.completed.load(Ordering::SeqCst) {
                let curr = seg.current.load(Ordering::SeqCst);
                let end = seg.end.load(Ordering::SeqCst);
                if end > curr {
                    let remaining = end - curr;
                    if remaining > max_remaining {
                        max_remaining = remaining;
                        candidate_idx = Some(idx);
                    }
                }
            }
        }

        if max_remaining >= MIN_SPLIT_THRESHOLD * 2 {
            if let Some(idx) = candidate_idx {
                let target = &segments[idx];
                let curr = target.current.load(Ordering::SeqCst);
                let old_end = target.end.load(Ordering::SeqCst);
                let half = (old_end - curr) / 2;
                let split_point = curr + half;

                target.end.store(split_point - 1, Ordering::SeqCst);

                let new_id = self.next_id.fetch_add(1, Ordering::SeqCst);
                let new_seg = ActiveSegment::new(new_id, split_point, split_point, old_end, false);
                new_seg.active.store(true, Ordering::SeqCst);

                segments.push(new_seg.clone());
                return Some((new_seg, true));
            }
        }

        None
    }

    /// Exports current state of all segments for persistence.
    pub async fn snapshot(&self) -> Vec<SegmentState> {
        let segments = self.segments.lock().await;
        segments.iter().map(|s| s.to_state()).collect()
    }

    /// Checks if every segment in the pool has finished downloading.
    pub async fn is_all_completed(&self) -> bool {
        let segments = self.segments.lock().await;
        segments.iter().all(|s| s.completed.load(Ordering::SeqCst))
    }
}
