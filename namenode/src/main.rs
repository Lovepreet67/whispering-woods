mod chunk_generator;
mod client_handler;
mod data_structure;
mod datanode;
mod ledger;
mod namenode_state;
mod state_mantainer;

use client_handler::ClientHandler;
use datanode::handler::DatanodeHandler;
use ledger::{default_ledger::DefaultLedger, replayer::Replayer};
use proto::generated::{
    client_namenode::client_name_node_server::ClientNameNodeServer,
    datanode_namenode::datanode_namenode_server::DatanodeNamenodeServer,
};
use state_mantainer::StateMantainer;
use std::{error::Error, sync::Arc};
use tokio::sync::Mutex;
use tonic::transport::Server;
use utilities::logger::{Level, error, info, init_logger, span};

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    let env = std::env::var("ENV").unwrap_or("local".to_owned());
    let self_base_addrs = std::env::var("BASE_URL").unwrap_or("127.0.0.1".to_owned());
    let grpc_port = std::env::var("GRPC_PORT").unwrap_or("3000".to_owned());
    let namenode_id = std::env::var("NAMENODE_ID").unwrap_or(format!("namenode_{grpc_port}"));
    let _gaurd = init_logger("Namenode", &namenode_id);
    let root_span = span!(Level::INFO, "root", service = "Namenode",node_id=%namenode_id);
    let _entered = root_span.enter();
    let addr = format!("{self_base_addrs}:{grpc_port}");
    info!("Starting the grpc server on address : {addr}");
    let ledger_file = match &env[..] {
        "local" => "./temp/namenode/history.log",
        _ => "/state/history.log",
    };
    info!(path=%ledger_file,"Creating a ledger");
    let ledger = match DefaultLedger::new(ledger_file).await {
        Ok(v) => v,
        Err(e) => {
            error!(error=%e,"Error while intiating the ledger Hence shuting down");
            return Err(e);
        }
    };
    let state_history = match ledger.replay() {
        Ok(v) => v,
        Err(e) => {
            error!(error=%e,"Error while reading the logs from ledger Hence shuting down");
            return Err(e);
        }
    };
    let state = Arc::new(Mutex::new(state_history));
    let state_mantainer = StateMantainer::new(state.clone());
    state_mantainer.start();
    // first we will start grpc server
    Server::builder()
        .add_service(ClientNameNodeServer::new(ClientHandler::new(
            state.clone(),
            Box::new(ledger),
        )))
        .add_service(DatanodeNamenodeServer::new(DatanodeHandler::new(
            state.clone(),
        )))
        .serve(format!("0.0.0.0:{grpc_port}").parse()?)
        .await?;
    Ok(())
}
