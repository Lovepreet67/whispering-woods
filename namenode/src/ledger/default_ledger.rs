use core::error;
use std::{
    error::Error,
    io::{BufRead, BufReader},
    time::UNIX_EPOCH,
};

use tokio::{
    io::AsyncWriteExt,
    sync::mpsc::{self, Sender},
};
use tonic::async_trait;
use utilities::logger::{debug, error};

use crate::{data_structure::ChunkBounderies, namenode_state::NamenodeState};

use super::{recorder::Recorder, replayer::Replayer};
pub trait Ledger: Replayer + Recorder {}
impl<T: Recorder + Replayer> Ledger for T {}

pub struct DefaultLedger {
    log_store: String,
    producer: Sender<String>, // we should add file in ARC but currently logs will be generated only when
}
impl DefaultLedger {
    pub async fn new(log_store: &str) -> Result<Self, Box<dyn Error>> {
        let (tx, mut rx) = mpsc::channel::<String>(16);
        let mut appendable = tokio::fs::File::options()
            .append(true)
            .create(true)
            .open(log_store)
            .await?;
        tokio::spawn(async move {
            while let Some(log) = rx.recv().await {
                match appendable.write_all(log.as_bytes()).await {
                    Ok(_) => {}
                    Err(e) => {
                        error!(error = %e,"Error while appending log to file");
                    }
                }
            }
        });
        Ok(Self {
            log_store: log_store.to_owned(),
            producer: tx,
        })
    }
    async fn insert_log(&self, log: String) {
        let timestamp = std::time::SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap();
        let stamped_log = format!("{timestamp:?} {log:}\n");
        match self.producer.send(stamped_log).await {
            Ok(_) => {}
            Err(e) => {
                error!(error = %e,%log,"Error while sending log to producer");
            }
        }
    }
}

#[async_trait]
impl Recorder for DefaultLedger {
    //impl Ledger for DefaultLedger{
    async fn store_file(&self, file_name: &str, no_of_chunks: u64) {
        let log = format!("store_file {file_name:},{no_of_chunks}");
        self.insert_log(log).await;
    }
    async fn store_chunk(
        &self,
        file_name: &str,
        order: u64,
        chunk_id: &str,
        start_offset: u64,
        end_offset: u64,
    ) {
        let log = format!("store_chunk {file_name},{order},{chunk_id},{start_offset},{end_offset}");
        self.insert_log(log).await;
    }
    async fn delete_file(&self, file_name: &str) {
        let log = format!("delete_file {file_name}");
        self.insert_log(log).await;
    }
    async fn delete_chunk(&self, file_name: &str, chunk_id: &str) {
        let log = format!("delete_chunk {file_name},{chunk_id}");
        self.insert_log(log).await
    }
}

impl Replayer for DefaultLedger {
    fn replay(&self) -> Result<crate::namenode_state::NamenodeState, Box<dyn std::error::Error>> {
        debug!(filepath = %self.log_store,"file path");
        let log_file = std::fs::OpenOptions::new()
            .read(true)
            .open(self.log_store.clone())?;
        debug!("opened log file");
        let logs = BufReader::new(log_file).lines().map(|l| l.unwrap());
        debug!("created Buf reader");
        let mut state = NamenodeState::new();
        for log in logs {
            debug!("iterating over logs");
            // we got the log entry
            let parts: Vec<&str> = log.split(' ').collect();
            if parts.len() >= 3 {
                //let timestamp = parts[0];
                let operation = parts[1];
                let item = parts[2];
                match operation {
                    "store_file" => {
                        // it will be structred as item = file_name,number_of_chunks
                        let file_details: Vec<&str> = item.split(',').collect();
                        let filename = file_details[0].to_owned();
                        state.file_to_chunk_map.insert(filename, vec![]);
                    }
                    "store_chunk" => {
                        // it will be of structure filename,order,chunk_id,start_offset,end_offset
                        let chunk_details: Vec<&str> = item.split(',').collect();
                        let filename = chunk_details[0];
                        let chunk_id = chunk_details[2].to_owned();
                        let start_offset: u64 = chunk_details[3]
                            .parse()
                            .expect("Invalid start offset value");
                        let end_offset: u64 =
                            chunk_details[4].parse().expect("Invalid end offset value");

                        if chunk_details.len() < 5 {
                            error!(%log,"Invalid store_chunk log format");
                        }
                        let chunk_boundry = ChunkBounderies {
                            chunk_id: chunk_id.clone(),
                            start_offset,
                            end_offset,
                        };
                        let chunks = state
                            .file_to_chunk_map
                            .get_mut(filename)
                            .expect("Got chunk details before file store");
                        chunks.push(chunk_id.clone());
                        state.chunk_to_boundry_map.insert(chunk_id, chunk_boundry);
                    }
                    "delete_file" => {
                        // it will only contain file_name
                        state.file_to_chunk_map.remove(item); // we just remove the file
                    }
                    "delete_chunk" => {
                        // it will only contain file_name,chunk_id
                        let chunk_details: Vec<&str> = item.split(',').collect();
                        state.deleted_chunks.insert(chunk_details[1].to_owned());
                        state.chunk_to_boundry_map.remove(item);
                    }
                    _ => {
                        error!(%log,"Log with invalid operation found");
                        return Err("Invalid operation".into());
                    }
                }
            } else {
                error!(%log,"Error while replaying log, found malformed log");
            }
        }
        // now we will read the file line by line
        Ok(state)
    }
}
