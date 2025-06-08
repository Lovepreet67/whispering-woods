use std::{os::macos::raw::stat, sync::Arc, time::Instant};

use log::{info, trace};
use tokio::sync::Mutex;

use crate::namenode_state;
use crate::namenode_state::NamenodeState;

use proto::generated::datanode_namenode::{
    ConnectionRequest, ConnectionResponse, HeartBeatRequest, HeartBeatResponse, StateSyncRequest,
    StateSyncResponse, datanode_namenode_server::DatanodeNamenode,
};

pub struct DatanodeHandler {
    state: Arc<Mutex<NamenodeState>>,
}
impl DatanodeHandler {
    pub fn new(namenode_state: Arc<Mutex<NamenodeState>>) -> Self {
        Self {
            state: namenode_state,
        }
    }
}

#[tonic::async_trait]
impl DatanodeNamenode for DatanodeHandler {
    async fn heart_beat(
        &self,
        request: tonic::Request<HeartBeatRequest>,
    ) -> Result<tonic::Response<HeartBeatResponse>, tonic::Status> {
        let heart_beat_request = request.into_inner();
        //trace!("got heartbeat request {:?}", heart_beat_request);
        let mut state = self.state.lock().await;
        let response = if state
            .datanode_to_meta_map
            .contains_key(&heart_beat_request.datanode_id)
        {
            state
                .datanode_to_heart_beat_time
                .insert(heart_beat_request.datanode_id, Instant::now());
            HeartBeatResponse {
                connection_alive: true,
            }
        } else {
            HeartBeatResponse {
                connection_alive: false,
            }
        };
        Ok(tonic::Response::new(response))
    }
    async fn connection(
        &self,
        request: tonic::Request<ConnectionRequest>,
    ) -> Result<tonic::Response<ConnectionResponse>, tonic::Status> {
        let connection_request = request.into_inner();
        /*trace!(
            "got connection request from data node {}",
            connection_request.id
        );*/
        let mut state = self.state.lock().await;
        // if the connection already exist for this we will not accept connection
        let response = if state
            .datanode_to_meta_map
            .contains_key(&connection_request.id)
        {
            ConnectionResponse {
                connected: false,
                msg: "Connection already exist for the specified id".to_owned(),
            }
        } else {
            state.datanode_to_meta_map.insert(
                connection_request.id.clone(),
                proto::generated::client_namenode::DataNodeMeta {
                    id: connection_request.id,
                    name: connection_request.name,
                    addrs: connection_request.addrs,
                },
            );
            ConnectionResponse {
                connected: true,
                msg: "Connected successfully".to_owned(),
            }
        };
        drop(state);
        Ok(tonic::Response::new(response))
    }
    async fn state_sync(
        &self,
        request: tonic::Request<StateSyncRequest>,
    ) -> Result<tonic::Response<StateSyncResponse>, tonic::Status> {
        let state_sync_request = request.into_inner();
        let mut state = self.state.lock().await;
        state.datanode_to_state_map.insert(
            state_sync_request.id.clone(),
            namenode_state::DatanodeState {
                storage_remaining: state_sync_request.availabe_storage,
            },
        );
        for chunk_id in state_sync_request.available_chunks {
            let mut locations = state
                .chunk_to_location_map
                .remove(&chunk_id)
                .unwrap_or(vec![]);

            //TODO: store location in set so that we can directly insert it instead for first checking

            // check if location already exist
            let mut already_exist = false;
            for location in &locations {
                if *location == state_sync_request.id {
                    already_exist = true;
                    break;
                }
            }
            if !already_exist {
                locations.push(state_sync_request.id.clone());
            }
            state.chunk_to_location_map.insert(chunk_id, locations);
        }
        let response = StateSyncResponse {};
        Ok(tonic::Response::new(response))
    }
}
