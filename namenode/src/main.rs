use std::{env, error::Error, sync::Arc};

use client_handler::ClientHandler;
use datanode_handler::DatanodeHandler;
use log::info;
use namenode_state::NamenodeState;
use proto::generated::{
    client_namenode::client_name_node_server::ClientNameNodeServer,
    datanode_namenode::datanode_namenode_server::DatanodeNamenodeServer,
};
use state_mantainer::StateMantainer;
use tokio::sync::Mutex;
use tonic::transport::Server;

mod chunk_generator;
mod client_handler;
mod data_structure;
mod datanode_handler;
mod datanode_selection_policy;
mod datanode_service;
mod namenode_state;
mod state_mantainer;

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    env_logger::init();
    let args: Vec<String> = env::args().collect();
    let grpc_port = if args.len() > 1 {
        args[1].clone()
    } else {
        "3000".to_owned()
    };
    let addr = format!("127.0.0.1:{}", grpc_port).parse()?;
    info!("Starting the grpc server on address : {addr}");

    let state = Arc::new(Mutex::new(NamenodeState::default()));
    let state_mantainer = StateMantainer::new(state.clone());
    state_mantainer.start();
    // first we will start grpc server
    Server::builder()
        .add_service(ClientNameNodeServer::new(ClientHandler::new(state.clone())))
        .add_service(DatanodeNamenodeServer::new(DatanodeHandler::new(
            state.clone(),
        )))
        .serve(addr)
        .await?;
    Ok(())
}
