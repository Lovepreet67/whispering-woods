use std::collections::HashMap;

use tokio::net::TcpStream;
#[derive(Debug)]
pub struct DatanodeState {
    pub chunk_to_pipline: HashMap<String, TcpStream>,
    pub available_storage: usize,
    pub available_chunks: Vec<String>,
    pub chunk_to_next_replica: HashMap<String, String>, // this will store the address of next
}
impl DatanodeState {
    pub fn new() -> Self {
        Self {
            chunk_to_pipline: HashMap::default(),
            available_storage: 0,
            available_chunks: vec![],
            chunk_to_next_replica: HashMap::default(),
        }
    }
}
