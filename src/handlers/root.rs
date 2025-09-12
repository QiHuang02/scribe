use axum::{Router, routing::get};
use std::sync::Arc;

use crate::server::app::AppState;

pub fn create_router() -> Router<Arc<AppState>> {
    Router::new().route("/", get(root_handler))
}

async fn root_handler() -> &'static str {
    "Hello Scribe"
}
