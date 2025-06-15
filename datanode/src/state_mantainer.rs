use std::{sync::Arc, time::Duration};

use storage::{file_storage::FileStorage, storage::Storage};
use tokio::{sync::Mutex, time::interval};
use utilities::logger::{error, instrument, tracing};

use crate::datanode_state::DatanodeState;

pub struct StateMantainer {
    store: FileStorage,
    state: Arc<Mutex<DatanodeState>>,
}
impl StateMantainer {
    pub fn new(store: FileStorage, state: Arc<Mutex<DatanodeState>>) -> Self {
        Self { store, state }
    }
    #[instrument(skip(self, duration))]
    pub fn start_sync_loop(self, duration: Duration) {
        tokio::spawn(async move {
            let mut ticker = interval(duration);
            loop {
                ticker.tick().await;
                let available_chunks = match self.store.available_chunks().await {
                    Ok(v) => v,
                    Err(e) => {
                        error!(
                            "Skiping datanode state sync : Error while fetching the available chunks list from store {e}."
                        );
                        continue;
                    }
                };
                let available_storage = /*match*/ self.store.available_storage().await;
                /*.await{
                    Ok(v)=>v,
                   Err(e)=>{
                       error!("Skipping datanode state sync: Error while fetching the available storage {e}");
                           continue;
                   }
                };*/
                let mut state = self.state.lock().await;
                state.available_chunks = available_chunks;
                state.available_storage = available_storage;
            }
        });
    }
}
