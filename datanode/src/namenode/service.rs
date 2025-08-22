use std::sync::Arc;

use proto::generated::datanode_namenode::{
    ConnectionRequest, HeartBeatRequest, StateSyncRequest,
    datanode_namenode_client::DatanodeNamenodeClient,
};
use tokio::sync::Mutex;
use tonic::transport::Channel;
use utilities::{
    grpc_channel_pool::GRPC_CHANNEL_POOL,
    logger::{error, info, instrument, trace, tracing},
    result::Result,
};

use crate::{config::CONFIG, datanode_state::DatanodeState};

pub struct NamenodeService {
    state: Arc<Mutex<DatanodeState>>,
}
impl NamenodeService {
    pub fn new(state: Arc<Mutex<DatanodeState>>) -> Self {
        Self { state }
    }
    async fn get_grpc_connection(&self, addrs: &str) -> Result<DatanodeNamenodeClient<Channel>> {
        let channel = GRPC_CHANNEL_POOL.get_channel(addrs).await.unwrap();
        Ok(DatanodeNamenodeClient::new(channel))
    }
    #[instrument(name = "service_namenode_connect", skip(self))]
    pub async fn connect(&self) -> Result<bool> {
        // now we will send connection Request
        let connection_request = ConnectionRequest {
            name: CONFIG.datanode_id.clone(),
            id: CONFIG.datanode_id.clone(),
            addrs: CONFIG.external_grpc_addrs.clone(),
        };
        let mut namenode_client = self.get_grpc_connection(&CONFIG.namenode_addrs).await?;
        match namenode_client
            .connection(tonic::Request::new(connection_request))
            .await
        {
            Ok(connected) => {
                info!("Connected to namenode sucessfully");
                Ok(connected.into_inner().connected)
            }
            Err(tonic_status) => {
                error!(error = ?tonic_status,"Error while connecting to namenode");
                Err(format!("Error while connecting to namenode {tonic_status}").into())
            }
        }
    }
    #[instrument(name = "service_namenode_send_heart_beat", skip(self))]
    pub async fn send_heart_beat(&self) -> Result<()> {
        // now we will send this address to the datanode
        let heart_beat_request = HeartBeatRequest {
            datanode_id: CONFIG.datanode_id.clone(),
        };
        let mut namenode_client = self.get_grpc_connection(&CONFIG.namenode_addrs).await?;
        namenode_client
            .heart_beat(tonic::Request::new(heart_beat_request))
            .await?;
        Ok(())
    }
    #[instrument(name = "service_namenode_state_sync", skip(self))]
    pub async fn state_sync(&self) -> Result<()> {
        let mut state = self.state.lock().await;
        trace!(?state, "sending state sync with");
        let state_sync_request = StateSyncRequest {
            id: CONFIG.datanode_id.clone(),
            available_chunks: state.available_chunks.clone(),
            availabe_storage: state.available_storage as u64,
        };
        let mut namenode_client = self.get_grpc_connection(&CONFIG.namenode_addrs).await?;
        let state_sync_response = match namenode_client
            .state_sync(tonic::Request::new(state_sync_request))
            .await
        {
            Ok(response) => response,
            Err(e) => {
                error!("Error while sending the state sync message to namenode, {e}");
                return Err(e.into());
            }
        };

        state_sync_response
            .into_inner()
            .chunks_to_be_deleted
            .into_iter()
            .for_each(|chunk| {
                state.to_be_deleted_chunks.insert(chunk);
            });
        Ok(())
    }
}
