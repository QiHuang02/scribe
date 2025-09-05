use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use axum::Json;
use serde_json::json;
use serde_yaml::Error as SerdeYAMLError;
use std::io::Error as IoError;

#[derive(Debug)]
pub enum AppError {
    NotFound(String),
}

impl IntoResponse for AppError {
    fn into_response(self) -> Response {
        let (status, error_message) = match self {
            AppError::NotFound(msg) => (StatusCode::NOT_FOUND, msg),
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
    Io(()),
    YamlParse(()),
    MatterParse(()),
    InvalidFileName(()),
    MissingFrontMatter(()),
}

impl From<IoError> for LoadError {
    fn from(_err: IoError) -> Self {
        LoadError::Io(())
    }
}

impl From<SerdeYAMLError> for LoadError {
    fn from(_err: SerdeYAMLError) -> Self {
        LoadError::YamlParse(())
    }
}