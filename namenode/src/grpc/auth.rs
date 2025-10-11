use crate::config::CONFIG;
use std::collections::HashMap;
use tonic::service::Interceptor;
use utilities::{auth::AuthManager, logger::trace};

#[derive(Clone, Debug)]
pub struct AuthIntercepter {
    auth_manager: AuthManager,
}

impl AuthIntercepter {
    pub fn new(auth_manager: AuthManager) -> Self {
        Self { auth_manager }
    }
}

impl Interceptor for AuthIntercepter {
    fn call(&mut self, mut req: tonic::Request<()>) -> Result<tonic::Request<()>, tonic::Status> {
        let mut headers = HashMap::new();
        vec!["auth_type", "jwt_token", "id", "node_type", "cert"]
            .into_iter()
            .for_each(|key| {
                if let Some(value) = req.metadata().get(key) {
                    // we know these header will be strings
                    if let Ok(str_value) = value.to_str() {
                        headers.insert(key, str_value);
                    }
                };
            });
        match self.auth_manager.authenticate(&headers) {
            Ok(node_metadata) => {
                trace!("node autheticated : {:?}",node_metadata);
                req.extensions_mut().insert(node_metadata);
            Ok(req)
            },
            Err(e)=>{
                match e  {
                   utilities::auth::authentication_error::AuthenticationError::InvalidCredentials=>{
                       Err(tonic::Status::new(tonic::Code::Unauthenticated,"Invalid credentials"))
                   },
                   utilities::auth::authentication_error::AuthenticationError::InvalidAuthenticationMethod(msg)=>{
                       Err(tonic::Status::new(tonic::Code::InvalidArgument, &msg))
                   },
                   utilities::auth::authentication_error::AuthenticationError::MissingCredentials =>{
                       Err(tonic::Status::new(tonic::Code::InvalidArgument, "Please Provide credentails"))
                   }
                   utilities::auth::authentication_error::AuthenticationError::Internal(msg)=>{
                       Err(tonic::Status::new(tonic::Code::Internal, msg))
                   }
                }
            }}
    }
}

pub fn get_auth_intercepter_layer(
    root_cert_pem: &str,
) -> tonic::service::InterceptorLayer<AuthIntercepter> {
    let auth_manager = AuthManager::builder()
        // .upsert_no_auth_authenticator()
        .upsert_jwt_token_authenticator(CONFIG.jwt_sign_key.clone())
        .upsert_cert_authenticator(root_cert_pem);
    tonic::service::InterceptorLayer::new(AuthIntercepter::new(auth_manager))
}
