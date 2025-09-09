use crate::handlers::error::{AppError, ERR_BAD_REQUEST, ERR_INTERNAL_SERVER};
use crate::server::app::AppState;
use axum::extract::{Query, State};
use axum::http::{
    HeaderMap, HeaderValue,
    header::{COOKIE, SET_COOKIE},
};
use axum::response::Redirect;
use axum::routing::get;
use axum::{Json, Router};
use oauth2::basic::BasicClient;
use oauth2::reqwest::async_http_client;
use oauth2::{
    AuthUrl, AuthorizationCode, ClientId, ClientSecret, CsrfToken, RedirectUrl, Scope,
    TokenResponse, TokenUrl,
};
use reqwest::header::USER_AGENT;
use serde::{Deserialize, Serialize};
use std::sync::Arc;

pub fn create_router() -> Router<Arc<AppState>> {
    Router::new()
        .route("/api/auth/github/login", get(github_login))
        .route("/api/auth/github/callback", get(github_callback))
}

fn oauth_client(state: &AppState) -> BasicClient {
    BasicClient::new(
        ClientId::new(state.config.github_client_id.clone()),
        Some(ClientSecret::new(state.config.github_client_secret.clone())),
        AuthUrl::new("https://github.com/login/oauth/authorize".to_string()).unwrap(),
        Some(TokenUrl::new("https://github.com/login/oauth/access_token".to_string()).unwrap()),
    )
    .set_redirect_uri(RedirectUrl::new(state.config.github_redirect_url.clone()).unwrap())
}

async fn github_login(State(state): State<Arc<AppState>>) -> impl axum::response::IntoResponse {
    let client = oauth_client(&state);
    let (auth_url, csrf_token) = client
        .authorize_url(CsrfToken::new_random)
        .add_scope(Scope::new("read:user".to_string()))
        .url();

    let mut headers = HeaderMap::new();
    let cookie = format!(
        "github_oauth_state={}; HttpOnly; SameSite=Lax",
        csrf_token.secret()
    );
    headers.insert(SET_COOKIE, HeaderValue::from_str(&cookie).unwrap());

    (headers, Redirect::to(auth_url.as_str()))
}

#[derive(Deserialize)]
struct AuthRequest {
    code: String,
    state: String,
}

#[derive(Serialize, Deserialize)]
struct GitHubUser {
    id: u64,
    login: String,
}

async fn github_callback(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Query(query): Query<AuthRequest>,
) -> Result<Json<GitHubUser>, AppError> {
    let state_cookie = headers
        .get(COOKIE)
        .and_then(|value| value.to_str().ok())
        .and_then(|cookie_header| {
            cookie_header.split(';').find_map(|cookie| {
                let cookie = cookie.trim();
                cookie
                    .strip_prefix("github_oauth_state=")
                    .map(|v| v.to_string())
            })
        })
        .ok_or_else(|| AppError::BadRequest {
            code: ERR_BAD_REQUEST,
            message: "missing OAuth state".to_string(),
        })?;
    if state_cookie != query.state {
        return Err(AppError::BadRequest {
            code: ERR_BAD_REQUEST,
            message: "invalid OAuth state".to_string(),
        });
    }

    let client = oauth_client(&state);
    let token = client
        .exchange_code(AuthorizationCode::new(query.code))
        .request_async(async_http_client)
        .await
        .map_err(|e| AppError::InternalServerError {
            code: ERR_INTERNAL_SERVER,
            message: e.to_string(),
        })?;

    let user: GitHubUser = reqwest::Client::new()
        .get("https://api.github.com/user")
        .header(USER_AGENT, "scribe")
        .bearer_auth(token.access_token().secret())
        .send()
        .await
        .map_err(|e| AppError::InternalServerError {
            code: ERR_INTERNAL_SERVER,
            message: e.to_string(),
        })?
        .json()
        .await
        .map_err(|e| AppError::InternalServerError {
            code: ERR_INTERNAL_SERVER,
            message: e.to_string(),
        })?;

    Ok(Json(user))
}
