use std::collections::HashMap;

use tokio::net::TcpStream;

pub struct DatanodeState {
    pub chunk_to_pipline: HashMap<String, TcpStream>,
    pub tcp_server_addrs: String,
}
impl DatanodeState {
    pub fn new(tcp_server_addrs: String) -> Self {
        Self {
            chunk_to_pipline: HashMap::default(),
            tcp_server_addrs,
        }
    }
}
