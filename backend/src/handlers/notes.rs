use crate::handlers::error::{AppError, ERR_BAD_REQUEST, ERR_NOTE_NOT_FOUND};
use crate::models::article::{
    ArticleContent, ArticleRepresentation, ArticleTeaser, PaginatedArticles,
};
use crate::server::app::AppState;
use axum::extract::{Path, Query, State};
use axum::response::IntoResponse;
use axum::routing::get;
use axum::{Json, Router};
use serde::Deserialize;
use std::sync::Arc;

#[derive(Deserialize, Debug)]
pub struct NoteParams {
    tag: Option<String>,
    category: Option<String>,
    q: Option<String>,
    include_content: Option<bool>,
    #[serde(default = "default_page")]
    page: usize,
    #[serde(default = "default_limit")]
    limit: usize,
}

fn default_page() -> usize {
    1
}

fn default_limit() -> usize {
    10
}

pub fn create_router() -> Router<Arc<AppState>> {
    Router::new()
        .route("/api/notes", get(get_notes_list))
        .route("/api/notes/*path", get(get_note_by_slug))
}

async fn get_notes_list(
    State(state): State<Arc<AppState>>,
    Query(params): Query<NoteParams>,
) -> Result<impl IntoResponse, AppError> {
    let store = state.note_store.read().await;

    let all_matching = {
        let mut notes = store.query(|note| !note.metadata.draft);
        if let Some(tag) = &params.tag {
            notes.retain(|a| a.metadata.tags.contains(tag));
        }
        if let Some(category) = &params.category {
            notes.retain(|a| a.metadata.category.as_ref() == Some(category));
        }
        if let Some(query) = &params.q {
            let query_lower = query.to_lowercase();
            notes.retain(|a| {
                let content = store.load_content_for(a).unwrap_or_else(|_| String::new());
                a.metadata.title.to_lowercase().contains(&query_lower)
                    || a.metadata.description.to_lowercase().contains(&query_lower)
                    || content.to_lowercase().contains(&query_lower)
            });
        }
        notes
    };

    let total_notes = all_matching.len();
    let limit = if params.limit > 0 { params.limit } else { 10 };
    let total_pages = (total_notes as f64 / limit as f64).ceil() as usize;
    let page = if params.page > 0 { params.page } else { 1 };
    let skip = (page - 1) * limit;

    let paginated = all_matching.into_iter().skip(skip).take(limit);

    let result = if params.include_content.unwrap_or(false) {
        let notes_with_content = paginated
            .map(|note| {
                let content = store
                    .load_content_for(note)
                    .unwrap_or_else(|_| String::new());
                ArticleRepresentation::Full(ArticleContent {
                    slug: note.slug_with_category(),
                    metadata: note.metadata.clone(),
                    content,
                })
            })
            .collect::<Vec<_>>();
        Json(PaginatedArticles {
            articles: notes_with_content,
            total_pages,
            current_page: page,
        })
    } else {
        let teasers = paginated
            .map(|note| {
                ArticleRepresentation::Teaser(ArticleTeaser {
                    slug: note.slug_with_category(),
                    metadata: note.metadata.clone(),
                })
            })
            .collect::<Vec<_>>();
        Json(PaginatedArticles {
            articles: teasers,
            total_pages,
            current_page: page,
        })
    };

    Ok(result)
}

async fn get_note_by_slug(
    State(state): State<Arc<AppState>>,
    Path(path): Path<String>,
) -> Result<impl IntoResponse, AppError> {
    let store = state.note_store.read().await;

    let (category, slug) = match path.rsplit_once('/') {
        Some((cat, slug)) => (Some(cat.to_string()), slug.to_string()),
        None => (None, path.clone()),
    };

    let note = store
        .query(|n| n.slug == slug && n.metadata.category.as_deref() == category.as_deref())
        .into_iter()
        .next();

    match note {
        Some(note) if !note.metadata.draft => {
            let content = store
                .load_content_for(note)
                .map_err(|e| AppError::BadRequest {
                    code: ERR_BAD_REQUEST,
                    message: e.to_string(),
                })?;
            Ok(Json(ArticleContent {
                slug: note.slug_with_category(),
                metadata: note.metadata.clone(),
                content,
            }))
        }
        _ => Err(AppError::NotFound {
            code: ERR_NOTE_NOT_FOUND,
            message: format!("Note with slug {} not found", path),
        }),
    }
}
