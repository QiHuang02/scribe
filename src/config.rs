use serde::Deserialize;
use std::fs;
use std::net::SocketAddr;
use std::path::Path;
use std::sync::Arc;
use tracing_subscriber::layer::SubscriberExt;
use tracing_subscriber::util::SubscriberInitExt;
use tracing_subscriber::EnvFilter;

#[derive(Deserialize, Debug)]
pub struct Config {
    pub article_dir: String,
    pub log_level: String,
    pub server_addr: String,
    pub latest_articles_count: usize,
    #[serde(default)]
    pub enable_nested_categories: bool,
    #[serde(default = "default_search_index_dir")]
    pub search_index_dir: String,
    #[serde(default)]
    pub enable_full_text_search: bool,
    #[serde(default = "default_search_index_heap_size")]
    pub search_index_heap_size: usize,
    #[serde(default = "default_content_search_limit")]
    pub content_search_limit: usize,
}

impl Config {
    pub fn validate(&self) -> Result<(), String> {
        if !Path::new(&self.article_dir).exists() {
            return Err(format!("Article directory does not exist: {}", self.article_dir));
        }

        if self.server_addr.parse::<SocketAddr>().is_err() {
            return Err(format!("Invalid server address: {}", self.server_addr));
        }

        if EnvFilter::try_new(&self.log_level).is_err() {
            return Err(format!("Invalid log level: {}", self.log_level));
        }

        if self.latest_articles_count == 0 {
            return Err("Latest articles count must be greater than 0".to_string());
        }

        if self.search_index_heap_size < 1_000_000 {
            return Err(format!("Search index heap size too small: {} bytes (minimum: 1MB)", self.search_index_heap_size));
        }

        Ok(())
    }
}

fn default_search_index_dir() -> String {
    "search_index".to_string()
}

fn default_search_index_heap_size() -> usize {
    50_000_000
}

fn default_content_search_limit() -> usize {
    10_000
}

pub fn load_config() -> Result<Config, Box<dyn std::error::Error>> {
    let config_content = fs::read_to_string("config.toml")?;
    let config: Config = toml::from_str(&config_content)?;
    Ok(config)
}

pub fn initialize_config() -> Result<Arc<Config>, Box<dyn std::error::Error>> {
    let config = load_config()?;
    config.validate()
        .map_err(|e| format!("Configuration validation failed: {}", e))?;
    Ok(Arc::new(config))
}

pub fn initialize_logging(config: &Config) {
    tracing_subscriber::registry()
        .with(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| config.log_level.clone().into()),
        )
        .with(tracing_subscriber::fmt::layer())
        .init();
}
