/// Represents a continuous byte range allocated to an individual worker.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Segment {
    pub id: usize,
    pub start: u64,
    pub end: u64,
}

impl Segment {
    /// Computes the total byte length of this segment.
    pub fn length(&self) -> u64 {
        if self.end >= self.start {
            (self.end - self.start) + 1
        } else {
            0
        }
    }
}

/// Splits a total byte size into a specified number of uniform byte segments.
pub fn calculate_segments(total_size: u64, concurrency: usize) -> Vec<Segment> {
    if total_size == 0 || concurrency <= 1 {
        return vec![Segment {
            id: 0,
            start: 0,
            end: total_size.saturating_sub(1),
        }];
    }

    let count = concurrency.min(total_size as usize);
    let chunk_size = total_size / count as u64;
    let remainder = total_size % count as u64;

    let mut segments = Vec::with_capacity(count);
    let mut current_offset = 0u64;

    for id in 0..count {
        let extra = if (id as u64) < remainder { 1 } else { 0 };
        let segment_length = chunk_size + extra;
        let start = current_offset;
        let end = start + segment_length - 1;

        segments.push(Segment { id, start, end });
        current_offset = end + 1;
    }

    segments
}
