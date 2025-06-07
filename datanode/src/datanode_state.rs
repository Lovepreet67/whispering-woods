use std::collections::HashMap;

use tokio::net::TcpStream;
#[derive(Debug)]
pub struct DatanodeState {
    id: String,
    pub chunk_to_pipline: HashMap<String, TcpStream>,
    pub tcp_server_addrs: String,
    pub grpc_server_addrs: String,
    pub namenode_addrs: String,
    pub available_storage: usize,
    pub available_chunks: Vec<String>,
}
impl DatanodeState {
    pub fn new(
        id: String,
        grpc_server_addrs: String,
        tcp_server_addrs: String,
        namenode_addrs: String,
    ) -> Self {
        Self {
            id,
            chunk_to_pipline: HashMap::default(),
            grpc_server_addrs,
            tcp_server_addrs,
            namenode_addrs,
            available_storage: 0,
            available_chunks: vec![],
        }
    }
    pub fn get_id(&self) -> String {
        self.id.clone()
    }
}
