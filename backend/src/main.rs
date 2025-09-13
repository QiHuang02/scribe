use crate::config::{initialize_config, initialize_logging};
use crate::server::app::{create_app_state, start_file_watcher, start_server};
use std::sync::Arc;

mod config;
mod handlers;
mod models;
mod server;
mod services;
mod db;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let config = initialize_config()?;
    initialize_logging(&config);
    let _db = db::init_db("sqlite://comments.db").await?;
    let app_state = create_app_state(&config).await?;
    start_file_watcher(Arc::clone(&app_state));
    start_server(app_state, &config).await?;
    Ok(())
}
