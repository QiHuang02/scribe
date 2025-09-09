use crate::server::app::AppState;
use axum::{Router, routing::get};
use std::sync::Arc;

pub fn create_router() -> Router<Arc<AppState>> {
    Router::new().route("/api/comments", get(not_implemented))
}

async fn not_implemented() -> &'static str {
    "Comments feature not implemented"
}
