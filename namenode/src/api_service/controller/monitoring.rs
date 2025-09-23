use crate::api_service::middleware::auth::Username;
use crate::namenode_state::state_snapshot::{NamenodeStateSnapshot, SnapshotStore};
use rocket::serde::json::Json;
use rocket::{State, get};

#[get("/snapshot")]
pub async fn get_snapshot(
    _username: Username,
    store: &State<SnapshotStore>,
) -> Json<NamenodeStateSnapshot> {
    let snapshot = store.get_snapshot().await;
    Json(snapshot)
}
