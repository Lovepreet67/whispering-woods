use proto::generated::client_namenode::ChunkMeta;
use tokio::io::{AsyncSeekExt, AsyncWriteExt, copy};
use utilities::{
    logger::{instrument, trace, tracing},
    result::Result,
};

#[derive(Clone)]
pub struct ChunkJoiner {
    file_path: String,
}

impl ChunkJoiner {
    #[instrument(name = "new_chunk_joiner")]
    pub async fn new(file_path: String, file_size: u64) -> Result<Self> {
        trace!("Creating file");
        // we are resorving space for file we are going to store
        let mut file = tokio::fs::OpenOptions::new()
            .create_new(true)
            .write(true)
            .open(&file_path)
            .await
            .map_err(|e| format!("Error while opening the file {e}"))?;
        file.seek(std::io::SeekFrom::Start(file_size - 1))
            .await
            .map_err(|e| format!("Error while resorving space {e}"))?;
        file.write_all(&[0])
            .await
            .map_err(|e| format!("Error while writing to file intitaly {e:?}"))?;
        Ok(Self { file_path })
    }
    #[instrument(skip(self, reader))]
    pub async fn join_chunk(
        &self,
        chunk_details: &ChunkMeta,
        reader: &mut (impl tokio::io::AsyncRead + Unpin),
    ) -> Result<()> {
        // this function create diffrent file descripter every time
        let mut file = tokio::fs::OpenOptions::new()
            .write(true)
            .open(&self.file_path)
            .await
            .map_err(|e| format!("Error while opening file  {e:?}"))?;
        file.seek(tokio::io::SeekFrom::Start(chunk_details.start_offset))
            .await
            .map_err(|e| format!("Error while seeking to start offset of chunk in file {e:?}"))?;
        copy(reader, &mut file)
            .await
            .map_err(|e| format!("Error while copying chunk from reader to file {e:?}"))?;
        Ok(())
    }
    #[instrument(name = "abort_join_chunk", skip(self))]
    pub async fn abort(&self) {
        tokio::fs::remove_file(&self.file_path).await;
    }
}
