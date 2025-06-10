use std::{error::Error, io};

use command_runner::CommandRunner;
use namenode_handler::NamenodeHandler;
use utilities::logger::{self, info};
mod chunk_handler;
mod command_runner;
mod file_chunker;
mod namenode_handler;
#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    let _gaurd = logger::init_logger("Client", "Client_0");
    let namenode_addrs = std::env::args()
        .nth(1)
        .expect("Please provide Name Node address.");
    let namenode = NamenodeHandler::new(namenode_addrs).await;
    let mut command_executer = CommandRunner::new(namenode);
    info!("starting the Client");
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
