use utilities::{
    logger::{Instrument, error, info, instrument, trace, tracing},
    result::Result,
    retry_policy::retry_with_backoff,
};

use crate::{
    chunk_joiner::ChunkJoiner, datanode_service::DatanodeService,
    namenode::service::NamenodeService,
};

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
    ) -> Result<String> {
        trace!("fetching the file {remote_file_name}");
        let fetch_file_response = self.namenode.fetch_file(remote_file_name.clone()).await?;
        trace!(chunk_details = ?fetch_file_response.chunk_list,"got chunk details for file");
        // now we will open a write stream to the target file
        let file_size =
            fetch_file_response.chunk_list[fetch_file_response.chunk_list.len() - 1].end_offset;
        trace!("Creating chunk joiner");
        let chunk_joiner = ChunkJoiner::new(local_file_name.clone(), file_size).await?;
        trace!("Chunk joiner created successfully");
        let mut handles = vec![];
        for chunk_detail in &fetch_file_response.chunk_list {
            let chunk_joiner = chunk_joiner.clone();
            let datanode = self.datanode.clone();
            let chunk_detail = chunk_detail.clone();
            handles.push(tokio::spawn(
                async move {
                    retry_with_backoff(
                        || async {
                            let fetch_chunk_result = datanode
                                .fetch_chunk(
                                    chunk_detail.id.clone(),
                                    chunk_detail.location[0].addrs.clone(),
                                )
                                .await;
                            match fetch_chunk_result {
                                Ok(mut read_stream) => {
                                    let _ = chunk_joiner
                                        .join_chunk(&chunk_detail, &mut read_stream)
                                        .await;
                                }
                                Err(e) => {
                                    error!(error = %e,"Error during chunk fetching");
                                    return Err(e);
                                }
                            }
                            Ok(())
                        },
                        3,
                    )
                    .await
                }
                .in_current_span(),
            ));
        }

        for handle in handles {
            match handle.await {
                Ok(_) => {}
                Err(e) => {
                    error!("Error during fetching chunk {e:?}");
                    info!("Freeing the reserverd space");
                    chunk_joiner.abort().await;
                    info!("Space freed");
                    return Err(format!("Error in one handler {e:?}").into());
                }
            }
        }
        Ok("File fetched successfully".to_owned())
    }
}
