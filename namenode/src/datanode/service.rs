use std::{error::Error, str::FromStr, time::Duration};

use proto::generated::namenode_datanode::{
    DeleteChunkRequest, namenode_datanode_client::NamenodeDatanodeClient,
};
use tonic::transport::{Channel, Endpoint};
use utilities::{grpc_channel_pool::GRPC_CHANNEL_POOL, logger::debug};

#[derive(Clone, Copy)]
pub struct DatanodeService {}

impl DatanodeService {
    pub fn new() -> Self {
        Self {}
    }
    async fn get_connection(
        addrs: &str,
    ) -> Result<NamenodeDatanodeClient<Channel>, Box<dyn Error + Send + Sync>> {
        let channel = GRPC_CHANNEL_POOL.get_channel(addrs).await.unwrap();
        Ok(NamenodeDatanodeClient::new(channel))
    }

    pub async fn delete_chunk(
        &self,
        datanode_addrs: &str,
        chunk_id: &str,
    ) -> Result<bool, Box<dyn Error + Send + Sync>> {
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
