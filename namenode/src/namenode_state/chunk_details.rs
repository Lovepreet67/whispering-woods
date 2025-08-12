use std::{collections::HashSet, time::Instant};

#[derive(Default, Debug, Clone)]
pub enum ChunkState {
    #[default]
    Initialized,
    Commited,
    Deleted(Instant),
}
#[derive(Default, Debug, Clone)]
pub struct ChunkDetails {
    pub id: String,
    pub locations: HashSet<String>,
    pub start_offset: u64,
    pub end_offset: u64,
    pub state: ChunkState,
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
    pub fn get_locations(&self)->Vec<String>{
        self.locations.clone().into_iter().collect()
    }
    pub fn add_location(&mut self,datanode_id:&str){
        self.state = ChunkState::Commited;
        self.locations.insert(datanode_id.to_owned());
    }
    pub fn mark_deleted(&mut self){
        self.state = ChunkState::Deleted(Instant::now())
    }
    pub fn is_deleted(&mut self)->bool{
       match self.state {
           ChunkState::Deleted(_)=>{
               self.state = ChunkState::Deleted(Instant::now());
               true
           },
           _=>false
       } 
    }
}

