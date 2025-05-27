use proto::generated::client_namenode::DataNodeMeta;
use std::{error::Error, sync::Arc};
use tokio::sync::Mutex;
use tonic::async_trait;

use crate::namenode_state::NamenodeState;

#[async_trait]
pub trait DatanodeSelectionPolicy {
    async fn get_datanodes(&self, chunk_size: u64) -> Result<Vec<DataNodeMeta>, Box<dyn Error>>;
}

pub struct DefaultDatanodeSelectionPolicy {
    namenode_state: Arc<Mutex<NamenodeState>>,
}
impl DefaultDatanodeSelectionPolicy {
    pub fn new(namenode_state: Arc<Mutex<NamenodeState>>) -> Self {
        Self { namenode_state }
    }
}
// default policy will return first three nodes which can store the data
#[async_trait]
impl DatanodeSelectionPolicy for DefaultDatanodeSelectionPolicy {
    async fn get_datanodes(&self, chunk_size: u64) -> Result<Vec<DataNodeMeta>, Box<dyn Error>> {
        let namenode_state = self.namenode_state.lock().await;
        let datanodes: Vec<_> = namenode_state
            .active_datanodes
            .iter()
            .filter_map(|datanode| {
                let state = namenode_state.datanode_to_state_map.get(datanode)?;
                let meta = namenode_state.datanode_to_meta_map.get(datanode)?;
                if state.storage_remaining > chunk_size {
                    return Some(meta.clone());
                }
                return None;
            })
            .take(3)
            .collect();
        Ok(datanodes)
    }
}
