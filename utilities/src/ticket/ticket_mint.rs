use crate::result::Result;
use crate::ticket::{
    ticket_generator::TicketGenerator,
    types::{ClientTicket, Operation, ServerTicket},
};
use base64::Engine;
use base64::prelude::BASE64_STANDARD;
use std::time::{SystemTime, UNIX_EPOCH};
use tracing::trace;

pub struct TicketMint {
    ticket_generator: Box<dyn TicketGenerator + Send + Sync>,
}

impl TicketMint {
    pub fn new(ticket_generator: Box<dyn TicketGenerator + Send + Sync>) -> Self {
        Self { ticket_generator }
    }
    pub fn mint_ticket(
        &mut self,
        source_id: &str,
        target_id: &str,
        op: Operation,
    ) -> Result<String> {
        trace!(
            "minting ticket for source :{}, target:{}, op:{:?}",
            source_id, target_id, op
        );
        let minted_at_secs = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs();
        // default ttl to 5 minutes
        let ttl_secs = 5 * 60;
        let st = ServerTicket {
            target_node_id: target_id.to_string(),
            minted_at_secs,
            ttl_secs,
            operation: op,
        };
        let encrypted_st = self.ticket_generator.encrypt_server_ticket(&st)?;
        let ct = ClientTicket {
            node_id: source_id.to_string(),
            encrypted_server_ticket: BASE64_STANDARD.encode(encrypted_st),
            ttl_secs,
            minted_at_secs,
        };
        let ticket = self.ticket_generator.encrypt_client_ticket(&ct)?;
        Ok(BASE64_STANDARD.encode(ticket))
    }
    pub fn add_node_key(&mut self, node_id: &str) -> Result<String> {
        self.ticket_generator.upsert_node_key(node_id)
    }
    pub fn add_node_key_with_key(&mut self, node_id: &str, encoded_key: &str) -> Result<()> {
        self.ticket_generator
            .upsert_node_key_with_key(node_id, encoded_key)
    }
}
