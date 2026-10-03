use synced_core::{preallocate_file, ChunkWriter};
use tokio::fs;

#[tokio::test]
async fn test_buffered_writer_flush() {
    let temp_dir = std::env::temp_dir();
    let target = temp_dir.join("test_buffered_writer.bin");

    preallocate_file(&target, 20).await.unwrap();

    let mut writer = ChunkWriter::open_at_with_capacity(&target, 5, 10)
        .await
        .unwrap();
    writer.write_bytes(b"hello").await.unwrap();
    assert_eq!(writer.write_bytes(b"world12345").await.is_ok(), true);

    writer.write_bytes(b"!").await.unwrap();
    writer.flush().await.unwrap();

    let data = fs::read(&target).await.unwrap();
    assert_eq!(&data[5..21], b"helloworld12345!");

    let _ = fs::remove_file(&target).await;
}
