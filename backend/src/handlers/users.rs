use crate::handlers::error::{AppError, ERR_INTERNAL_SERVER};
use crate::models::user::User;
use crate::models::user_preferences::UserPreferences;
use reqwest::header::USER_AGENT;
use serde::Deserialize;

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
