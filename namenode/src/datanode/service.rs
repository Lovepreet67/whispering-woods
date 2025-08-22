use proto::generated::{
    client_namenode::DataNodeMeta,
    namenode_datanode::{
        DeleteChunkRequest, ReplicateChunkRequest, namenode_datanode_client::NamenodeDatanodeClient,
    },
};
use tonic::transport::Channel;
use utilities::{
    grpc_channel_pool::GRPC_CHANNEL_POOL,
    logger::{instrument, tracing},
    result::Result,
};

#[derive(Clone, Copy, Default, Debug)]
pub struct DatanodeService {}

impl DatanodeService {
    pub fn new() -> Self {
        Self {}
    }
    async fn get_connection(addrs: &str) -> Result<NamenodeDatanodeClient<Channel>> {
        let channel = GRPC_CHANNEL_POOL.get_channel(addrs).await.unwrap();
        Ok(NamenodeDatanodeClient::new(channel))
    }
    #[instrument(name = "service_datanode_delete_chunk", skip(self))]
    pub async fn delete_chunk(&self, datanode_addrs: &str, chunk_id: &str) -> Result<bool> {
        let request = DeleteChunkRequest {
            id: chunk_id.to_owned(),
        };
        let response = Self::get_connection(datanode_addrs).await?
            .delete_chunk(tonic::Request::new(request)).await
            .map_err(|e|
         format!("Error while sending the delete message to datanode : {datanode_addrs},  for chunk : {chunk_id}, error : {e}"))?;
        Ok(response.get_ref().available)
    }
    #[instrument(name = "service_datanode_replicate_chunk", skip(self))]
    pub async fn replicate_chunk(
        &self,
        source_datanode_meta: DataNodeMeta,
        target_datanode_meta: DataNodeMeta,
        chunk_id: &str,
    ) -> Result<()> {
        let request = ReplicateChunkRequest {
            target_data_node: target_datanode_meta.addrs,
            chunk_id: chunk_id.to_owned(),
        };
        let source_address = source_datanode_meta.addrs;
        let _ = Self::get_connection(&source_address)
            .await?
            .replicate_chunk(tonic::Request::new(request))
            .await
            .map_err(|e| {
                format!(
                    "Error while sending the replicate chunk message to datanode : {source_address}, {e}"
                )
            })?;
        Ok(())
    }
}
