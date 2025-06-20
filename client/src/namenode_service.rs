use std::{error::Error, time::Duration};

use proto::generated::client_namenode::{
    ChunkMeta, DeleteFileRequest, FetchFileRequest, StoreFileRequest,
    client_name_node_client::ClientNameNodeClient,
};
use tonic::transport::{Channel, Endpoint};
use utilities::logger::{debug, info, instrument, tracing};

#[derive(Clone, Debug)]
pub struct NamenodeService {
    //address: String,
    connection: ClientNameNodeClient<Channel>,
}

impl NamenodeService {
    pub async fn new(addrs: String) -> Self {
        info!("connecting namenode at {:?}", addrs);
        let connection = Self::get_connection(addrs.clone()).await.unwrap();
        Self {
            //address: addrs,
            connection,
        }
    }
    async fn get_connection(
        addrs: String,
    ) -> Result<ClientNameNodeClient<Channel>, Box<dyn Error>> {
        let endpoint = Endpoint::from_shared(addrs.clone())
            .map_err(|e| format!("Error while creating endpoint {:?}", e))?
            .keep_alive_timeout(Duration::from_secs(60))
            .tcp_keepalive(Some(Duration::from_secs(60)))
            .connect_timeout(Duration::from_secs(5));
        let chanel = endpoint
            .connect()
            .await
            .map_err(|e| format!("Error while connecting to given address {:?}", e))?;
        info!("Connected to namenode successfully");
        Ok(ClientNameNodeClient::new(chanel))
    }
    #[instrument(skip(self))]
    pub async fn store_file(
        &mut self,
        file_name: String,
        file_size: u64,
    ) -> Result<Vec<ChunkMeta>, Box<dyn Error>> {
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
    pub async fn fetch_file(
        &mut self,
        file_name: String,
    ) -> Result<Vec<ChunkMeta>, Box<dyn Error>> {
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
        Ok(fetch_file_response.chunk_list.clone())
    }
    pub async fn delete_file(&mut self, file_name: String) -> Result<bool, Box<dyn Error>> {
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
