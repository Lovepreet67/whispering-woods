use rocket::{
    Request,
    http::Status,
    request::{self, FromRequest},
};
use std::collections::HashMap;
use utilities::auth::{AuthManager, types::NodeMetadata};

pub struct NodeMetadataWrapper(pub NodeMetadata);
#[rocket::async_trait]
impl<'r> FromRequest<'r> for NodeMetadataWrapper {
    type Error = Box<dyn std::error::Error>;
    async fn from_request(req: &'r Request<'_>) -> request::Outcome<Self, Self::Error> {
        let mut headers = HashMap::new();
        vec!["auth_type", "jwt_token", "id", "node_type", "cert"]
            .into_iter()
            .for_each(|key| {
                if let Some(value) = req.headers().get_one(key) {
                    headers.insert(key, value);
                };
            });
        let authenticator = req.rocket().state::<AuthManager>().unwrap();
        match authenticator.authenticate(&headers) {
            Ok(node_meta) => return request::Outcome::Success(NodeMetadataWrapper(node_meta)),
            Err(e) => match e {
                utilities::auth::authentication_error::AuthenticationError::InvalidCredentials => {
                    return request::Outcome::Error((
                        Status::NonAuthoritativeInformation,
                        "Invalid credentials".to_string().into(),
                    ));
                }
                utilities::auth::authentication_error::AuthenticationError::MissingCredentials => {
                    return request::Outcome::Error((
                        Status::NonAuthoritativeInformation,
                        "Please provide the auth token it is required"
                            .to_string()
                            .into(),
                    ));
                }
                _ => {
                    return request::Outcome::Error((
                        Status::NonAuthoritativeInformation,
                        "Invalid method choosen".to_string().into(),
                    ));
                }
            },
        }
    }
}
