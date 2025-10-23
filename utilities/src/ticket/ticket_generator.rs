use crate::ticket::types::ServerTicket;
use crate::{result::Result, ticket::types::ClientTicket};
use aes_gcm::aead::{Aead, OsRng};
use aes_gcm::{AeadCore, Aes256Gcm, Key, KeyInit};
use base64::Engine;
use base64::prelude::BASE64_STANDARD;
use std::collections::HashMap;
use tracing::{debug, info};

pub trait TicketGenerator {
    fn upsert_node_key(&mut self, node_id: &str) -> Result<String>;
    fn upsert_node_key_with_key(&mut self, node_id: &str, encoded_key: &str) -> Result<()>;
    fn encrypt_server_ticket(&mut self, st: &ServerTicket) -> Result<Vec<u8>>;
    fn encrypt_client_ticket(&mut self, st: &ClientTicket) -> Result<Vec<u8>>;
}

pub struct DefaultTicketGenerator {
    node_to_key: HashMap<String, Key<Aes256Gcm>>,
}

impl DefaultTicketGenerator {
    pub fn new() -> Self {
        Self {
            node_to_key: HashMap::default(),
        }
    }
    fn encrypt_with_node_key(&mut self, encoded_ticket: Vec<u8>, node_id: &str) -> Result<Vec<u8>> {
        debug!("encrypting the ticket : {:?}", self.node_to_key);
        if let Some(node_key) = self.node_to_key.get(node_id) {
            let cipher = Aes256Gcm::new(node_key);
            let nonce = Aes256Gcm::generate_nonce(&mut OsRng); // 96-bits; unique per message
            match cipher.encrypt(&nonce, encoded_ticket.as_ref()) {
                Ok(mut v) => {
                    let mut merged = nonce.to_vec();
                    merged.append(&mut v);
                    Ok(merged)
                }
                Err(_e) => Err("Error while encrypting the data".into()),
            }
        } else {
            Err("Invalid node id in server ticket".into())
        }
    }
}
impl TicketGenerator for DefaultTicketGenerator {
    fn encrypt_server_ticket(&mut self, st: &ServerTicket) -> Result<Vec<u8>> {
        let encoded_ticket = serde_json::to_vec(&st)?;
        self.encrypt_with_node_key(encoded_ticket, st.target_node_id.as_str())
    }
    fn encrypt_client_ticket(&mut self, ct: &ClientTicket) -> Result<Vec<u8>> {
        let encoded_ticket = serde_json::to_vec(&ct)?;
        self.encrypt_with_node_key(encoded_ticket, ct.node_id.as_str())
    }
    fn upsert_node_key(&mut self, node_id: &str) -> Result<String> {
        let key = Aes256Gcm::generate_key(&mut OsRng);
        self.node_to_key.insert(node_id.to_string(), key);
        Ok(BASE64_STANDARD.encode(key))
    }
    fn upsert_node_key_with_key(&mut self, node_id: &str, encoded_key: &str) -> Result<()> {
        let decoded = BASE64_STANDARD.decode(encoded_key)?;
        if decoded.len() != 32 {
            return Err("Invalid key length: expected 32 bytes for AES-256".into());
        }
        let key = Key::<Aes256Gcm>::from_slice(&decoded);
        self.node_to_key.insert(node_id.to_string(), *key);
        Ok(())
    }
}
