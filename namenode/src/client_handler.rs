use std::{sync::Arc, vec};

use proto::generated::client_namenode::{
    ChunkMeta, DeleteFileRequest, DeleteFileResponse, FetchFileRequest, FetchFileResponse,
    StoreFileRequest, StoreFileResponse, client_name_node_server::ClientNameNode,
};
use tokio::sync::Mutex;
use tonic::Code;
use utilities::logger::{instrument, trace, tracing};

use crate::{
    chunk_generator::{ChunkGenerator, DefaultChunkGenerator},
    datanode::selection_policy::{
        default_selection_policy::DefaultDatanodeSelectionPolicy,
        selection_policy::DatanodeSelectionPolicy,
    },
    ledger::default_ledger::Ledger,
    namenode_state::NamenodeState,
};

pub struct ClientHandler {
    state: Arc<Mutex<NamenodeState>>,
    datanode_selector: Box<dyn DatanodeSelectionPolicy + Send + Sync>,
    chunk_generator: Box<dyn ChunkGenerator + Send + Sync>,
    ledger: Box<dyn Ledger + Send + Sync>,
}
impl ClientHandler {
    pub fn new(state: Arc<Mutex<NamenodeState>>, ledger: Box<dyn Ledger + Send + Sync>) -> Self {
        let datanode_selection_policy =
            Box::new(DefaultDatanodeSelectionPolicy::new(state.clone()));
        let chunk_generator = Box::new(DefaultChunkGenerator::new((64 * 1024 * 1024) as u64));
        Self {
            state,
            datanode_selector: datanode_selection_policy,
            chunk_generator,
            ledger,
        }
    }
}
#[tonic::async_trait]
impl ClientNameNode for ClientHandler {
    #[instrument(name="grpc_client_store_file",skip(self,request),fields(file_name= %request.get_ref().file_name,file_size= %request.get_ref().file_size))]
    async fn store_file(
        &self,
        request: tonic::Request<StoreFileRequest>,
    ) -> Result<tonic::Response<StoreFileResponse>, tonic::Status> {
        let store_file_request = request.get_ref();
        let chunk_details = self
            .chunk_generator
            .get_chunks(store_file_request.file_size, &store_file_request.file_name);
        trace!(bounderies = ?chunk_details,"Got chunk_bounderies");
        self.ledger
            .store_file(&store_file_request.file_name, chunk_details.len() as u64)
            .await;
        let mut chunk_meta = vec![];
        for (index, chunk) in chunk_details.iter().enumerate() {
            self.ledger
                .store_chunk(
                    &store_file_request.file_name,
                    index as u64,
                    &chunk.id,
                    chunk.start_offset,
                    chunk.end_offset,
                )
                .await;
            let location = self
                .datanode_selector
                .get_datanodes_to_store(chunk.end_offset - chunk.start_offset)
                .await
                .map_err(|e| tonic::Status::internal(format!("{e}")))?;
            chunk_meta.push(ChunkMeta {
                id: chunk.id.clone(),
                start_offset: chunk.start_offset,
                end_offset: chunk.end_offset,
                location,
            });
        }
        // add this detail to namenode meta
        let mut state = self.state.lock().await;
        state.file_to_chunk_map.insert(
            store_file_request.file_name.clone(),
            chunk_details.iter().map(|chunk| chunk.id.clone()).collect(),
        );
        // inserting the chunk boundary detail in state
        chunk_details.into_iter().for_each(|chunk| {
            state.chunk_id_to_detail_map.insert(chunk.id.clone(), chunk);
        });
        trace!(chunk_meta = ?chunk_meta,"Handled request");
        let store_file_response = StoreFileResponse {
            file_name: store_file_request.file_name.clone(),
            chunk_list: chunk_meta,
        };
        Ok(tonic::Response::new(store_file_response))
    }
    #[instrument(name="grpc_client_fetch_file",skip(self,request),fields(file_name= %request.get_ref().file_name))]
    async fn fetch_file(
        &self,
        request: tonic::Request<FetchFileRequest>,
    ) -> Result<tonic::Response<FetchFileResponse>, tonic::Status> {
        let fetch_file_request = request.get_ref();
        //TODO: find something better than cloning state
        //if we don't clone here this becomes deadlock when we can function get_datanodes_to_serve
        //which also uses state may be reader writer locak will help
        let state = self.state.lock().await.clone();
        if let Some(chunks) = state.file_to_chunk_map.get(&fetch_file_request.file_name) {
            let mut chunk_list: Vec<ChunkMeta> = vec![];
            for chunk in chunks {
                let location = match self.datanode_selector.get_datanodes_to_serve(chunk).await {
                    Ok(location) => location,
                    Err(e) => {
                        return Err(tonic::Status::not_found(format!("{e}")));
                    }
                };
                let chunk_details = match state.chunk_id_to_detail_map.get(chunk) {
                    Some(v) => v,
                    None => {
                        return Err(tonic::Status::not_found("Error while geting chunk meta"));
                    }
                };
                chunk_list.push(ChunkMeta {
                    id: chunk.to_string(),
                    location: vec![location],
                    start_offset: chunk_details.start_offset,
                    end_offset: chunk_details.end_offset,
                });
            }
            trace!(chunk_list = ?chunk_list,"fetch file request Handled");
            let fetch_file_response = FetchFileResponse {
                file_name: fetch_file_request.file_name.clone(),
                chunk_list,
            };
            return Ok(tonic::Response::new(fetch_file_response));
        }
        Err(tonic::Status::not_found(format!(
            "Can't find the file meta in namenode filename : {}",
            fetch_file_request.file_name
        )))
    }
    #[instrument(name="grpc_client_delete_file",skip(self,request),fields(file_name= %request.get_ref().file_name))]
    async fn delete_file(
        &self,
        request: tonic::Request<DeleteFileRequest>,
    ) -> Result<tonic::Response<DeleteFileResponse>, tonic::Status> {
        let delete_file_request = request.get_ref();
        self.ledger
            .delete_file(&delete_file_request.file_name)
            .await;
        let mut state = self.state.lock().await;
        let chunks = match state
            .file_to_chunk_map
            .remove(&delete_file_request.file_name)
        {
            Some(v) => v.clone(),
            None => {
                let delete_file_response = DeleteFileResponse { file_present: true };
                return Ok(tonic::Response::new(delete_file_response));
            }
        };
        trace!(?chunks, "got chunks ");
        for chunk in &chunks {
            self.ledger
                .delete_chunk(&delete_file_request.file_name, chunk)
                .await;
            if let Some(chunk_details) = state.chunk_id_to_detail_map.get_mut(chunk) {
                chunk_details.mark_deleted();
            } else {
                return Err(tonic::Status::new(
                    Code::Internal,
                    format!("Can't find any location of chunk : {chunk}"),
                ));
            }
        }
        trace!("delete file request handeled");
        let delete_file_response = DeleteFileResponse { file_present: true };
        Ok(tonic::Response::new(delete_file_response))
    }
}
