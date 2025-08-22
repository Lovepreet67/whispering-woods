use std::collections::HashSet;
use std::{sync::Arc, time::Duration};

use futures::future::join_all;
use tokio::{sync::Mutex, time::interval};
use utilities::logger::{Level, error, span};

use crate::datanode::selection_policy::default_selection_policy::DefaultDatanodeSelectionPolicy;
use crate::datanode::selection_policy::selection_policy::DatanodeSelectionPolicy;
use crate::datanode::service::DatanodeService;
use crate::namenode_state::NamenodeState;
use crate::namenode_state::chunk_details::ChunkReplicationStatus;

/// To mantain the state of the namenode based on the heartbeat
pub struct StateMantainer {
    datanode_service: DatanodeService,
    datanode_selection_policy: Arc<Mutex<Box<dyn DatanodeSelectionPolicy + Send + Sync>>>,
    namenode_state: Arc<Mutex<NamenodeState>>,
}

impl StateMantainer {
    pub fn new(namenode_state: Arc<Mutex<NamenodeState>>) -> Self {
        Self {
            datanode_service: DatanodeService::default(),
            datanode_selection_policy: Arc::new(Mutex::new(Box::new(
                DefaultDatanodeSelectionPolicy::new(namenode_state.clone()),
            ))),
            namenode_state,
        }
    }
    // this function is fire and forget
    fn handle_undereplicated_chunk(&self, chunk_id: &str) {
        let datanode_service = self.datanode_service;
        let datanode_selection_policy = self.datanode_selection_policy.clone();
        let chunk_id = chunk_id.to_owned();

        tokio::spawn(async move {
            let datanode_selection_policy = datanode_selection_policy.lock().await;
            let datanode_pair = match datanode_selection_policy
                .get_datanodes_to_repair(&chunk_id)
                .await
            {
                Ok(pair) => pair,
                Err(e) => {
                    error!("Error happend while selecting datanode pair for repair {e}");
                    return;
                }
            };
            drop(datanode_selection_policy);
            if let Err(e) = datanode_service
                .replicate_chunk(datanode_pair.0, datanode_pair.1, &chunk_id)
                .await
            {
                error!("Error happend while replicting, {e}");
            }
        });
    }
    // this function is fire and forget
    fn handler_overreplicated_chunk(&self, chunk_id: &str, count: u8) {
        let chunk_id = chunk_id.to_owned();
        let datanode_selection_policy = self.datanode_selection_policy.clone();
        let datanode_service = self.datanode_service;
        tokio::spawn(async move {
            let datanode_selection_policy = datanode_selection_policy.lock().await;
            let datanodes_to_offload = match datanode_selection_policy
                .get_datanode_to_offload(&chunk_id, count as usize)
                .await
            {
                Ok(list) => list,
                Err(e) => {
                    error!("Error happend while selection datanode list to offload {e}");
                    return;
                }
            };
            drop(datanode_selection_policy);
            // we fire send delete request to all of these
            let delete_chunk_futures = datanodes_to_offload
                .iter()
                .map(|candidate| datanode_service.delete_chunk(&candidate.addrs, &chunk_id));
            join_all(delete_chunk_futures).await;
        });
    }
    pub fn start(self) {
        tokio::spawn(async move {
            let mut ticker = interval(Duration::from_secs(5));
            // here we do all the garbage collection
            loop {
                ticker.tick().await;
                let span = span!(Level::INFO, "namenode_state_sync");
                let _entered = span.enter();
                let mut state = self.namenode_state.lock().await;

                // remaving all the chunks which are deleted and last seen 23 or more seconds ago
                // i.e two statesync intervals
                state.chunk_id_to_detail_map.retain(|_, value| {
                    if let crate::namenode_state::chunk_details::ChunkState::Deleted(last_seen) =
                        value.state
                    {
                        return last_seen.elapsed() < Duration::from_secs(23);
                    }
                    true
                });
                // checking the undereplicated and overreplicated ChunkState
                let inactive_datanodes: HashSet<String> = state
                    .datanode_to_detail_map
                    .iter()
                    .filter_map(|(datanode_id, datanode_details)| {
                        if !datanode_details.is_active() {
                            Some(datanode_id.to_owned())
                        } else {
                            None
                        }
                    })
                    .collect();
                state
                    .chunk_id_to_detail_map
                    .iter_mut()
                    .for_each(|(chunk_id, chunk_details)| {
                        chunk_details.remove_invalid_locations(&inactive_datanodes);
                        match chunk_details.get_replication_status() {
                            ChunkReplicationStatus::Undereplicated(_) => {
                                // we skipping the count here we will increase replication by 1
                                self.handle_undereplicated_chunk(chunk_id);
                            }
                            ChunkReplicationStatus::Overreplicated(count) => {
                                self.handler_overreplicated_chunk(chunk_id, count);
                            }
                            ChunkReplicationStatus::Lost => {
                                // invalid situation we just lost all our
                            }
                            ChunkReplicationStatus::Balanced => {}
                        }
                    });
            }
        });
    }
}
