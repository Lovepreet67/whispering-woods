use crate::{
    chunk_handler::ChunkHandler, file_chunker::FileChunker, namenode_handler::NamenodeHandler,
};
use std::{collections::HashMap, error::Error};
use tokio::io::{AsyncWriteExt, copy};
use utilities::logger::{debug, error, info, span, trace, tracing::Level};

pub struct CommandRunner {
    namenode: NamenodeHandler,
    datanode: ChunkHandler,
}
impl CommandRunner {
    pub fn new(namenode: NamenodeHandler) -> Self {
        CommandRunner {
            namenode,
            datanode: ChunkHandler::new(),
        }
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
        span!(Level::TRACE,"storing file",%local_file_path,%remote_file_name);
        // get the file metadata
        trace!("Fetching file metadata");
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
        info!("file size : {}", file_metadata.len());
        let chunk_details = self
            .namenode
            .store_file(remote_file_name, file_metadata.len())
            .await?;
        //debug!("got namenode response : {:?}",chunk_details);
        let mut file_chunker = FileChunker::new(local_file_path.clone(), &chunk_details);
        // send each data node to setup pilepline
        for chunk_detail in &chunk_details {
            span!(Level::TRACE ,"working on chunk",chunk_id = %chunk_detail.id);
            let mut read_stream = file_chunker.next_chunk().await?;
            let res = self
                .datanode
                .store_chunk(
                    chunk_detail.id.clone(),
                    chunk_detail.location.clone(),
                    &mut read_stream,
                )
                .await;
            match res {
                Ok(_) => {}
                Err(e) => {
                    error!("{}", e);
                }
            }
        }
        Ok("File stored successfully".to_owned())
    }
    async fn handle_fetch_file_command(
        &mut self,
        remote_file_name: String,
        local_file_name: String,
    ) -> Result<String, Box<dyn Error>> {
        span!(Level::TRACE,"fetching file ",%remote_file_name,%local_file_name);
        trace!("fetching the file {remote_file_name}");
        let chunk_details = self.namenode.fetch_file(remote_file_name.clone()).await?;
        trace!(%remote_file_name,chunk_details = ?chunk_details,"got chunk details for file");
        debug!("got chunk details for remote file {:?}", chunk_details);
        //let mut chunk_to_read_stream_map = HashMap::new();
        // now we will open a write stream to the target file
        let mut target_file = tokio::fs::File::options()
            .append(true)
            .create(true)
            .open(local_file_name)
            .await?;
        trace!("opened file in append only mode");
        for chunk_detail in &chunk_details {
            span!(Level::TRACE ,"working on chunk",chunk_id = %chunk_detail.id);
            let fetch_chunk_result = {
                self.datanode
                    .fetch_chunk(
                        chunk_detail.id.clone(),
                        chunk_detail.location[0].addrs.clone(),
                    )
                    .await
            };
            match fetch_chunk_result {
                Ok(mut read_stream) => {
                    //chunk_to_read_stream_map.insert(chunk_detail.id.clone(), read_stream);
                    copy(&mut read_stream, &mut target_file).await?;
                }
                Err(e) => {
                    error!(error = %e,"Error during chunk fetching");
                    return Err(e);
                }
            }
        }
        Ok("File fetched successfully".to_owned())
    }
    async fn handle_delete_file_command(
        &mut self,
        remote_file_name: String,
    ) -> Result<String, Box<dyn Error>> {
        trace!("sending a delete file request to the namenode");
        let delete_node = self.namenode.delete_file(remote_file_name).await?;
        if !delete_node {
            return Ok("File was not present".to_owned());
        }
        Ok("File deleted successfully".to_owned())
    }
}
