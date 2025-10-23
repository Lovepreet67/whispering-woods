use crate::result::Result;
use crate::ticket::types::{ClientTicket, ServerTicket};
use aes_gcm::aead::Aead;
use aes_gcm::{Aes256Gcm, Key, KeyInit, Nonce};
use base64::Engine;
use base64::prelude::BASE64_STANDARD;

pub trait TicketDecrypter: Send + Sync {
    fn decrypt_server_ticket(&self, encoded_ticket: &str) -> Result<ServerTicket>;
    fn decrypt_client_ticket(&self, encoded_ticket: &str) -> Result<ClientTicket>;
}

pub struct DefaultTicketDecrypter {
    key: Key<Aes256Gcm>,
}

impl DefaultTicketDecrypter {
    pub fn new(encoded_key: &str) -> Result<Self> {
        let decoded_key = BASE64_STANDARD.decode(encoded_key)?;
        if decoded_key.len() != 32 {
            return Err("Invalid key size expcted key of len 32".into());
        }
        let key = Key::<Aes256Gcm>::from_slice(&decoded_key);
        Ok(Self { key: *key })
    }
    fn decrypt(&self, encoded_ticket: &str) -> Result<Vec<u8>> {
        let decoded_ticket = BASE64_STANDARD.decode(encoded_ticket)?;
        let (nonce_str, cipher_text) = decoded_ticket.split_at(12);
        let cipher = Aes256Gcm::new(&self.key);
        cipher
            .decrypt(Nonce::from_slice(nonce_str), cipher_text)
            .map_err(|_e| "Error while decrypting the ticket".into())
    }
}

impl TicketDecrypter for DefaultTicketDecrypter {
    fn decrypt_server_ticket(&self, encoded_ticket: &str) -> Result<ServerTicket> {
        let decrypt_u8 = self.decrypt(encoded_ticket)?;
        let st: ServerTicket = serde_json::from_slice(&decrypt_u8)?;
        Ok(st)
    }
    fn decrypt_client_ticket(&self, encoded_ticket: &str) -> Result<ClientTicket> {
        let decrypt_u8 = self.decrypt(encoded_ticket)?;
        let ct: ClientTicket = serde_json::from_slice(&decrypt_u8)?;
        Ok(ct)
    }
}
