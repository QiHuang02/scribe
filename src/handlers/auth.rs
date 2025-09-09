use crate::handlers::error::{AppError, ERR_INTERNAL_SERVER};
use crate::server::app::AppState;
use axum::extract::{Query, State};
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
        Some(ClientSecret::new(
            state.config.github_client_secret.clone(),
        )),
        AuthUrl::new("https://github.com/login/oauth/authorize".to_string()).unwrap(),
        Some(TokenUrl::new(
            "https://github.com/login/oauth/access_token".to_string(),
        )
        .unwrap()),
    )
    .set_redirect_uri(
        RedirectUrl::new(state.config.github_redirect_url.clone()).unwrap(),
    )
}

async fn github_login(State(state): State<Arc<AppState>>) -> Redirect {
    let client = oauth_client(&state);
    let (auth_url, _csrf_token) = client
        .authorize_url(CsrfToken::new_random)
        .add_scope(Scope::new("read:user".to_string()))
        .url();
    Redirect::to(auth_url.as_str())
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
    Query(query): Query<AuthRequest>,
) -> Result<Json<GitHubUser>, AppError> {
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
