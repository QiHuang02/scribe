use axum::Json;
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use serde_json::json;
use serde_yaml::Error as SerdeYAMLError;
use std::io::Error as IoError;
use tracing::error;

pub const ERR_INTERNAL_SERVER: &str = "ERR_INTERNAL_SERVER";
pub const ERR_BAD_REQUEST: &str = "ERR_BAD_REQUEST";
pub const ERR_ARTICLE_NOT_FOUND: &str = "ERR_ARTICLE_NOT_FOUND";
pub const ERR_VERSION_NOT_FOUND: &str = "ERR_VERSION_NOT_FOUND";
pub const ERR_FULLTEXT_DISABLED: &str = "ERR_FULLTEXT_DISABLED";
pub const ERR_EMPTY_SEARCH_QUERY: &str = "ERR_EMPTY_SEARCH_QUERY";
#[allow(dead_code)]
pub const ERR_INVALID_SESSION: &str = "ERR_INVALID_SESSION";

#[derive(Debug)]
pub enum AppError {
    NotFound { code: &'static str, message: String },
    BadRequest { code: &'static str, message: String },
    InternalServerError { code: &'static str, message: String },
}

impl IntoResponse for AppError {
    fn into_response(self) -> Response {
        let (status, code, message) = match self {
            AppError::NotFound { code, message } => (StatusCode::NOT_FOUND, code, message),
            AppError::BadRequest { code, message } => (StatusCode::BAD_REQUEST, code, message),
            AppError::InternalServerError { code, message } => {
                (StatusCode::INTERNAL_SERVER_ERROR, code, message)
            }
        };

        error!(error_code = code, message = %message);

        let body = Json(json!({
            "error_code": code,
            "message": message,
        }));

        (status, body).into_response()
    }
}

#[derive(Debug)]
pub enum LoadError {
    Io(IoError),
    YamlParse(SerdeYAMLError),
    MatterParse(String),
    InvalidFileName(String),
    MissingFrontMatter(String),
}

impl std::fmt::Display for LoadError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            LoadError::Io(err) => write!(f, "IO error: {}", err),
            LoadError::YamlParse(err) => write!(f, "YAML parsing error: {}", err),
            LoadError::MatterParse(msg) => write!(f, "Front matter parsing error: {}", msg),
            LoadError::InvalidFileName(filename) => write!(f, "Invalid file name: {}", filename),
            LoadError::MissingFrontMatter(filename) => {
                write!(f, "Missing front matter in file: {}", filename)
            }
        }
    }
}

impl std::error::Error for LoadError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            LoadError::Io(err) => Some(err),
            LoadError::YamlParse(err) => Some(err),
            _ => None,
        }
    }
}

impl From<IoError> for LoadError {
    fn from(err: IoError) -> Self {
        LoadError::Io(err)
    }
}

impl From<SerdeYAMLError> for LoadError {
    fn from(err: SerdeYAMLError) -> Self {
        LoadError::YamlParse(err)
    }
}
