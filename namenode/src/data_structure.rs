#[derive(Clone, Default, Debug)]
pub struct ChunkBounderies {
    pub chunk_id: String,
    pub start_offset: u64,
    pub end_offset: u64,
}
