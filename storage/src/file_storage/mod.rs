mod platform_utility;
use std::{
    os::unix::fs::MetadataExt,
    path::{Path, PathBuf},
};
use tracing::{error, info, instrument};

use crate::{
    file_storage::platform_utility::{available_storage, create_mount, detach_device},
    storage::{Result, Storage},
};
use tokio::{
    fs::{self, File},
    io::copy,
};

#[derive(Clone)]
pub struct FileStorage {
    root: String,
    device_id: Option<String>,
}
pub struct FileStorageConfig {
    pub root: String,
    pub create_mount: bool,
    pub mount_size_in_mega_byte: u64,
}
impl FileStorage {
    pub async fn new(config: FileStorageConfig) -> Self {
        let root: &str = config.root.as_ref();
        let device_id = match create_mount(&config).await {
            Ok(v) => {
                info!(%root,"Created root for storage");
                v
            }
            Err(e) => {
                error!(%root,error=%e,"Error while creating the root for storage");
                panic!("Error during creating directory")
            }
        };
        match std::fs::create_dir_all(format!("{root}/staged")) {
            Ok(_v) => {
                info!(%root,"Created staging dir for storage");
            }
            Err(e) => {
                error!(%root,error=%e,"Error while creating the staging for storage");
                panic!("Error during staging directory")
            }
        }
        FileStorage {
            root: root.to_owned(),
            device_id,
        }
    }
    // not implementing drop trait because we need data to persist in crash
    pub fn cleanup(&self) {
        if let Some(device_id) = &self.device_id {
            match detach_device(device_id) {
                Ok(()) => {
                    // this will never panic
                    std::fs::remove_dir_all(self.root.clone()).unwrap();
                }
                Err(e) => {
                    panic!("Error while cleaning up storage {}", e);
                }
            }
        }
    }
    fn get_committed_path(&self, chunk_id: &str) -> PathBuf {
        Path::new(&self.root).join(chunk_id).to_path_buf()
    }
    fn get_staged_path(&self, chunk_id: &str) -> PathBuf {
        Path::new(&self.root).join("staged").join(chunk_id)
    }
}
impl Storage for FileStorage {
    #[instrument(name = "file_storage_write", skip(self, chunk_stream))]
    async fn write(
        &self,
        chunk_id: String,
        chunk_stream: &mut (impl tokio::io::AsyncRead + Unpin),
    ) -> Result<u64> {
        let chunk_path = self.get_staged_path(&chunk_id);
        let mut chunk_file = File::create_new(chunk_path).await?;
        let writer_byte_count = copy(chunk_stream, &mut chunk_file).await?;
        info!(%chunk_id,"data copied successfully");
        Ok(writer_byte_count)
    }
    #[instrument(name = "file_storage_commit", skip(self))]
    async fn commit(&self, chunk_id: String) -> Result<bool> {
        // check if file is in staged area
        let staged_path = self.get_staged_path(&chunk_id);
        let committed_path = self.get_committed_path(&chunk_id);
        if fs::metadata(staged_path.clone()).await.is_ok() {
            // move file from staged area to commited area
            tokio::fs::rename(staged_path, committed_path).await?;
        } else if fs::metadata(committed_path.clone()).await.is_err() {
            // this means file is neither committed not
            return Err("File is neither staged neither commited".into());
        }
        Ok(true)
    }
    #[instrument(name = "file_storage_read", skip(self))]
    async fn read(&self, chunk_id: String) -> Result<Box<dyn tokio::io::AsyncRead + Unpin + Send>> {
        let chunk_path = self.get_committed_path(&chunk_id);
        let chunk_file = File::open(chunk_path).await?;
        Ok(Box::new(chunk_file))
    }
    async fn delete(&self, chunk_id: String) -> Result<bool> {
        let exists = match fs::try_exists(self.get_committed_path(&chunk_id)).await {
            Ok(v) => v,
            Err(e) => {
                error!("error while checking if chunk exist e : {}", e);
                false
            }
        };
        if exists {
            fs::remove_file(self.get_committed_path(&chunk_id)).await?;
        }
        Ok(exists)
    }
    #[instrument(name = "file_storage_available_chunk", skip(self))]
    async fn available_chunks(&self) -> Result<Vec<String>> {
        info!(root=%self.root,"Reading the dir to get available chunks");
        let mut dir_enteries = fs::read_dir(&self.root).await?;
        let mut chunk_ids = vec![];
        while let Some(chunk) = dir_enteries.next_entry().await? {
            if !chunk
                .file_type()
                .await
                .map_err(|_| "Error checking if file is Dir")
                .unwrap()
                .is_dir()
            {
                chunk_ids.push(
                    chunk
                        .file_name()
                        .into_string()
                        .map_err(|_| "Invalid file name")?,
                );
            }
        }
        Ok(chunk_ids)
    }
    #[instrument(name = "file_storage_chunk_size", skip(self))]
    async fn get_chunk_size(&self, chunk_id: &str) -> Result<u64> {
        let chunk_path = self.get_committed_path(chunk_id);
        let chunk_file_metadata = fs::metadata(chunk_path).await?;
        Ok(chunk_file_metadata.size())
    }
    fn available_storage(&self) -> Result<usize> {
        available_storage(&self.root)
    }
}
#[cfg(test)]
impl Drop for FileStorage {
    fn drop(&mut self) {
        self.cleanup();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::storage::tests::storage_test;
    #[tokio::test]
    async fn file_storage_test() -> Result<()> {
        let config = FileStorageConfig {
            root: "./test".to_owned(),
            mount_size_in_mega_byte: 128,
            create_mount: true,
        };
        let storage = FileStorage::new(config).await;
        // fs::create_dir_all(&storage.root).await?;
        storage_test(storage).await // we have to await here because other wise
    }
}
