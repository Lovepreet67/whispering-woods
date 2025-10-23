use rocket::{Build, Config, Rocket, fairing::AdHoc};

pub mod controller;
pub mod middleware;
pub mod routes;

use crate::{
    api_service::routes::{auth, cert_isssuer, monitoring},
    certificates::certificate_generator::CertificateAuthority,
    config::CONFIG,
    ledger::default_ledger::Ledger,
    namenode_state::state_snapshot::SnapshotStore,
};
use rocket_cors::CorsOptions;
use std::sync::Arc;
use tokio::sync::Mutex;
use utilities::{auth::AuthManager, logger::info, ticket::ticket_mint::TicketMint};

pub fn rocket(
    snapshot_store: SnapshotStore,
    ca: Arc<CertificateAuthority>,
    ticket_mint: Arc<Mutex<TicketMint>>,
    ledger: Box<dyn Ledger + Send + Sync>,
) -> Rocket<Build> {
    let cors = CorsOptions::default()
        .to_cors()
        .expect("error creating CORS fairing");
    let config = Config {
        address: "0.0.0.0".parse().unwrap(),
        port: CONFIG.api_port.unwrap_or(8080),
        ..Config::default()
    };

    info!("Starting a rocket");
    let root_cert_pem = ca.get_root_cert().pem();
    let auth_manager = AuthManager::builder()
        .upsert_jwt_token_authenticator(CONFIG.jwt_sign_key.clone())
        .upsert_cert_authenticator(&root_cert_pem);
    rocket::custom(config)
        .manage(snapshot_store)
        .manage(auth_manager)
        .manage(ca)
        .manage(ticket_mint)
        .manage(ledger)
        .mount("/monitoring", monitoring::routes())
        .mount("/auth", auth::routes())
        .mount("/cert", cert_isssuer::routes())
        .attach(cors)
        .attach(AdHoc::on_ignite("Monitoring Controller", |rocket| async {
            rocket
        }))
}
