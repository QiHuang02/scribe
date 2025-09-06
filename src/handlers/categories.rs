use crate::server::app::AppState;
use axum::extract::State;
use axum::routing::get;
use axum::{Json, Router};
use std::sync::Arc;

pub fn create_router() -> Router<Arc<AppState>> {
    Router::new().route("/api/categories", get(get_all_categories))
}

async fn get_all_categories(State(state): State<Arc<AppState>>) -> Json<Vec<String>> {
    let store = state.store.read().unwrap();
    let categories = store.get_all_categories();
    Json(categories)
}