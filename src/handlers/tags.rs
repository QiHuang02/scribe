use crate::handlers::error::AppError;
use crate::server::app::AppState;
use axum::extract::State;
use axum::response::IntoResponse;
use axum::routing::get;
use axum::{Json, Router};
use std::sync::Arc;

pub fn create_router() -> Router<Arc<AppState>> {
    Router::new().route("/api/tags", get(get_all_tags))
}

async fn get_all_tags(State(state): State<Arc<AppState>>) -> Result<impl IntoResponse, AppError> {
    let store = state.store.read()
        .map_err(|_| AppError::BadRequest("Failed to acquire store lock".to_string()))?;
    let tags = store.get_all_tags();
    Ok(Json(tags))
}