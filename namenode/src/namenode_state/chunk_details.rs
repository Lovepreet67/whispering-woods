use serde::Serialize;
use std::{collections::HashSet, time::Instant};

#[derive(Default, Debug, Clone, Serialize)]
pub enum ChunkState {
    #[default]
    Initialized,
    Commited,
    Deleted(#[serde(skip_serializing)] Instant),
}
#[derive(Default, Debug, Clone, Serialize)]
pub struct ChunkDetails {
    pub id: String,
    pub locations: HashSet<String>,
    pub start_offset: u64,
    pub end_offset: u64,
    pub state: ChunkState,
}

#[derive(Default, Debug, Clone)]
pub enum ChunkReplicationStatus {
    Undereplicated(u8),
    Overreplicated(u8),
    #[default]
    Balanced,
    Lost,
}

impl ChunkDetails {
    pub fn new(id: String, start_offset: u64, end_offset: u64) -> Self {
        Self {
            id,
            locations: HashSet::default(),
            start_offset,
            end_offset,
            state: ChunkState::Initialized,
        }
    }
    pub fn get_locations(&self) -> Vec<String> {
        self.locations.clone().into_iter().collect()
    }
    pub fn remove_invalid_locations(&mut self, invalid_locations: &HashSet<String>) {
        self.locations
            .retain(|datanode_id| !invalid_locations.contains(datanode_id));
    }
    pub fn remove_location(&mut self, location: &str) {
        self.locations.remove(location);
    }
    pub fn add_location(&mut self, datanode_id: &str) {
        self.state = ChunkState::Commited;
        self.locations.insert(datanode_id.to_owned());
    }
    pub fn get_replication_status(&self) -> ChunkReplicationStatus {
        if self.locations.is_empty() {
            return ChunkReplicationStatus::Lost;
        }
        if self.locations.len() > 3 {
            return ChunkReplicationStatus::Overreplicated((self.locations.len() - 3) as u8);
        } else if self.locations.len() < 3 {
            return ChunkReplicationStatus::Undereplicated((3 - self.locations.len()) as u8);
        }
        ChunkReplicationStatus::Balanced
    }
    pub fn mark_deleted(&mut self) {
        self.state = ChunkState::Deleted(Instant::now())
    }
    pub fn is_deleted(&mut self) -> bool {
        match self.state {
            ChunkState::Deleted(_) => {
                self.state = ChunkState::Deleted(Instant::now());
                true
            }
            _ => false,
        }
    }
}
