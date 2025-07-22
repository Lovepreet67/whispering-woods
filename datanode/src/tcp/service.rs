use core::str;
use std::sync::Arc;
use storage::{file_storage::FileStorage, storage::Storage};
use tokio::{
    io::{AsyncReadExt, AsyncWriteExt, copy},
    net::{TcpListener, TcpStream},
    sync::Mutex,
};
use utilities::{
    logger::{Instrument, Level, Span, error, span, trace},
    result::Result,
};

use crate::{datanode_state::DatanodeState, tcp::stream_tee};

pub struct TCPService {
    listener: TcpListener,
    store: FileStorage,
    state: Arc<Mutex<DatanodeState>>,
}

impl TCPService {
    pub async fn new(
        address: String,
        store: FileStorage,
        state: Arc<Mutex<DatanodeState>>,
    ) -> Result<Self> {
        let listener = TcpListener::bind(address).await?;
        Ok(TCPService {
            listener,
            store,
            state,
        })
    }
    pub async fn start_and_accept(&self) -> Result<()> {
        loop {
            let (tcp_stream, _) = self.listener.accept().await?;
            let store = self.store.clone();
            let state = self.state.clone();
            let span = Span::current();
            tokio::spawn(
                async move {
                    if let Err(e) = Self::handle_connection(tcp_stream, store, state).await {
                        error!("error while handling the tcp connection {e}");
                    }
                }
                .instrument(span),
            );
        }
    }
    async fn handle_connection(
        mut tcp_stream: TcpStream,
        store: FileStorage,
        state: Arc<Mutex<DatanodeState>>,
    ) -> Result<()> {
        // first we will fetch the chunk_id
        let mut chunk_id_bytes = [0u8; 36];
        tcp_stream.read_exact(&mut chunk_id_bytes).await?;
        let chunk_id = str::from_utf8(&chunk_id_bytes)
            .map_err(|e| format!("Error while converting the chunk id bytes to string {}", e))?
            .to_owned();
        trace!(%chunk_id,"Got TCP stream");
        // now we will fetch the mode (client wants to read or write)
        let mut mode: u8 = 0;
        tcp_stream
            .read_exact(std::slice::from_mut(&mut mode))
            .await?;
        if mode == 1 {
            let span = span!(Level::INFO,"service_tcp_write_chunk",%chunk_id);
            let _gaurd = span.enter();
            trace!(%chunk_id,"Mode set to write");
            // read a file from the
            // after reading the chunk_id and mode  we will post that details to the pipeline
            if let Some(mut pipeline) = state.lock().await.chunk_to_pipline.remove(&chunk_id) {
                // create tee only if you need one otherwise 2nd stream will not be consumed and
                // program will be in lockin
                let (mut stream1, mut stream2) = stream_tee::tee_tcp_stream(tcp_stream);
                pipeline.write_all(&chunk_id_bytes).await?;
                pipeline.write_u8(mode).await?;
                // faced issue when not running below task parrally because if we do one by one
                // after first finish tx1 and tx2 both will be dropped which will hang state when
                // fetching data from other stream. :)
                tokio::spawn(
                    async move {
                        match copy(&mut stream1, &mut pipeline).await {
                            Ok(_) => {
                                let bytes_recieved_by_pipeline =
                                    pipeline.read_u64().await.unwrap_or(0);
                                let _ = stream1.write_u64(bytes_recieved_by_pipeline).await;
                            }
                            Err(e) => {
                                error!("Error while sending data to pipeline {e}");
                                let _ = stream1.write_u64(0).await;
                            }
                        }
                    }
                    .in_current_span(),
                );
                tokio::spawn(
                    async move {
                        match store.write(chunk_id.clone(), &mut stream2).await {
                            Ok(bytes_written) => {
                                let _ = stream2.write_u64(bytes_written).await;
                            }
                            Err(e) => {
                                error!("Error while writing data to store {e}");
                                let _ = stream2.write_u64(0).await;
                            }
                        }
                    }
                    .in_current_span(),
                );
            } else {
                match store.write(chunk_id, &mut tcp_stream).await {
                    Ok(bytes_written_to_file) => {
                        println!("{bytes_written_to_file} bytes written to file");
                        tcp_stream.write_u64(bytes_written_to_file).await?;
                    }
                    Err(_) => {
                        tcp_stream.write_u64(0).await?;
                    }
                }
            }
        } else if mode == 2 {
            let span = span!(Level::INFO,"service_tcp_read_chunk",%chunk_id);
            let _gaurd = span.enter();
            trace!(%chunk_id,"Mode set to read");

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
}
