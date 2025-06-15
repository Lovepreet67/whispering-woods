use std::error::Error;
use std::sync::Arc;
use proto::generated::client_datanode::client_data_node_server::ClientDataNode;
use proto::generated::client_datanode::{
    EchoRequest, EchoResponse, FetchChunkRequest, FetchChunkResponse, StoreChunkRequest,
    StoreChunkResponse,
};
use tokio::net::TcpStream;
use tokio::sync::Mutex;
use utilities::logger::{tracing,debug, instrument, trace};

use crate::datanode_state::DatanodeState;
use crate::peer::service::PeerService;

pub struct ClientHandler {
    state: Arc<Mutex<DatanodeState>>,
    peer_service: PeerService,
}
impl ClientHandler {
    pub fn new(state: Arc<Mutex<DatanodeState>>) -> Self {
        Self {
            state,
            peer_service: PeerService {},
        }
    }
    async fn get_tcp_connection(&self, addrs: &str) -> Result<TcpStream, Box<dyn Error>> {
        Ok(TcpStream::connect(addrs).await.map_err(|e| {
            format!(
                "error while creating the tcp connection addrs : {}, error: {}",
                addrs, e
            )
        })?)
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
                .insert(store_request.chunk_id.clone(), tcp_connection);
            debug!(
                "successfully established the tcp connection and stored in chunk to pipeline {:?}",
                state.chunk_to_pipline
            );
        }
        let response = StoreChunkResponse {
            address: self.state.lock().await.tcp_server_addrs.clone(),
        };
        Ok(tonic::Response::new(response))
    }
    #[instrument(skip(self,request),fields(chunk_id = %request.get_ref().chunk_id))]
    async fn fetch_chunk(
        &self,
        request: tonic::Request<FetchChunkRequest>,
    ) -> Result<tonic::Response<FetchChunkResponse>, tonic::Status> {
        let fetch_chunk_request = request.into_inner();
        trace!(request = ?fetch_chunk_request,"Got request");
        let state = self.state.lock().await;
        trace!(?state,"Got current state of datanode");
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
