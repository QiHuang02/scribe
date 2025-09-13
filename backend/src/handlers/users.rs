use crate::handlers::error::{AppError, ERR_BAD_REQUEST, ERR_INTERNAL_SERVER};
use crate::models::user::User;
use crate::models::user_preferences::UserPreferences;
use crate::server::app::AppState;
use axum::extract::State;
use axum::routing::put;
use axum::{Json, Router};
use reqwest::header::USER_AGENT;
use serde::Deserialize;
use std::sync::Arc;

/// Representation of the subset of GitHub's user profile fields we care about
#[derive(Debug, Deserialize)]
pub struct GitHubProfile {
    pub id: u64,
    pub login: String,
    pub name: Option<String>,
    pub bio: Option<String>,
    #[serde(rename = "avatar_url")]
    pub avatar_url: Option<String>,
}

/// Fetches the GitHub profile of the currently authenticated user using
/// the provided OAuth access token.
pub async fn fetch_github_profile(token: &str) -> Result<GitHubProfile, AppError> {
    let profile: GitHubProfile = reqwest::Client::new()
        .get("https://api.github.com/user")
        .header(USER_AGENT, "scribe")
        .bearer_auth(token)
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
    Ok(profile)
}

/// Applies GitHub profile data to the given user, respecting any
/// overrides provided in `UserPreferences`.
pub fn apply_github_profile(
    user: &mut User,
    profile: &GitHubProfile,
    prefs: Option<&UserPreferences>,
) {
    if prefs.map_or(true, |p| p.display_name.is_none()) {
        user.display_name = profile
            .name
            .clone()
            .or_else(|| Some(profile.login.clone()));
    } else if let Some(p) = prefs {
        user.display_name = p.display_name.clone();
    }

    if prefs.map_or(true, |p| p.bio.is_none()) {
        user.bio = profile.bio.clone();
    } else if let Some(p) = prefs {
        user.bio = p.bio.clone();
    }

    if prefs.map_or(true, |p| p.avatar.is_none()) {
        user.avatar = profile.avatar_url.clone();
    } else if let Some(p) = prefs {
        user.avatar = p.avatar.clone();
    }
}

#[derive(Deserialize)]
pub struct UpdateProfileRequest {
    pub display_name: Option<String>,
    pub bio: Option<String>,
    pub website: Option<String>,
    pub theme: Option<String>,
    pub language: Option<String>,
}

const ALLOWED_THEMES: &[&str] = &["light", "dark"];
const ALLOWED_LANGUAGES: &[&str] = &["en", "zh"];

fn validate_profile(input: &UpdateProfileRequest) -> Result<(), AppError> {
    if let Some(name) = &input.display_name {
        if name.len() > 50 {
            return Err(AppError::BadRequest {
                code: ERR_BAD_REQUEST,
                message: "display_name too long".to_string(),
            });
        }
        if !name
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || c == ' ' || c == '-' || c == '_')
        {
            return Err(AppError::BadRequest {
                code: ERR_BAD_REQUEST,
                message: "display_name contains invalid characters".to_string(),
            });
        }
    }

    if let Some(bio) = &input.bio {
        if bio.len() > 160 {
            return Err(AppError::BadRequest {
                code: ERR_BAD_REQUEST,
                message: "bio too long".to_string(),
            });
        }
    }

    if let Some(website) = &input.website {
        if website.len() > 200 {
            return Err(AppError::BadRequest {
                code: ERR_BAD_REQUEST,
                message: "website too long".to_string(),
            });
        }
        if reqwest::Url::parse(website).is_err() {
            return Err(AppError::BadRequest {
                code: ERR_BAD_REQUEST,
                message: "invalid website".to_string(),
            });
        }
    }

    if let Some(theme) = &input.theme {
        if !ALLOWED_THEMES.contains(&theme.as_str()) {
            return Err(AppError::BadRequest {
                code: ERR_BAD_REQUEST,
                message: "invalid theme".to_string(),
            });
        }
    }

    if let Some(language) = &input.language {
        if !ALLOWED_LANGUAGES.contains(&language.as_str()) {
            return Err(AppError::BadRequest {
                code: ERR_BAD_REQUEST,
                message: "invalid language".to_string(),
            });
        }
    }

    Ok(())
}

async fn update_profile(
    State(_state): State<Arc<AppState>>,
    Json(payload): Json<UpdateProfileRequest>,
) -> Result<Json<UserPreferences>, AppError> {
    validate_profile(&payload)?;
    let prefs = UserPreferences {
        display_name: payload.display_name,
        bio: payload.bio,
        avatar: None,
        website: payload.website,
        theme: payload.theme,
        language: payload.language,
    };
    Ok(Json(prefs))
}

pub fn create_router() -> Router<Arc<AppState>> {
    Router::new().route("/api/users/me/profile", put(update_profile))
}
