use crate::config::CONFIG;
use jsonwebtoken::{DecodingKey, TokenData, Validation, decode, errors::Error};
use rocket::{
    Request,
    http::Status,
    outcome::Outcome,
    request::{self, FromRequest},
};
use serde::{Deserialize, Serialize};
#[derive(Debug, Serialize, Deserialize)]
pub struct Claims {
    pub sub: String,
    pub exp: usize,
}
pub struct Username(String);
#[rocket::async_trait]
impl<'r> FromRequest<'r> for Username {
    type Error = Box<dyn std::error::Error>;
    async fn from_request(req: &'r Request<'_>) -> request::Outcome<Self, Self::Error> {
        let token: &str = match req.headers().get_one("Authorization") {
            Some(v) => v,
            None => {
                return request::Outcome::Error((
                    Status::NonAuthoritativeInformation,
                    "Please provide the auth token it is required"
                        .to_string()
                        .into(),
                ));
            }
        };
        println!("{}", token);
        let decoded_result: Result<TokenData<Claims>, Error> = decode::<Claims>(
            token,
            &DecodingKey::from_secret(CONFIG.api_jwt_sign_key.as_ref()),
            &Validation::default(),
        );
        match decoded_result {
            Ok(data) => {
                return Outcome::Success(Username(data.claims.sub));
            }
            Err(e) => {
                println!("{:?}", e);
                return request::Outcome::Error((
                    Status::NonAuthoritativeInformation,
                    "Please provide valid auth token ".to_string().into(),
                ));
            }
        }
    }
}

