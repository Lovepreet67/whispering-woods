use tonic::async_trait;

#[async_trait]
pub trait Recorder {
    async fn store_file(&self, file_name: &str, no_of_chunks: u64);
    async fn store_chunk(
        &self,
        file_name: &str,
        order: u64,
        chunk_id: &str,
        start_offset: u64,
        end_offset: u64,
    );
    async fn delete_file(&self, file_name: &str);
    async fn delete_chunk(&self, file_name: &str, chunk_id: &str);
}
