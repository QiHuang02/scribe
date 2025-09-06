use crate::handlers::error::AppError;
use crate::models::article::{ArticleRepresentation, ArticleTeaser, PaginatedArticles};
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
    let store = state.store.read()
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
            // 如果启用了全文搜索，使用搜索服务
            if let Some(ref search_service) = state.search_service {
                // 使用全文搜索
                match search_service.search(query, 1000, false) {
                    Ok(search_results) => {
                        // 从搜索结果中获取对应的文章
                        let search_slugs: std::collections::HashSet<String> =
                            search_results.into_iter().map(|r| r.slug).collect();
                        articles.retain(|a| search_slugs.contains(&a.slug));
                    }
                    Err(e) => {
                        tracing::warn!("Full-text search failed, falling back to simple search: {:?}", e);
                        // 回退到简单搜索
                        let query_lower = query.to_lowercase();
                        articles.retain(|a| {
                            a.metadata.title.to_lowercase().contains(&query_lower)
                                || a.metadata.description.to_lowercase().contains(&query_lower)
                                || a.content.to_lowercase().contains(&query_lower)
                        });
                    }
                }
            } else {
                // 使用传统的简单搜索
                let query_lower = query.to_lowercase();
                articles.retain(|a| {
                    let content_to_search = if a.content.len() > state.config.content_search_limit {
                        &a.content[..state.config.content_search_limit]
                    } else {
                        &a.content
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
            .map(|article| ArticleRepresentation::Full(article.clone()))
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
    let store = state.store.read()
        .map_err(|_| AppError::BadRequest("Failed to acquire store lock".to_string()))?;
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
