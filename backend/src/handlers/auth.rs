use crate::config::{get_author_github_username, get_github_client_id, get_github_client_secret};
use crate::handlers::error::{AppError, ERR_INTERNAL_SERVER, ERR_UNAUTHORIZED};
use crate::handlers::users::{apply_github_profile, fetch_github_profile};
use crate::models::user::{User, UserInfo};
use crate::models::user_preferences::UserPreferences;
use crate::server::app::AppState;
use axum::extract::{FromRef, Query, State};
use axum::response::Redirect;
use axum::routing::get;
use axum::{Json, Router};
use axum_extra::extract::cookie::{Cookie, SameSite, SignedCookieJar};
use cookie::Key;
use oauth2::basic::BasicClient;
use oauth2::reqwest::async_http_client;
use oauth2::{
    AuthUrl, AuthorizationCode, ClientId, ClientSecret, CsrfToken, RedirectUrl, Scope,
    TokenResponse, TokenUrl,
};
use serde::Deserialize;
use std::sync::Arc;

#[derive(Clone)]
struct CookieKey(Key);

impl FromRef<Arc<AppState>> for CookieKey {
    fn from_ref(app: &Arc<AppState>) -> Self {
        CookieKey(app.cookie_key.clone())
    }
}

impl Into<Key> for CookieKey {
    fn into(self) -> Key {
        self.0
    }
}

type SignedJar = SignedCookieJar<CookieKey>;

pub fn create_router() -> Router<Arc<AppState>> {
    Router::new()
        .route("/api/auth/github/login", get(github_login))
        .route("/api/auth/github/callback", get(github_callback))
        .route("/api/auth/me", get(get_current_user))
}

fn oauth_client(state: &AppState) -> BasicClient {
    let client_id = get_github_client_id().expect("GITHUB_CLIENT_ID must be set");
    let client_secret = get_github_client_secret().expect("GITHUB_CLIENT_SECRET must be set");
    BasicClient::new(
        ClientId::new(client_id),
        Some(ClientSecret::new(client_secret)),
        AuthUrl::new("https://github.com/login/oauth/authorize".to_string()).unwrap(),
        Some(TokenUrl::new("https://github.com/login/oauth/access_token".to_string()).unwrap()),
    )
    .set_redirect_uri(RedirectUrl::new(state.config.github_redirect_url.clone()).unwrap())
}

async fn github_login(State(state): State<Arc<AppState>>, jar: SignedJar) -> (SignedJar, Redirect) {
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

async fn github_callback(
    State(state): State<Arc<AppState>>,
    jar: SignedJar,
    Query(query): Query<AuthRequest>,
) -> Result<(SignedJar, Redirect), AppError> {
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

    let profile = fetch_github_profile(token.access_token().secret()).await?;

    // Determine if user is the author
    let author_username = get_author_github_username().unwrap_or_default();
    let is_author = profile.login == author_username;

    let mut user = User::new(profile.id, profile.login.clone(), is_author);

    // In a real application, preferences would be loaded from storage
    let prefs = UserPreferences::default();
    apply_github_profile(&mut user, &profile, Some(&prefs));

    // Create signed cookie with user info
    let user_json = serde_json::to_string(&user).map_err(|e| AppError::InternalServerError {
        code: ERR_INTERNAL_SERVER,
        message: e.to_string(),
    })?;

    let is_secure_cookie = state.config.github_redirect_url.starts_with("https://");
    let jar = jar.add(
        Cookie::build(("user_session", user_json))
            .http_only(true)
            .same_site(SameSite::Lax)
            .secure(is_secure_cookie)
            .max_age(cookie::time::Duration::days(30))
            .build(),
    );

    Ok((jar, Redirect::to("http://localhost:8080/author")))
}

async fn get_current_user(jar: SignedJar) -> Result<Json<UserInfo>, AppError> {
    let user_cookie = jar.get("user_session").ok_or(AppError::Unauthorized {
        code: ERR_UNAUTHORIZED,
        message: "Not authenticated".to_string(),
    })?;

    let user: User = serde_json::from_str(user_cookie.value()).map_err(|_| AppError::Unauthorized {
        code: ERR_UNAUTHORIZED,
        message: "Invalid session".to_string(),
    })?;

    Ok(Json(UserInfo::from(user)))
}
