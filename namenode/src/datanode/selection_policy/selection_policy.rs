use proto::generated::client_namenode::DataNodeMeta;
use std::error::Error;
use tonic::async_trait;

#[async_trait]
pub trait DatanodeSelectionPolicy {
    async fn get_datanodes_to_store(
        &self,
        chunk_size: u64,
    ) -> Result<Vec<DataNodeMeta>, Box<dyn Error>>;
    async fn get_datanodes_to_serve(&self, chunk_id: &str) -> Result<DataNodeMeta, Box<dyn Error>>;
    async fn get_datanodes_to_repair(
        &self,
        chunk_id: &str,
    ) -> Result<(DataNodeMeta, DataNodeMeta), Box<dyn Error>>;
    async fn get_datanode_to_offload(
        &self,
        chunk_id: &str,
        count: usize,
    ) -> Result<Vec<DataNodeMeta>, Box<dyn Error>>;
}
