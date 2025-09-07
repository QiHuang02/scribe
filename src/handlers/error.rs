use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use axum::Json;
use serde_json::json;
use serde_yaml::Error as SerdeYAMLError;
use std::io::Error as IoError;

#[derive(Debug)]
pub enum AppError {
    NotFound(String),
    BadRequest(String),
    InternalServerError(String),
}

impl IntoResponse for AppError {
    fn into_response(self) -> Response {
        let (status, error_message) = match self {
            AppError::NotFound(msg) => (StatusCode::NOT_FOUND, msg),
            AppError::BadRequest(msg) => (StatusCode::BAD_REQUEST, msg),
            AppError::InternalServerError(msg) => (StatusCode::INTERNAL_SERVER_ERROR, msg),
        };

        let body = Json(json!({
            "error": status.canonical_reason().unwrap_or("Unknown error"),
            "message": error_message,
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
            LoadError::MissingFrontMatter(filename) => write!(f, "Missing front matter in file: {}", filename),
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