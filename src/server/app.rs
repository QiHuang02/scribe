use crate::config::Config;
use crate::server::cache::ResponseCacheLayer;
use crate::services::search::SearchService;
use crate::services::service::ArticleStore;
use axum::Router;
use moka2::future::Cache;
use notify::{RecursiveMode, Watcher};
use std::net::SocketAddr;
use std::sync::{Arc, RwLock};
use std::time::Duration;
use tokio::sync::mpsc;
use tracing::info;

pub struct AppState {
    pub store: Arc<RwLock<ArticleStore>>,
    pub config: Arc<Config>,
    pub search_service: Option<Arc<SearchService>>,
    pub cache: Arc<Cache<String, String>>,
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
                if let Err(e) = service.index_articles(
                    &article_store
                        .query(|_| true)
                        .into_iter()
                        .cloned()
                        .collect::<Vec<_>>(),
                    config.search_index_heap_size,
                ) {
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

    Ok(Arc::new(AppState {
        store: Arc::new(RwLock::new(article_store)),
        config: Arc::clone(config),
        search_service,
        cache: Arc::new(cache),
    }))
}

pub fn start_file_watcher(app_state: Arc<AppState>) {
    tokio::spawn(watch_articles(app_state));
}

pub async fn start_server(app_state: Arc<AppState>, config: &Config) {
    let app = Router::new()
        .merge(crate::handlers::articles::create_router())
        .merge(crate::handlers::tags::create_router())
        .merge(crate::handlers::categories::create_router())
        .merge(crate::handlers::search::create_router())
        .layer(ResponseCacheLayer::new(app_state.cache.clone()))
        .with_state(app_state);

    let addr: SocketAddr = config.server_addr.parse().expect("Invalid server address");
    info!("Starting server on {}", addr);
    let listener = tokio::net::TcpListener::bind(&addr).await.unwrap();
    axum::serve(listener, app).await.unwrap();
}

async fn watch_articles(state: Arc<AppState>) {
    let (tx, mut rx) = mpsc::channel(10);

    let mut watcher = notify::recommended_watcher(move |res: notify::Result<notify::Event>| {
        if let Ok(event) = res
            && (event.kind.is_modify() || event.kind.is_create() || event.kind.is_remove())
        {
            tx.blocking_send(()).unwrap();
        }
    })
    .unwrap();

    watcher
        .watch(state.config.article_dir.as_ref(), RecursiveMode::Recursive)
        .unwrap();

    info!("Hot reloading enable for '{}'", state.config.article_dir);

    while rx.recv().await.is_some() {
        tokio::time::sleep(tokio::time::Duration::from_millis(500)).await;

        info!("File change detected, performing incremental update...");
        let mut store_guard = state.store.write().unwrap();

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
        let articles: Vec<_> = store.query(|_| true).into_iter().cloned().collect();
        if let Err(e) =
            search_service.index_articles(&articles, state.config.search_index_heap_size)
        {
            tracing::warn!("Failed to reindex articles for search: {:?}", e);
        } else {
            info!("Search index updated successfully!");
        }
    }
}
