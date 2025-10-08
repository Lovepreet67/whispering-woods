use crate::{
    api_service::middleware::auth::NodeMetadataWrapper,
    certificates::certificate_generator::CertificateAuthority, ledger::default_ledger::Ledger,
};
use base64::{Engine, prelude::BASE64_STANDARD};
use rocket::{State, get, post, response::status, serde::json::Json};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tokio::sync::Mutex;
use utilities::{auth::types::NodeType, logger::error, ticket::ticket_mint::TicketMint};

#[derive(Clone, Debug, Serialize)]
pub struct IssueCertifcateResponse {
    cert: String,
    key: String,
}
#[derive(Clone, Debug, Serialize)]
pub struct IssueCertifcateErrorResponse {
    message: String,
}

#[derive(Clone, Debug, Deserialize)]
pub struct IssueCertifcateRequest {
    node_id: String,
    node_type: NodeType,
}

#[post("/issue", data = "<req>")]
pub async fn issue_cert(
    req: Json<IssueCertifcateRequest>,
    node_meta: NodeMetadataWrapper,
    ca: &State<Arc<CertificateAuthority>>,
    tm: &State<Arc<Mutex<TicketMint>>>,
    ledger: &State<Box<dyn Ledger + Send + Sync>>,
) -> Result<Json<IssueCertifcateResponse>, status::Custom<Json<IssueCertifcateErrorResponse>>> {
    let (cert, _key_pair) = match ca.issue_certificate(req.node_id.clone(), req.node_type.clone()) {
        Ok(v) => v,
        Err(e) => {
            error!("Erorr while generating the certifcate: {}", e);
            return Err(status::Custom(
                rocket::http::Status::InternalServerError,
                Json(IssueCertifcateErrorResponse {
                    message: "Error while generating certifcate".to_string(),
                }),
            ));
        }
    };
    let mut tm_locked = tm.lock().await;
    let key = match tm_locked.add_node_key(&node_meta.0.id) {
        Ok(v) => v,
        Err(e) => {
            println!("{}", e);
            return Err(status::Custom(
                rocket::http::Status::InternalServerError,
                Json(IssueCertifcateErrorResponse {
                    message: "Error while generating rsa key".to_string(),
                }),
            ));
        }
    };
    ledger.generate_key(&node_meta.0.id, &key).await;
    Ok(Json(IssueCertifcateResponse {
        cert: BASE64_STANDARD.encode(cert.der()),
        key,
    }))
}

#[derive(Clone, Debug, Serialize)]
pub struct RootCertifcateResponse {
    cert: String,
}

#[get("/")]
pub fn get_root_ca(ca: &State<Arc<CertificateAuthority>>) -> Json<RootCertifcateResponse> {
    Json(RootCertifcateResponse {
        cert: ca.get_root_cert().pem(),
    })
}
