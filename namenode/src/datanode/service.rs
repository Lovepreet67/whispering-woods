use proto::generated::namenode_datanode::{
    DeleteChunkRequest, namenode_datanode_client::NamenodeDatanodeClient,
};
use tonic::transport::Channel;
use utilities::{grpc_channel_pool::GRPC_CHANNEL_POOL, logger::{instrument,tracing}, result::Result};

#[derive(Clone, Copy)]
pub struct DatanodeService {}

impl DatanodeService {
    pub fn new() -> Self {
        Self {}
    }
    async fn get_connection(
        addrs: &str,
    ) -> Result<NamenodeDatanodeClient<Channel>> {
        let channel = GRPC_CHANNEL_POOL.get_channel(addrs).await.unwrap();
        Ok(NamenodeDatanodeClient::new(channel))
    }
    #[instrument(name="service_datanode_delete_chunk",skip(self))]
    pub async fn delete_chunk(
        &self,
        datanode_addrs: &str,
        chunk_id: &str,
    ) -> Result<bool> {
        let request = DeleteChunkRequest {
            id: chunk_id.to_owned(),
        };
        let response = Self::get_connection(datanode_addrs).await?
            .delete_chunk(tonic::Request::new(request)).await
            .map_err(|e|
         format!("Error while sending the delete message to datanode : {},  for chunk : {}, error : {}",datanode_addrs,chunk_id,e))?;
        Ok(response.get_ref().available)
    }
}
