use crate::config::{initialize_config, initialize_logging};
use crate::server::app::{create_app_state, start_file_watcher, start_server};
use std::sync::Arc;

mod models;
mod handlers;
mod services;
mod config;
mod server;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let config = initialize_config()?;
    initialize_logging(&config);
    let app_state = create_app_state(&config).await?;
    start_file_watcher(Arc::clone(&app_state));
    start_server(app_state, &config).await;
    Ok(())
}
