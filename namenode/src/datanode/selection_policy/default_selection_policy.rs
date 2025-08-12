use super::selection_policy::DatanodeSelectionPolicy;
use crate::namenode_state::NamenodeState;
use proto::generated::client_namenode::DataNodeMeta;
use std::{error::Error, sync::Arc};
use tokio::sync::Mutex;
use tonic::async_trait;
use utilities::logger::{instrument, tracing};
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
    #[instrument(name = "policy_datanode_selection_to_store", skip(self))]
    async fn get_datanodes_to_store(
        &self,
        chunk_size: u64,
    ) -> Result<Vec<DataNodeMeta>, Box<dyn Error>> {
        let state = self.namenode_state.lock().await;
        let candidates: Vec<DataNodeMeta> = state
            .datanode_to_detail_map
            .values()
            .filter(|datanode_detail| datanode_detail.can_store(chunk_size))
            .take(3)
            .map(|datanode_detail| datanode_detail.into())
            .collect();
        if candidates.is_empty() {
            return Err("No datanode available to store chunk".into());
        }
        return Ok(candidates);
    }
    #[instrument(name = "policy_datanode_selection_to_serve", skip(self))]
    async fn get_datanodes_to_serve(&self, chunk_id: &str) -> Result<DataNodeMeta, Box<dyn Error>> {
        let namenode_state = self.namenode_state.lock().await;
        let candidate = namenode_state
            .chunk_id_to_detail_map
            .get(chunk_id)
            .expect(&format!(
                "Error while fetching chunk detals for chunk {chunk_id}"
            ))
            .get_locations()
            .iter()
            .map(|location| namenode_state.datanode_to_detail_map.get(location))
            .find_map(|datanode_details| {
                if datanode_details.is_some() && datanode_details.unwrap().is_active() {
                    return datanode_details;
                }
                None
            })
            .expect(&format!(
                "No chunk details available to provide chunk {chunk_id}"
            ));
        return Ok(candidate.into());
    }
}
