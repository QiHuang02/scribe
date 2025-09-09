use crate::config::get_admin_token_hash;
use crate::handlers::error::{AppError, ERR_UNAUTHORIZED};
use axum::body::Body;
use axum::http::{Request, header::AUTHORIZATION};
use axum::middleware::Next;
use axum::response::Response;
use sha2::{Digest, Sha256};
use subtle::ConstantTimeEq;

pub async fn require_admin(req: Request<Body>, next: Next) -> Result<Response, AppError> {
    let auth_header = req
        .headers()
        .get(AUTHORIZATION)
        .and_then(|h| h.to_str().ok());

    let stored_hash = get_admin_token_hash().expect("ADMIN_TOKEN_HASH must be set");

    if let Some(token) = auth_header {
        let provided_hash: [u8; 32] = Sha256::digest(token.as_bytes()).into();
        if provided_hash.ct_eq(&stored_hash).unwrap_u8() == 1 {
            return Ok(next.run(req).await);
        }
    }

    Err(AppError::Unauthorized {
        code: ERR_UNAUTHORIZED,
        message: "Invalid admin token".to_string(),
    })
}
