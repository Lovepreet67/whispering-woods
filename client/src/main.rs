use std::{error::Error, io};

use crate::{
    config::CONFIG,
    namenode::{auth_intercepter::NamenodeAuthIntercepter, service::NamenodeService},
};
use command_runner::CommandRunner;
use proto::generated::client_namenode::client_name_node_client::ClientNameNodeClient;
use utilities::{
    grpc_channel_pool::GRPC_CHANNEL_POOL,
    logger::{self, error, info},
};
mod chunk_joiner;
mod command_runner;
mod config;
mod datanode_service;
mod file_chunker;
mod namenode;
#[tokio::main]
async fn main() -> Result<(), Box<dyn Error + Send + Sync>> {
    let _gaurd = logger::init_logger(
        "Client",
        &CONFIG.client_id,
        CONFIG.log_level.clone(),
        &CONFIG.apm_endpoint,
        &CONFIG.log_base,
    );
    info!("Starting the Client");
    info!(namenode_addrs = %CONFIG.namenode_addrs,"Connecting to Namenode");
    let namenode_channel = match GRPC_CHANNEL_POOL.get_channel(&CONFIG.namenode_addrs).await {
        Ok(v) => v,
        Err(e) => {
            error!(error = %e,"Error while creating namnode channel so shutting down");
            return Err(e);
        }
    };
    let namenode = NamenodeService::new(ClientNameNodeClient::with_interceptor(
        namenode_channel,
        NamenodeAuthIntercepter,
    ));
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
