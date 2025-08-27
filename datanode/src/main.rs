mod client;
mod config;
mod datanode_state;
mod namenode;
mod peer;
mod state_mantainer;
mod tcp;

use client::handler::ClientHandler;
use datanode_state::DatanodeState;
use namenode::handler::NamenodeHandler;
use namenode::service::NamenodeService;
use proto::generated::client_datanode::client_data_node_server::ClientDataNodeServer;
use proto::generated::datanode_datanode::peer_server::PeerServer;
use proto::generated::namenode_datanode::namenode_datanode_server::NamenodeDatanodeServer;
use state_mantainer::StateMantainer;
use std::error::Error;
use std::sync::Arc;
use std::time::Duration;
use storage::file_storage;
use tokio::sync::Mutex;
use tokio::time::sleep;
use tonic::transport::Server;
use utilities::logger::{error, info, init_logger, trace};

use crate::config::CONFIG;

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    let _gaurd = init_logger(
        "Datanode",
        &CONFIG.datanode_id,
        CONFIG.log_level.clone(),
        &CONFIG.apm_endpoint,
        &CONFIG.log_base,
    );
    info!(
        grpc_server_addrs = CONFIG.external_grpc_addrs,
        "Starting the grpc server on address"
    );
    let state = Arc::new(Mutex::new(DatanodeState::new()));
    info!(storage_path=%CONFIG.storage_path,"Creating storage");
    let store = file_storage::FileStorage::new(CONFIG.storage_path.clone());

    let ch = ClientHandler::new(state.clone(), store.clone());
    let ph = peer::handler::PeerHandler::new(state.clone(), store.clone());
    // first we will start grpc server
    info!(grpc_addr = %CONFIG.external_grpc_addrs,"Creating grpc server");
    let grpc_server = Server::builder()
        .add_service(ClientDataNodeServer::new(ch))
        .add_service(PeerServer::new(ph))
        .add_service(NamenodeDatanodeServer::new(NamenodeHandler::new(
            store.clone(),
        )))
        .serve(format!("0.0.0.0:{}", CONFIG.internal_grpc_port).parse()?);
    tokio::spawn(grpc_server);
    info!(grpc_addrs = %CONFIG.external_grpc_addrs,"grpc server is now running");
    // we will create storage which will be used by the tcp service to serve a file
    info!(tcp_addrs = %CONFIG.external_tcp_addrs,"Starting the tcp server");
    let tcp_handler = tcp::service::TCPService::new(
        format!("0.0.0.0:{}", CONFIG.internal_tcp_port).clone(),
        store.clone(),
        state.clone(),
    )
    .await
    .unwrap();
    tokio::spawn(async move {
        match tcp_handler.start_and_accept().await {
            Ok(_) => {}
            Err(e) => {
                error!("erorr while accepting tcp request {e}");
            }
        }
    });
    info!(tcp_addrs = %CONFIG.external_tcp_addrs,"TCP server is running now");

    // starting datanode state mantainer for datanode
    let state_mantainer = StateMantainer::new(store.clone(), state.clone());
    state_mantainer.start_sync_loop(Duration::from_secs(5));

    // heartbeat sending logic
    let namenode_service = NamenodeService::new(state.clone());
    tokio::spawn(async move {
        let max_retry_count = 5;
        let mut retry_count = 0;
        loop {
            match namenode_service.connect().await {
                Ok(v) => {
                    if v {
                        info!("successfully connected to the namenode");
                        break;
                    } else {
                        if retry_count == max_retry_count {
                            error!("Namenode refuse to connect after {max_retry_count} hence terminating");
                            std::process::exit(1);
                        }
                        retry_count += 1;
                        info!("Namenode refused to connect retrying...");
                    }
                }
                Err(e) => {
                    error!("{e}");
                    std::process::exit(1);
                }
            }
            sleep(Duration::from_secs(retry_count*5)).await;
        }
        // after every 10 heartbeats we will share state with name node;
        let mut x: u8 = 0;
        loop {
            sleep(Duration::from_secs(3)).await;
            match namenode_service.send_heart_beat().await {
                Ok(_) => {
                    //trace!("sent heartbeat successfully")
                }
                Err(e) => {
                    error!("error while sending heartbeat {e}");
                }
            }
            if x % 10 == 0 {
                x = 0;
                match namenode_service.state_sync().await {
                    Ok(_) => {
                        trace!("Sent state sync message to namenode");
                    }
                    Err(e) => {
                        error!("Error while sending the state sync method to namenode {e}");
                    }
                }
            }
            x += 1;
        }
    })
    .await?;
    Ok(())
}
