use crate::namenode_state::{NamenodeState, chunk_details, datanode_details::DatanodeDetail};
use serde::Serialize;
use std::{collections::HashMap, hash::Hash, time::SystemTime};

#[derive(Clone, Debug, Hash, PartialEq, Serialize)]
pub struct DatanodeStateSnapshot {
    pub is_active: bool,
    pub storage_remaining: u64,
    pub addrs: String,
}

impl From<DatanodeDetail> for DatanodeStateSnapshot {
    fn from(value: DatanodeDetail) -> Self {
        Self {
            is_active: value.is_active(),
            storage_remaining: value.storage_remaining,
            addrs: value.addrs.clone(),
        }
    }
}

#[derive(Clone, Debug, Serialize)]
pub struct NamenodeStateSnapshot {
    pub timestamp: SystemTime,
    pub datanode_to_detail_map: HashMap<String, DatanodeStateSnapshot>,
    pub file_to_chunk_map: HashMap<String, Vec<String>>,
    pub chunk_id_to_detail_map: HashMap<String, chunk_details::ChunkDetails>,
}

impl From<NamenodeState> for NamenodeStateSnapshot {
    fn from(value: NamenodeState) -> Self {
        Self {
            timestamp: SystemTime::now(),
            datanode_to_detail_map: value
                .datanode_to_detail_map
                .into_iter()
                .map(|(k, v)| (k, v.into()))
                .collect(),
            file_to_chunk_map: value.file_to_chunk_map,
            chunk_id_to_detail_map: value.chunk_id_to_detail_map,
        }
    }
}

impl PartialEq for NamenodeStateSnapshot {
    fn eq(&self, other: &Self) -> bool {
        self.chunk_id_to_detail_map == other.chunk_id_to_detail_map
            && self.datanode_to_detail_map == other.datanode_to_detail_map
            && self.file_to_chunk_map == other.file_to_chunk_map
    }
}

use std::sync::Arc;
use tokio::sync::Mutex;

#[derive(Clone, Debug)]
pub struct SnapshotStore {
    snapshot: Arc<Mutex<NamenodeStateSnapshot>>,
}

impl SnapshotStore {
    pub fn new() -> Self {
        Self {
            snapshot: Arc::new(Mutex::new(NamenodeStateSnapshot::from(
                NamenodeState::default(),
            ))),
        }
    }

    pub async fn update_snapshot(&self, new_snapshot: NamenodeStateSnapshot) {
        let mut curr_snapshot = self.snapshot.lock().await;
        *curr_snapshot = new_snapshot;
    }

    pub async fn get_snapshot(&self) -> NamenodeStateSnapshot {
        let curr_snapshot = self.snapshot.lock().await;
        curr_snapshot.clone()
    }
}
