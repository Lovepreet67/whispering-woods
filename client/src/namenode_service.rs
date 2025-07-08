use proto::generated::client_namenode::{
    ChunkMeta, DeleteFileRequest, FetchFileRequest, FetchFileResponse, StoreFileRequest,
    client_name_node_client::ClientNameNodeClient,
};
use tonic::transport::Channel;
use utilities::{
    logger::{debug, instrument, tracing},
    result::Result,
};

#[derive(Clone, Debug)]
pub struct NamenodeService {
    connection: ClientNameNodeClient<Channel>,
}

impl NamenodeService {
    pub fn new(connection: ClientNameNodeClient<Channel>) -> Self {
        Self { connection }
    }
    #[instrument(name="namenode_store_file",skip(self))]
    pub async fn store_file(
        &mut self,
        file_name: String,
        file_size: u64,
    ) -> Result<Vec<ChunkMeta>> {
        let store_file_request = StoreFileRequest {
            file_name: file_name.clone(),
            file_size,
        };
        let tonic_request = tonic::Request::new(store_file_request);
        let store_file_response = self
            .connection
            .store_file(tonic_request)
            .await
            .map_err(|e| {
                format!(
                    "error while storing a file {} to the namenode {:?}",
                    file_name, e
                )
            })?
            .into_inner();
        Ok(store_file_response.chunk_list.clone())
    }
    #[instrument(name="namenode_fetch_file",skip(self))]
    pub async fn fetch_file(&mut self, file_name: String) -> Result<FetchFileResponse> {
        let fetch_file_request = FetchFileRequest {
            file_name: file_name.clone(),
        };
        let tonic_request = tonic::Request::new(fetch_file_request);
        let fetch_file_response = self
            .connection
            .fetch_file(tonic_request)
            .await
            .map_err(|e| {
                format!(
                    "error while fetching a file {} from the namenode {:?}",
                    file_name, e
                )
            })?
            .into_inner();
        Ok(fetch_file_response)
    }
    #[instrument(name="namenode_delete_file",skip(self))]
    pub async fn delete_file(&mut self, file_name: String) -> Result<bool> {
        debug!("delete file for #{}#", file_name);
        let delete_file_request = DeleteFileRequest {
            file_name: file_name.clone(),
        };
        let tonic_request = tonic::Request::new(delete_file_request);
        let delete_file_response = self
            .connection
            .delete_file(tonic_request)
            .await
            .map_err(|e| {
                format!(
                    "error while fetching a file {} from the namenode {:?}",
                    file_name, e
                )
            })?
            .into_inner();
        Ok(delete_file_response.file_present)
    }
}
