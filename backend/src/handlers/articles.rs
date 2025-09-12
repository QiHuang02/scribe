use crate::config::{ARTICLE_DIR, ENABLE_NESTED_CATEGORIES};
use crate::handlers::error::{
    AppError, ERR_ARTICLE_NOT_FOUND, ERR_BAD_REQUEST, ERR_INTERNAL_SERVER,
};
use crate::models::article::{
    Article, ArticleContent, ArticleRepresentation, ArticleTeaser, Metadata, PaginatedArticles,
};
use crate::server::app::{AppState, reindex_all_content};
use crate::server::auth::require_admin;
use crate::services::article_service::save_version;
use crate::services::service::ArticleStore;
use axum::extract::{Path, Query, State};
use axum::middleware;
use axum::response::IntoResponse;
use axum::routing::{get, post, put};
use axum::{Json, Router};
use chrono::Utc;
use serde::Deserialize;
use serde_json::json;
use slug::slugify;
use std::fs;
use std::path::Path as StdPath;
use std::sync::Arc;
use std::time::SystemTime;

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

#[derive(Deserialize, Debug)]
pub struct CreateArticleRequest {
    pub title: String,
    pub content: String,
    pub tags: Option<Vec<String>>,
    pub category: Option<String>,
    pub description: Option<String>,
    pub draft: Option<bool>,
}

#[derive(Deserialize, Debug)]
pub struct UpdateArticleRequest {
    pub title: String,
    pub content: String,
    pub tags: Option<Vec<String>>,
    pub category: Option<String>,
    pub description: Option<String>,
    pub draft: Option<bool>,
}

fn default_page() -> usize {
    1
}

fn default_limit() -> usize {
    10
}

fn write_article_to_file(
    metadata: &Metadata,
    content: &str,
    file_path: &StdPath,
) -> Result<(), AppError> {
    if let Some(parent) = file_path.parent() {
        fs::create_dir_all(parent).map_err(|e| AppError::InternalServerError {
            code: ERR_INTERNAL_SERVER,
            message: e.to_string(),
        })?;
    }

    let front_matter =
        serde_yaml::to_string(metadata).map_err(|e| AppError::InternalServerError {
            code: ERR_INTERNAL_SERVER,
            message: e.to_string(),
        })?;
    let file_content = format!("---\n{}---\n\n{}", front_matter, content);
    fs::write(file_path, file_content).map_err(|e| AppError::InternalServerError {
        code: ERR_INTERNAL_SERVER,
        message: e.to_string(),
    })?;
    Ok(())
}

pub fn create_router() -> Router<Arc<AppState>> {
    Router::new()
        .route("/api/articles", get(get_articles_list))
        .route(
            "/api/articles",
            post(create_article).route_layer(middleware::from_fn(require_admin)),
        )
        .route("/api/articles/{slug}", get(get_article_by_slug))
        .route(
            "/api/articles/{slug}",
            put(update_article).route_layer(middleware::from_fn(require_admin)),
        )
}

async fn filter_articles<'a>(
    store: &'a ArticleStore,
    params: &ArticleParams,
    state: &AppState,
    offset: usize,
    limit: usize,
) -> (Vec<&'a Article>, usize) {
    let tag = params.tag.clone();
    let category = params.category.clone();
    let query = params.q.clone();

    let search_slugs = if let Some(ref q) = query {
        if let Some(ref search_service) = state.search_service {
            match search_service.search(q, 1000, false).await {
                Ok(search_results) => Some(
                    search_results
                        .into_iter()
                        .map(|r| r.slug)
                        .collect::<std::collections::HashSet<_>>(),
                ),
                Err(_) => None,
            }
        } else {
            None
        }
    } else {
        None
    };

    let tag1 = tag.clone();
    let category1 = category.clone();
    let query_lower = query.clone().map(|q| q.to_lowercase());
    let search_slugs1 = search_slugs.clone();
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
        if let Some(ref slugs) = search_slugs1 {
            slugs.contains(&a.slug)
        } else if let Some(ref ql) = query_lower {
            a.metadata.title.to_lowercase().contains(ql)
                || a.metadata.description.to_lowercase().contains(ql)
        } else {
            true
        }
    };

    let articles: Vec<&Article> = store.query(filter, offset, limit).collect();

    let tag2 = tag;
    let category2 = category;
    let query_lower2 = query.map(|q| q.to_lowercase());
    let search_slugs2 = search_slugs;
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
        if let Some(ref slugs) = search_slugs2 {
            slugs.contains(&a.slug)
        } else if let Some(ref ql) = query_lower2 {
            a.metadata.title.to_lowercase().contains(ql)
                || a.metadata.description.to_lowercase().contains(ql)
        } else {
            true
        }
    };
    let total = store.query(filter_total, 0, usize::MAX).count();

    (articles, total)
}

