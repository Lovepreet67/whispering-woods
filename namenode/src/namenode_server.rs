use std::{sync::Arc, vec};

use proto::generated::client_namenode::{
    ChunkMeta, DeleteFileRequest, DeleteFileResponse, FetchFileRequest, FetchFileResponse,
    StoreFileRequest, StoreFileResponse, client_name_node_server::ClientNameNode,
};
use tokio::sync::Mutex;

use crate::{
    chunk_generator::{ChunkGenerator, DefaultChunkGenerator},
    datanode_selection_policy::{DatanodeSelectionPolicy, DefaultDatanodeSelectionPolicy},
    namenode_state::NamenodeState,
};

pub struct NamenodeServer {
    state: Arc<Mutex<NamenodeState>>,
    datanode_selector: Box<dyn DatanodeSelectionPolicy + Send + Sync>,
    chunk_generator: Box<dyn ChunkGenerator + Send + Sync>,
}
impl NamenodeServer {
    pub fn new() -> Self {
        let state = Arc::new(Mutex::new(NamenodeState::default()));
        let datanode_selection_policy =
            Box::new(DefaultDatanodeSelectionPolicy::new(state.clone()));
        let chunk_generator = Box::new(DefaultChunkGenerator::new((64 * 1024 * 1024) as u64));
        NamenodeServer {
            state,
            datanode_selector: datanode_selection_policy,
            chunk_generator,
        }
    }
}
#[tonic::async_trait]
impl ClientNameNode for NamenodeServer {
    async fn store_file(
        &self,
        request: tonic::Request<StoreFileRequest>,
    ) -> Result<tonic::Response<StoreFileResponse>, tonic::Status> {
        let store_file_request = request.get_ref();
        let chunk_bounderies = self
            .chunk_generator
            .get_chunks(store_file_request.file_size, &store_file_request.file_name);
        let mut chunk_meta: Vec<ChunkMeta> = vec![];
        for chunk_boundery in chunk_bounderies {
            let location = self
                .datanode_selector
                .get_datanodes(chunk_boundery.end_offset - chunk_boundery.start_offset)
                .await
                .map_err(|e| tonic::Status::internal(format!("{}", e)))?;
            chunk_meta.push(ChunkMeta {
                id: chunk_boundery.chunk_id,
                start_offset: chunk_boundery.start_offset,
                end_offset: chunk_boundery.end_offset,
                location,
            });
        }
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

        let fetch_file_response = FetchFileResponse {
            file_name: fetch_file_request.file_name.clone(),
            chunk_list: Vec::<ChunkMeta>::new(),
        };
        Ok(tonic::Response::new(fetch_file_response))
    }
    async fn delete_file(
        &self,
        request: tonic::Request<DeleteFileRequest>,
    ) -> Result<tonic::Response<DeleteFileResponse>, tonic::Status> {
        let _delete_file_request = request.get_ref();

        let delete_file_response = DeleteFileResponse { file_present: true };
        Ok(tonic::Response::new(delete_file_response))
    }
}
