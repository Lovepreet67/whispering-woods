use proto::generated::client_namenode::DataNodeMeta;
use std::{error::Error, sync::Arc};
use tokio::sync::Mutex;
use tonic::async_trait;

use crate::namenode_state::NamenodeState;

#[async_trait]
pub trait DatanodeSelectionPolicy {
    async fn get_datanodes_to_store(
        &self,
        chunk_size: u64,
    ) -> Result<Vec<DataNodeMeta>, Box<dyn Error>>;
    async fn get_datanodes_to_serve(&self, chunk_id: &str) -> Result<DataNodeMeta, Box<dyn Error>>;
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
    async fn get_datanodes_to_store(
        &self,
        chunk_size: u64,
    ) -> Result<Vec<DataNodeMeta>, Box<dyn Error>> {
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
                None
            })
            .take(3)
            .collect();
        Ok(datanodes)
    }
    async fn get_datanodes_to_serve(&self, chunk_id: &str) -> Result<DataNodeMeta, Box<dyn Error>> {
        let namenode_state = self.namenode_state.lock().await;
        if let Some(location) = namenode_state.chunk_to_location_map.get(chunk_id) {
            // return the first datanode
            if !location.is_empty()
                && namenode_state
                    .datanode_to_meta_map
                    .contains_key(&location[0])
            {
                return Ok(namenode_state
                    .datanode_to_meta_map
                    .get(&location[0])
                    .expect("Error while fetching datanode map")
                    .clone());
            }
        }
        return Err(format!("can't locate chunk : {}", chunk_id).into());
    }
}
