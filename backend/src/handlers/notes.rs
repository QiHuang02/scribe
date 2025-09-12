use crate::handlers::error::{AppError, ERR_BAD_REQUEST, ERR_NOTE_NOT_FOUND};
use crate::models::article::{
    Article, ArticleContent, ArticleRepresentation, ArticleTeaser, PaginatedArticles,
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
        .route("/api/notes/{path}", get(get_note_by_slug))
}

async fn get_notes_list(
    State(state): State<Arc<AppState>>,
    Query(params): Query<NoteParams>,
) -> Result<impl IntoResponse, AppError> {
    let store = state.note_store.read().await;
    let limit = if params.limit > 0 { params.limit } else { 10 };
    let page = if params.page > 0 { params.page } else { 1 };
    let offset = (page - 1) * limit;

    let tag = params.tag.clone();
    let category = params.category.clone();
    let query_lower = params.q.clone().map(|q| q.to_lowercase());

    let tag1 = tag.clone();
    let category1 = category.clone();
    let query1 = query_lower.clone();
    let filter = move |a: &Article| {
        if a.metadata.draft {
            return false;
        }
        if let Some(ref t) = tag1 {
            if !a.metadata.tags.contains(t) {
                return false;
            }
        }
        if let Some(ref c) = category1 {
            if a.metadata.category.as_ref() != Some(c) {
                return false;
            }
        }
        if let Some(ref ql) = query1 {
            a.metadata.title.to_lowercase().contains(ql)
                || a.metadata.description.to_lowercase().contains(ql)
        } else {
            true
        }
    };

    let paginated_vec: Vec<&Article> = store.query(filter, offset, limit).collect();

    let tag2 = tag;
    let category2 = category;
    let query2 = query_lower;
    let filter_total = move |a: &Article| {
        if a.metadata.draft {
            return false;
        }
        if let Some(ref t) = tag2 {
            if !a.metadata.tags.contains(t) {
                return false;
            }
        }
        if let Some(ref c) = category2 {
            if a.metadata.category.as_ref() != Some(c) {
                return false;
            }
        }
        if let Some(ref ql) = query2 {
            a.metadata.title.to_lowercase().contains(ql)
                || a.metadata.description.to_lowercase().contains(ql)
        } else {
            true
        }
    };
    let total_notes = store.query(filter_total, 0, usize::MAX).count();
    let total_pages = (total_notes as f64 / limit as f64).ceil() as usize;

    let paginated = paginated_vec.into_iter();

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
        .query(
            |n| n.slug == slug && n.metadata.category.as_deref() == category.as_deref(),
            0,
            usize::MAX,
        )
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
