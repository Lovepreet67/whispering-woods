use std::{sync::Arc, time::Duration};

use futures::future::join_all;
use storage::{file_storage::FileStorage, storage::Storage};
use tokio::{sync::Mutex, time::interval};
use utilities::logger::{error, span, trace, Level};

use crate::datanode_state::DatanodeState;

pub struct StateMantainer {
    store: FileStorage,
    state: Arc<Mutex<DatanodeState>>,
}
impl StateMantainer {
    pub fn new(store: FileStorage, state: Arc<Mutex<DatanodeState>>) -> Self {
        Self { store, state }
    }
    pub fn start_sync_loop(self, duration: Duration) {
        tokio::spawn(async move {
            let mut ticker = interval(duration);
            loop {
                ticker.tick().await;
                let span = span!(Level::INFO, "datanode_state_sync");
                let _entered = span.enter();
                let available_chunks = match self.store.available_chunks().await {
                    Ok(v) => v,
                    Err(e) => {
                        error!(
                            "Skiping datanode state sync : Error while fetching the available chunks list from store {e}."
                        );
                        continue;
                    }
                };
                let available_storage = match self.store.available_storage() {
                    Ok(v) => v,
                    Err(e) => {
                        error!(
                            "Skipping datanode state sync: Error while fetching the available storage {e}"
                        );
                        continue;
                    }
                };
                let mut state = self.state.lock().await;
                let to_be_deleted = std::mem::take(&mut state.to_be_deleted_chunks);
                state.available_chunks = available_chunks.into_iter()
                    .filter(|chunk| !to_be_deleted.contains(chunk))
                    .collect();
                state.available_storage = available_storage;
                drop(state);
                trace!("Deleting chunks {to_be_deleted:?}");
                let delete_promise = to_be_deleted.into_iter().map( |chunk| 
                    self.store.delete(chunk)
                );
                join_all(delete_promise).await;
            }
        });
    }
}
