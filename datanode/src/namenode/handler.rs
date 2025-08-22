use proto::generated::namenode_datanode::{
    DeleteChunkRequest, DeleteChunkResponse, ReplicateChunkRequest, ReplicateChunkResponse,
    namenode_datanode_server::NamenodeDatanode,
};
use storage::{file_storage::FileStorage, storage::Storage};
use tokio::io::{AsyncWriteExt, copy};
use utilities::{
    logger::{error, instrument, tracing},
    tcp_pool::TCP_CONNECTION_POOL,
};

use crate::peer::service::PeerService;

pub struct NamenodeHandler {
    store: FileStorage,
    peer_service: PeerService,
}

impl NamenodeHandler {
    pub fn new(store: FileStorage) -> Self {
        Self {
            store,
            peer_service: PeerService::default(),
        }
    }
    async fn transfer_chunk_content(
        &self,
        tcp_address: String,
        chunk_id: String,
    ) -> utilities::result::Result<()> {
        // now we will transfer this chunk to target
        let mut tcp_stream = TCP_CONNECTION_POOL.get_connection(&tcp_address).await?;
        tcp_stream.write_all(chunk_id.as_bytes()).await?;
        // first we will wrtie mode to write
        tcp_stream.write_u8(1).await?;
        let chunk_size = self.store.get_chunk_size(&chunk_id).await?;
        tcp_stream.write_u64(chunk_size).await?;
        let mut chunk_stream = self.store.read(chunk_id.clone()).await?;
        copy(&mut chunk_stream, &mut tcp_stream).await?;
        Ok(())
    }
}

#[tonic::async_trait]
impl NamenodeDatanode for NamenodeHandler {
    #[instrument(name="grpc_namenode_delete_chunk_handler",skip(self,request), fields(chunk_id = %request.get_ref().id))]
    async fn delete_chunk(
        &self,
        request: tonic::Request<DeleteChunkRequest>,
    ) -> Result<tonic::Response<DeleteChunkResponse>, tonic::Status> {
        let delete_chunk_request = request.into_inner();
        let chunk_id = delete_chunk_request.id;
        let exists = match self.store.delete(chunk_id).await {
            Ok(v) => v,
            Err(e) => {
                error!(%e,"error while deleting chunk in datanode ");
                return Err(tonic::Status::new(tonic::Code::Unavailable, e.to_string()));
            }
        };
        let delete_chunk_response = DeleteChunkResponse { available: exists };
        Ok(tonic::Response::new(delete_chunk_response))
    }
    #[instrument(name="grpc_namenode_replicate_chunk_handler",skip(self,request),fields(chunk_id= %request.get_ref().chunk_id,target_datanode=%request.get_ref().target_data_node))]
    async fn replicate_chunk(
        &self,
        request: tonic::Request<ReplicateChunkRequest>,
    ) -> Result<tonic::Response<ReplicateChunkResponse>, tonic::Status> {
        let replicate_chunk_request = request.into_inner();
        let chunk_id = replicate_chunk_request.chunk_id;
        // first we will send the grpc call
        let target_tcp_address = match self
            .peer_service
            .store_chunk(&chunk_id, &replicate_chunk_request.target_data_node)
            .await
        {
            Ok(addrs) => addrs,
            Err(e) => {
                error!("{e}");
                return Err(tonic::Status::new(tonic::Code::Internal, format!("{e}")));
            }
        };
        match self
            .transfer_chunk_content(target_tcp_address, chunk_id.clone())
            .await
        {
            Ok(_) => {}
            Err(e) => {
                error!("Error while transferring the chunk content, {e}");
                return Err(tonic::Status::new(tonic::Code::Internal, format!("{e}")));
            }
        };
        // next we will send commit chunk Request
        match self
            .peer_service
            .commit_chunk(&chunk_id, &replicate_chunk_request.target_data_node)
            .await
        {
            Ok(_) => {}
            Err(e) => {
                error!("Error while sending commit message , {e}");
                return Err(tonic::Status::new(tonic::Code::Unavailable, format!("{e}")));
            }
        }
        let store_chunk_response = ReplicateChunkResponse {};
        Ok(tonic::Response::new(store_chunk_response))
    }
}
