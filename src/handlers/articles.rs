use crate::error::AppError;
use crate::models::article::ArticleTeaser;
use crate::AppState;
use axum::extract::{Path, Query, State};
use axum::response::{IntoResponse, Response};
use axum::routing::get;
use axum::{Json, Router};
use serde::Deserialize;
use std::sync::Arc;

#[derive(Deserialize, Debug)]
pub struct ArticleParams {
    tag: Option<String>,
    q: Option<String>,
    include_content: Option<bool>,
}

pub fn create_router() -> Router<Arc<AppState>> {
    Router::new()
        .route("/api/articles", get(get_articles_list))
        .route("/api/articles/{slug}", get(get_article_by_slug))
}

async fn get_articles_list(
    State(state): State<Arc<AppState>>,
    Query(params): Query<ArticleParams>,
) -> Response {
    let store = state.store.read().unwrap();

    let mut articles = match params.tag {
        Some(tag) => store.query(|article| {
            !article.metadata.draft && article.metadata.tags.contains(&tag)
        }),
        None => store.get_latest(state.config.latest_articles_count)
            .into_iter()
            .filter(|article| !article.metadata.draft)
            .collect(),
    };

    if let Some(query) = params.q {
        let query_lower = query.to_lowercase();
        articles.retain(|article| {
            article.metadata.title.to_lowercase().contains(&query_lower)
                || article.metadata.description.to_lowercase().contains(&query_lower)
                || article.content.to_lowercase().contains(&query_lower)
        });
    }

    if params.include_content.unwrap_or(false) {
        let articles_with_content: Vec<_> = articles.iter().map(|&article| article.clone()).collect();
        Json(articles_with_content).into_response()
    } else {
        let teasers = articles
            .iter()
            .map(|article| ArticleTeaser {
                slug: article.slug.clone(),
                metadata: article.metadata.clone(),
            })
            .collect::<Vec<_>>();
        Json(teasers).into_response()
    }
}

async fn get_article_by_slug(
    State(state): State<Arc<AppState>>,
    Path(slug): Path<String>,
) -> Result<impl IntoResponse, AppError> {
    let store = state.store.read().unwrap();
    let article = store.get_by_slug(&slug);

    match article {
        Some(article) if !article.metadata.draft => Ok(Json(article.clone())),
        Some(_) => Err(AppError::NotFound(format!(
            "Article with slug {} not found", slug
        ))),
        None => Err(AppError::NotFound(format!(
            "Article with slug {} not found", slug
        ))),
    }
}
