use crate::auth::{
    Authenticator,
    authentication_error::AuthenticationError,
    types::{Credentials, NodeMetadata, NodeType},
};
use jsonwebtoken::{DecodingKey, TokenData, Validation, decode, errors::Error};
use serde::{Deserialize, Serialize};

/// Authonticator implementation to allow any Connection
#[derive(Clone, Debug)]
pub(crate) struct JwtTokenAuthenticator {
    secret: String,
}
impl JwtTokenAuthenticator {
    pub fn new(secret: String) -> Self {
        Self { secret }
    }
}
#[derive(Debug, Serialize, Deserialize)]
pub struct Claims {
    pub id: String,
    pub node_type: NodeType,
    pub exp: usize,
}

impl Authenticator for JwtTokenAuthenticator {
    fn authenticate(
        &self,
        credentials: Credentials,
    ) -> Result<super::types::NodeMetadata, super::authentication_error::AuthenticationError> {
        if let Credentials::JwtTokenAuth { token } = credentials {
            let decoded_result: Result<TokenData<Claims>, Error> = decode::<Claims>(
                &token,
                &DecodingKey::from_secret(self.secret.as_ref()),
                &Validation::default(),
            );
            match decoded_result {
                Ok(data) => Ok(NodeMetadata {
                    id: data.claims.id,
                    node_type: data.claims.node_type,
                    authenticated_using: crate::auth::types::AuthType::JwtTokenAuth,
                }),
                Err(_e) => Err(AuthenticationError::InvalidCredentials),
            }
        } else {
            panic!("Invalid credentials type for JwtAuthenticator")
        }
    }
}
