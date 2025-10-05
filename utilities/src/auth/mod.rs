use crate::auth::{
    authentication_error::AuthenticationError,
    cert::CertAuthenticator,
    jwt_token::JwtTokenAuthenticator,
    noauth::NoAuthAuthenticator,
    types::{Credentials, NodeMetadata},
};
use std::collections::HashMap;

pub mod authentication_error;
pub mod cert;
pub mod jwt_token;
pub mod noauth;
pub mod types;

/// This trait should be implemented in order to use custom authenticator
pub(crate) trait Authenticator {
    fn authenticate(&self, credentials: Credentials) -> Result<NodeMetadata, AuthenticationError>;
}

#[derive(Clone, Debug)]
pub struct AuthManager {
    no_auth_authenticator: Option<NoAuthAuthenticator>,
    jwt_token_authenticator: Option<JwtTokenAuthenticator>,
    cert_authenticator: Option<CertAuthenticator>,
}

impl AuthManager {
    pub fn builder() -> Self {
        Self {
            no_auth_authenticator: None,
            jwt_token_authenticator: None,
            cert_authenticator: None,
        }
    }
    pub fn upsert_no_auth_authenticator(mut self) -> Self {
        self.no_auth_authenticator = Some(NoAuthAuthenticator::new());
        self
    }
    pub fn upsert_jwt_token_authenticator(mut self, secret: String) -> Self {
        self.jwt_token_authenticator = Some(JwtTokenAuthenticator::new(secret));
        self
    }
    pub fn upsert_cert_authenticator(mut self, root_cert_pem: &str) -> Self {
        self.cert_authenticator = Some(CertAuthenticator::new(root_cert_pem).unwrap());
        self
    }
    pub fn authenticate(
        &self,
        headers: &HashMap<&str, &str>,
    ) -> Result<NodeMetadata, AuthenticationError> {
        // check for authentication type
        match *headers.get("auth_type").unwrap_or(&"NoAuth") {
            "NoAuth" if self.no_auth_authenticator.is_some() => {
                let credentials = Credentials::NoAuthAuth {
                    id: headers.get("id").unwrap_or(&"Unknown").to_string(),
                    node_type: headers.get("node_type").into(),
                };
                self.no_auth_authenticator
                    .as_ref()
                    .unwrap()
                    .authenticate(credentials)
            }
            "JwtTokenAuth" if self.jwt_token_authenticator.is_some() => {
                let credentials = match headers.get("jwt_token") {
                    Some(val) => Credentials::JwtTokenAuth {
                        token: val.to_string(),
                    },
                    None => return Err(AuthenticationError::MissingCredentials),
                };
                self.jwt_token_authenticator
                    .as_ref()
                    .unwrap()
                    .authenticate(credentials)
            }
            "CertAuth" if self.cert_authenticator.is_some() => {
                let credentials = match headers.get("cert") {
                    Some(val) => Credentials::CertAuth {
                        cert_pem: val.to_string(),
                    },
                    None => return Err(AuthenticationError::MissingCredentials),
                };
                self.cert_authenticator
                    .as_ref()
                    .unwrap()
                    .authenticate(credentials)
            }
            _ => Err(AuthenticationError::InvalidAuthenticationMethod(
                "Authentication method {_} is not supported".to_string(),
            )),
        }
    }
}
