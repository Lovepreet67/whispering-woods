use utilities::{
    logger::{Instrument, error, info, instrument, trace, tracing},
    result::Result,
    retry_policy::retry_with_backoff,
    ticket::ticket_decrypter::TicketDecrypter,
};

use crate::{
    datanode_service::DatanodeService, file_chunker::FileChunker,
    namenode::service::NamenodeService,
};
use std::sync::Arc;

pub struct StoreFileHandler {
    namenode: NamenodeService,
    datanode: DatanodeService,
    ticket_decrypter: Arc<Box<dyn TicketDecrypter>>,
}
impl StoreFileHandler {
    pub fn new(
        namenode: NamenodeService,
        datanode: DatanodeService,
        ticket_decrypter: Arc<Box<dyn TicketDecrypter>>,
    ) -> Self {
        Self {
            namenode,
            datanode,
            ticket_decrypter,
        }
    }
    #[instrument(skip(self))]
    pub async fn store_file(
        &mut self,
        local_file_path: String,
        remote_file_name: String,
    ) -> Result<String> {
        // get the file metadata
        trace!("Fetching file metadata");
        let file_metadata = match tokio::fs::metadata(local_file_path.clone()).await {
            Ok(metadata) => metadata,
            Err(e) => {
                return Err(format!("Errror while reading file metadata : {e:?}").into());
            }
        };
        if file_metadata.is_dir() {
            return Err(format!("Provided file path ({local_file_path}) is dir").into());
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
        let mut handles = vec![];
        for chunk_detail in &chunk_details {
            let datanode = self.datanode.clone();
            let file_chunk = file_chunker.next_chunk().unwrap();
            let chunk_detail = chunk_detail.clone();
            let server_ticket = self
                .ticket_decrypter
                .decrypt_client_ticket(&chunk_detail.ticket)?
                .encrypted_server_ticket;

            handles.push(tokio::spawn(
                async move {
                    retry_with_backoff(
                        || async {
                            trace!(id = %chunk_detail.id,"working on chunk");
                            let read_stream = file_chunk.get_read_stream().await?;
                            let res = datanode
                                .store_chunk(
                                    chunk_detail.id.clone(),
                                    chunk_detail.end_offset - chunk_detail.start_offset,
                                    chunk_detail.location.clone(),
                                    server_ticket.clone(),
                                    read_stream,
                                )
                                .await;
                            match res {
                                Ok(_) => Ok(()),
                                Err(e) => {
                                    error!("{}", e);
                                    Err(e)
                                }
                            }
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
                    error!(error=%e,"aborting the store file operation");
                    // we will implement something to tell namenode that this file store has been
                    // aborted
                    return Err(e.into());
                }
            }
        }
        // if all things go well we will tell namenode to commit
        Ok("File stored successfully".to_owned())
    }
}
