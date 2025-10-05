use crate::api_service::controller::cert_issuer::{get_root_ca, issue_cert};
use rocket::{Route, routes};

pub fn routes() -> Vec<Route> {
    routes![issue_cert, get_root_ca]
}
