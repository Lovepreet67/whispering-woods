use std::error::Error;

use tokio::io::copy;
use utilities::logger::{error, instrument, trace, tracing};

use crate::{datanode_service::DatanodeService, namenode_service::NamenodeService};

pub struct FetchFileHandler {
    namenode: NamenodeService,
    datanode: DatanodeService,
}
impl FetchFileHandler {
    pub fn new(namenode: NamenodeService, datanode: DatanodeService) -> Self {
        Self { namenode, datanode }
    }
    #[instrument(skip(self))]
    pub async fn fetch_file(
        &mut self,
        remote_file_name: String,
        local_file_name: String,
    ) -> Result<String, Box<dyn Error>> {
        trace!("fetching the file {remote_file_name}");
        let chunk_details = self.namenode.fetch_file(remote_file_name.clone()).await?;
        trace!(chunk_details = ?chunk_details,"got chunk details for file");
        // now we will open a write stream to the target file
        let mut target_file = tokio::fs::File::options()
            .append(true)
            .create(true)
            .open(local_file_name)
            .await?;
        trace!("opened file in append only mode");
        for chunk_detail in &chunk_details {
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
}
