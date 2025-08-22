use std::time::{Duration, Instant};

use proto::generated::client_namenode::DataNodeMeta;

#[derive(Debug, Clone)]
pub enum DatanodeState {
    Active,
    Inactive(Instant),
}

#[derive(Debug, Clone)]
pub struct DatanodeDetail {
    pub id: String,
    pub name: String,
    pub addrs: String,
    pub storage_remaining: u64,
    pub hearbeat_instant: Instant,
    pub state: DatanodeState,
}

impl DatanodeDetail {
    pub fn new(id: String, name: String, addrs: String) -> Self {
        Self {
            id,
            name,
            addrs,
            storage_remaining: 0,
            hearbeat_instant: Instant::now(),
            state: DatanodeState::Active,
        }
    }
    pub fn mark_heartbeat(&mut self) {
        self.hearbeat_instant = Instant::now();
    }
    pub fn sync_state(&mut self, storage_remaining: u64) {
        self.storage_remaining = storage_remaining;
        self.hearbeat_instant = Instant::now();
    }
    pub fn is_active(&self) -> bool {
        if self.hearbeat_instant.elapsed() > Duration::from_secs(6) {
            return false;
        }
        true
    }
    pub fn can_store(&self, chunk_size: u64) -> bool {
        self.is_active() && self.storage_remaining > chunk_size
    }
}

impl Into<DataNodeMeta> for &DatanodeDetail {
    fn into(self) -> DataNodeMeta {
        DataNodeMeta {
            id: self.id.clone(),
            name: self.name.clone(),
            addrs: self.addrs.clone(),
        }
    }
}
