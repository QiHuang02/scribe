use std::sync::Arc;
use axum::extract::State;
use axum::{Json, Router};
use axum::routing::get;
use crate::AppState;
use crate::models::article::Article;

pub fn create_router() -> Router<Arc<AppState>> {
    Router::new().route("/api/tags", get(get_all_tags))
}

async fn get_all_tags(State(state): State<Arc<AppState>>) -> Json<Vec<String>> {
    let store = state.store.read().unwrap();
    let tags = store.get_all_tags();
    Json(tags)
}