use crate::handlers::error::{AppError, ERR_UNAUTHORIZED};
use crate::server::app::AppState;
use axum::body::Body;
use axum::extract::State;
use axum::http::{Request, header::AUTHORIZATION};
use axum::middleware::Next;
use axum::response::Response;
use std::sync::Arc;

pub async fn require_admin(
    State(state): State<Arc<AppState>>,
    req: Request<Body>,
    next: Next,
) -> Result<Response, AppError> {
    let auth_header = req
        .headers()
        .get(AUTHORIZATION)
        .and_then(|h| h.to_str().ok());

    if auth_header != Some(state.config.admin_token.as_str()) {
        return Err(AppError::Unauthorized {
            code: ERR_UNAUTHORIZED,
            message: "Invalid admin token".to_string(),
        });
    }

    Ok(next.run(req).await)
}
