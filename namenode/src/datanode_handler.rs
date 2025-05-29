use std::{sync::Arc, time::Instant};

use tokio::sync::Mutex;

use crate::namenode_state::NamenodeState;

use proto::generated::datanode_namenode::{
    HeartBeatRequest, HeartBeatResponse, datanode_namenode_server::DatanodeNamenode,
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
        let mut state = self.state.lock().await;
        state
            .datanode_to_heart_beat_time
            .insert(heart_beat_request.datanode_id, Instant::now());
        let response = HeartBeatResponse {};
        Ok(tonic::Response::new(response))
    }
}
