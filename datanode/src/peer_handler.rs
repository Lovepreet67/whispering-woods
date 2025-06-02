use std::{error::Error, sync::Arc};

use proto::generated::datanode_datanode::{
    CreatePipelineRequest, CreatePipelineResponse, peer_server::Peer,
};
use tokio::{net::TcpStream, sync::Mutex};

use crate::{datanode_state::DatanodeState, peer_service::PeerService};
pub struct PeerHandler {
    state: Arc<Mutex<DatanodeState>>,
    peer_service: PeerService,
}

impl PeerHandler {
    pub fn new(state: Arc<Mutex<DatanodeState>>) -> Self {
        Self {
            state,
            peer_service: PeerService::new(),
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
impl Peer for PeerHandler {
    async fn create_pipeline(
        &self,
        request: tonic::Request<CreatePipelineRequest>,
    ) -> Result<tonic::Response<CreatePipelineResponse>, tonic::Status> {
        let mut create_pipeline_request = request.get_ref();
        // first we will send the create pipeling request to the next replica
        if create_pipeline_request.replica_set.len() > 1 {
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
        }
        let response = CreatePipelineResponse {
            address: self.state.lock().await.tcp_server_addrs.clone(),
        };
        Ok(tonic::Response::new(response))
    }
}
