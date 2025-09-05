use crate::server::app::AppState;
use axum::extract::State;
use axum::routing::get;
use axum::{Json, Router};
use std::sync::Arc;

pub fn create_router() -> Router<Arc<AppState>> {
    Router::new().route("/api/tags", get(get_all_tags))
}

async fn get_all_tags(State(state): State<Arc<AppState>>) -> Json<Vec<String>> {
    let store = state.store.read().unwrap();
    let tags = store.get_all_tags();
    Json(tags)
}