use crate::config::CONFIG;
use jsonwebtoken::{EncodingKey, Header, encode};
use rocket::{post, serde::json::Json};
use serde::{Deserialize, Serialize};
use std::time::{SystemTime, UNIX_EPOCH};
use utilities::auth::jwt_token::Claims;

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct LoginRequest {
    username: String,
    password: String,
}
#[derive(Clone, Debug, Deserialize, Serialize)]
pub enum LoginResponse {
    Token(String),
    Error(String),
}

#[post("/login", data = "<body>")]
pub async fn login(body: Json<LoginRequest>) -> Json<LoginResponse> {
    let body = body.into_inner();
    if body.username == CONFIG.api_username && body.password == CONFIG.api_password {
        let exp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("Time went backwards")
            .as_secs()
            + 100001_u64;
        match encode(
            &Header::default(),
            &Claims {
                id: body.username,
                node_type: utilities::auth::types::NodeType::Client,
                exp: exp as usize,
            },
            &EncodingKey::from_secret(CONFIG.jwt_sign_key.as_ref()),
        ) {
            Ok(signed_body) => {
                return Json::from(LoginResponse::Token(signed_body));
            }
            Err(e) => {
                return Json::from(LoginResponse::Error(format!(
                    "Error while signing key : {e}",
                )));
            }
        }
    }
    Json::from(LoginResponse::Error(
        "Invalid username or password".to_string(),
    ))
}
