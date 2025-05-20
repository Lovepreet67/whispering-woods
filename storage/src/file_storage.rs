use std::{
    error::Error,
    path::{Path, PathBuf},
};

use crate::storage::Storage;
use tokio::{
    fs::{self, File},
    io::copy,
};

#[derive(Clone)]
pub struct FileStorage {
    root: String,
}
impl FileStorage {
    pub fn new(root: String) -> Self {
        FileStorage { root }
    }
    fn get_path(&self, chunk_id: &str) -> PathBuf {
        Path::new(&self.root).join(chunk_id).to_path_buf()
    }
}
impl Storage for FileStorage {
    async fn write(
        &self,
        chunk_id: String,
        chunk_stream: &mut (impl tokio::io::AsyncRead + Unpin),
    ) -> Result<u64, Box<dyn Error>> {
        let chunk_path = self.get_path(&chunk_id);
        let mut chunk_file = File::create_new(chunk_path).await?;
        let writer_byte_count = copy(chunk_stream, &mut chunk_file).await?;
        Ok(writer_byte_count)
    }
    async fn read(
        &self,
        chunk_id: String,
    ) -> Result<Box<dyn tokio::io::AsyncRead + Unpin + Send>, Box<dyn Error>> {
        let chunk_path = self.get_path(&chunk_id);
        let chunk_file = File::open(chunk_path).await?;
        Ok(Box::new(chunk_file))
    }
    async fn delete(&self, chunk_id: String) -> Result<(), Box<dyn Error>> {
        fs::remove_file(self.get_path(&chunk_id)).await?;
        Ok(())
    }
    async fn available_chunks(&self) -> Result<Vec<String>, Box<dyn Error>> {
        let mut dir_enteries = fs::read_dir(&self.root).await?;
        let mut chunk_ids = vec![];
        while let Some(chunk) = dir_enteries.next_entry().await? {
            chunk_ids.push(
                chunk
                    .file_name()
                    .into_string()
                    .map_err(|_| "Invalid file name")?,
            );
        }
        Ok(chunk_ids)
    }
    async fn available_storage(&self) -> usize {
        10_usize
    }
}

#[cfg(test)]
mod tests {
    use crate::storage::tests::storage_test;
    use tokio::fs;

    use super::*;
    #[tokio::test]
    async fn file_storage_test() -> Result<(), Box<dyn Error>> {
        let storage = FileStorage::new("./temp".into());
        fs::create_dir_all(&storage.root).await?;
        let test_result = storage_test(storage).await; // we have to await here because other wise
        // directory will be removed before running
        // test
        // cleaning up directory
        fs::remove_dir("./temp").await?;
        test_result
    }
}
