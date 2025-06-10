use std::{error::Error, str::FromStr, sync::Arc, time::Duration};

use proto::generated::datanode_namenode::{
    ConnectionRequest, HeartBeatRequest, HeartBeatResponse, StateSyncRequest,
    datanode_namenode_client::DatanodeNamenodeClient, datanode_namenode_server::DatanodeNamenode,
};
use tokio::sync::Mutex;
use tonic::transport::{Channel, Endpoint};
use utilities::logger::{info, trace};

use crate::datanode_state::DatanodeState;

pub struct NamenodeService {
    state: Arc<Mutex<DatanodeState>>,
}
impl NamenodeService {
    pub fn new(state: Arc<Mutex<DatanodeState>>) -> Self {
        Self { state }
    }
    async fn get_grpc_connection(
        &self,
        addrs: &str,
    ) -> Result<DatanodeNamenodeClient<Channel>, Box<dyn Error>> {
        let endpoint = Endpoint::from_str(addrs)
            .map_err(|e| format!("Error while creating endpoint {}", e))?
            .connect_timeout(Duration::from_secs(5));
        let channel = endpoint.connect().await.map_err(|e| {
            format!(
                "error while connecting to namenode at addrs : {} , error : {}",
                addrs, e
            )
        })?;
        Ok(DatanodeNamenodeClient::new(channel))
    }

    pub async fn connect(&self) -> Result<bool, Box<dyn Error>> {
        let state = self.state.lock().await;
        let namenode_addrs = state.namenode_addrs.clone();
        let id = state.get_id();
        let grpc_server_addrs = state.grpc_server_addrs.clone();
        drop(state);
        // now we will send connection Request
        let connection_request = ConnectionRequest {
            id,
            addrs: grpc_server_addrs,
            name: "no one".to_owned(),
        };
        let mut namenode_client = self.get_grpc_connection(&namenode_addrs).await?;
        match namenode_client
            .connection(tonic::Request::new(connection_request))
            .await
        {
            Ok(connected) => Ok(connected.into_inner().connected),
            Err(tonic_status) => {
                Err(format!("Error while connecting to namenode {}", tonic_status).into())
            }
        }
    }

    pub async fn send_heart_beat(&self) -> Result<(), Box<dyn Error>> {
        let state = self.state.lock().await;
        let namenode_addrs = state.namenode_addrs.clone();
        let id = state.get_id();
        drop(state);
        // now we will send this address to the datanode
        let heart_beat_request = HeartBeatRequest { datanode_id: id };
        let mut namenode_client = self.get_grpc_connection(&namenode_addrs).await?;
        namenode_client
            .heart_beat(tonic::Request::new(heart_beat_request))
            .await?;
        Ok(())
    }
    pub async fn state_sync(&self) -> Result<(), Box<dyn Error>> {
        let state = self.state.lock().await;
        let namenode_addrs = state.namenode_addrs.clone();
        trace!("sending state sync with {:?}", state);
        let state_sync_request = StateSyncRequest {
            id: state.get_id(),
            available_chunks: state.available_chunks.clone(),
            availabe_storage: state.available_storage as u64,
        };

        drop(state);
        let mut namenode_client = self.get_grpc_connection(&namenode_addrs).await?;
        namenode_client
            .state_sync(tonic::Request::new(state_sync_request))
            .await?;
        Ok(())
    }
}
