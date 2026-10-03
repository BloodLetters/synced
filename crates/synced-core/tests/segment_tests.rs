use synced_core::{calculate_segments, Segment};

#[test]
fn test_calculate_segments_even_split() {
    let segments = calculate_segments(100, 4);
    assert_eq!(segments.len(), 4);
    assert_eq!(segments[0], Segment { id: 0, start: 0, end: 24 });
    assert_eq!(segments[3], Segment { id: 3, start: 75, end: 99 });
    assert_eq!(segments.iter().map(|s| s.length()).sum::<u64>(), 100);
}

#[test]
fn test_calculate_segments_remainder() {
    let segments = calculate_segments(10, 3);
    assert_eq!(segments.len(), 3);
    assert_eq!(segments[0], Segment { id: 0, start: 0, end: 3 });
    assert_eq!(segments[1], Segment { id: 1, start: 4, end: 6 });
    assert_eq!(segments[2], Segment { id: 2, start: 7, end: 9 });
    assert_eq!(segments.iter().map(|s| s.length()).sum::<u64>(), 10);
}

#[test]
fn test_calculate_segments_single_concurrency() {
    let segments = calculate_segments(500, 1);
    assert_eq!(segments.len(), 1);
    assert_eq!(segments[0], Segment { id: 0, start: 0, end: 499 });
}
