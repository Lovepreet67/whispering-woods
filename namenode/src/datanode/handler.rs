use std::sync::Arc;

use tokio::sync::Mutex;
use utilities::{
    logger::{instrument, tracing},
    ticket::ticket_mint::TicketMint,
};

use crate::namenode_state::NamenodeState;
use crate::namenode_state::datanode_details::DatanodeDetail;

use proto::generated::datanode_namenode::{
    ConnectionRequest, ConnectionResponse, HeartBeatRequest, HeartBeatResponse, StateSyncRequest,
    StateSyncResponse, StoreChunkTicketRequest, StoreChunkTicketResponse,
    datanode_namenode_server::DatanodeNamenode,
};

pub struct DatanodeHandler {
    state: Arc<Mutex<NamenodeState>>,
    ticket_mint: Arc<Mutex<TicketMint>>,
}
impl DatanodeHandler {
    pub fn new(
        namenode_state: Arc<Mutex<NamenodeState>>,
        ticket_mint: Arc<Mutex<TicketMint>>,
    ) -> Self {
        Self {
            state: namenode_state,
            ticket_mint,
        }
    }
}

#[tonic::async_trait]
impl DatanodeNamenode for DatanodeHandler {
    #[instrument(name="grpc_datanode_heart_beat",skip(self,request),fields(datanode_id= %request.get_ref().datanode_id))]
    async fn heart_beat(
        &self,
        request: tonic::Request<HeartBeatRequest>,
    ) -> Result<tonic::Response<HeartBeatResponse>, tonic::Status> {
        let heart_beat_request = request.into_inner();
        //trace!("got heartbeat request {:?}", heart_beat_request);
        let mut state = self.state.lock().await;
        let response = if let Some(datanode_details) = state
            .datanode_to_detail_map
            .get_mut(&heart_beat_request.datanode_id)
        {
            datanode_details.mark_heartbeat();
            HeartBeatResponse {
                connection_alive: true,
            }
        } else {
            HeartBeatResponse {
                connection_alive: false,
            }
        };
        Ok(tonic::Response::new(response))
    }
    #[instrument(name="grpc_datanode_connection",skip(self,request),fields(datanode_id= %request.get_ref().id))]
    async fn connection(
        &self,
        request: tonic::Request<ConnectionRequest>,
    ) -> Result<tonic::Response<ConnectionResponse>, tonic::Status> {
        let connection_request = request.into_inner();
        /*trace!(
            "got connection request from data node {}",
            connection_request.id
        );*/
        let mut state = self.state.lock().await;
        // if the connection already exist we will accept the connection and mark node as active
        let response = if let Some(datanode_details) =
            state.datanode_to_detail_map.get_mut(&connection_request.id)
        {
            if datanode_details.is_active() {
                ConnectionResponse {
                    connected: false,
                    msg: "Connection already exist for the specified id".to_owned(),
                }
            } else {
                datanode_details.mark_heartbeat();
                ConnectionResponse {
                    connected: true,
                    msg: "Connection restablished".to_owned(),
                }
            }
        } else {
            state.datanode_to_detail_map.insert(
                connection_request.id.clone(),
                DatanodeDetail::new(
                    connection_request.id,
                    connection_request.name,
                    connection_request.addrs,
                ),
            );
            ConnectionResponse {
                connected: true,
                msg: "Connected successfully".to_owned(),
            }
        };
        drop(state);
        Ok(tonic::Response::new(response))
    }
    #[instrument(name="grpc_datanode_state_sync",skip(self,request),fields(datanode_id= %request.get_ref().id))]
    async fn state_sync(
        &self,
        request: tonic::Request<StateSyncRequest>,
    ) -> Result<tonic::Response<StateSyncResponse>, tonic::Status> {
        let state_sync_request = request.into_inner();
        let mut state = self.state.lock().await;
        if let Some(datanode_details) = state.datanode_to_detail_map.get_mut(&state_sync_request.id)
        {
            datanode_details.sync_state(state_sync_request.availabe_storage);
        }
        let mut chunks_to_be_deleted = vec![];
        for chunk_id in &state_sync_request.available_chunks {
            if let Some(chunk_details) = state.chunk_id_to_detail_map.get_mut(chunk_id) {
                if chunk_details.is_deleted() {
                    chunks_to_be_deleted.push(chunk_id.to_owned());
                    continue;
                }
                chunk_details.add_location(&state_sync_request.id);
            } else {
                chunks_to_be_deleted.push(chunk_id.to_owned());
            }
        }
        state
            .chunk_id_to_detail_map
            .iter_mut()
            .filter(|(_, chunk_meta)| chunk_meta.locations.contains(&state_sync_request.id))
            .for_each(|(chunk_id, chunk_meta)| {
                if !state_sync_request.available_chunks.contains(chunk_id) {
                    chunk_meta.remove_location(&state_sync_request.id);
                }
            });
        let response = StateSyncResponse {
            chunks_to_be_deleted,
        };
        Ok(tonic::Response::new(response))
    }
    #[instrument(name="grpc_datanode_store_chunk_ticket",skip(self,request),fields(datanode_id= %request.get_ref().source_id,target_id = %request.get_ref().target_id, chunk_id = %request.get_ref().chunk_id))]
    async fn store_chunk_ticket(
        &self,
        request: tonic::Request<StoreChunkTicketRequest>,
    ) -> Result<tonic::Response<StoreChunkTicketResponse>, tonic::Status> {
        let mut tm = self.ticket_mint.lock().await;
        let store_chunk_request = request.get_ref();
        let ticket = tm
            .mint_ticket(
                &store_chunk_request.source_id,
                &store_chunk_request.target_id,
                utilities::ticket::types::Operation::CreatePipeline {
                    chunk_id: store_chunk_request.chunk_id.to_string(),
                },
            )
            .map_err(|e| {
                tonic::Status::new(
                    tonic::Code::Internal,
                    format!("Error while generating the ticket {e}"),
                )
            })?;
        let pipeline_response = StoreChunkTicketResponse { ticket };
        Ok(tonic::Response::new(pipeline_response))
    }
}
