use log::{error, info, trace};
use std::{error::Error, sync::Arc};
use storage::{file_storage::FileStorage, storage::Storage};
use tokio::{
    io::{AsyncReadExt, AsyncWriteExt, copy},
    net::{TcpListener, TcpStream},
    sync::Mutex,
};

use crate::{
    datanode_state::{self, DatanodeState},
    tcp_stream_tee::{self, tee_tcp_stream},
};

pub struct TCPService {
    listener: TcpListener,
    store: FileStorage,
    state: Arc<Mutex<DatanodeState>>,
}

impl TCPService {
    pub async fn new(
        port: String,
        store: FileStorage,
        state: Arc<Mutex<DatanodeState>>,
    ) -> Result<Self, Box<dyn Error>> {
        let address = format!("127.0.0.1:{}", port).to_owned();
        let listener = TcpListener::bind(address).await?;
        Ok(TCPService {
            listener,
            store,
            state,
        })
    }
    pub async fn start_and_accept(&self) -> Result<(), Box<dyn Error>> {
        loop {
            let (tcp_stream, _) = self.listener.accept().await?;
            let store = self.store.clone();
            let state = self.state.clone();
            tokio::spawn(async move {
                if let Err(e) = Self::handle_connection(tcp_stream, store, state).await {
                    error!("error while handling the tcp connection {e}");
                }
            });
        }
    }
    async fn handle_connection(
        mut tcp_stream: TcpStream,
        store: FileStorage,
        state: Arc<Mutex<DatanodeState>>,
    ) -> Result<(), Box<dyn Error>> {
        // first we will fetch the chunk_id
        let mut chunk_id_bytes = [0u8; 16];
        tcp_stream.read_exact(&mut chunk_id_bytes).await?;
        let chunk_id = Self::bytes_to_uuid(chunk_id_bytes);
        info!("got chunk_id {chunk_id}");
        // now we will fetch the mode (client wants to read or write)
        let mut mode: u8 = 0;
        tcp_stream
            .read_exact(std::slice::from_mut(&mut mode))
            .await?;
        if mode == 1 {
            trace!("accepted request for chunk id {} for write mode", chunk_id);
            // read a file from the
            // after reading the chunk_id and mode  we will post that details to the pipeline
            let (mut stream1, mut stream2) = tee_tcp_stream(tcp_stream);
            store.write(chunk_id.clone(), &mut stream1).await?;
            let mut curr_state = state.lock().await;
            if let Some(mut pipeline) = curr_state.chunk_to_pipline.remove(&chunk_id) {
                pipeline.write_all(&chunk_id_bytes).await?;
                pipeline.write_all(&[mode]).await?;
                copy(&mut stream2, &mut pipeline).await?;
            }
        } else if mode == 2 {
            trace!("accepted request for chunk id {} for read mode", chunk_id);
            //if file is already present just delete it (for future)
            let reader = store.read(chunk_id).await?;
            copy(&mut reader.take(u64::MAX), &mut tcp_stream).await?;
            tcp_stream.flush().await?;
        } else {
            return Err(format!(
                "accepted request for chunk id {} for unknown mode",
                chunk_id
            )
            .into());
        }
        Ok(())
    }
    fn bytes_to_uuid(chunk_id_bytes: [u8; 16]) -> String {
        chunk_id_bytes
            .iter()
            .map(|b| format!("{:02x}", b))
            .collect::<String>()
    }
}
