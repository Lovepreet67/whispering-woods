use proto::generated::client_datanode::client_data_node_server::ClientDataNode;
use proto::generated::client_datanode::{
    CommitChunkRequest, CommitChunkResponse, EchoRequest, EchoResponse, FetchChunkRequest,
    FetchChunkResponse, StoreChunkRequest, StoreChunkResponse,
};
use std::sync::Arc;
use storage::file_storage::FileStorage;
use storage::storage::Storage;
use tokio::sync::Mutex;
use utilities::logger::{error, instrument, trace, tracing};
use utilities::tcp_pool::TCP_CONNECTION_POOL;

use crate::datanode_state::DatanodeState;
use crate::peer::service::PeerService;

pub struct ClientHandler {
    state: Arc<Mutex<DatanodeState>>,
    peer_service: PeerService,
    store: FileStorage,
}
impl ClientHandler {
    pub fn new(state: Arc<Mutex<DatanodeState>>, store: FileStorage) -> Self {
        Self {
            state,
            peer_service: PeerService {},
            store,
        }
    }
}
#[tonic::async_trait]
impl ClientDataNode for ClientHandler {
    async fn echo(
        &self,
        request: tonic::Request<EchoRequest>,
    ) -> Result<tonic::Response<EchoResponse>, tonic::Status> {
        let request = request.get_ref();
        let response = EchoResponse {
            message: format!("echo {}", request.message).clone(),
        };
        Ok(tonic::Response::new(response))
    }
    #[instrument(skip(self,request),fields(chunk_id =  %request.get_ref().chunk_id))]
    async fn store_chunk(
        &self,
        request: tonic::Request<StoreChunkRequest>,
    ) -> Result<tonic::Response<StoreChunkResponse>, tonic::Status> {
        let store_request = request.get_ref();
        trace!(request = ?store_request,"Got request");
        // first we will send the create pipeling request to the next replica
        if store_request.replica_set.len() > 1 {
            trace!("replica set is >1 so we are creating piplines");
            let tcp_address = match self
                .peer_service
                .create_pipeline(&store_request.chunk_id, &store_request.replica_set[1..])
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
            trace!(tcp_addrs = %tcp_address,"Got the pipeline address");
            let tcp_connection = match TCP_CONNECTION_POOL.get_connection(&tcp_address).await {
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
                .insert(store_request.chunk_id.clone(), tcp_connection);
            state.chunk_to_next_replica.insert(
                store_request.chunk_id.clone(),
                store_request.replica_set[1].addrs.clone(),
            );
            trace!(
                "successfully established the tcp connection and stored in chunk to pipeline {:?}",
                state.chunk_to_pipline
            );
        }
        let response = StoreChunkResponse {
            address: self.state.lock().await.tcp_server_addrs.clone(),
        };
        Ok(tonic::Response::new(response))
    }
    #[instrument(skip(self,request),fields(chunk_id =  %request.get_ref().chunk_id))]
    async fn commit_chunk(
        &self,
        request: tonic::Request<CommitChunkRequest>,
    ) -> Result<tonic::Response<CommitChunkResponse>, tonic::Status> {
        trace!("Got commit chunk request from client");
        let commit_chunk_request = request.into_inner();
        let mut state = self.state.lock().await;
        if let Some(next_replica_node_grpc) = state
            .chunk_to_next_replica
            .remove(&commit_chunk_request.chunk_id)
        {
            trace!("Have next chunk so sending commit to it again");
            drop(state); // droping state to remove lock
            //send commit message to next_replica_node_grpc
            let _ = match self
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
        trace!("after if condition");
        //else {
        //    state.available_chunks
        //}
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
    #[instrument(skip(self,request),fields(chunk_id = %request.get_ref().chunk_id))]
    async fn fetch_chunk(
        &self,
        request: tonic::Request<FetchChunkRequest>,
    ) -> Result<tonic::Response<FetchChunkResponse>, tonic::Status> {
        let fetch_chunk_request = request.into_inner();
        trace!(request = ?fetch_chunk_request,"Got request");
        let state = self.state.lock().await;
        trace!(?state, "Got current state of datanode");
        if !state
            .available_chunks
            .contains(&fetch_chunk_request.chunk_id)
        {
            trace!(available_chunks = ?state.available_chunks,"chunk not available");
            return Err(tonic::Status::new(
                tonic::Code::NotFound,
                format!("chunk {} not available", fetch_chunk_request.chunk_id),
            ));
        }
        let fetch_chunk_response = FetchChunkResponse {
            address: state.tcp_server_addrs.clone(),
        };
        Ok(tonic::Response::new(fetch_chunk_response))
    }
}
