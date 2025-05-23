use std::error::Error;

use crate::namenode_handler::NamenodeHandler;

pub struct CommandRunner {
    namenode: NamenodeHandler,
}
impl CommandRunner {
    pub fn new(namenode: NamenodeHandler) -> Self {
        CommandRunner { namenode }
    }
    pub async fn handle_input(&mut self, command: &mut str) -> Result<String, Box<dyn Error>> {
        match command {
            fetch_command if fetch_command.starts_with("fetch") => {
                let inputs: Vec<&str> = fetch_command.split_whitespace().collect();
                if inputs.len() < 3 {
                    return Err("Invalid fetch command ussage please use <help> to get help".into());
                }
                return self
                    .handle_fetch_file_command(inputs[1].to_owned(), inputs[2].to_owned())
                    .await;
            }
            store_command if store_command.starts_with("store") => {
                let inputs: Vec<&str> = store_command.split_whitespace().collect();
                if inputs.len() < 3 {
                    return Err("Invalid store command usage please use <help> to get help".into());
                }
                return self
                    .handle_store_file_command(inputs[1].to_owned(), inputs[2].to_owned())
                    .await;
            }
            delete_command if delete_command.starts_with("delete") => {
                let inputs: Vec<&str> = delete_command.split_whitespace().collect();
                if inputs.len() < 2 {
                    return Err(
                        "Invalid delete command ussage please use <help> to get help".into(),
                    );
                }
                return self.handle_delete_file_command(inputs[1].to_owned()).await;
            }
            help_command if help_command == "help\n" => {
                Ok("fetch command : fetch remote_file_location target_file_path\nstore command : store source_file_location target_remote_file_name\ndelete command : delete target_remote_file_name\n".to_owned())
            }
            _ => {
                Err(
                    "Invalid Command Please use valid command use :help to list available commands"
                        .into(),
                )
            }
        }
    }
    async fn handle_store_file_command(
        &mut self,
        local_file_path: String,
        remote_file_name: String,
    ) -> Result<String, Box<dyn Error>> {
        // get the file metadata
        let file_metadata = match tokio::fs::metadata(local_file_path.clone()).await {
            Ok(metadata) => metadata,
            Err(e) => {
                return Err(format!("Errror while reading file metadata : {:?}", e).into());
            }
        };
        if file_metadata.is_dir() {
            return Err(format!("Provided file path ({}) is dir", local_file_path).into());
        }
        // request namenode for chunk details
        println!("file size : {}", file_metadata.len());
        //let chunk_details = self.namenode.store_file(remote_file_name,file_metadata.len()).await?;
        // send each data node to setup pilepline
        // use chunk handler to send this data;
        // handler the chunk transformation
        Ok("File stored successfully".to_owned())
    }
    async fn handle_fetch_file_command(
        &mut self,
        remote_file_name: String,
        local_file_name: String,
    ) -> Result<String, Box<dyn Error>> {
        Ok("File fetched successfully".to_owned())
    }
    async fn handle_delete_file_command(
        &mut self,
        remote_file_name: String,
    ) -> Result<String, Box<dyn Error>> {
        Ok("File deleted successfully".to_owned())
    }
}
