use synced_gui::{format_bytes, format_speed};

#[test]
fn test_format_bytes() {
    assert_eq!(format_bytes(500), "500 B");
    assert_eq!(format_bytes(2048), "2.0 KB");
    assert_eq!(format_bytes(10 * 1024 * 1024), "10.0 MB");
    assert_eq!(format_bytes(1024 * 1024 * 1024), "1.0 GB");
}

#[test]
fn test_format_speed() {
    assert_eq!(format_speed(1024.0 * 1024.0), "1.0 MB/s");
    assert_eq!(format_speed(41.1 * 1024.0 * 1024.0), "41.1 MB/s");
}
