use rocket::{Build, Config, Rocket, catch, fairing::AdHoc};

pub mod controller;
pub mod middleware;
pub mod routes;

use crate::{
    api_service::routes::{auth, monitoring},
    config::CONFIG,
    namenode_state::state_snapshot::SnapshotStore,
};
use rocket_cors::CorsOptions;

pub fn rocket(snapshot_store: SnapshotStore) -> Rocket<Build> {
    let cors = CorsOptions::default()
        .to_cors()
        .expect("error creating CORS fairing");
    let config = Config {
        address: "0.0.0.0".parse().unwrap(),
        port: CONFIG.api_port.unwrap_or(8080),
        ..Config::default()
    };
    rocket::custom(config)
        .manage(snapshot_store)
        .mount("/monitoring", monitoring::routes())
        .mount("/auth", auth::routes())
        .attach(cors)
        .attach(AdHoc::on_ignite("Monitoring Controller", |rocket| async {
            rocket
        }))
}
