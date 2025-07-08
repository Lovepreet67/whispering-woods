use std::{sync::Arc, vec};

use proto::generated::client_namenode::{
    ChunkMeta, DeleteFileRequest, DeleteFileResponse, FetchFileRequest, FetchFileResponse,
    StoreFileRequest, StoreFileResponse, client_name_node_server::ClientNameNode,
};
use tokio::sync::Mutex;
use tonic::Code;
use utilities::logger::{debug, instrument, span, trace, Level};

use crate::{
    chunk_generator::{ChunkGenerator, DefaultChunkGenerator},
    data_structure::ChunkBounderies,
    datanode::{
        selection_policy::{
            default_selection_policy::DefaultDatanodeSelectionPolicy,
            selection_policy::DatanodeSelectionPolicy,
        },
        service::DatanodeService,
    },
    ledger::default_ledger::Ledger,
    namenode_state::NamenodeState,
};

pub struct ClientHandler {
    state: Arc<Mutex<NamenodeState>>,
    datanode_selector: Box<dyn DatanodeSelectionPolicy + Send + Sync>,
    chunk_generator: Box<dyn ChunkGenerator + Send + Sync>,
    datanode_service: DatanodeService,
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
            datanode_service: DatanodeService::new(),
            ledger,
        }
    }
}
#[tonic::async_trait]
impl ClientNameNode for ClientHandler {
    async fn store_file(
        &self,
        request: tonic::Request<StoreFileRequest>,
    ) -> Result<tonic::Response<StoreFileResponse>, tonic::Status> {
        let store_file_request = request.get_ref();
        let fn_span = span!(Level::INFO,"grpc_client_store_file",file_name = %store_file_request.file_name,file_size=%store_file_request.file_size);
        let _entered = fn_span.enter();
        let chunk_bounderies = self
            .chunk_generator
            .get_chunks(store_file_request.file_size, &store_file_request.file_name);
        trace!(bounderies = ?chunk_bounderies,"Got chunk_bounderies");
        self.ledger
            .store_file(&store_file_request.file_name, chunk_bounderies.len() as u64)
            .await;
        let mut chunk_meta: Vec<ChunkMeta> = vec![];
        for chunk_boundery in chunk_bounderies {
            self.ledger
                .store_chunk(
                    &store_file_request.file_name,
                    chunk_meta.len() as u64,
                    &chunk_boundery.chunk_id,
                    chunk_boundery.start_offset,
                    chunk_boundery.end_offset,
                )
                .await;
            let location = self
                .datanode_selector
                .get_datanodes_to_store(chunk_boundery.end_offset - chunk_boundery.start_offset)
                .await
                .map_err(|e| tonic::Status::internal(format!("{}", e)))?;
            chunk_meta.push(ChunkMeta {
                id: chunk_boundery.chunk_id,
                start_offset: chunk_boundery.start_offset,
                end_offset: chunk_boundery.end_offset,
                location,
            });
        }
        // add this detail to namenode meta
        let mut state = self.state.lock().await;
        state.file_to_chunk_map.insert(
            store_file_request.file_name.clone(),
            chunk_meta.iter().map(|chunk| chunk.id.clone()).collect(),
        );
        // inserting the chunk boundary detail in state
        chunk_meta.iter().for_each(|chunk| {
            state.chunk_to_boundry_map.insert(
                chunk.id.clone(),
                ChunkBounderies {
                    chunk_id: chunk.id.clone(),
                    start_offset: chunk.start_offset,
                    end_offset: chunk.end_offset,
                },
            );
        });
        trace!(chunk_meta = ?chunk_meta,"Handled request");
        let store_file_response = StoreFileResponse {
            file_name: store_file_request.file_name.clone(),
            chunk_list: chunk_meta,
        };
        Ok(tonic::Response::new(store_file_response))
    }
    async fn fetch_file(
        &self,
        request: tonic::Request<FetchFileRequest>,
    ) -> Result<tonic::Response<FetchFileResponse>, tonic::Status> {
        let fetch_file_request = request.get_ref();
        let fn_span = span!(Level::INFO,"grpc_client_fetch_file",file_name = %fetch_file_request.file_name);
        let _entered = fn_span.enter();
        //TODO: find something better than cloning state
        //if we don't clone here this becomes deadlock when we can function get_datanodes_to_serve
        //which also uses state may be reader writer locak will help
        let state = self.state.lock().await.clone();
        if let Some(chunks) = state.file_to_chunk_map.get(&fetch_file_request.file_name) {
            debug!("found file");
            let mut chunk_list: Vec<ChunkMeta> = vec![];
            for chunk in chunks {
                let location = match self.datanode_selector.get_datanodes_to_serve(chunk).await {
                    Ok(location) => location,
                    Err(e) => {
                        return Err(tonic::Status::not_found(format!("{}", e)));
                    }
                };
                let chunk_boundery = match state.chunk_to_boundry_map.get(chunk) {
                    Some(v) => v,
                    None => {
                        return Err(tonic::Status::not_found("Error while geting chunk meta"));
                    }
                };
                chunk_list.push(ChunkMeta {
                    id: chunk.to_string(),
                    location: vec![location],
                    start_offset: chunk_boundery.start_offset,
                    end_offset: chunk_boundery.end_offset,
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
    async fn delete_file(
        &self,
        request: tonic::Request<DeleteFileRequest>,
    ) -> Result<tonic::Response<DeleteFileResponse>, tonic::Status> {
        let delete_file_request = request.get_ref();
        let fn_span = span!(
            Level::INFO,
            "grpc_client_delete_file",
            file_name = delete_file_request.file_name
        );
        let _enter = fn_span.enter();
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
                .delete_chunk(&delete_file_request.file_name, &chunk)
                .await;
            if let Some(location) = state.chunk_to_location_map.get(chunk) {
                for datanode_id in location {
                    debug!(?datanode_id, ?chunk, "deleting on this location ");
                    if let Some(datanode_meta) = state.datanode_to_meta_map.get(datanode_id) {
                        let datanode_service = self.datanode_service;
                        let addrs = datanode_meta.addrs.clone();
                        let chunk = chunk.clone();
                        tokio::spawn(async move {
                            let _ = datanode_service.clone().delete_chunk(&addrs, &chunk).await;
                            //{
                            //Ok(v)=>{
                            //    debug!(?datanode_id,?chunk,"deletion done")
                            //}
                            //Err(e)=>{
                            //    error!(%e,"error while deleting chunk from datanode")
                            //}
                            //}
                        });
                    }
                }
            } else {
                return Err(tonic::Status::new(
                    Code::Internal,
                    format!("Can't find any location of chunk : {}", chunk),
                ));
            }
        }
        for chunk in &chunks {
            state.deleted_chunks.insert(chunk.to_owned());
        }
        trace!("delete file request handeled");
        let delete_file_response = DeleteFileResponse { file_present: true };
        Ok(tonic::Response::new(delete_file_response))
    }
}
