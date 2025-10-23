use proto::generated::{
    client_datanode::{
        CommitChunkRequest, FetchChunkRequest, StoreChunkRequest,
        client_data_node_client::ClientDataNodeClient,
    },
    client_namenode::DataNodeMeta,
};
use std::str::FromStr;
use tokio::io::AsyncRead;
use tonic::{metadata::MetadataValue, transport::Channel};
use utilities::{
    data_packet::DataPacket,
    grpc_channel_pool::GRPC_CHANNEL_POOL,
    logger::{error, instrument, trace, tracing},
    result::Result,
    tcp_pool::TCP_CONNECTION_POOL,
};

#[derive(Clone, Debug)]
pub struct DatanodeService {}
impl DatanodeService {
    pub fn new() -> Self {
        Self {}
    }
    async fn get_grpc_connection(&self, addrs: &str) -> Result<ClientDataNodeClient<Channel>> {
        let channel = GRPC_CHANNEL_POOL.get_channel(addrs).await.unwrap();
        Ok(ClientDataNodeClient::new(channel))
    }
    #[instrument(name = "datanode_service_store_chunk", skip(self, read_stream))]
    pub async fn store_chunk(
        &self,
        chunk_id: String,
        chunk_size: u64,
        replica_set: Vec<DataNodeMeta>,
        ticket: String,
        mut read_stream: (impl AsyncRead + Unpin),
    ) -> Result<()> {
        if replica_set.is_empty() {
            return Err("Empty replica set".into());
        }
        let mut store_chunk_request = tonic::Request::new(StoreChunkRequest {
            chunk_id: chunk_id.clone(),
            replica_set: replica_set.clone(),
        });
        store_chunk_request
            .metadata_mut()
            .insert("ticket", MetadataValue::from_str(&ticket)?);
        let mut data_node_grpc_client = self.get_grpc_connection(&replica_set[0].addrs).await?;
        // getting tcp address for the first replica set to which we will stream the read stream
        trace!("Sending store chunk request");
        let store_chunk_response = data_node_grpc_client
            .store_chunk(store_chunk_request)
            .await?;
        let tcp_addrs = &store_chunk_response.get_ref().address;
        trace!(%tcp_addrs,"Got tcp address");
        // we will first create the headers for tcp stream
        let mut tcp_headers = DataPacket::new();
        tcp_headers.insert("mode".to_string(), "Write".to_string());
        tcp_headers.insert("chunk_id".to_string(), chunk_id.clone());
        tcp_headers.insert("chunk_size".to_string(), chunk_size.to_string());
        tcp_headers.insert("ticket".to_string(), ticket.clone());
        let mut tcp_header_stream = tcp_headers.encode();
        // connect to tcp stream
        let mut tcp_stream = TCP_CONNECTION_POOL.get_connection(tcp_addrs).await?;
        trace!("Writing tcp headers to stream");
        tokio::io::copy(&mut tcp_header_stream, &mut tcp_stream).await?;
        trace!("tcp headers written to stream");
        let bytes_written = tokio::io::copy(&mut read_stream, &mut tcp_stream).await?;
        trace!("{bytes_written} Bytes written");
        // we will check for the number of writen bytes to stream
        let reply_packet = DataPacket::decode(&mut tcp_stream).await?;
        trace!("reply packet from the {:?}", reply_packet);
        let bytes_recieved_by_datanode: u64 = reply_packet.get("bytes_received")?.parse()?;
        if bytes_written != bytes_recieved_by_datanode {
            error!(
                "Bytes written to stream and recieved by stream are diffrent BytesWritten: {bytes_written}, BytesRecieved: {bytes_recieved_by_datanode}"
            );
            return Err("Bytes recieved are diffrent from bytes written".into());
        }
        trace!("Sending commit message");
        let mut commit_chunk_request = tonic::Request::new(CommitChunkRequest { chunk_id });
        commit_chunk_request
            .metadata_mut()
            .insert("ticket", MetadataValue::from_str(&ticket)?);
        data_node_grpc_client
            .commit_chunk(commit_chunk_request)
            .await?;
        trace!("committed successfully");
        Ok(())
    }
    #[instrument(name = "datanode_service_fetch_chunk", skip(self))]
    pub async fn fetch_chunk(
        &self,
        chunk_id: String,
        datanode_addrs: String,
        ticket: String,
    ) -> Result<impl AsyncRead + Unpin + Send + Sync> {
        let mut fetch_chunk_request = tonic::Request::new(FetchChunkRequest {
            chunk_id: chunk_id.clone(),
        });
        fetch_chunk_request
            .metadata_mut()
            .insert("ticket", MetadataValue::from_str(&ticket)?);
        let mut data_node_grpc_client = self.get_grpc_connection(&datanode_addrs).await?;
        // getting tcp address for the first replica set to which we will stream the read stream
        trace!("Sending fetch chunk request");
        let fetch_chunk_response = data_node_grpc_client
            .fetch_chunk(fetch_chunk_request)
            .await?
            .into_inner();
        trace!(tcp_addrs = %fetch_chunk_response.address,"Got tcp stream addres for datanode");
        let mut tcp_headers = DataPacket::new();
        tcp_headers.insert("chunk_id".to_string(), chunk_id.clone());
        tcp_headers.insert("ticket".to_string(), ticket.clone());
        tcp_headers.insert("mode".to_string(), "Read".to_string());
        let mut tcp_header_stream = tcp_headers.encode();
        let mut tcp_stream = TCP_CONNECTION_POOL
            .get_connection(&fetch_chunk_response.address)
            .await?;
        trace!("writing headers");
        tokio::io::copy(&mut tcp_header_stream, &mut tcp_stream).await?;
        // returning tcp stream as reader since data node will push file content to tcp stream now
        Ok(tcp_stream)
    }
}
