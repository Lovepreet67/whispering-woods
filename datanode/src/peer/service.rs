use proto::generated::{
    client_namenode::DataNodeMeta,
    datanode_datanode::{
        CommitChunkRequest, CreatePipelineRequest, StoreChunkRequest, peer_client::PeerClient,
    },
};
use std::str::FromStr;
use tonic::{metadata::MetadataValue, transport::Channel};
use utilities::{
    grpc_channel_pool::GRPC_CHANNEL_POOL,
    logger::{instrument, trace, tracing},
    result::Result,
    retry_policy::retry_with_backoff,
};

#[derive(Default)]
pub struct PeerService {}

impl PeerService {
    pub fn new() -> Self {
        Self {}
    }

    async fn get_grpc_connection(&self, addrs: &str) -> Result<PeerClient<Channel>> {
        let channel = GRPC_CHANNEL_POOL.get_channel(addrs).await?;
        Ok(PeerClient::new(channel))
    }
    #[instrument(name = "service_peer_create_pipeline", skip(self))]
    pub async fn create_pipeline(
        &self,
        chunk_id: &str,
        replica_set: &[DataNodeMeta],
        ticket: &str,
    ) -> Result<String> {
        trace!("Sending create pipeline request to peers");
        // since there are other replica we will have to send create pipeline message to next
        // replica
        let response = retry_with_backoff(
            // we retry everything for now
            || async {
                let mut create_pipeline_request = tonic::Request::new(CreatePipelineRequest {
                    chunk_id: chunk_id.to_owned(),
                    replica_set: replica_set.to_vec(),
                });
                create_pipeline_request
                    .metadata_mut()
                    .insert("ticket", MetadataValue::from_str(ticket)?);
                // send this request to request
                let mut client = self.get_grpc_connection(&replica_set[0].addrs).await?;
                client
                    .create_pipeline(create_pipeline_request)
                    .await
                    .map_err(|e| {
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
    #[instrument(name = "service_peer_store_chunk", skip(self))]
    pub async fn store_chunk(&self, chunk_id: &str, addrs: &str, ticket: &str) -> Result<String> {
        let response = retry_with_backoff(
            || async {
                let mut store_chunk_request =tonic::Request::new( StoreChunkRequest {
                    chunk_id: chunk_id.to_owned(),
                });
                store_chunk_request.metadata_mut().insert("ticket", MetadataValue::from_str(ticket)?);
                let mut client = self.get_grpc_connection(addrs).await?;
                client.store_chunk(store_chunk_request).await.map_err(|e| {
                    format!(
                        "Error while sending store chunk request to {addrs}, for chunk {chunk_id}, {e:?}"
                    )
                    .into()
                })
            },
            3,
        )
        .await?;
        let store_chunk_response = response.into_inner();
        Ok(store_chunk_response.address)
    }
    #[instrument(name = "service_peer_commit_chunk", skip(self))]
    pub async fn commit_chunk(&self, chunk_id: &str, addrs: &str, ticket: &str) -> Result<bool> {
        let response = retry_with_backoff(
            || async {
                let mut commit_chunk_request = tonic::Request::new(CommitChunkRequest {
                    chunk_id: chunk_id.to_owned(),
                });
                commit_chunk_request
                    .metadata_mut()
                    .insert("ticket", MetadataValue::from_str(ticket)?);

                let mut client = self.get_grpc_connection(addrs).await?;
                client
                    .commit_chunk(commit_chunk_request)
                    .await
                    .map_err(|e| {
                        format!("error while sending the commit message, error : {}", e).into()
                    })
            },
            3,
        )
        .await?;
        let commit_chunk_response = response.into_inner();
        Ok(commit_chunk_response.committed)
    }
}
