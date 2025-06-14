use std::{error::Error, io};

use command_runner::CommandRunner;
use utilities::logger::{self, Level, info, span};
mod command_runner;
mod datanode_service;
mod file_chunker;
mod namenode_service;
#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    let _gaurd = logger::init_logger("Client", "Client_0");
    let root_span = span!(Level::INFO, "root", service = "Client");
    let _entered = root_span.enter();
    let namenode_addrs = std::env::args()
        .nth(1)
        .expect("Please provide Name Node address.");
    info!(namenode_addrs = %namenode_addrs,"starting the Client");
    let namenode = namenode_service::NamenodeService::new(namenode_addrs).await;
    let mut command_executer = CommandRunner::new(namenode);
    loop {
        let mut input = String::new();
        match io::stdin().read_line(&mut input) {
            Ok(_bytes) => match command_executer.handle_input(&mut input).await {
                Ok(message) => {
                    println!("Success : {}", message);
                }
                Err(message) => {
                    println!("Error : {}", message);
                }
            },
            Err(e) => {
                println!("error while reading the command {:?}", e);
            }
        }
    }
}
