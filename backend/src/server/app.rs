use crate::config::Config;
use crate::server::cache::{CachedResponse, ResponseCacheLayer};
use crate::services::search::SearchService;
use crate::services::service::ArticleStore;
use axum::body::Body;
use axum::middleware::{self, Next};
use axum::response::Response;
use axum::{Router, http::Request};
use cookie::Key;
use moka2::future::Cache;
use notify::{RecursiveMode, Watcher};
use std::env;
use std::net::SocketAddr;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::RwLock;
use tokio::sync::mpsc;
use tracing::{error, info};

pub struct AppState {
    pub store: Arc<RwLock<ArticleStore>>,
    pub config: Arc<Config>,
    pub search_service: Option<Arc<SearchService>>,
    pub cache: Arc<Cache<String, CachedResponse>>,
    pub cookie_key: Key,
}

pub async fn create_app_state(
    config: &Arc<Config>,
) -> Result<Arc<AppState>, Box<dyn std::error::Error>> {
    let article_store = ArticleStore::new(&config.article_dir, config.enable_nested_categories)?;
    let cache = Cache::builder()
        .max_capacity(config.cache_max_capacity)
        .time_to_live(Duration::from_secs(config.cache_ttl_seconds))
        .build();

    let search_service = if config.enable_full_text_search {
        match SearchService::new(&config.search_index_dir) {
            Ok(service) => {
                let articles = article_store.load_full_articles();
                if let Err(e) = service.index_articles(&articles, config.search_index_heap_size) {
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
        config: Arc::clone(config),
        search_service,
        cache: Arc::new(cache),
        cookie_key,
    }))
}

pub fn start_file_watcher(app_state: Arc<AppState>) {
    tokio::spawn(watch_articles(app_state));
}

pub async fn start_server(
    app_state: Arc<AppState>,
    config: &Config,
) -> Result<(), Box<dyn std::error::Error>> {
    let mut app = Router::new()
        .merge(crate::handlers::root::create_router())
        .merge(crate::handlers::articles::create_router())
        .merge(crate::handlers::article_versions::create_router())
        .merge(crate::handlers::tags::create_router())
        .merge(crate::handlers::categories::create_router())
        .merge(crate::handlers::search::create_router())
        .merge(crate::handlers::sitemap::create_router());

    if config.enable_comments {
        app = app
            .merge(crate::handlers::auth::create_router())
            .merge(crate::handlers::comments::create_router());
    }

    let app = app
        .layer(middleware::from_fn(log_errors))
        .layer(ResponseCacheLayer::new(app_state.cache.clone()))
        .with_state(app_state);

    let addr: SocketAddr = config.server_addr.parse()?;
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

    if let Err(e) = watcher.watch(state.config.article_dir.as_ref(), RecursiveMode::Recursive) {
        error!(
            "Failed to watch directory '{}': {:?}",
            state.config.article_dir, e
        );
        return;
    }

    info!("Hot reloading enable for '{}'", state.config.article_dir);

    while rx.recv().await.is_some() {
        tokio::time::sleep(tokio::time::Duration::from_millis(500)).await;

        info!("File change detected, performing incremental update...");
        let mut store_guard = state.store.write().await;

        match store_guard.incremental_update(
            &state.config.article_dir,
            state.config.enable_nested_categories,
        ) {
            Ok(true) => {
                reindex_articles_with_logging(&state, &store_guard);
                state.cache.invalidate_all();
                info!("Articles updated incrementally!");
            }
            Ok(false) => {
                tracing::debug!("No file changes detected, skipping update");
            }
            Err(e) => {
                tracing::error!("Error during incremental update: {:?}", e);
                info!("Falling back to full reload...");
                match ArticleStore::new(
                    &state.config.article_dir,
                    state.config.enable_nested_categories,
                ) {
                    Ok(new_store) => {
                        *store_guard = new_store;

                        reindex_articles_with_logging(&state, &store_guard);
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

fn reindex_articles_with_logging(state: &Arc<AppState>, store: &ArticleStore) {
    if let Some(ref search_service) = state.search_service {
        let articles = store.load_full_articles();
        if let Err(e) =
            search_service.index_articles(&articles, state.config.search_index_heap_size)
        {
            tracing::warn!("Failed to reindex articles for search: {:?}", e);
        } else {
            info!("Search index updated successfully!");
        }
    }
}
