use proto::generated::client_data_node_server::ClientDataNode;
use proto::generated::{EchoRequest, EchoResponse, SaveChunkRequest, SaveChunkResponse};

#[derive(Default, Debug)]
pub struct ClientHandler;

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
    async fn save_chunk(
        &self,
        _request: tonic::Request<SaveChunkRequest>,
    ) -> Result<tonic::Response<SaveChunkResponse>, tonic::Status> {
        let response = SaveChunkResponse {
            address: "saved".to_owned(),
        };
        Ok(tonic::Response::new(response))
    }
}
