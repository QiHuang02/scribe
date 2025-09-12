use crate::config::{
    ARTICLE_DIR, CACHE_MAX_CAPACITY, CACHE_TTL_SECONDS, Config, ENABLE_NESTED_CATEGORIES,
    NOTES_DIR, SERVER_ADDR,
};
use crate::models::article::ArticleContent;
use crate::server::cache::{CachedResponse, ResponseCacheLayer};
use crate::services::search::SearchService;
use crate::services::service::{ArticleStore, FileChange};
use axum::body::Body;
use axum::middleware::{self, Next};
use axum::response::Response;
use axum::{Router, http::Request};
use cookie::Key;
use moka2::future::Cache;
use notify::{RecursiveMode, Watcher};
use std::collections::HashMap;
use std::env;
use std::net::SocketAddr;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::RwLock;
use tokio::sync::mpsc;
use tracing::{error, info};

pub struct AppState {
    pub store: Arc<RwLock<ArticleStore>>,
    pub note_store: Arc<RwLock<ArticleStore>>,
    pub config: Arc<Config>,
    pub search_service: Option<Arc<SearchService>>,
    pub cache: Arc<Cache<String, CachedResponse>>,
    pub cookie_key: Key,
}

pub async fn create_app_state(
    config: &Arc<Config>,
) -> Result<Arc<AppState>, Box<dyn std::error::Error>> {
    let article_store = ArticleStore::new(ARTICLE_DIR, ENABLE_NESTED_CATEGORIES)?;
    let note_store = ArticleStore::new(NOTES_DIR, true)?;
    let cache = Cache::builder()
        .max_capacity(CACHE_MAX_CAPACITY)
        .time_to_live(Duration::from_secs(CACHE_TTL_SECONDS))
        .build();

    let search_service = if config.enable_full_text_search {
        match SearchService::new(&config.search_index_dir) {
            Ok(service) => {
                let mut all = article_store.load_full_articles();
                let mut notes = note_store.load_full_articles();
                for n in &mut notes {
                    n.slug = format!("notes/{}", n.slug_with_category());
                }
                all.extend(notes);
                if let Err(e) = service.index_articles(&all, config.search_index_heap_size) {
                    tracing::warn!("Failed to index articles: {:?}", e);
                    None
                } else {
                    info!("Search index updated successfully!");
                    Some(Arc::new(service))
                }
            }
            Err(e) => {
                tracing::warn!("Failed to initialize search service: {:?}", e);
                None
            }
        }
    } else {
        None
    };

    let cookie_secret =
        env::var("COOKIE_SECRET").map_err(|_| "COOKIE_SECRET environment variable must be set")?;
    let cookie_key = Key::derive_from(cookie_secret.as_bytes());

    Ok(Arc::new(AppState {
        store: Arc::new(RwLock::new(article_store)),
        note_store: Arc::new(RwLock::new(note_store)),
        config: Arc::clone(config),
        search_service,
        cache: Arc::new(cache),
        cookie_key,
    }))
}

pub fn start_file_watcher(app_state: Arc<AppState>) {
    let article_state = Arc::clone(&app_state);
    tokio::spawn(watch_articles(article_state));
    tokio::spawn(watch_notes(app_state));
}

pub async fn start_server(
    app_state: Arc<AppState>,
    config: &Config,
) -> Result<(), Box<dyn std::error::Error>> {
    let mut app = Router::new()
        .merge(crate::handlers::root::create_router())
        .merge(crate::handlers::articles::create_router())
        .merge(crate::handlers::notes::create_router())
        .merge(crate::handlers::article_versions::create_router())
        .merge(crate::handlers::tags::create_router())
        .merge(crate::handlers::categories::create_router())
        .merge(crate::handlers::search::create_router())
        .merge(crate::handlers::sitemap::create_router());

    if config.comments {
        app = app
            .merge(crate::handlers::auth::create_router())
            .merge(crate::handlers::comments::create_router());
    }

    let app = app
        .layer(middleware::from_fn(log_errors))
        .layer(ResponseCacheLayer::new(app_state.cache.clone()))
        .with_state(app_state);

    let addr: SocketAddr = SERVER_ADDR.parse()?;
    info!("Starting server on http://{}", addr);
    let listener = tokio::net::TcpListener::bind(&addr).await?;
    axum::serve(listener, app).await?;
    Ok(())
}

