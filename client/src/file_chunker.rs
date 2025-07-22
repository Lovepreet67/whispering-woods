use proto::generated::client_namenode::ChunkMeta;
use tokio::{
    fs::OpenOptions,
    io::{AsyncRead, AsyncReadExt, AsyncSeekExt},
};
use utilities::{logger::instrument, logger::tracing, result::Result};

#[derive(Clone)]
pub struct FileChunk {
    file_path: String,
    start_offset: u64,
    end_offset: u64,
}
impl FileChunk {
    pub async fn get_read_stream(&self) -> Result<impl AsyncRead + Unpin + Send + Sync> {
        let mut file = OpenOptions::new()
            .read(true)
            .open(&self.file_path)
            .await
            .map_err(|e| format!("Error while openning the file for chunk {e:?}"))?;
        file.seek(tokio::io::SeekFrom::Start(self.start_offset))
            .await
            .map_err(|e| format!("Error while seeking to starting offset {e:?}"))?;
        Ok(file.take(self.end_offset - self.start_offset))
    }
}

pub struct FileChunker<'a> {
    file_path: String,
    chunk_details: &'a Vec<ChunkMeta>,
    current_index: usize,
}

impl<'a> FileChunker<'a> {
    pub fn new(file_path: String, chunk_details: &'a Vec<ChunkMeta>) -> FileChunker<'a> {
        FileChunker {
            file_path,
            chunk_details,
            current_index: 0,
        }
    }
    #[instrument(name = "file_chunker_next_chunk", skip(self))]
    pub fn next_chunk(&mut self) -> Option<FileChunk> {
        if self.current_index >= self.chunk_details.len() {
            return None;
        }
        let index = self.current_index;
        self.current_index += 1;
        Some(FileChunk {
            file_path: self.file_path.clone(),
            start_offset: self.chunk_details[index].start_offset,
            end_offset: self.chunk_details[index].end_offset,
        })
    }
}
