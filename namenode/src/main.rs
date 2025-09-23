mod api_service;
mod chunk_generator;
mod client_handler;
mod config;
mod datanode;
mod ledger;
mod namenode_state;
use client_handler::ClientHandler;
use config::CONFIG;
use datanode::handler::DatanodeHandler;
use ledger::{default_ledger::DefaultLedger, replayer::Replayer};
use proto::generated::{
    client_namenode::client_name_node_server::ClientNameNodeServer,
    datanode_namenode::datanode_namenode_server::DatanodeNamenodeServer,
};
use std::{error::Error, sync::Arc};
use tokio::sync::Mutex;
use tonic::transport::Server;
use utilities::logger::{error, info, init_logger};

use crate::{
    api_service::rocket,
    namenode_state::{state_mantainer::StateMantainer, state_snapshot::SnapshotStore},
};

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    let _gaurd = init_logger(
        "Namenode",
        &CONFIG.id,
        CONFIG.log_level.clone(),
        &CONFIG.apm_endpoint,
        &CONFIG.log_base,
    );
    //let root_span = span!(Level::INFO, "root", service = "Namenode",node_id=%namenode_id);
    //let _entered = root_span.enter();
    info!(grcp_addrs=%CONFIG.external_grpc_addrs,"Starting the grpc server on address");
    info!(path=%CONFIG.ledger_file,"Creating a ledger");
    let ledger = match DefaultLedger::new(&CONFIG.ledger_file).await {
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
    let snapshot_store = SnapshotStore::new();
    let state_mantainer = StateMantainer::new(state.clone(), snapshot_store.clone()).await;
    state_mantainer.start();
    // starting API service
    tokio::spawn(async move {
        info!("Starting a rocket server");
        let _ = rocket(snapshot_store).launch().await;
    });
    // first we will start grpc server
    Server::builder()
        .add_service(ClientNameNodeServer::new(ClientHandler::new(
            state.clone(),
            Box::new(ledger),
        )))
        .add_service(DatanodeNamenodeServer::new(DatanodeHandler::new(
            state.clone(),
        )))
        .serve(format!("0.0.0.0:{}", CONFIG.internal_grpc_port).parse()?)
        .await?;
    Ok(())
}
