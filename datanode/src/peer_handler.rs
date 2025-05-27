use proto::generated::datanode_datanode::{
    CreatePipelineRequest, CreatePipelineResponse, peer_server::Peer,
};
pub struct PeerHandler;

#[tonic::async_trait]
impl Peer for PeerHandler {
    async fn create_pipeline(
        &self,
        _request: tonic::Request<CreatePipelineRequest>,
    ) -> Result<tonic::Response<CreatePipelineResponse>, tonic::Status> {
        let response = CreatePipelineResponse {
            address: "welcome".to_owned(),
        };
        Ok(tonic::Response::new(response))
    }
}
