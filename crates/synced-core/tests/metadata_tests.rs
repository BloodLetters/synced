use synced_core::{
    delete_state, load_state, metadata_path, save_state, DownloadState, SegmentState,
};

#[tokio::test]
async fn test_state_serialization_roundtrip() {
    let temp_dir = std::env::temp_dir();
    let target = temp_dir.join("test_download.bin");
    let meta = metadata_path(&target);

    let state = DownloadState {
        url: "https://example.com/file.bin".to_string(),
        file_name: "file.bin".to_string(),
        total_size: 1000,
        etag: Some("test-etag".to_string()),
        segments: vec![SegmentState {
            id: 0,
            start: 0,
            current: 500,
            end: 999,
            completed: false,
        }],
    };

    save_state(&meta, &state).await.unwrap();
    let loaded = load_state(&meta).await.unwrap().unwrap();
    assert_eq!(loaded.total_size, 1000);
    assert_eq!(loaded.segments[0].current, 500);

    delete_state(&meta).await.unwrap();
    assert!(!meta.exists());
}
