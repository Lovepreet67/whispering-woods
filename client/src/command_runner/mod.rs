mod delete_file_handler;
mod fetch_file_handler;
mod store_file_handler;

use crate::datanode_service::DatanodeService;
use delete_file_handler::DeleteFileHandler;
use fetch_file_handler::FetchFileHandler;
use store_file_handler::StoreFileHandler;
use utilities::result::Result;

pub struct CommandRunner {
    store_file_handler: StoreFileHandler,
    fetch_file_handler: FetchFileHandler,
    delete_file_handler: DeleteFileHandler,
}
impl CommandRunner {
    pub fn new(namenode: crate::namenode::service::NamenodeService) -> Self {
        CommandRunner {
            store_file_handler: StoreFileHandler::new(namenode.clone(), DatanodeService::new()),
            fetch_file_handler: FetchFileHandler::new(namenode.clone(), DatanodeService::new()),
            delete_file_handler: DeleteFileHandler::new(namenode),
        }
    }
    pub async fn handle_input(&mut self, command: &mut str) -> Result<String> {
        match command {
            fetch_command if fetch_command.starts_with("fetch") => {
                let inputs: Vec<&str> = fetch_command.split_whitespace().collect();
                if inputs.len() < 3 {
                    return Err("Invalid fetch command ussage please use <help> to get help".into());
                }
                return self.fetch_file_handler.fetch_file(inputs[1].to_owned(), inputs[2].to_owned()).await;
            }
            store_command if store_command.starts_with("store") => {
                let inputs: Vec<&str> = store_command.split_whitespace().collect();
                if inputs.len() < 3 {
                    return Err("Invalid store command usage please use <help> to get help".into());
                }
                return self.store_file_handler.store_file(inputs[1].to_owned(), inputs[2].to_owned())
                    .await;
            }
            delete_command if delete_command.starts_with("delete") => {
                let inputs: Vec<&str> = delete_command.split_whitespace().collect();
                if inputs.len() < 2 {
                    return Err(
                        "Invalid delete command ussage please use <help> to get help".into(),
                    );
                }
                return self.delete_file_handler.delete_file(inputs[1].to_owned()).await;
            }
            help_command if help_command == "help\n" => {
                Ok("\nfetch command : fetch remote_file_location target_file_path\nstore command : store source_file_location target_remote_file_name\ndelete command : delete target_remote_file_name\n".to_owned())
            }
            _ => {
                Err(
                    "Invalid Command Please use valid command use :help to list available commands"
                        .into(),
                )
            }
        }
    }
}
