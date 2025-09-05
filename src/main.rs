use crate::services::service::ArticleStore;

use crate::config::Config;
use axum::Router;
use notify::{RecursiveMode, Watcher};
use std::net::SocketAddr;
use std::sync::{Arc, RwLock};
use tokio::sync::mpsc;
use tracing::info;
use tracing_subscriber::layer::SubscriberExt;
use tracing_subscriber::util::SubscriberInitExt;

mod models;
mod handlers;
mod services;
pub mod error;
mod config;

pub struct AppState {
    store: Arc<RwLock<ArticleStore>>,
    config: Arc<Config>,
}

#[tokio::main]
async fn main() {
    let config = initialize_config();
    initialize_logging(&config);
    let app_state = create_app_state(&config).await;
    start_file_watcher(Arc::clone(&app_state));
    start_server(app_state, &config).await;
}

fn initialize_config() -> Arc<Config> {
    match config::load_config() {
        Ok(cfg) => Arc::new(cfg),
        Err(e) => {
            eprintln!("Failed to load config: {}", e);
            std::process::exit(1);
        }
    }
}

fn initialize_logging(config: &Config) {
    tracing_subscriber::registry()
        .with(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| config.log_level.clone().into()),
        )
        .with(tracing_subscriber::fmt::layer())
        .init();
}

async fn create_app_state(config: &Arc<Config>) -> Arc<AppState> {
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

fn start_file_watcher(app_state: Arc<AppState>) {
    tokio::spawn(watch_articles(app_state));
}

async fn start_server(app_state: Arc<AppState>, config: &Config) {
    let app = Router::new()
        .merge(handlers::articles::create_router())
        .merge(handlers::tags::create_router())
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