async fn log_errors(req: Request<Body>, next: Next) -> Response {
    let res = next.run(req).await;
    if res.status().is_server_error() {
        error!("Internal server error: {}", res.status());
    }
    res
}

async fn watch_articles(state: Arc<AppState>) {
    let (tx, mut rx) = mpsc::unbounded_channel();

    let tx_watcher = tx.clone();
    let mut watcher =
        match notify::recommended_watcher(move |res: notify::Result<notify::Event>| {
            if let Ok(event) = res
                && (event.kind.is_modify() || event.kind.is_create() || event.kind.is_remove())
            {
                if tx_watcher.send(()).is_err() {
                    error!("File change notification receiver dropped");
                }
            }
        }) {
            Ok(w) => w,
            Err(e) => {
                error!("Failed to initialize file watcher: {:?}", e);
                return;
            }
        };

    if let Err(e) = watcher.watch(std::path::Path::new(ARTICLE_DIR), RecursiveMode::Recursive) {
        error!("Failed to watch directory '{}': {:?}", ARTICLE_DIR, e);
        return;
    }

    info!("Hot reloading enable for '{}'", ARTICLE_DIR);

    while rx.recv().await.is_some() {
        tokio::time::sleep(tokio::time::Duration::from_millis(500)).await;

        info!("File change detected, performing incremental update...");
        let mut store_guard = state.store.write().await;

        let changes = match store_guard.detect_file_changes(ARTICLE_DIR, ENABLE_NESTED_CATEGORIES) {
            Ok(c) => c,
            Err(e) => {
                tracing::error!("Error detecting file changes: {:?}", e);
                continue;
            }
        };

        if changes.is_empty() {
            tracing::debug!("No file changes detected, skipping update");
            continue;
        }

        let mut removed_map = HashMap::new();
        for change in &changes {
            if matches!(change.change_type, FileChange::Removed) {
                if let Some(article) = store_guard.query(|a| a.file_path == change.path).next() {
                    removed_map.insert(change.path.clone(), article.slug.clone());
                }
            }
        }

        match store_guard.incremental_update(ARTICLE_DIR, ENABLE_NESTED_CATEGORIES) {
            Ok(true) => {
                if let Some(search_service) = &state.search_service {
                    for change in &changes {
                        match change.change_type {
                            FileChange::Added | FileChange::Modified => {
                                if let Some(article) =
                                    store_guard.query(|a| a.file_path == change.path).next()
                                {
                                    match store_guard.load_content_for(article) {
                                        Ok(content) => {
                                            let article_content = ArticleContent {
                                                slug: article.slug.clone(),
                                                metadata: article.metadata.clone(),
                                                content,
                                            };
                                            if let Err(e) = search_service.index_article(
                                                &article_content,
                                                state.config.search_index_heap_size,
                                            ) {
                                                tracing::warn!(
                                                    "Failed to index article {}: {:?}",
                                                    article.slug,
                                                    e
                                                );
                                            }
                                        }
                                        Err(e) => {
                                            tracing::warn!(
                                                "Failed to load content for article {}: {:?}",
                                                article.slug,
                                                e
                                            );
                                        }
                                    }
                                }
                            }
                            FileChange::Removed => {
                                if let Some(slug) = removed_map.get(&change.path) {
                                    if let Err(e) = search_service
                                        .remove_article(slug, state.config.search_index_heap_size)
                                    {
                                        tracing::warn!(
                                            "Failed to remove article {}: {:?}",
                                            slug,
                                            e
                                        );
                                    }
                                }
                            }
                        }
                    }
                }

                state.cache.invalidate_all();
                info!("Articles updated incrementally!");
            }
            Ok(false) => {
                tracing::debug!("No file changes detected, skipping update");
            }
            Err(e) => {
                tracing::error!("Error during incremental update: {:?}", e);
                info!("Falling back to full reload...");
                match ArticleStore::new(ARTICLE_DIR, ENABLE_NESTED_CATEGORIES) {
                    Ok(new_store) => {
                        *store_guard = new_store;

                        reindex_all_content(&state).await;
                        state.cache.invalidate_all();

                        info!("Full reload completed successfully!");
                    }
                    Err(e) => {
                        tracing::error!("Full reload also failed: {:?}", e);
                    }
                }
            }
        }
    }
}

