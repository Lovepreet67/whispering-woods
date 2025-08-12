use std::{sync::Arc, time::Duration};

use tokio::{sync::Mutex, time::interval};
use utilities::logger::{Level, span};

use crate::namenode_state::NamenodeState;

/// To mantain the state of the namenode based on the heartbeat
pub struct StateMantainer {
    namenode_state: Arc<Mutex<NamenodeState>>,
}

impl StateMantainer {
    pub fn new(namenode_state: Arc<Mutex<NamenodeState>>) -> Self {
        Self { namenode_state }
    }
    pub fn start(self) {
        tokio::spawn(async move {
            let mut ticker = interval(Duration::from_secs(5));
            loop {
                ticker.tick().await;
                let span = span!(Level::INFO, "namenode_state_sync");
                let _entered = span.enter();
                let mut state = self.namenode_state.lock().await;
                // remaving all the chunks which are deleted and last seen 23 or more seconds ago
                // i.e two statesync intervals
                state.chunk_id_to_detail_map.retain(
                    |_,value|{
                    if let crate::namenode_state::chunk_details::ChunkState::Deleted(last_seen)  = value.state{
                        return last_seen.elapsed() < Duration::from_secs(23);
                    }
                    return true;
                });
            }
        });
    }
}
