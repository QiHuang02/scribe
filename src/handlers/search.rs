use crate::handlers::error::AppError;
use crate::server::app::AppState;
use crate::services::search::SearchResult;
use axum::extract::{Query, State};
use axum::response::IntoResponse;
use axum::routing::get;
use axum::{Json, Router};
use serde::{Deserialize, Serialize};
use std::sync::Arc;

#[derive(Deserialize, Debug)]
pub struct SearchParams {
    pub q: String,
    pub limit: Option<usize>,
    pub highlights: Option<bool>,
}

#[derive(Serialize)]
pub struct SearchResponse {
    pub results: Vec<SearchResult>,
    pub query: String,
    pub total_found: usize,
}

#[derive(Serialize)]
pub struct PopularSearchResponse {
    pub searches: Vec<PopularSearch>,
}

#[derive(Serialize)]
pub struct PopularSearch {
    pub query: String,
    pub count: usize,
}

pub fn create_router() -> Router<Arc<AppState>> {
    Router::new()
        .route("/api/search", get(search_articles))
        .route("/api/search/popular", get(get_popular_searches))
}

async fn search_articles(
    State(state): State<Arc<AppState>>,
    Query(params): Query<SearchParams>,
) -> Result<impl IntoResponse, AppError> {
    // 检查是否启用了搜索服务
    let search_service = state.search_service.as_ref().ok_or_else(|| {
        AppError::BadRequest("Full-text search is not enabled".to_string())
    })?;

    let limit = params.limit.unwrap_or(20);
    let highlights = params.highlights.unwrap_or(true);

    // 如果查询为空，回退到传统搜索
    if params.q.trim().is_empty() {
        return Err(AppError::BadRequest("Search query cannot be empty".to_string()));
    }

    match search_service.search(&params.q, limit, highlights) {
        Ok(results) => {
            let response = SearchResponse {
                total_found: results.len(),
                query: params.q,
                results,
            };
            Ok(Json(response))
        }
        Err(e) => {
            tracing::error!("Search error: {:?}", e);

            // 回退到传统搜索
            let store = state.store.read().unwrap();
            let query_lower = params.q.to_lowercase();
            let articles = store.query(|article| {
                !article.metadata.draft && (
                    article.metadata.title.to_lowercase().contains(&query_lower)
                        || article.metadata.description.to_lowercase().contains(&query_lower)
                        || article.content.to_lowercase().contains(&query_lower)
                )
            });

            let fallback_results: Vec<SearchResult> = articles
                .into_iter()
                .take(limit)
                .map(|article| SearchResult {
                    slug: article.slug.clone(),
                    title: article.metadata.title.clone(),
                    description: article.metadata.description.clone(),
                    score: 1.0, // 默认评分
                    highlights: None,
                })
                .collect();

            let response = SearchResponse {
                total_found: fallback_results.len(),
                query: params.q,
                results: fallback_results,
            };
            Ok(Json(response))
        }
    }
}

async fn get_popular_searches(
    State(state): State<Arc<AppState>>,
) -> Result<impl IntoResponse, AppError> {
    let search_service = state.search_service.as_ref().ok_or_else(|| {
        AppError::BadRequest("Full-text search is not enabled".to_string())
    })?;

    let popular_searches = search_service.get_popular_searches(10);
    let searches: Vec<PopularSearch> = popular_searches
        .into_iter()
        .map(|(query, count)| PopularSearch { query, count })
        .collect();

    Ok(Json(PopularSearchResponse { searches }))
}