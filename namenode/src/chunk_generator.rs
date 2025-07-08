use std::cmp::min;

use utilities::logger::{instrument,tracing};

use crate::data_structure::ChunkBounderies;

pub trait ChunkGenerator {
    fn get_chunks(&self, file_size: u64, file_name: &str) -> Vec<ChunkBounderies>;
}

pub struct DefaultChunkGenerator {
    max_chunk_size: u64,
}

impl DefaultChunkGenerator {
    pub fn new(max_chunk_size: u64) -> Self {
        Self { max_chunk_size }
    }
}

impl ChunkGenerator for DefaultChunkGenerator {
    #[instrument(name="namenode_get_chunks",skip(self))]
    fn get_chunks(&self, file_size: u64, _file_name: &str) -> Vec<ChunkBounderies> {
        let mut curr_offset: u64 = 0;
        let mut chunks: Vec<ChunkBounderies> = vec![];
        while curr_offset < file_size {
            chunks.push(ChunkBounderies {
                chunk_id: uuid::Uuid::new_v4().to_string(),
                start_offset: curr_offset,
                end_offset: min(curr_offset + self.max_chunk_size, file_size),
            });
            curr_offset += self.max_chunk_size;
        }
        chunks
    }
}
