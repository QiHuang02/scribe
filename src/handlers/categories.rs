use crate::handlers::error::AppError;
use crate::server::app::AppState;
use axum::extract::State;
use axum::response::IntoResponse;
use axum::routing::get;
use axum::{Json, Router};
use std::sync::Arc;

pub fn create_router() -> Router<Arc<AppState>> {
    Router::new().route("/api/categories", get(get_all_categories))
}

async fn get_all_categories(
    State(state): State<Arc<AppState>>,
) -> Result<impl IntoResponse, AppError> {
    let store = state.store.read().await;
    let categories = store.get_all_categories();
    Ok(Json(categories))
}
