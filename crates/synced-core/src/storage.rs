use std::io::SeekFrom;
use std::path::Path;
use tokio::fs::{create_dir_all, File, OpenOptions};
use tokio::io::{AsyncSeekExt, AsyncWriteExt};

/// Default in-memory write buffer capacity per chunk worker (1 MB).
pub const DEFAULT_CHUNK_BUFFER_SIZE: usize = 1024 * 1024;

/// Pre-allocates a file on disk with the exact requested byte size.
pub async fn preallocate_file(path: &Path, size: u64) -> Result<(), std::io::Error> {
    if let Some(parent) = path.parent() {
        create_dir_all(parent).await?;
    }

    let file = OpenOptions::new()
        .create(true)
        .write(true)
        .truncate(true)
        .open(path)
        .await?;

    file.set_len(size).await?;
    Ok(())
}

/// Buffered writer for streaming chunk bytes directly to designated file offsets.
pub struct ChunkWriter {
    file: File,
    buffer: Vec<u8>,
    capacity: usize,
}

impl ChunkWriter {
    /// Opens an existing file, seeks to start offset, and initializes a 1MB RAM buffer.
    pub async fn open_at(path: &Path, offset: u64) -> Result<Self, std::io::Error> {
        Self::open_at_with_capacity(path, offset, DEFAULT_CHUNK_BUFFER_SIZE).await
    }

    /// Opens an existing file with a custom in-memory buffer capacity.
    pub async fn open_at_with_capacity(
        path: &Path,
        offset: u64,
        capacity: usize,
    ) -> Result<Self, std::io::Error> {
        let mut file = OpenOptions::new()
            .write(true)
            .open(path)
            .await?;

        file.seek(SeekFrom::Start(offset)).await?;

        Ok(Self {
            file,
            buffer: Vec::with_capacity(capacity),
            capacity,
        })
    }

    /// Buffers incoming bytes in RAM and flushes to disk when capacity is reached.
    pub async fn write_bytes(&mut self, buf: &[u8]) -> Result<(), std::io::Error> {
        self.buffer.extend_from_slice(buf);
        if self.buffer.len() >= self.capacity {
            self.flush_buffer().await?;
        }
        Ok(())
    }

    /// Flushes any pending RAM buffer bytes and forces OS file sync.
    pub async fn flush(&mut self) -> Result<(), std::io::Error> {
        self.flush_buffer().await?;
        self.file.flush().await
    }

    /// Writes all accumulated RAM buffer bytes sequentially to disk.
    async fn flush_buffer(&mut self) -> Result<(), std::io::Error> {
        if !self.buffer.is_empty() {
            self.file.write_all(&self.buffer).await?;
            self.buffer.clear();
        }
        Ok(())
    }
}
