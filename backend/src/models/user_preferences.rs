use serde::{Deserialize, Serialize};

/// UserPreferences stores optional overrides for profile fields
/// that may differ from data provided by GitHub. When a field is
/// `Some`, the application should respect that value instead of
/// refreshing it from GitHub.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct UserPreferences {
    pub display_name: Option<String>,
    pub bio: Option<String>,
    pub avatar: Option<String>,
    pub website: Option<String>,
    pub theme: Option<String>,
    pub language: Option<String>,
}
