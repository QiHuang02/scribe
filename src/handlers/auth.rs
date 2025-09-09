use crate::handlers::error::{AppError, ERR_INTERNAL_SERVER, ERR_UNAUTHORIZED};
use crate::server::app::AppState;
use axum::extract::{Query, State};
use axum::response::Redirect;
use axum::routing::get;
use axum::{Json, Router};
use axum_extra::extract::cookie::{Cookie, CookieJar, SameSite};
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

async fn github_login(State(state): State<Arc<AppState>>, jar: CookieJar) -> (CookieJar, Redirect) {
    let client = oauth_client(&state);
    let (auth_url, csrf_token) = client
        .authorize_url(CsrfToken::new_random)
        .add_scope(Scope::new("read:user".to_string()))
        .url();

    let is_secure_cookie = state.config.github_redirect_url.starts_with("https://");

    let jar = jar.add(
        Cookie::build(("oauth_state", csrf_token.secret().to_string()))
            .http_only(true)
            .same_site(SameSite::Lax)
            .secure(is_secure_cookie)
            .build(),
    );

    (jar, Redirect::to(auth_url.as_str()))
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
    jar: CookieJar,
    Query(query): Query<AuthRequest>,
) -> Result<(CookieJar, Json<GitHubUser>), AppError> {
    let state_cookie = jar.get("oauth_state").ok_or(AppError::Unauthorized {
        code: ERR_UNAUTHORIZED,
        message: "missing oauth state".to_string(),
    })?;

    if state_cookie.value() != query.state {
        return Err(AppError::Unauthorized {
            code: ERR_UNAUTHORIZED,
            message: "invalid oauth state".to_string(),
        });
    }

    let jar = jar.remove(Cookie::from("oauth_state"));

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

    Ok((jar, Json(user)))
}
