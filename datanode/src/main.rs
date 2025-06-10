use datanode_state::DatanodeState;
use namenode_service::NamenodeService;
use state_mantainer::StateMantainer;
use std::env;
use std::error::Error;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::Mutex;
use tokio::time::sleep;
use utilities::logger::{error, info, init_logger, trace};

pub mod client_handler;
mod datanode_state;
pub mod peer_handler;
mod peer_service;
mod state_mantainer;
pub mod tcp_service;
use client_handler::ClientHandler;
use proto::generated::client_datanode::client_data_node_server::ClientDataNodeServer;
use proto::generated::datanode_datanode::peer_server::PeerServer;
use storage::file_storage;
use tonic::transport::Server;
mod namenode_service;
mod tcp_stream_tee;

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
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
    let namenode_addrs = if args.len() > 3 {
        args[3].clone()
    } else {
        // defaut namenode address
        "http://127.0.0.1:7000".to_owned()
    };
    let _gaurd = init_logger("Datanode", &grpc_port);
    let addr = format!("127.0.0.1:{}", grpc_port).parse()?;
    info!("Starting the grpc server on address : {addr}");
    let state = Arc::new(Mutex::new(DatanodeState::new(
        grpc_port.clone(),
        format!("http://{}", addr),
        format!("127.0.0.1:{}", tcp_port),
        namenode_addrs,
    )));
    let ch = ClientHandler::new(state.clone());
    let ph = peer_handler::PeerHandler::new(state.clone());
    // first we will start grpc server
    let grpc_server = Server::builder()
        .add_service(ClientDataNodeServer::new(ch))
        .add_service(PeerServer::new(ph))
        .serve(addr);
    tokio::spawn(grpc_server);
    // we will create storage which will be used by the tcp service to serve a file
    let store = file_storage::FileStorage::new(format!("./temp/{}", grpc_port));
    info!("Starting the tcp server on grpc port: {}", tcp_port);
    let tcp_handler = tcp_service::TCPService::new(tcp_port, store.clone(), state.clone()).await?;
    tokio::spawn(async move {
        match tcp_handler.start_and_accept().await {
            Ok(_) => {}
            Err(e) => {
                error!("erorr while accepting tcp request {e}");
            }
        }
    });
    info!("Server s address : {addr}");

    // starting datanode state mantainer for datanode
    let state_mantainer = StateMantainer::new(store.clone(), state.clone());
    state_mantainer.start_sync_loop(Duration::from_secs(5));

    // heartbeat sending logic
    let namenode_service = NamenodeService::new(state.clone());
    tokio::spawn(async move {
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
                    trace!("sent heartbeat successfully")
                }
                Err(e) => {
                    error!("rror while sending heartbeat {e}");
                }
            }
            if x % 10 == 0 {
                x = 0;
                match namenode_service.state_sync().await {
                    Ok(_) => {
                        trace!("Sent state sync method to namenode");
                    }
                    Err(e) => {
                        error!("Error while sending the state sync method to namenode {e}");
                    }
                }
            }
            x += 1;
        }
    })
    .await;
    Ok(())
}
