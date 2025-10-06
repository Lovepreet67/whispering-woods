use crate::config::CONFIG;
use tonic::{metadata::MetadataValue, service::Interceptor};

pub struct NamenodeAuthIntercepter;

impl Interceptor for NamenodeAuthIntercepter {
    fn call(&mut self, mut req: tonic::Request<()>) -> Result<tonic::Request<()>, tonic::Status> {
        let req_meta = req.metadata_mut();
        req_meta.insert("auth_type", MetadataValue::from_static("CertAuth"));
        req_meta.insert("cert", MetadataValue::from_static(&CONFIG.namenode_cert));
        Ok(req)
    }
}
