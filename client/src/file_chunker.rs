use std::error::Error;

use proto::generated::client_namenode::ChunkMeta;
use tokio::{
    fs::OpenOptions,
    io::{AsyncRead, AsyncReadExt, AsyncSeekExt},
};

pub struct FileChunker<'a> {
    file_path: String,
    chunk_details: &'a Vec<ChunkMeta>,
    current_index: usize,
}

impl<'a> FileChunker<'a> {
    pub fn new(file_path: String, chunk_details: &'a Vec<ChunkMeta>) -> FileChunker {
        FileChunker {
            file_path,
            chunk_details,
            current_index: 0,
        }
    }
    pub async fn next_chunk(&mut self) -> Result<impl AsyncRead + Unpin, Box<dyn Error>> {
        let mut chunk_header = OpenOptions::new()
            .read(true)
            .open(self.file_path.clone())
            .await
            .map_err(|e| {
                format!(
                    "Error while opening the file : {:} , error: {:?}",
                    self.file_path, e
                )
            })?;

        // Moving to the chunk header
        chunk_header
            .seek(std::io::SeekFrom::Start(
                self.chunk_details[self.current_index].start_offset,
            ))
            .await
            .map_err(|e| {
                format!(
                    "Error while creating the chunk header for chunk : {}, offset : {}, error : {}",
                    self.current_index, self.chunk_details[self.current_index].start_offset, e
                )
            })?;

        let tor = Ok(chunk_header.take(
            self.chunk_details[self.current_index].end_offset
                - self.chunk_details[self.current_index].start_offset,
        ));
        self.current_index += 1;
        tor
    }
}
