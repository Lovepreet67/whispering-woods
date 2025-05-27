use std::collections::{HashMap, HashSet};

use proto::generated::client_namenode::DataNodeMeta;

#[derive(Debug, Default)]
pub struct DatanodeState {
    pub storage_remaining: u64,
}

#[derive(Default, Debug)]
pub struct NamenodeState {
    pub file_to_chunk_map: HashMap<String, Vec<String>>,
    pub chunk_to_location_map: HashMap<String, Vec<String>>,
    pub active_datanodes: HashSet<String>,
    pub datanode_to_state_map: HashMap<String, DatanodeState>,
    pub datanode_to_meta_map: HashMap<String, DataNodeMeta>,
}
impl NamenodeState {
    pub fn new() -> Self {
        Self {
            file_to_chunk_map: HashMap::default(),
            chunk_to_location_map: HashMap::default(),
            active_datanodes: HashSet::default(),
            datanode_to_state_map: HashMap::default(),
            datanode_to_meta_map: HashMap::default(),
        }
    }
}