async fn watch_notes(state: Arc<AppState>) {
    let (tx, mut rx) = mpsc::unbounded_channel();

    let tx_watcher = tx.clone();
    let mut watcher =
        match notify::recommended_watcher(move |res: notify::Result<notify::Event>| {
            if let Ok(event) = res
                && (event.kind.is_modify() || event.kind.is_create() || event.kind.is_remove())
            {
                if tx_watcher.send(()).is_err() {
                    error!("File change notification receiver dropped");
                }
            }
        }) {
            Ok(w) => w,
            Err(e) => {
                error!("Failed to initialize file watcher: {:?}", e);
                return;
            }
        };

    if let Err(e) = watcher.watch(std::path::Path::new(NOTES_DIR), RecursiveMode::Recursive) {
        error!("Failed to watch directory '{}': {:?}", NOTES_DIR, e);
        return;
    }

    info!("Hot reloading enable for '{}'", NOTES_DIR);

    while rx.recv().await.is_some() {
        tokio::time::sleep(tokio::time::Duration::from_millis(500)).await;

        info!("File change detected, performing incremental update...");
        let mut store_guard = state.note_store.write().await;

        let changes = match store_guard.detect_file_changes(NOTES_DIR, true) {
            Ok(c) => c,
            Err(e) => {
                tracing::error!("Error detecting file changes: {:?}", e);
                continue;
            }
        };

        if changes.is_empty() {
            tracing::debug!("No file changes detected, skipping update");
            continue;
        }

        let mut removed_map = HashMap::new();
        for change in &changes {
            if matches!(change.change_type, FileChange::Removed) {
                if let Some(article) = store_guard.query(|a| a.file_path == change.path).next() {
                    removed_map.insert(
                        change.path.clone(),
                        format!("notes/{}", article.slug_with_category()),
                    );
                }
            }
        }

        match store_guard.incremental_update(NOTES_DIR, true) {
            Ok(true) => {
                if let Some(search_service) = &state.search_service {
                    for change in &changes {
                        match change.change_type {
                            FileChange::Added | FileChange::Modified => {
                                if let Some(article) =
                                    store_guard.query(|a| a.file_path == change.path).next()
                                {
                                    match store_guard.load_content_for(article) {
                                        Ok(content) => {
                                            let article_content = ArticleContent {
                                                slug: format!(
                                                    "notes/{}",
                                                    article.slug_with_category()
                                                ),
                                                metadata: article.metadata.clone(),
                                                content,
                                            };
                                            if let Err(e) = search_service.index_article(
                                                &article_content,
                                                state.config.search_index_heap_size,
                                            ) {
                                                tracing::warn!(
                                                    "Failed to index note {}: {:?}",
                                                    article.slug,
                                                    e
                                                );
                                            }
                                        }
                                        Err(e) => {
                                            tracing::warn!(
                                                "Failed to load content for note {}: {:?}",
                                                article.slug,
                                                e
                                            );
                                        }
                                    }
                                }
                            }
                            FileChange::Removed => {
                                if let Some(slug) = removed_map.get(&change.path) {
                                    if let Err(e) = search_service
                                        .remove_article(slug, state.config.search_index_heap_size)
                                    {
                                        tracing::warn!("Failed to remove note {}: {:?}", slug, e);
                                    }
                                }
                            }
                        }
                    }
                }

                state.cache.invalidate_all();
                info!("Notes updated incrementally!");
            }
            Ok(false) => {
                tracing::debug!("No file changes detected, skipping update");
            }
            Err(e) => {
                tracing::error!("Error during incremental update: {:?}", e);
                info!("Falling back to full reload...");
                match ArticleStore::new(NOTES_DIR, true) {
                    Ok(new_store) => {
                        *store_guard = new_store;

                        reindex_all_content(&state).await;
                        state.cache.invalidate_all();

                        info!("Full reload completed successfully!");
                    }
                    Err(e) => {
                        tracing::error!("Full reload also failed: {:?}", e);
                    }
                }
            }
        }
    }
}

pub async fn reindex_all_content(state: &Arc<AppState>) {
    if let Some(ref search_service) = state.search_service {
        let store = state.store.read().await;
        let mut all = store.load_full_articles();
        drop(store);
        let notes_store = state.note_store.read().await;
        let mut notes = notes_store.load_full_articles();
        drop(notes_store);
        for n in &mut notes {
            n.slug = format!("notes/{}", n.slug_with_category());
        }
        all.extend(notes);
        if let Err(e) = search_service.index_articles(&all, state.config.search_index_heap_size) {
            tracing::warn!("Failed to reindex articles for search: {:?}", e);
        } else {
            info!("Search index updated successfully!");
        }
    }
}
