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
use std::{env, error::Error, sync::Arc};
use tokio::sync::Mutex;
use tonic::transport::Server;
use utilities::logger::{Level, error, info, init_logger, span};

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    let args: Vec<String> = env::args().collect();
    let grpc_port = if args.len() > 1 {
        args[1].clone()
    } else {
        "3000".to_owned()
    };
    let _gaurd = init_logger("Namenode", &grpc_port);
    let root_span = span!(Level::INFO, "root", service = "Namenode",node_id=%grpc_port);
    let _entered = root_span.enter();
    let addr = format!("127.0.0.1:{}", grpc_port).parse()?;
    info!("Starting the grpc server on address : {addr}");
    info!("Creating a ledger");
    let ledger = match DefaultLedger::new("./temp/namenode/history.log").await {
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
        .serve(addr)
        .await?;
    Ok(())
}
