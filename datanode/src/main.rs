use std::error::Error;

use proto::generated::client_data_node_server::{ClientDataNode, ClientDataNodeServer};
use proto::generated::{EchoRequest, EchoResponse,CreatePipelineRequest,CreatePipelineResponse,SaveChunkRequest,SaveChunkResponse};

use tonic::transport::Server;

#[derive(Default, Debug)]
struct ClientHandler {}

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
    async fn create_pipeline(&self,request:tonic::Request<CreatePipelineRequest>)->Result<tonic::Response<CreatePipelineResponse>,tonic::Status>{
        let response = CreatePipelineResponse {
            address: "hello".to_owned()
        };
        Ok(tonic::Response::new(response))
    }
    async fn save_chunk(&self,request: tonic::Request<SaveChunkRequest>) ->Result<tonic::Response<SaveChunkResponse>,tonic::Status>{
        let response = SaveChunkResponse{
            address:"saved".to_owned()
        };
        Ok(tonic::Response::new(response))
    }


}

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    let addr = "127.0.0.1:3000".parse()?;
    let ch = ClientHandler::default();
    Server::builder()
        .add_service(ClientDataNodeServer::new(ch))
        .serve(addr)
        .await?;
    Ok(())
}
