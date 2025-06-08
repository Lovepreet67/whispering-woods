use std::{error::Error, str::FromStr, time::Duration};

use proto::generated::{
    client_datanode::{StoreChunkRequest, client_data_node_client::ClientDataNodeClient},
    client_namenode::DataNodeMeta,
};
use tokio::io::{AsyncRead, AsyncWriteExt};
use tonic::transport::{Channel, Endpoint};

pub struct ChunkHandler {}
impl ChunkHandler {
    pub fn new() -> Self {
        ChunkHandler {}
    }
    async fn get_grpc_connection(
        &mut self,
        addrs: &str,
    ) -> Result<ClientDataNodeClient<Channel>, Box<dyn Error>> {
        let endpoint = Endpoint::from_str(addrs)
            .map_err(|e| format!("Error while creating an endpoint {}", e))?
            .connect_timeout(Duration::from_secs(5));
        let channel = endpoint
            .connect()
            .await
            .map_err(|e| format!("Error while connecting to address {:?}", e))?;
        Ok(ClientDataNodeClient::new(channel))
    }
    async fn get_tcp_connection(
        &mut self,
        addrs: &str,
    ) -> Result<tokio::net::TcpStream, Box<dyn Error>> {
        tokio::net::TcpStream::connect(addrs)
            .await
            .map_err(|e| format!("Error while connecting to stream at {:?} {:?}", addrs, e).into())
    }
    pub async fn store_chunk(
        &mut self,
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
        let store_chunk_response = data_node_grpc_client
            .store_chunk(tonic::Request::new(store_chunk_request))
            .await?;
        let tcp_addrs = &store_chunk_response.get_ref().address;
        // connect to tcp stream and push data as [chunk_id,write_mode,bytes from stream]
        let mut tcp_stream = self.get_tcp_connection(tcp_addrs).await?;
        tcp_stream.write_all(chunk_id.as_bytes()).await?;
        tcp_stream.write_u8(1).await?;
        tokio::io::copy(read_stream, &mut tcp_stream).await?;
        Ok(())
    }
    pub async fn fetch_chunk(
        &mut self,
        chunk_id: String,
        datanode_addrs: String,
    ) -> Result<impl AsyncRead + Unpin, Box<dyn Error>> {
        let mut tcp_stream = self.get_tcp_connection(&datanode_addrs).await?;
        tcp_stream.write_all(chunk_id.as_bytes()).await?;
        Ok(tcp_stream)
    }
}
