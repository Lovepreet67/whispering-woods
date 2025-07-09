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
                // do the state mantaining task
                let heartbeat_details = state.datanode_to_heart_beat_time.clone();
                // now based on these heartbeat details we will remove the nodes from active
                for (datanode_id, timestamp) in heartbeat_details.into_iter() {
                    // check if timestamp is older than 5 secs and remove datanode from active nodes
                    if timestamp.elapsed() > Duration::from_secs(5) {
                        state.active_datanodes.remove(&datanode_id);
                    } else {
                        state.active_datanodes.insert(datanode_id);
                    }
                }
            }
        });
    }
}
