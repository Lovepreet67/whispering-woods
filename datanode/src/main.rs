mod client;
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
use utilities::{logger::{error, info, init_logger, span, trace, Instrument, Level}, tcp_pool};

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    let env = std::env::var("ENV").unwrap_or("local".to_owned());

    // For now we are using default port only
    let grpc_port = std::env::var("INTERNAL_GRPC_PORT").unwrap_or("3000".to_owned());
    let tcp_port = std::env::var("INTERNAL_TCP_PORT").unwrap_or("3001".to_owned());

    let external_grpc_addrs =
        std::env::var("EXTERNAL_GRPC_ADDRS").unwrap_or(format!("http://127.0.0.1:{}", grpc_port));
    let external_tcp_addrs =
        std::env::var("EXTERNAL_TCP_ADDRS").unwrap_or(format!("127.0.0.1:{}", tcp_port));

    let datanode_id = std::env::var("NODE_ID").unwrap_or(grpc_port.clone());
    let _gaurd = init_logger("Datanode", &datanode_id);
    let root_span = span!(Level::INFO, "root", service = "Datanode",%env,node_id=%datanode_id);
    let _entered = root_span.enter();
    let namenode_addrs = match std::env::var("NAMENODE_ADDRS") {
        Ok(v) => v,
        Err(e) => {
            error!(error= %e,"Error while getting namenode address hence shutting down");
            return Err("Error while getting namenode addrs".into());
        }
    };
    info!("Starting the grpc server on address : {external_grpc_addrs}");
    let state = Arc::new(Mutex::new(DatanodeState::new(
        datanode_id.clone(),
        external_grpc_addrs.clone(),
        external_tcp_addrs.clone(),
        namenode_addrs,
    )));
    let storage_path = match &env[..] {
        "local" => format!("./temp/{}", datanode_id),
        _ => "data".to_owned(),
    };
    info!(%storage_path,"Creating storage");
    let store = file_storage::FileStorage::new(storage_path);

    let ch = ClientHandler::new(state.clone(), store.clone());
    let ph = peer::handler::PeerHandler::new(state.clone(), store.clone());
    // first we will start grpc server
    info!(grpc_addr = %external_grpc_addrs,"Creating grpc server");
    let grpc_server = Server::builder()
        .add_service(ClientDataNodeServer::new(ch))
        .add_service(PeerServer::new(ph))
        .add_service(NamenodeDatanodeServer::new(NamenodeHandler::new(
            store.clone(),
        )))
        .serve(format!("0.0.0.0:{grpc_port}").parse()?);
    tokio::spawn(grpc_server.instrument(root_span.clone()));
    info!(grpc_addrs = %external_grpc_addrs,"grpc server is now running");
    // we will create storage which will be used by the tcp service to serve a file
    info!(tcp_addrs = %external_tcp_addrs,"Starting the tcp server");
    let tcp_handler = tcp::service::TCPService::new(
        format!("0.0.0.0:{tcp_port}").clone(),
        store.clone(),
        state.clone(),
    )
    .await?;
    let tcp_span = root_span.clone();
    tokio::spawn(
        async move {
            let _gaurd = tcp_span.enter();
            match tcp_handler.start_and_accept().await {
                Ok(_) => {}
                Err(e) => {
                    error!("erorr while accepting tcp request {e}");
                }
            }
        }
        .instrument(root_span.clone()),
    );
    info!(tcp_addrs = %external_tcp_addrs,"TCP server is running now");

    // starting datanode state mantainer for datanode
    let state_mantainer = StateMantainer::new(store.clone(), state.clone());
    state_mantainer.start_sync_loop(Duration::from_secs(5));

    // heartbeat sending logic
    let namenode_service = NamenodeService::new(state.clone());
    let namenode_service_span = root_span.clone();
    tokio::spawn(
        async move {
            let _gaurd = namenode_service_span.enter();
            match namenode_service.connect().await {
                Ok(v) => {
                    if v {
                        info!("successfully connected to the namenode");
                    } else {
                        info!("Namenode refused to connect hence terminating");
                        std::process::exit(1);
                    }
                }
                Err(e) => {
                    error!("{e}");
                    std::process::exit(1);
                }
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
        }
        .instrument(root_span.clone()),
    )
    .await?;
    Ok(())
}
