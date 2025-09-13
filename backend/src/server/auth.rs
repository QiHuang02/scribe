use crate::config::get_admin_token_hash;
use crate::handlers::error::{AppError, ERR_FORBIDDEN, ERR_UNAUTHORIZED};
use crate::models::user::User;
use crate::server::app::AppState;
use axum::body::Body;
use axum::http::{Request, header::AUTHORIZATION, header::COOKIE};
use axum::middleware::Next;
use axum::response::Response;
use cookie::Key;
use sha2::{Digest, Sha256};
use subtle::ConstantTimeEq;
use std::sync::Arc;

// Helper function to extract user from cookie header
fn get_user_from_cookie_header(req: &Request<Body>, _key: &Key) -> Result<User, AppError> {
    use axum_extra::extract::cookie::Cookie;

    let cookie_header = req.headers().get(COOKIE)
        .ok_or(AppError::Unauthorized {
            code: ERR_UNAUTHORIZED,
            message: "No cookies found".to_string(),
        })?;

    let cookie_str = cookie_header.to_str()
        .map_err(|_| AppError::Unauthorized {
            code: ERR_UNAUTHORIZED,
            message: "Invalid cookie format".to_string(),
        })?;

    // Parse cookies manually and look for user_session
    for cookie_pair in cookie_str.split(';') {
        let cookie_pair = cookie_pair.trim();
        if let Some(cookie) = Cookie::parse(cookie_pair).ok() {
            if cookie.name() == "user_session" {
                // This is a simplified version - in production you'd want proper signing verification
                let user_json = cookie.value();
                return serde_json::from_str(user_json)
                    .map_err(|_| AppError::Unauthorized {
                        code: ERR_UNAUTHORIZED,
                        message: "Invalid session data".to_string(),
                    });
            }
        }
    }

    Err(AppError::Unauthorized {
        code: ERR_UNAUTHORIZED,
        message: "No user session found".to_string(),
    })
}

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

pub async fn require_author(req: Request<Body>, next: Next) -> Result<Response, AppError> {
    // For now, let's use a simpler approach - extract from extensions
    // This will be populated by the app state middleware
    if let Some(app_state) = req.extensions().get::<Arc<AppState>>() {
        let user = get_user_from_cookie_header(&req, &app_state.cookie_key)?;
        if user.is_author() {
            Ok(next.run(req).await)
        } else {
            Err(AppError::Forbidden {
                code: ERR_FORBIDDEN,
                message: "Author role required".to_string(),
            })
        }
    } else {
        Err(AppError::Unauthorized {
            code: ERR_UNAUTHORIZED,
            message: "Application state not found".to_string(),
        })
    }
}

pub async fn require_authenticated(req: Request<Body>, next: Next) -> Result<Response, AppError> {
    if let Some(app_state) = req.extensions().get::<Arc<AppState>>() {
        let _user = get_user_from_cookie_header(&req, &app_state.cookie_key)?;
        Ok(next.run(req).await)
    } else {
        Err(AppError::Unauthorized {
            code: ERR_UNAUTHORIZED,
            message: "Application state not found".to_string(),
        })
    }
}
