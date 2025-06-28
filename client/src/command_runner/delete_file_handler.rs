use crate::namenode_service::NamenodeService;
use utilities::{
    logger::{instrument, trace, tracing},
    result::Result,
};

#[derive(Debug)]
pub struct DeleteFileHandler {
    namenode: NamenodeService,
}
impl DeleteFileHandler {
    pub fn new(namenode: NamenodeService) -> Self {
        Self { namenode }
    }
    #[instrument(skip(self))]
    pub async fn delete_file(&mut self, remote_file_name: String) -> Result<String> {
        trace!("sending a delete file request to the namenode");
        let delete_node = self.namenode.delete_file(remote_file_name).await?;
        if !delete_node {
            return Ok("File was not present".to_owned());
        }
        Ok("File deleted successfully".to_owned())
    }
}
