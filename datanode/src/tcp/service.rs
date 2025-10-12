use std::sync::Arc;
use storage::{file_storage::FileStorage, storage::Storage};
use tokio::{
    io::{AsyncReadExt, AsyncWriteExt, copy},
    net::{TcpListener, TcpStream},
    sync::Mutex,
};
use utilities::{
    data_packet::DataPacket,
    logger::{Instrument, Level, Span, error, span, trace},
    result::Result,
    ticket::{ticket_decrypter::TicketDecrypter, types::Operation},
};

use crate::{config::CONFIG, datanode_state::DatanodeState, tcp::stream_tee};

pub struct TCPService {
    listener: TcpListener,
    store: FileStorage,
    state: Arc<Mutex<DatanodeState>>,
    ticket_decrypter: Arc<Box<dyn TicketDecrypter>>,
}

impl TCPService {
    pub async fn new(
        address: String,
        store: FileStorage,
        state: Arc<Mutex<DatanodeState>>,
        ticket_decrypter: Arc<Box<dyn TicketDecrypter>>,
    ) -> Result<Self> {
        let listener = TcpListener::bind(address).await?;
        Ok(TCPService {
            listener,
            store,
            state,
            ticket_decrypter,
        })
    }
    pub async fn start_and_accept(&self) -> Result<()> {
        loop {
            let (tcp_stream, _) = self.listener.accept().await?;
            let store = self.store.clone();
            let state = self.state.clone();
            let ticket_decrypter = self.ticket_decrypter.clone();
            let span = Span::current();
            tokio::spawn(
                async move {
                    if let Err(e) =
                        Self::handle_connection(tcp_stream, store, state, ticket_decrypter).await
                    {
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
        ticket_decrypter: Arc<Box<dyn TicketDecrypter>>,
    ) -> Result<()> {
        // first we will fetch the chunk_id
        let headers = DataPacket::decode(&mut tcp_stream).await?;
        let ticket = ticket_decrypter.decrypt_server_ticket(headers.get("ticket")?)?;
        let chunk_id = headers.get("chunk_id")?.to_string();
        trace!(%chunk_id,"Got TCP stream {:?}",ticket);
        // now we will fetch the mode (client wants to read or write)
        let mode = headers.get("mode")?;
        if mode == "Write" {
            let is_valid_ticket = match ticket.operation {
                Operation::StoreChunk {
                    chunk_id: ticket_chunk_id,
                } if ticket.target_node_id == CONFIG.datanode_id => ticket_chunk_id == chunk_id,
                Operation::CreatePipeline {
                    chunk_id: ticket_chunk_id,
                } if ticket.target_node_id == CONFIG.datanode_id => ticket_chunk_id == chunk_id,

                _ => false,
            };

            if !is_valid_ticket {
                return Err("Ticket provided is not valid".into());
            }
            let span = span!(Level::INFO,"service_tcp_write_chunk",%chunk_id);
            let _gaurd = span.enter();
            trace!(%chunk_id,"Mode set to write");
            let chunk_size: u64 = headers.get("chunk_size")?.parse()?;
            trace!(%chunk_size,"Bytes to be written from the chunk");
            let (read_stream, mut write_stream) = tcp_stream.into_split();
            let mut limited_read_stream = read_stream.take(chunk_size);
            // read a file from the
            // after reading the chunk_id and mode  we will post that details to the pipeline
            let pipeline_options = {
                let mut state_lock = state.lock().await;
                state_lock.chunk_to_pipline.remove(&chunk_id)
            };
            if let Some(mut pipeline) = pipeline_options {
                // if there is pipeline there will be definately a ticket
                let ticket = {
                    let state_lock = state.lock().await;
                    state_lock
                        .chunk_to_namenode_store_ticket
                        .get(&chunk_id)
                        .unwrap()
                        .to_string()
                };
                // create tee only if you need one otherwise 2nd stream will not be consumed and
                // program will be in lockin
                let (mut stream1, mut stream2) =
                    stream_tee::tee_tcp_stream(limited_read_stream, write_stream);
                let mut pipeline_headers = DataPacket::new();
                pipeline_headers.insert("chunk_id".to_string(), chunk_id.clone());
                pipeline_headers.insert("mode".to_string(), "Write".to_string());
                pipeline_headers.insert("ticket".to_string(), ticket);
                pipeline_headers.insert("chunk_size".to_string(), chunk_size.to_string());
                tokio::io::copy(&mut pipeline_headers.encode(), &mut pipeline).await?;

                // faced issue when not running below task parrally because if we do one by one
                // after first finish tx1 and tx2 both will be dropped which will hang state when
                // fetching data from other stream. :)
                tokio::spawn(
                    async move {
                        let bytes_received = match copy(&mut stream1, &mut pipeline).await {
                            Ok(_) => match DataPacket::decode(&mut pipeline).await {
                                Ok(v) => v
                                    .get("bytes_received")
                                    .unwrap_or("0")
                                    .parse()
                                    .unwrap_or(0_u64),
                                Err(_e) => 0,
                            },
                            Err(e) => {
                                error!("Error while sending data to pipeline {e}");
                                0
                            }
                        };
                        stream1.write_u64(bytes_received).await;
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
                let written_bytes = match store.write(chunk_id, &mut limited_read_stream).await {
                    Ok(bytes_written_to_file) => {
                        trace!("{} bytes written to file", bytes_written_to_file);
                        bytes_written_to_file
                    }
                    Err(e) => {
                        error!("Error while storing the chunk");
                        error!("{}", e);
                        0
                    }
                };
                let mut reply_packet = DataPacket::new();
                reply_packet.insert("bytes_received".to_string(), written_bytes.to_string());
                let mut reply_packet_stream = reply_packet.encode();
                let _ = tokio::io::copy(&mut reply_packet_stream, &mut write_stream).await;
                match write_stream.flush().await {
                    Ok(_) => {}
                    Err(e) => {
                        error!("Error while flushing the written bytes to client");
                        error!("{}", e);
                    }
                }
            }
        } else if mode == "Read" {
            let is_valid_ticket = match ticket.operation {
                Operation::FetchChunk {
                    chunk_id: ticket_chunk_id,
                } if ticket.target_node_id == CONFIG.datanode_id => ticket_chunk_id == chunk_id,
                _ => false,
            };

            if !is_valid_ticket {
                trace!("Ticket provided is not valid");
                return Err("Ticket provided is not valid".into());
            }

            let span = span!(Level::INFO,"service_tcp_read_chunk",%chunk_id);
            let _gaurd = span.enter();
            trace!(%chunk_id,"Mode set to read");

            //if file is already present just delete it (for future)
            let reader = store.read(chunk_id).await?;
            copy(&mut reader.take(u64::MAX), &mut tcp_stream).await?;
            tcp_stream.flush().await?;
        } else {
            return Err(
                format!("accepted request for chunk id {chunk_id} for unknown mode",).into(),
            );
        }
        Ok(())
    }
}
