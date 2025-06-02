use std::error::Error;
use std::sync::Arc;

use proto::generated::client_datanode::client_data_node_server::ClientDataNode;
use proto::generated::client_datanode::{
    EchoRequest, EchoResponse, StoreChunkRequest, StoreChunkResponse,
};
use tokio::net::TcpStream;
use tokio::sync::Mutex;

use crate::datanode_state::DatanodeState;
use crate::peer_service::PeerService;

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
    async fn store_chunk(
        &self,
        request: tonic::Request<StoreChunkRequest>,
    ) -> Result<tonic::Response<StoreChunkResponse>, tonic::Status> {
        let mut store_request = request.get_ref();
        // first we will send the create pipeling request to the next replica
        if store_request.replica_set.len() > 1 {
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
        }
        let response = StoreChunkResponse {
            address: self.state.lock().await.tcp_server_addrs.clone(),
        };
        Ok(tonic::Response::new(response))
    }
}
