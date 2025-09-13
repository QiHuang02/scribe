use crate::handlers::error::{
    AppError, ERR_ARTICLE_NOT_FOUND, ERR_INTERNAL_SERVER, ERR_VERSION_NOT_FOUND,
};
use crate::models::version::VersionRecord;
use crate::server::app::AppState;
use crate::server::auth::require_author;
use crate::services::article_service::save_version;
use axum::extract::{Path, State};
use axum::middleware;
use axum::routing::{get, post};
use axum::{Json, Router};
use chrono::{DateTime, Utc};
use std::fs;
use std::path::Path as StdPath;
use std::sync::Arc;
use std::time::SystemTime;

pub fn create_router() -> Router<Arc<AppState>> {
    Router::new()
        .route("/api/articles/{id}/versions", get(list_versions))
        .route("/api/articles/{id}/versions/{version}", get(get_version))
        .route(
            "/api/articles/{id}/versions/{version}/restore",
            post(restore_version).route_layer(middleware::from_fn(require_author)),
        )
}

async fn list_versions(
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
) -> Result<Json<Vec<VersionRecord>>, AppError> {
    let store = state.store.read().await;
    let article = store.get_by_slug(&id).ok_or_else(|| AppError::NotFound {
        code: ERR_ARTICLE_NOT_FOUND,
        message: "Article not found".to_string(),
    })?;
    if article.metadata.draft {
        return Err(AppError::NotFound {
            code: ERR_ARTICLE_NOT_FOUND,
            message: "Article not found".to_string(),
        });
    }
    let slug = article.slug.clone();
    let version_dir = format!("data/articles/{}/versions", slug);
    if !StdPath::new(&version_dir).exists() {
        return Ok(Json(vec![]));
    }
    let mut records = Vec::new();
    let entries = fs::read_dir(&version_dir).map_err(|e| AppError::InternalServerError {
        code: ERR_INTERNAL_SERVER,
        message: e.to_string(),
    })?;
    for entry in entries {
        let entry = entry.map_err(|e| AppError::InternalServerError {
            code: ERR_INTERNAL_SERVER,
            message: e.to_string(),
        })?;
        let file_name = entry.file_name();
        let file_name = file_name.to_string_lossy();
        if let Some(num_str) = file_name.strip_suffix(".md") {
            if let Ok(ver) = num_str.parse::<u64>() {
                let path = entry.path();
                let content = fs::read_to_string(&path).unwrap_or_default();
                let metadata = entry.metadata().ok();
                let modified = metadata
                    .and_then(|m| m.modified().ok())
                    .unwrap_or(SystemTime::UNIX_EPOCH);
                let timestamp: DateTime<Utc> = modified.into();
                records.push(VersionRecord {
                    article_id: slug.clone(),
                    version: ver,
                    content,
                    timestamp,
                    editor: "system".to_string(),
                });
            }
        }
    }
    records.sort_by_key(|r| r.version);
    Ok(Json(records))
}

async fn get_version(
    State(state): State<Arc<AppState>>,
    Path((id, version)): Path<(String, u64)>,
) -> Result<Json<VersionRecord>, AppError> {
    let store = state.store.read().await;
    let article = store.get_by_slug(&id).ok_or_else(|| AppError::NotFound {
        code: ERR_ARTICLE_NOT_FOUND,
        message: "Article not found".to_string(),
    })?;
    if article.metadata.draft {
        return Err(AppError::NotFound {
            code: ERR_ARTICLE_NOT_FOUND,
            message: "Article not found".to_string(),
        });
    }
    let slug = article.slug.clone();
    let path = format!("data/articles/{}/versions/{}.md", slug, version);
    let content = fs::read_to_string(&path).map_err(|_| AppError::NotFound {
        code: ERR_VERSION_NOT_FOUND,
        message: "Version not found".to_string(),
    })?;
    let metadata = fs::metadata(&path).map_err(|e| AppError::InternalServerError {
        code: ERR_INTERNAL_SERVER,
        message: e.to_string(),
    })?;
    let modified = metadata
        .modified()
        .map_err(|e| AppError::InternalServerError {
            code: ERR_INTERNAL_SERVER,
            message: e.to_string(),
        })?;
    let timestamp: DateTime<Utc> = modified.into();
    Ok(Json(VersionRecord {
        article_id: slug,
        version,
        content,
        timestamp,
        editor: "system".to_string(),
    }))
}

async fn restore_version(
    State(state): State<Arc<AppState>>,
    Path((id, version)): Path<(String, u64)>,
) -> Result<Json<VersionRecord>, AppError> {
    let store = state.store.read().await;
    let article = store.get_by_slug(&id).ok_or_else(|| AppError::NotFound {
        code: ERR_ARTICLE_NOT_FOUND,
        message: "Article not found".to_string(),
    })?;
    let version_path = format!("data/articles/{}/versions/{}.md", id, version);
    let content = fs::read_to_string(&version_path).map_err(|_| AppError::NotFound {
        code: ERR_VERSION_NOT_FOUND,
        message: "Version not found".to_string(),
    })?;
    fs::write(&article.file_path, &content).map_err(|e| AppError::InternalServerError {
        code: ERR_INTERNAL_SERVER,
        message: e.to_string(),
    })?;
    save_version(article).map_err(|e| AppError::InternalServerError {
        code: ERR_INTERNAL_SERVER,
        message: e.to_string(),
    })?;
    let timestamp = Utc::now();
    Ok(Json(VersionRecord {
        article_id: id,
        version,
        content,
        timestamp,
        editor: "system".to_string(),
    }))
}