async fn get_articles_list(
    State(state): State<Arc<AppState>>,
    Query(params): Query<ArticleParams>,
) -> Result<impl IntoResponse, AppError> {
    let store = state.store.read().await;
    let limit = if params.limit > 0 { params.limit } else { 10 };
    let page = if params.page > 0 { params.page } else { 1 };
    let offset = (page - 1) * limit;
    let (paginated_articles_vec, total_articles) =
        filter_articles(&store, &params, &state, offset, limit).await;
    let total_pages = (total_articles as f64 / limit as f64).ceil() as usize;
    let paginated_articles = paginated_articles_vec.into_iter();

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

async fn create_article(
    State(state): State<Arc<AppState>>,
    Json(payload): Json<CreateArticleRequest>,
) -> Result<impl IntoResponse, AppError> {
    if payload.title.trim().is_empty() || payload.content.trim().is_empty() {
        return Err(AppError::BadRequest {
            code: ERR_BAD_REQUEST,
            message: "Title and content cannot be empty".to_string(),
        });
    }

    let base_slug = slugify(&payload.title);
    if base_slug.is_empty() {
        return Err(AppError::BadRequest {
            code: ERR_BAD_REQUEST,
            message: "Invalid title for slug generation".to_string(),
        });
    }

    let mut slug_candidate = base_slug.clone();
    let mut counter = 1;
    while state
        .store
        .read()
        .await
        .get_by_slug(&slug_candidate)
        .is_some()
    {
        if counter > 100 {
            return Err(AppError::BadRequest {
                code: ERR_BAD_REQUEST,
                message: "Exceeded maximum slug generation attempts".to_string(),
            });
        }
        slug_candidate = format!("{}-{}", base_slug, counter);
        counter += 1;
    }
    let slug = slug_candidate;

    let metadata = Metadata {
        title: payload.title.clone(),
        author: "system".to_string(),
        date: Utc::now(),
        tags: payload.tags.unwrap_or_default(),
        description: payload.description.unwrap_or_default(),
        draft: payload.draft.unwrap_or(false),
        last_updated: None,
        category: payload.category.clone(),
    };
    let file_path = if let Some(ref cat) = payload.category {
        StdPath::new(ARTICLE_DIR)
            .join(cat)
            .join(format!("{}.md", slug))
    } else {
        StdPath::new(ARTICLE_DIR).join(format!("{}.md", slug))
    };

    write_article_to_file(&metadata, &payload.content, &file_path)?;

    let last_modified = fs::metadata(&file_path)
        .and_then(|m| m.modified())
        .unwrap_or(SystemTime::now());
    let article = Article {
        slug: slug.clone(),
        metadata: metadata.clone(),
        version: 1,
        updated_at: Utc::now(),
        file_path: file_path.to_string_lossy().to_string(),
        last_modified,
        deleted: false,
    };
    save_version(&article).map_err(|e| AppError::InternalServerError {
        code: ERR_INTERNAL_SERVER,
        message: e.to_string(),
    })?;

    {
        let mut store = state.store.write().await;
        if let Err(e) = store.incremental_update(ARTICLE_DIR, ENABLE_NESTED_CATEGORIES) {
            return Err(AppError::InternalServerError {
                code: ERR_INTERNAL_SERVER,
                message: e.to_string(),
            });
        }
    }

    reindex_all_content(&state).await;
    state.cache.invalidate_all();

    Ok(Json(json!({ "slug": slug, "message": "Article created" })))
}

async fn update_article(
    State(state): State<Arc<AppState>>,
    Path(slug): Path<String>,
    Json(payload): Json<UpdateArticleRequest>,
) -> Result<impl IntoResponse, AppError> {
    if payload.title.trim().is_empty() || payload.content.trim().is_empty() {
        return Err(AppError::BadRequest {
            code: ERR_BAD_REQUEST,
            message: "Title and content cannot be empty".to_string(),
        });
    }

    let existing = {
        let store = state.store.read().await;
        store.get_by_slug(&slug).cloned()
    };

    let mut existing_article = existing.ok_or_else(|| AppError::NotFound {
        code: ERR_ARTICLE_NOT_FOUND,
        message: format!("Article with slug {} not found", slug),
    })?;

    let metadata = Metadata {
        title: payload.title.clone(),
        author: existing_article.metadata.author.clone(),
        date: existing_article.metadata.date,
        tags: payload
            .tags
            .clone()
            .unwrap_or(existing_article.metadata.tags.clone()),
        description: payload
            .description
            .clone()
            .unwrap_or(existing_article.metadata.description.clone()),
        draft: payload.draft.unwrap_or(existing_article.metadata.draft),
        last_updated: Some(Utc::now().to_rfc3339()),
        category: payload
            .category
            .clone()
            .or(existing_article.metadata.category.clone()),
    };

    let file_path = if let Some(ref cat) = metadata.category {
        StdPath::new(ARTICLE_DIR)
            .join(cat)
            .join(format!("{}.md", slug))
    } else {
        StdPath::new(ARTICLE_DIR).join(format!("{}.md", slug))
    };

    write_article_to_file(&metadata, &payload.content, &file_path)?;

    let last_modified = fs::metadata(&file_path)
        .and_then(|m| m.modified())
        .unwrap_or(SystemTime::now());

    existing_article.metadata = metadata.clone();
    existing_article.file_path = file_path.to_string_lossy().to_string();
    existing_article.updated_at = Utc::now();
    existing_article.last_modified = last_modified;

    save_version(&existing_article).map_err(|e| AppError::InternalServerError {
        code: ERR_INTERNAL_SERVER,
        message: e.to_string(),
    })?;

    {
        let mut store = state.store.write().await;
        if let Err(e) = store.update_single_article(
            &existing_article.file_path,
            ARTICLE_DIR,
            ENABLE_NESTED_CATEGORIES,
        ) {
            return Err(AppError::InternalServerError {
                code: ERR_INTERNAL_SERVER,
                message: e.to_string(),
            });
        }
    }

    reindex_all_content(&state).await;
    state.cache.invalidate_all();

    Ok(Json(json!({ "slug": slug, "message": "Article updated" })))
}

async fn get_article_by_slug(
    State(state): State<Arc<AppState>>,
    Path(slug): Path<String>,
) -> Result<impl IntoResponse, AppError> {
    let store = state.store.read().await;
    let article = store.get_by_slug(&slug);

    match article {
        Some(article) if !article.metadata.draft => {
            let content = store
                .load_content_for(article)
                .map_err(|e| AppError::BadRequest {
                    code: ERR_BAD_REQUEST,
                    message: e.to_string(),
                })?;
            Ok(Json(ArticleContent {
                slug: article.slug.clone(),
                metadata: article.metadata.clone(),
                content,
            }))
        }
        Some(_) => Err(AppError::NotFound {
            code: ERR_ARTICLE_NOT_FOUND,
            message: format!("Article with slug {} not found", slug),
        }),
        None => Err(AppError::NotFound {
            code: ERR_ARTICLE_NOT_FOUND,
            message: format!("Article with slug {} not found", slug),
        }),
    }
}
