use std::error::Error;

pub type Result<T> = std::result::Result<T, Box<dyn Error + Send + Sync>>;

use tokio::io;
pub trait Storage {
    async fn write(
        &self,
        chunk_id: String,
        chunk_stream: &mut (impl io::AsyncRead + Unpin),
    ) -> Result<u64>;
    async fn commit(&self, chunk_id: String) -> Result<bool>;
    async fn read(&self, chunk_id: String) -> Result<Box<dyn io::AsyncRead + Unpin + Send>>;
    async fn delete(&self, chunk_id: String) -> Result<bool>;
    async fn available_chunks(&self) -> Result<Vec<String>>;
    async fn available_storage(&self) -> usize;
}

#[cfg(test)]
pub mod tests {
    use std::io::Cursor;
    use tokio::io::AsyncReadExt;
    use tokio::io::BufReader;

    use super::*;
    pub async fn storage_test(storage: impl Storage) -> Result<()> {
        let chunk_id = "test_chunk.bin".to_string();
        let original_data = b"hello world";

        // Write test data
        let mut input_stream = Cursor::new(original_data);
        let written = storage.write(chunk_id.clone(), &mut input_stream).await?;
        assert_eq!(written as usize, original_data.len());
        // testing availbale chunks
        let available_chunks = storage.available_chunks().await?;
        assert_eq!(available_chunks, vec!["test_chunk.bin".to_string()]);

        // Read and verify data
        let reader = storage.read(chunk_id.clone()).await?;
        let mut buf_reader = BufReader::new(reader);
        let mut read_buf = Vec::new();
        buf_reader.read_to_end(&mut read_buf).await?;
        assert_eq!(read_buf, original_data);

        // testing delete functionality
        storage.delete(chunk_id).await?;
        let available_chunks = storage.available_chunks().await?;
        assert_eq!(available_chunks.len(), 0);
        Ok(())
    }
}
