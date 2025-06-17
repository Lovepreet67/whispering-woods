use std::{error::Error, str::FromStr, time::Duration};

use proto::generated::{
    client_datanode::{
        FetchChunkRequest, StoreChunkRequest, client_data_node_client::ClientDataNodeClient,
    },
    client_namenode::DataNodeMeta,
};
use tokio::io::{AsyncRead, AsyncWriteExt};
use tonic::transport::{Channel, Endpoint};
use utilities::{
    logger::{instrument, trace, tracing},
    tcp_pool::TcpPool,
};

#[derive(Debug)]
pub struct DatanodeService {
    tcp_connection_pool: TcpPool,
}
impl DatanodeService {
    pub fn new() -> Self {
        Self {
            tcp_connection_pool: TcpPool::new(),
        }
    }
    async fn get_grpc_connection(
        &self,
        addrs: &str,
    ) -> Result<ClientDataNodeClient<Channel>, Box<dyn Error>> {
        trace!("Getting grpc connection addrs : {}", addrs);
        let endpoint = Endpoint::from_str(addrs)
            .map_err(|e| format!("Error while creating an endpoint {}", e))?
            .connect_timeout(Duration::from_secs(5));
        let channel = endpoint
            .connect()
            .await
            .map_err(|e| format!("Error while connecting to address {:?}", e))?;
        Ok(ClientDataNodeClient::new(channel))
    }
    #[instrument(skip(self, read_stream))]
    pub async fn store_chunk(
        &self,
        chunk_id: String,
        replica_set: Vec<DataNodeMeta>,
        read_stream: &mut (impl AsyncRead + Unpin),
    ) -> Result<(), Box<dyn Error>> {
        if replica_set.is_empty() {
            return Err("Empty replica set".into());
        }
        let store_chunk_request = StoreChunkRequest {
            chunk_id: chunk_id.clone(),
            replica_set: replica_set.clone(),
        };
        let mut data_node_grpc_client = self.get_grpc_connection(&replica_set[0].addrs).await?;
        // getting tcp address for the first replica set to which we will stream the read stream
        trace!("Sending store chunk request");
        let store_chunk_response = data_node_grpc_client
            .store_chunk(tonic::Request::new(store_chunk_request))
            .await?;
        let tcp_addrs = &store_chunk_response.get_ref().address;
        trace!(%tcp_addrs,"Got tcp address");
        // connect to tcp stream and push data as [chunk_id,write_mode,bytes from stream]
        let mut tcp_stream = self.tcp_connection_pool.get_connection(tcp_addrs).await?;
        trace!("writing chunk_id");
        tcp_stream.write_all(chunk_id.as_bytes()).await?;
        trace!("writing mode to file");
        tcp_stream.write_u8(1).await?;
        trace!("writing chunk to stream");
        tokio::io::copy(read_stream, &mut tcp_stream).await?;
        Ok(())
    }
    #[instrument(skip(self))]
    pub async fn fetch_chunk(
        &self,
        chunk_id: String,
        datanode_addrs: String,
    ) -> Result<impl AsyncRead + Unpin, Box<dyn Error>> {
        let fetch_chunk_request = FetchChunkRequest {
            chunk_id: chunk_id.clone(),
        };
        let mut data_node_grpc_client = self.get_grpc_connection(&datanode_addrs).await?;
        // getting tcp address for the first replica set to which we will stream the read stream
        trace!("Sending fetch chunk request");
        let fetch_chunk_response = data_node_grpc_client
            .fetch_chunk(tonic::Request::new(fetch_chunk_request))
            .await?
            .into_inner();
        trace!(tcp_addrs = %fetch_chunk_response.address,"Got tcp stream addres for datanode");
        let mut tcp_stream = self
            .tcp_connection_pool
            .get_connection(&fetch_chunk_response.address)
            .await?;
        trace!("writing chunk id");
        tcp_stream.write_all(chunk_id.as_bytes()).await?;
        // we will send the mode as read mode
        trace!("Writing mode to stream");
        tcp_stream.write_u8(2u8).await?;
        // returning tcp stream as reader since data node will push file content to tcp stream now
        Ok(tcp_stream)
    }
}
