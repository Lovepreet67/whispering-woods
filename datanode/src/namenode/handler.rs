use proto::generated::namenode_datanode::{
    DeleteChunkRequest, DeleteChunkResponse, namenode_datanode_server::NamenodeDatanode,
};
use storage::{file_storage::FileStorage, storage::Storage};
use utilities::logger::{error, instrument, tracing};

pub struct NamenodeHandler {
    store: FileStorage,
}

impl NamenodeHandler {
    pub fn new(store: FileStorage) -> Self {
        Self { store }
    }
}

#[tonic::async_trait]
impl NamenodeDatanode for NamenodeHandler {
    #[instrument(skip(self,request), fields(chunk_id = %request.get_ref().id))]
    async fn delete_chunk(
        &self,
        request: tonic::Request<DeleteChunkRequest>,
    ) -> Result<tonic::Response<DeleteChunkResponse>, tonic::Status> {
        let delete_chunk_request = request.into_inner();
        let chunk_id = delete_chunk_request.id;
        let exists = match self.store.delete(chunk_id).await {
            Ok(v) => v,
            Err(e) => {
                error!(%e,"error while deleting chunk in datanode ");
                return Err(tonic::Status::new(tonic::Code::Unavailable, e.to_string()));
            }
        };
        let delete_chunk_response = DeleteChunkResponse { available: exists };
        Ok(tonic::Response::new(delete_chunk_response))
    }
}
