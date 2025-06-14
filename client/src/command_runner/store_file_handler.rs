use std::error::Error;

use utilities::logger::{Level, error, info, instrument, span, trace, tracing};

use crate::{
    datanode_service::DatanodeService, file_chunker::FileChunker, namenode_service::NamenodeService,
};

pub struct StoreFileHandler {
    namenode: NamenodeService,
    datanode: DatanodeService,
}
impl StoreFileHandler {
    pub fn new(namenode: NamenodeService, datanode: DatanodeService) -> Self {
        Self { namenode, datanode }
    }
    #[instrument(skip(self))]
    pub async fn store_file(
        &mut self,
        local_file_path: String,
        remote_file_name: String,
    ) -> Result<String, Box<dyn Error>> {
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
        trace!(?chunk_details, "got namenode response");
        let mut file_chunker = FileChunker::new(local_file_path.clone(), &chunk_details);
        // send each data node to setup pilepline
        for chunk_detail in &chunk_details {
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
}
