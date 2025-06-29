use proto::generated::{
    client_namenode::DataNodeMeta,
    datanode_datanode::{CommitChunkRequest, CreatePipelineRequest, peer_client::PeerClient},
};
use tonic::transport::Channel;
use utilities::{
    grpc_channel_pool::GRPC_CHANNEL_POOL,
    logger::{instrument, trace, tracing},
    result::Result,
    retry_policy::retry_with_backoff,
};

pub struct PeerService {}

impl PeerService {
    pub fn new() -> Self {
        Self {}
    }

    async fn get_grpc_connection(&self, addrs: &str) -> Result<PeerClient<Channel>> {
        let channel = GRPC_CHANNEL_POOL.get_channel(addrs).await?;
        Ok(PeerClient::new(channel))
    }
    #[instrument(skip(self))]
    pub async fn create_pipeline(
        &self,
        chunk_id: &str,
        replica_set: &[DataNodeMeta],
    ) -> Result<String> {
        trace!("Sending create pipeline request to peers");
        // since there are other replica we will have to send create pipeline message to next
        // replica
        let response = retry_with_backoff(
            // we retry everything for now
            || async {
                let create_pipeline_request = CreatePipelineRequest {
                    chunk_id: chunk_id.to_owned(),
                    replica_set: replica_set.to_vec(),
                };
                // send this request to request
                let mut client = self.get_grpc_connection(&replica_set[0].addrs).await?;
                let request = tonic::Request::new(create_pipeline_request);
                client.create_pipeline(request).await.map_err(|e| {
                    format!(
                        "error while sending the create pipline request, error : {}",
                        e
                    )
                    .into()
                })
            },
            3,
        )
        .await?;
        let create_pipelince_response = response.get_ref();
        Ok(create_pipelince_response.address.to_owned())
    }
    pub async fn commit_chunk(&self, chunk_id: &str, addrs: &str) -> Result<bool> {
        let response = retry_with_backoff(
            || async {
                let commit_chunk_request = CommitChunkRequest {
                    chunk_id: chunk_id.to_owned(),
                };
                let mut client = self.get_grpc_connection(addrs).await?;
                let request = tonic::Request::new(commit_chunk_request);
                client.commit_chunk(request).await.map_err(|e| {
                    format!(
                        "error while sending the commit message, error : {}",
                        e
                    )
                    .into()
                })

            },
            3,
        ).await?;
        let commit_chunk_response = response.into_inner();
        Ok(commit_chunk_response.committed)
    }
}
