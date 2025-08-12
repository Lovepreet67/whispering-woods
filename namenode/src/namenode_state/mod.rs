pub mod datanode_details;
pub mod chunk_details;
pub mod state_mantainer;

use std::{
    collections::{HashMap},
};



#[derive(Default, Debug, Clone)]
pub struct NamenodeState{
    pub file_to_chunk_map: HashMap<String, Vec<String>>,
    pub chunk_id_to_detail_map:HashMap<String,chunk_details::ChunkDetails>,
    pub datanode_to_detail_map: HashMap<String, datanode_details::DatanodeDetail>,
}
impl NamenodeState {
    pub fn new() -> Self {
        Self {
            file_to_chunk_map: HashMap::default(),
            chunk_id_to_detail_map:HashMap::default(),
            datanode_to_detail_map: HashMap::default()
        }
    }
}
