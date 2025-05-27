use proto::generated::client_datanode::client_data_node_server::ClientDataNode;
use proto::generated::client_datanode::{
    EchoRequest, EchoResponse, StoreChunkRequest, StoreChunkResponse,
};

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
    async fn store_chunk(
        &self,
        _request: tonic::Request<StoreChunkRequest>,
    ) -> Result<tonic::Response<StoreChunkResponse>, tonic::Status> {
        let response = StoreChunkResponse {
            address: "saved".to_owned(),
        };
        Ok(tonic::Response::new(response))
    }
}
