use crate::namenode_state::NamenodeState;
use utilities::{result::Result, ticket::ticket_mint::TicketMint};

pub trait Replayer {
    fn replay(&self) -> Result<(NamenodeState, TicketMint)>;
}
