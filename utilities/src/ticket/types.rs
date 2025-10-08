use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize)]
pub enum Operation {
    FetchChunk { chunk_id: String },
    StoreChunk { chunk_id: String }, // this will be used for both commit and store
    CreatePipeline { chunk_id: String },
}

#[derive(Serialize, Deserialize)]
pub struct ServerTicket {
    pub target_node_id: String,
    pub operation: Operation,
    pub ttl_secs: u64,
    pub minted_at_secs: u64,
}

#[derive(Serialize, Deserialize)]
pub struct ClientTicket {
    pub encrypted_server_ticket: Vec<u8>,
    pub node_id: String,
    pub ttl_secs: u64,
    pub minted_at_secs: u64,
}
