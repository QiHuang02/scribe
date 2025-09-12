use crate::config::get_admin_token_hash;
use crate::handlers::error::{AppError, ERR_FORBIDDEN, ERR_UNAUTHORIZED};
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

    let token = match auth_header {
        Some(t) => t,
        None => {
            return Err(AppError::Unauthorized {
                code: ERR_UNAUTHORIZED,
                message: "Missing authorization token".to_string(),
            });
        }
    };

    let stored_hash = get_admin_token_hash().expect("ADMIN_TOKEN_HASH must be set");
    let provided_hash: [u8; 32] = Sha256::digest(token.as_bytes()).into();

    if provided_hash.ct_eq(&stored_hash).unwrap_u8() == 1 {
        Ok(next.run(req).await)
    } else {
        Err(AppError::Forbidden {
            code: ERR_FORBIDDEN,
            message: "Invalid admin token".to_string(),
        })
    }
}
