use crate::api_service::controller::monitoring;
use rocket::{Route, routes};

pub fn routes() -> Vec<Route> {
    routes![monitoring::get_snapshot,]
}
