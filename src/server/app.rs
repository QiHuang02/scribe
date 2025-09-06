use crate::config::Config;
use crate::services::search::SearchService;
use crate::services::service::ArticleStore;
use axum::Router;
use notify::{RecursiveMode, Watcher};
use std::net::SocketAddr;
use std::sync::{Arc, RwLock};
use tokio::sync::mpsc;
use tracing::info;

pub struct AppState {
    pub store: Arc<RwLock<ArticleStore>>,
    pub config: Arc<Config>,
    pub search_service: Option<Arc<SearchService>>,
}

pub async fn create_app_state(config: &Arc<Config>) -> Result<Arc<AppState>, Box<dyn std::error::Error>> {
    let article_store = ArticleStore::new(&config.article_dir, &config.article_extension, config.enable_nested_categories)?;

    // 初始化搜索服务（如果启用）
    let search_service = if config.enable_full_text_search {
        match SearchService::new(&config.search_index_dir) {
            Ok(service) => {
                // 索引现有文章
                if let Err(e) = service.index_articles(&article_store.query(|_| true).into_iter().cloned().collect::<Vec<_>>(), config.search_index_heap_size) {
                    tracing::warn!("Failed to index articles: {:?}", e);
                    None
                } else {
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
            && (event.kind.is_modify() || event.kind.is_create() || event.kind.is_remove()) {
                tx.blocking_send(()).unwrap();
            }
    })
        .unwrap();

    watcher.watch(
        state.config.article_dir.as_ref(),
        RecursiveMode::Recursive,
    )
        .unwrap();

    info!("Hot reloading enable for '{}'", state.config.article_dir);

    while rx.recv().await.is_some() {
        tokio::time::sleep(tokio::time::Duration::from_millis(500)).await;

        info!("File change detected, reloading articles...");
        match ArticleStore::new(&state.config.article_dir, &state.config.article_extension, state.config.enable_nested_categories) {
            Ok(new_store) => {
                let mut store_guard = state.store.write().unwrap();
                *store_guard = new_store;

                // 重新索引搜索（如果搜索服务启用）
                if let Some(ref search_service) = state.search_service {
                    let articles: Vec<_> = store_guard.query(|_| true).into_iter().cloned().collect();
                    if let Err(e) = search_service.index_articles(&articles, state.config.search_index_heap_size) {
                        tracing::warn!("Failed to reindex articles for search: {:?}", e);
                    } else {
                        info!("Search index updated successfully!");
                    }
                }

                info!("Articles reloaded successfully!");
            }
            Err(e) => {
                tracing::error!("Error reloading articles: {:?}", e);
            }
        }
    }
}