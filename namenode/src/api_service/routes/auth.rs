use crate::api_service::controller::auth;
use rocket::{Route, routes};

pub fn routes() -> Vec<Route> {
    routes![auth::login]
}
