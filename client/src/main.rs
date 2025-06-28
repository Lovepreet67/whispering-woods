use std::{error::Error, io};

use command_runner::CommandRunner;
use proto::generated::client_namenode::client_name_node_client::ClientNameNodeClient;
use utilities::{
    grpc_channel_pool::GRPC_CHANNEL_POOL,
    logger::{self, Level, error, info, span},
};
mod chunk_joiner;
mod command_runner;
mod datanode_service;
mod file_chunker;
mod namenode_service;
#[tokio::main]
async fn main() -> Result<(), Box<dyn Error + Send + Sync>> {
    let client_id = std::env::var("CLIENT_NAME").unwrap_or("Client_0".to_owned());
    let _gaurd = logger::init_logger("Client", &client_id);
    let root_span = span!(Level::INFO, "root", service = "Client",%client_id);
    let _entered = root_span.enter();
    let namenode_addrs = match std::env::var("NAMENODE_ADDRS") {
        Ok(v) => v,
        Err(e) => {
            error!(error=%e,"Error while fetching namenode address hence shutting down");
            return Err("Namenode address not found".into());
        }
    };
    info!(namenode_addrs = %namenode_addrs,"starting the Client");
    info!("connecting to namenode");
    let namenode_channel = match GRPC_CHANNEL_POOL.get_channel(&namenode_addrs).await {
        Ok(v) => v,
        Err(e) => {
            error!(error = %e,"Error while creating namnode channel so shutting down");
            return Err(e);
        }
    };
    let namenode =
        namenode_service::NamenodeService::new(ClientNameNodeClient::new(namenode_channel));
    let mut command_executer = CommandRunner::new(namenode);
    loop {
        let mut input = String::new();
        match io::stdin().read_line(&mut input) {
            Ok(_bytes) => match command_executer.handle_input(&mut input).await {
                Ok(message) => {
                    info!("Success : {}", message);
                }
                Err(message) => {
                    error!("Error : {}", message);
                }
            },
            Err(e) => {
                error!("error while reading the command {:?}", e);
            }
        }
    }
}
