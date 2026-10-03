use synced_core::parse_speed_limit;

#[test]
fn test_parse_speed_limit() {
    assert_eq!(parse_speed_limit("500").unwrap(), 500);
    assert_eq!(parse_speed_limit("10K").unwrap(), 10 * 1024);
    assert_eq!(parse_speed_limit("5MB").unwrap(), 5 * 1024 * 1024);
    assert_eq!(
        parse_speed_limit("1.5M").unwrap(),
        (1.5 * 1024.0 * 1024.0) as u64
    );
    assert_eq!(parse_speed_limit("2G").unwrap(), 2 * 1024 * 1024 * 1024);
}
