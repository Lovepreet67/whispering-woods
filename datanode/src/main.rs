use log::info;
use std::env;
use std::error::Error;

pub mod client_handler;
pub mod peer_handler;
pub mod tcp_service;
use client_handler::ClientHandler;
use proto::generated::client_data_node_server::ClientDataNodeServer;
use proto::generated::peer_server::PeerServer;
use storage::file_storage;
use tonic::transport::Server;

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    env_logger::init();
    let args: Vec<String> = env::args().collect();
    let grpc_port = if args.len() > 1 {
        args[1].clone()
    } else {
        "3000".to_owned()
    };
    let tcp_port = if args.len() > 2 {
        args[2].clone()
    } else {
        "3001".to_owned()
    };
    let addr = format!("127.0.0.1:{}", grpc_port).parse()?;
    info!("Starting the grpc server on address : {addr}");
    let ch = ClientHandler;
    let ph = peer_handler::PeerHandler;

    // first we will start grpc server
    let grpc_server = Server::builder()
        .add_service(ClientDataNodeServer::new(ch))
        .add_service(PeerServer::new(ph))
        .serve(addr);
    tokio::spawn(grpc_server);
    // we will create storage which will be used by the tcp service to serve a file
    let store = file_storage::FileStorage::new("./temp".to_owned());
    info!("Starting the tcp server on grpc port: {}", tcp_port);
    let tcp_handler = tcp_service::TCPService::new(tcp_port, store).await?;
    tcp_handler.start_and_accept().await?;
    info!("Server s address : {addr}");
    Ok(())
}
