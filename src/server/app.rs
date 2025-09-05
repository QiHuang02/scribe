use crate::config::Config;
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
}

pub async fn create_app_state(config: &Arc<Config>) -> Arc<AppState> {
    let article_store = match ArticleStore::new(&config.article_dir, &config.article_extension) {
        Ok(store) => store,
        Err(e) => {
            eprintln!("Failed to load articles: {:?}", e);
            std::process::exit(1);
        }
    };

    Arc::new(AppState {
        store: Arc::new(RwLock::new(article_store)),
        config: Arc::clone(config),
    })
}

pub fn start_file_watcher(app_state: Arc<AppState>) {
    tokio::spawn(watch_articles(app_state));
}

pub async fn start_server(app_state: Arc<AppState>, config: &Config) {
    let app = Router::new()
        .merge(crate::handlers::articles::create_router())
        .merge(crate::handlers::tags::create_router())
        .with_state(app_state);

    let addr: SocketAddr = config.server_addr.parse().expect("Invalid server address");
    tracing::info!("Starting server on {}", addr);
    let listener = tokio::net::TcpListener::bind(&addr).await.unwrap();
    axum::serve(listener, app).await.unwrap();
}

async fn watch_articles(state: Arc<AppState>) {
    let (tx, mut rx) = mpsc::channel(10);

    let mut watcher = notify::recommended_watcher(move |res: notify::Result<notify::Event>| {
        if let Ok(event) = res {
            if event.kind.is_modify() || event.kind.is_create() || event.kind.is_remove() {
                tx.blocking_send(()).unwrap();
            }
        }
    })
        .unwrap();

    watcher.watch(
        state.config.article_dir.as_ref(),
        RecursiveMode::Recursive,
    )
        .unwrap();

    info!("Hot reloading enable for '{}'", state.config.article_dir);

    while (rx.recv().await).is_some() {
        tokio::time::sleep(tokio::time::Duration::from_millis(500)).await;

        info!("File change detected, reloading articles...");
        match ArticleStore::new(&state.config.article_dir, &state.config.article_extension) {
            Ok(new_store) => {
                let mut store_guard = state.store.write().unwrap();
                *store_guard = new_store;
                info!("Articles reloaded successfully!");
            }
            Err(e) => {
                eprintln!("Error reloading articles: {:?}", e);
            }
        }
    }
}