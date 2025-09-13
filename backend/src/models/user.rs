use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum UserRole {
    Author,
    Visitor,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct User {
    pub github_id: u64,
    pub github_login: String,
    pub role: UserRole,
    pub display_name: Option<String>,
    pub bio: Option<String>,
    pub avatar: Option<String>,
}

impl User {
    pub fn new(github_id: u64, github_login: String, is_author: bool) -> Self {
        Self {
            github_id,
            github_login: github_login.clone(),
            role: if is_author { UserRole::Author } else { UserRole::Visitor },
            display_name: Some(github_login),
            bio: None,
            avatar: None,
        }
    }

    pub fn is_author(&self) -> bool {
        matches!(self.role, UserRole::Author)
    }

    pub fn is_visitor(&self) -> bool {
        matches!(self.role, UserRole::Visitor)
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct UserInfo {
    pub github_id: u64,
    pub github_login: String,
    pub role: UserRole,
    pub display_name: String,
    pub bio: Option<String>,
    pub avatar: Option<String>,
    pub is_author: bool,
}

impl From<User> for UserInfo {
    fn from(user: User) -> Self {
        let github_login_clone = user.github_login.clone();
        let is_author = user.is_author();
        Self {
            github_id: user.github_id,
            github_login: user.github_login,
            role: user.role,
            display_name: user.display_name.unwrap_or(github_login_clone),
            bio: user.bio,
            avatar: user.avatar,
            is_author,
        }
    }
}