use proto::generated::namenode_datanode::{
    DeleteChunkRequest, DeleteChunkResponse, ReplicateChunkRequest, ReplicateChunkResponse,
    namenode_datanode_server::NamenodeDatanode,
};
use storage::{file_storage::FileStorage, storage::Storage};
use utilities::{
    data_packet::DataPacket,
    logger::{error, instrument, tracing},
    tcp_pool::TCP_CONNECTION_POOL,
    ticket::{
        ticket_decrypter::TicketDecrypter,
        types::{Operation, ServerTicket},
    },
};

use crate::peer::service::PeerService;
use std::sync::Arc;

pub struct NamenodeHandler {
    store: FileStorage,
    peer_service: PeerService,
    ticket_decrypter: Arc<Box<dyn TicketDecrypter>>,
}

impl NamenodeHandler {
    pub fn new(store: FileStorage, ticket_decrypter: Arc<Box<dyn TicketDecrypter>>) -> Self {
        Self {
            store,
            peer_service: PeerService::default(),
            ticket_decrypter,
        }
    }
    async fn transfer_chunk_content(
        &self,
        tcp_address: String,
        chunk_id: String,
        ticket: &str,
    ) -> utilities::result::Result<()> {
        // now we will transfer this chunk to target
        let mut tcp_headers = DataPacket::new();
        tcp_headers.insert("mode".to_string(), "Write".to_string());
        tcp_headers.insert("chunk_id".to_string(), chunk_id.clone());
        let chunk_size = self.store.get_chunk_size(&chunk_id).await?;
        tcp_headers.insert("chunk_size".to_string(), chunk_size.to_string());
        tcp_headers.insert("ticket".to_string(), ticket.to_string());
        let mut tcp_header_stream = tcp_headers.encode();
        // connect to tcp stream
        let mut tcp_stream = TCP_CONNECTION_POOL.get_connection(&tcp_address).await?;
        tokio::io::copy(&mut tcp_header_stream, &mut tcp_stream).await?;
        let mut chunk_stream = self.store.read(chunk_id.clone()).await?;
        tokio::io::copy(&mut chunk_stream, &mut tcp_stream).await?;
        let reply_packet = DataPacket::decode(&mut tcp_stream).await?;
        let _bytes_recieved_by_datanode: u64 = reply_packet.get("bytes_received")?.parse()?;
        Ok(())
    }
}

#[tonic::async_trait]
impl NamenodeDatanode for NamenodeHandler {
    #[instrument(name="grpc_namenode_delete_chunk_handler",skip(self,request), fields(chunk_id = %request.get_ref().id))]
    async fn delete_chunk(
        &self,
        mut request: tonic::Request<DeleteChunkRequest>,
    ) -> Result<tonic::Response<DeleteChunkResponse>, tonic::Status> {
        let server_ticket = request.extensions_mut().remove::<ServerTicket>().unwrap();
        let delete_chunk_request = request.into_inner();
        if let Operation::DeleteChunk { chunk_id } = server_ticket.operation {
            if delete_chunk_request.id != chunk_id {
                return Err(tonic::Status::new(
                    tonic::Code::InvalidArgument,
                    "Chunk id in ticket is not matching with chunk to be deleted",
                ));
            }
        } else {
            return Err(tonic::Status::new(
                tonic::Code::InvalidArgument,
                "Ticket not valid for operation",
            ));
        }

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
        mut request: tonic::Request<ReplicateChunkRequest>,
    ) -> Result<tonic::Response<ReplicateChunkResponse>, tonic::Status> {
        let server_ticket = request.extensions_mut().remove::<ServerTicket>().unwrap();
        let replicate_chunk_request = request.into_inner();
        if let Operation::ReplicateChunk { chunk_id } = server_ticket.operation {
            if replicate_chunk_request.chunk_id != chunk_id {
                return Err(tonic::Status::new(
                    tonic::Code::InvalidArgument,
                    "Chunk id in ticket is not matching with chunk to be replicated",
                ));
            }
        } else {
            return Err(tonic::Status::new(
                tonic::Code::InvalidArgument,
                "Ticket not valid for operation",
            ));
        }

        let chunk_id = replicate_chunk_request.chunk_id;
        let client_ticket = self
            .ticket_decrypter
            .decrypt_client_ticket(&replicate_chunk_request.ticket)
            .map_err(|e| {
                tonic::Status::new(
                    tonic::Code::InvalidArgument,
                    format!("Error while decrypting the ticket {:?}", e),
                )
            })?;
        // first we will send the grpc call
        let target_tcp_address = match self
            .peer_service
            .store_chunk(
                &chunk_id,
                &replicate_chunk_request.target_data_node,
                &client_ticket.encrypted_server_ticket,
            )
            .await
        {
            Ok(addrs) => addrs,
            Err(e) => {
                error!("{e}");
                return Err(tonic::Status::new(tonic::Code::Internal, format!("{e}")));
            }
        };
        match self
            .transfer_chunk_content(
                target_tcp_address,
                chunk_id.clone(),
                &client_ticket.encrypted_server_ticket,
            )
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
            .commit_chunk(
                &chunk_id,
                &replicate_chunk_request.target_data_node,
                &client_ticket.encrypted_server_ticket,
            )
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
