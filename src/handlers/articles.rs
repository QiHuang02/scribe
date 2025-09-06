use crate::handlers::error::AppError;
use crate::models::article::{ArticleContent, ArticleRepresentation, ArticleTeaser, PaginatedArticles};
use crate::server::app::AppState;
use axum::extract::{Path, Query, State};
use axum::response::IntoResponse;
use axum::routing::get;
use axum::{Json, Router};
use serde::Deserialize;
use std::sync::Arc;

#[derive(Deserialize, Debug)]
pub struct ArticleParams {
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
        .route("/api/articles", get(get_articles_list))
        .route("/api/articles/{slug}", get(get_article_by_slug))
}

async fn get_articles_list(
    State(state): State<Arc<AppState>>,
    Query(params): Query<ArticleParams>,
) -> Result<impl IntoResponse, AppError> {
    let store = state
        .store
        .read()
        .map_err(|_| AppError::BadRequest("Failed to acquire store lock".to_string()))?;

    let all_matching_articles = {
        let mut articles = store.query(|article| !article.metadata.draft);
        if let Some(tag) = &params.tag {
            articles.retain(|a| a.metadata.tags.contains(tag));
        }
        if let Some(category) = &params.category {
            articles.retain(|a| {
                a.metadata.category.as_ref() == Some(category)
            });
        }
        if let Some(query) = &params.q {
            if let Some(ref search_service) = state.search_service {
                match search_service.search(query, 1000, false) {
                    Ok(search_results) => {
                        let search_slugs: std::collections::HashSet<String> =
                            search_results.into_iter().map(|r| r.slug).collect();
                        articles.retain(|a| search_slugs.contains(&a.slug));
                    }
                    Err(e) => {
                        tracing::warn!(
                            "Full-text search failed, falling back to simple search: {:?}",
                            e
                        );
                        let query_lower = query.to_lowercase();
                        articles.retain(|a| {
                            let content =
                                store.load_content_for(a).unwrap_or_else(|_| String::new());
                            a.metadata.title.to_lowercase().contains(&query_lower)
                                || a.metadata.description.to_lowercase().contains(&query_lower)
                                || content.to_lowercase().contains(&query_lower)
                        });
                    }
                }
            } else {
                let query_lower = query.to_lowercase();
                articles.retain(|a| {
                    let content =
                        store.load_content_for(a).unwrap_or_else(|_| String::new());
                    let content_to_search = if content.len() > state.config.content_search_limit {
                        &content[..state.config.content_search_limit]
                    } else {
                        &content
                    };

                    a.metadata.title.to_lowercase().contains(&query_lower)
                        || a.metadata.description.to_lowercase().contains(&query_lower)
                        || content_to_search.to_lowercase().contains(&query_lower)
                });
            }
        }
        articles
    };

    let total_articles = all_matching_articles.len();
    let limit = if params.limit > 0 { params.limit } else { 10 };
    let total_pages = (total_articles as f64 / limit as f64).ceil() as usize;
    let page = if params.page > 0 { params.page } else { 1 };
    let skip = (page - 1) * limit;

    let paginated_articles = all_matching_articles.into_iter().skip(skip).take(limit);

    let result = if params.include_content.unwrap_or(false) {
        let articles_with_content = paginated_articles
            .map(|article| {
                let content = store
                    .load_content_for(article)
                    .unwrap_or_else(|_| String::new());
                ArticleRepresentation::Full(ArticleContent {
                    slug: article.slug.clone(),
                    metadata: article.metadata.clone(),
                    content,
                })
            })
            .collect::<Vec<_>>();
        Json(PaginatedArticles {
            articles: articles_with_content,
            total_pages,
            current_page: page,
        })
    } else {
        let teasers = paginated_articles
            .map(|article| {
                ArticleRepresentation::Teaser(ArticleTeaser {
                    slug: article.slug.clone(),
                    metadata: article.metadata.clone(),
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

async fn get_article_by_slug(
    State(state): State<Arc<AppState>>,
    Path(slug): Path<String>,
) -> Result<impl IntoResponse, AppError> {
    let store = state
        .store
        .read()
        .map_err(|_| AppError::BadRequest("Failed to acquire store lock".to_string()))?;
    let article = store.get_by_slug(&slug);

    match article {
        Some(article) if !article.metadata.draft => {
            let content = store
                .load_content_for(article)
                .map_err(|e| AppError::BadRequest(e.to_string()))?;
            Ok(Json(ArticleContent {
                slug: article.slug.clone(),
                metadata: article.metadata.clone(),
                content,
            }))
        }
        Some(_) => Err(AppError::NotFound(format!(
            "Article with slug {} not found", slug
        ))),
        None => Err(AppError::NotFound(format!(
            "Article with slug {} not found", slug
        ))),
    }
}
