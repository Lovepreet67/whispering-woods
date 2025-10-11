use proto::generated::{
    client_namenode::DataNodeMeta,
    namenode_datanode::{
        DeleteChunkRequest, ReplicateChunkRequest, namenode_datanode_client::NamenodeDatanodeClient,
    },
};
use std::sync::Arc;
use tokio::sync::Mutex;
use tonic::transport::Channel;
use utilities::{
    grpc_channel_pool::GRPC_CHANNEL_POOL,
    logger::{instrument, tracing},
    result::Result,
    ticket::ticket_mint::TicketMint,
};

#[derive(Clone)]
pub struct DatanodeService {
    ticket_mint: Arc<Mutex<TicketMint>>,
}

impl DatanodeService {
    pub fn new(ticket_mint: Arc<Mutex<TicketMint>>) -> Self {
        Self { ticket_mint }
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
        let ticket = self.ticket_mint.lock().await.mint_ticket(
            &source_datanode_meta.id,
            &target_datanode_meta.id,
            utilities::ticket::types::Operation::StoreChunk {
                chunk_id: chunk_id.to_string(),
            },
        )?;
        let request = ReplicateChunkRequest {
            target_data_node: target_datanode_meta.addrs,
            chunk_id: chunk_id.to_owned(),
            ticket,
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
