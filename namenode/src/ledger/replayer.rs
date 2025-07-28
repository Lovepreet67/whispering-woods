use std::error::Error;


use crate::namenode_state::NamenodeState;

pub trait Replayer {
    fn replay(&self) -> Result<NamenodeState, Box<dyn Error>>;
}
