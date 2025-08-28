use std::sync::Arc;

use proto::generated::datanode_datanode::{
    CommitChunkRequest, CommitChunkResponse, CreatePipelineRequest, CreatePipelineResponse,
    StoreChunkRequest, StoreChunkResponse, peer_server::Peer,
};
use storage::{file_storage::FileStorage, storage::Storage};
use tokio::{net::TcpStream, sync::Mutex};
use utilities::{
    logger::{error, instrument, trace, tracing},
    result::Result,
};

use crate::{config::CONFIG, datanode_state::DatanodeState, peer::service::PeerService};
pub struct PeerHandler {
    state: Arc<Mutex<DatanodeState>>,
    peer_service: PeerService,
    store: FileStorage,
}

impl PeerHandler {
    pub fn new(state: Arc<Mutex<DatanodeState>>, store: FileStorage) -> Self {
        Self {
            state,
            peer_service: PeerService::new(),
            store,
        }
    }
    async fn get_tcp_connection(&self, addrs: &str) -> Result<TcpStream> {
        Ok(TcpStream::connect(addrs).await.map_err(|e| {
            format!(
                "error while creating the tcp connection addrs : {}, error: {}",
                addrs, e
            )
        })?)
    }
}

#[tonic::async_trait]
impl Peer for PeerHandler {
    #[instrument(name="grpc_peer_create_pipeline",skip(self,request), fields(chunk_id = %request.get_ref().chunk_id))]
    async fn create_pipeline(
        &self,
        request: tonic::Request<CreatePipelineRequest>,
    ) -> std::result::Result<tonic::Response<CreatePipelineResponse>, tonic::Status> {
        let create_pipeline_request = request.get_ref();
        trace!(request = ?create_pipeline_request,"Got create pipeline request");
        // first we will send the create pipeling request to the next replica
        if create_pipeline_request.replica_set.len() > 1 {
            trace!(replica_set = ?create_pipeline_request.replica_set,"Passing create pipeline request to next");
            let tcp_address = match self
                .peer_service
                .create_pipeline(
                    &create_pipeline_request.chunk_id,
                    &create_pipeline_request.replica_set[1..],
                )
                .await
            {
                Ok(addrs) => addrs,
                Err(e) => {
                    return Err(tonic::Status::internal(format!(
                        "error while crating pipeline : {}",
                        e
                    )));
                }
            };
            let tcp_connection = match self.get_tcp_connection(&tcp_address).await {
                Ok(connection) => connection,
                Err(e) => {
                    return Err(tonic::Status::internal(format!(
                        "error while crating pipeline : {}",
                        e
                    )));
                }
            };
            let mut state = self.state.lock().await;
            state
                .chunk_to_pipline
                .insert(create_pipeline_request.chunk_id.clone(), tcp_connection);
            state.chunk_to_next_replica.insert(
                create_pipeline_request.chunk_id.clone(),
                create_pipeline_request.replica_set[1].addrs.clone(),
            );
        }
        let response = CreatePipelineResponse {
            address: CONFIG.external_tcp_addrs.clone(),
        };
        Ok(tonic::Response::new(response))
    }
    #[instrument(name="grpc_peer_store_chunk",skip(self,_request),fields(chunk_id = %_request.get_ref().chunk_id))]
    async fn store_chunk(
        &self,
        _request: tonic::Request<StoreChunkRequest>,
    ) -> std::result::Result<tonic::Response<StoreChunkResponse>, tonic::Status> {
        let response = StoreChunkResponse {
            address: CONFIG.external_tcp_addrs.clone(),
        };
        Ok(tonic::Response::new(response))
    }
    #[instrument(name="grpc_peer_commit_chunk",skip(self,request), fields(chunk_id = %request.get_ref().chunk_id))]
    async fn commit_chunk(
        &self,
        request: tonic::Request<CommitChunkRequest>,
    ) -> std::result::Result<tonic::Response<CommitChunkResponse>, tonic::Status> {
        trace!("Got commit chunk request from peer");
        let commit_chunk_request = request.into_inner();
        let mut state = self.state.lock().await;
        if let Some(next_replica_node_grpc) = state
            .chunk_to_next_replica
            .remove(&commit_chunk_request.chunk_id)
        {
            trace!("have next replica so transferring commit message");
            drop(state); // droping state to remove lock
            //send commit message to next_replica_node_grpc
            let _committed = match self
                .peer_service
                .commit_chunk(&commit_chunk_request.chunk_id, &next_replica_node_grpc)
                .await
            {
                Ok(v) => v,
                Err(e) => {
                    error!(error = %e,next_addrs=%next_replica_node_grpc,"Error while sending commit messag to next replica");
                    return Err(tonic::Status::new(
                        tonic::Code::Aborted,
                        "Error while commiting message on next replica",
                    ));
                }
            };
        }
        //else {
        //    state.available_chunks
        //}
        trace!("committed successfully");
        let committed = match self.store.commit(commit_chunk_request.chunk_id).await {
            Ok(v) => v,
            Err(e) => {
                error!(error=%e,"Error while commiting chunk");
                return Err(tonic::Status::new(
                    tonic::Code::Internal,
                    "Error while committing",
                ));
            }
        };
        trace!("commited successfully");
        let commit_chunk_response = CommitChunkResponse { committed };
        Ok(tonic::Response::new(commit_chunk_response))
    }
}
