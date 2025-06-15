mod client;
mod namenode;
mod peer;
mod tcp;
mod datanode_state;
mod state_mantainer;

use proto::generated::client_datanode::client_data_node_server::ClientDataNodeServer;
use proto::generated::datanode_datanode::peer_server::PeerServer;
use storage::file_storage;
use tonic::transport::Server;
use datanode_state::DatanodeState;
use client::handler::ClientHandler;
use namenode::handler::NamenodeHandler;
use namenode::service::NamenodeService;
use proto::generated::namenode_datanode::namenode_datanode_server::NamenodeDatanodeServer;
use state_mantainer::StateMantainer;
use std::env;
use std::error::Error;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::Mutex;
use tokio::time::sleep;
use utilities::logger::{error, info, init_logger, span, trace, Instrument, Level};

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
    let root_span = span!(Level::INFO, "root", service = "Datanode",node_id=%grpc_port);
    let _entered = root_span.enter();
    let addr = format!("127.0.0.1:{}", grpc_port).parse()?;
    info!("Starting the grpc server on address : {addr}");
    let state = Arc::new(Mutex::new(DatanodeState::new(
        grpc_port.clone(),
        format!("http://{}", addr),
        format!("127.0.0.1:{}", tcp_port),
        namenode_addrs,
    )));
    let ch = ClientHandler::new(state.clone());
    let ph = peer::handler::PeerHandler::new(state.clone());
    // first we will start grpc server
    let store = file_storage::FileStorage::new(format!("./temp/{}", grpc_port));
    let grpc_server = Server::builder()
        .add_service(ClientDataNodeServer::new(ch))
        .add_service(PeerServer::new(ph))
        .add_service(NamenodeDatanodeServer::new(NamenodeHandler::new(
            store.clone(),
        )))
        .serve(addr);
    tokio::spawn(grpc_server.instrument(root_span.clone()));
    // we will create storage which will be used by the tcp service to serve a file
    info!("Starting the tcp server on grpc port: {}", tcp_port);
    let tcp_handler = tcp::service::TCPService::new(tcp_port, store.clone(), state.clone()).await?;
    tokio::spawn(async move {
        match tcp_handler.start_and_accept().await {
            Ok(_) => {}
            Err(e) => {
                error!("erorr while accepting tcp request {e}");
            }
        }    }.instrument(root_span.clone())
 );
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
                    //trace!("sent heartbeat successfully")
                }
                Err(e) => {
                    error!("rror while sending heartbeat {e}");
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
    }.instrument(root_span.clone())
    ).await;
    Ok(())
}
