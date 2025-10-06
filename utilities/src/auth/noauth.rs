use crate::auth::{
    Authenticator,
    types::{Credentials, NodeMetadata},
};

/// Authonticator implementation to allow any Connection
#[derive(Clone, Debug, Default)]
pub(crate) struct NoAuthAuthenticator;

impl NoAuthAuthenticator {
    pub fn new() -> Self {
        Self
    }
}

impl Authenticator for NoAuthAuthenticator {
    fn authenticate(
        &self,
        credentials: Credentials,
    ) -> Result<super::types::NodeMetadata, super::authentication_error::AuthenticationError> {
        // for no auth it doesn't matter what type of credentails are present we will just return
        // the Node Metadata
        if let Credentials::NoAuthAuth { id, node_type } = credentials {
            Ok(NodeMetadata {
                id: id.to_string(),
                node_type,
                authenticated_using: super::types::AuthType::NoAuth,
            })
        } else {
            panic!("Invalid Credentail type for NoAuthAuthenticator")
        }
    }
}
