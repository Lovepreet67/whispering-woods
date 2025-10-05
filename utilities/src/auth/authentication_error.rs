use std::error::Error;
use std::fmt::Display;

#[derive(Clone, Debug)]
pub enum AuthenticationError {
    InvalidCredentials,
    MissingCredentials,
    InvalidAuthenticationMethod(String),
    Internal(String),
}
impl Display for AuthenticationError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            AuthenticationError::MissingCredentials => {
                write!(f, "Crendtials are missing")
            }
            AuthenticationError::InvalidCredentials => {
                write!(f, "Provided credentails are Invalid")
            }
            AuthenticationError::InvalidAuthenticationMethod(msg) => {
                write!(f, "Method Not allowed, allowed methods are : {}", msg)
            }
            AuthenticationError::Internal(msg) => {
                write!(f, "Internal error : {}", msg)
            }
        }
    }
}

impl From<AuthenticationError> for Box<dyn Error + Send + Sync> {
    fn from(value: AuthenticationError) -> Self {
        match value {
            AuthenticationError::InvalidCredentials => "Invalid Credentials".into(),
            AuthenticationError::MissingCredentials => "Missing credentials".into(),
            AuthenticationError::InvalidAuthenticationMethod(e) => e.into(),
            AuthenticationError::Internal(e) => e.into(),
        }
    }
}
