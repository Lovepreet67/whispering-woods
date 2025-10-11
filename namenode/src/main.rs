mod api_service;
mod certificates;
mod chunk_generator;
mod client_handler;
mod config;
mod datanode;
mod grpc;
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
use std::sync::Arc;
use tokio::sync::Mutex;
use tonic::transport::Server;
use utilities::{
    logger::{error, info, init_logger},
    result::Result,
};

use crate::{
    api_service::rocket,
    certificates::certificate_generator::CertificateAuthority,
    grpc::auth::get_auth_intercepter_layer,
    namenode_state::{state_mantainer::StateMantainer, state_snapshot::SnapshotStore},
};

#[tokio::main]
async fn main() -> Result<()> {
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
    let (state_history, ticket_mint) = match ledger.replay() {
        Ok(v) => v,
        Err(e) => {
            error!(error=%e,"Error while reading the logs from ledger Hence shuting down");
            return Err(e);
        }
    };
    let ca = match CertificateAuthority::new() {
        Ok(c) => Arc::new(c),
        Err(e) => {
            error!("Error while creating a certificate authority {:}", e);
            return Err(e);
        }
    };

    let state = Arc::new(Mutex::new(state_history));
    let snapshot_store = SnapshotStore::new();
    // ticket generating mechanism

    let ticket_mint_thrd_safe = Arc::new(Mutex::new(ticket_mint));

    let state_mantainer = StateMantainer::new(
        state.clone(),
        snapshot_store.clone(),
        ticket_mint_thrd_safe.clone(),
    )
    .await;
    state_mantainer.start();
    let rocket_ca = ca.clone();
    // starting API service
    let rocket_ledger = ledger.clone();
    let rocket_ticket_mint = ticket_mint_thrd_safe.clone();
    tokio::spawn(async move {
        info!("Starting : rocket server");
        let result = rocket(
            snapshot_store,
            rocket_ca,
            rocket_ticket_mint,
            Box::new(rocket_ledger),
        )
        .launch()
        .await;
        error!("Rocket service has returned result : {:?}", result);
    });
    info!("grpc server starting");
    let root_cert = ca.get_root_cert().pem();
    Server::builder()
        .layer(get_auth_intercepter_layer(&root_cert))
        .add_service(ClientNameNodeServer::new(ClientHandler::new(
            state.clone(),
            Box::new(ledger),
            ticket_mint_thrd_safe.clone(),
        )))
        .add_service(DatanodeNamenodeServer::new(DatanodeHandler::new(
            state.clone(),
            ticket_mint_thrd_safe.clone(),
        )))
        .serve(format!("0.0.0.0:{}", CONFIG.internal_grpc_port).parse()?)
        .await?;
    Ok(())
}
