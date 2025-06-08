use std::{
    collections::{HashMap, HashSet},
    time::Instant,
};

use proto::generated::client_namenode::DataNodeMeta;

use crate::data_structure::ChunkBounderies;

#[derive(Debug, Default, Clone)]
pub struct DatanodeState {
    pub storage_remaining: u64,
}

#[derive(Default, Debug, Clone)]
pub struct NamenodeState {
    pub file_to_chunk_map: HashMap<String, Vec<String>>,
    pub chunk_to_location_map: HashMap<String, Vec<String>>,
    pub chunk_to_boundry_map: HashMap<String, ChunkBounderies>,
    pub active_datanodes: HashSet<String>,
    pub datanode_to_state_map: HashMap<String, DatanodeState>,
    pub datanode_to_meta_map: HashMap<String, DataNodeMeta>,
    pub deleted_chunks: HashSet<String>,
    pub datanode_to_heart_beat_time: HashMap<String, Instant>,
}
impl NamenodeState {
    pub fn new() -> Self {
        Self {
            file_to_chunk_map: HashMap::default(),
            chunk_to_location_map: HashMap::default(),
            chunk_to_boundry_map: HashMap::default(),
            active_datanodes: HashSet::default(),
            datanode_to_state_map: HashMap::default(),
            datanode_to_meta_map: HashMap::default(),
            deleted_chunks: HashSet::default(),
            datanode_to_heart_beat_time: HashMap::default(),
        }
    }
}
