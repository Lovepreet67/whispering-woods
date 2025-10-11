use std::collections::{HashMap, HashSet};

use tokio::net::TcpStream;
#[derive(Debug)]
pub struct DatanodeState {
    pub chunk_to_pipline: HashMap<String, TcpStream>,
    pub available_storage: usize,
    pub available_chunks: Vec<String>,
    pub to_be_deleted_chunks: HashSet<String>,
    pub chunk_to_next_replica: HashMap<String, String>, // this will store the address of next
    pub chunk_to_namenode_store_ticket: HashMap<String, String>, // this will store the ticket that
                                                        // will be used to talk to peers
                                                        // in case of store file
}
impl DatanodeState {
    pub fn new() -> Self {
        Self {
            chunk_to_pipline: HashMap::default(),
            available_storage: 0,
            available_chunks: vec![],
            to_be_deleted_chunks: HashSet::default(),
            chunk_to_next_replica: HashMap::default(),
            chunk_to_namenode_store_ticket: HashMap::default(),
        }
    }
}
