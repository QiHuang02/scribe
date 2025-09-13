use serde::Deserialize;
use std::env;
use std::fs;
use std::path::Path;
use std::sync::Arc;
use tracing_subscriber::EnvFilter;
use tracing_subscriber::layer::SubscriberExt;
use tracing_subscriber::util::SubscriberInitExt;

pub const ARTICLE_DIR: &str = "article";
pub const NOTES_DIR: &str = "notes";
pub const SERVER_ADDR: &str = "127.0.0.1:3000";
pub const ENABLE_NESTED_CATEGORIES: bool = true;
pub const CACHE_MAX_CAPACITY: u64 = 1_000;
pub const CACHE_TTL_SECONDS: u64 = 60;

#[derive(Deserialize, Debug)]
pub struct Config {
    pub log_level: String,
    #[serde(default = "default_hostname")]
    pub hostname: String,
    pub latest_articles_count: usize,
    #[serde(default)]
    pub comments: bool,
    pub github_redirect_url: String,
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
        if !Path::new(ARTICLE_DIR).exists() {
            return Err(format!(
                "Article directory does not exist: {}",
                ARTICLE_DIR
            ));
        }

        if !Path::new(NOTES_DIR).exists() {
            return Err(format!(
                "Notes directory does not exist: {}",
                NOTES_DIR
            ));
        }

        if EnvFilter::try_new(&self.log_level).is_err() {
            return Err(format!("Invalid log level: {}", self.log_level));
        }

        if self.latest_articles_count == 0 {
            return Err("Latest articles count must be greater than 0".to_string());
        }

        if self.search_index_heap_size < 1_000_000 {
            return Err(format!(
                "Search index heap size too small: {} bytes (minimum: 1MB)",
                self.search_index_heap_size
            ));
        }

        if self.comments && self.github_redirect_url.trim().is_empty() {
            return Err(
                "GitHub redirect URL cannot be empty when comments are enabled".to_string(),
            );
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

fn default_hostname() -> String {
    "http://localhost:3000".to_string()
}

pub fn load_config() -> Result<Config, Box<dyn std::error::Error>> {
    let config_content = fs::read_to_string("config.toml")?;
    let mut config: Config = toml::from_str(&config_content)?;
    if config.hostname.trim().is_empty() {
        config.hostname = default_hostname();
    }
    Ok(config)
}

pub fn initialize_config() -> Result<Arc<Config>, Box<dyn std::error::Error>> {
    dotenvy::dotenv().ok();
    let config = load_config()?;
    config
        .validate()
        .map_err(|e| format!("Configuration validation failed: {}", e))?;
    // Validate required environment variables using their respective helpers
    get_admin_token_hash()?;
    get_author_github_username()?;
    if config.comments {
        get_github_client_id()?;
        get_github_client_secret()?;
    }
    Ok(Arc::new(config))
}

pub fn get_admin_token_hash() -> Result<[u8; 32], Box<dyn std::error::Error>> {
    let hash_hex = env::var("ADMIN_TOKEN_HASH")
        .map_err(|_| "ADMIN_TOKEN_HASH environment variable must be set")?;
    let bytes = hex::decode(hash_hex)?;
    if bytes.len() != 32 {
        return Err("ADMIN_TOKEN_HASH must be a 32-byte hex string".into());
    }
    let mut arr = [0u8; 32];
    arr.copy_from_slice(&bytes);
    Ok(arr)
}

pub fn get_github_client_id() -> Result<String, Box<dyn std::error::Error>> {
    env::var("GITHUB_CLIENT_ID")
        .map_err(|_| "GITHUB_CLIENT_ID environment variable must be set".into())
}

pub fn get_github_client_secret() -> Result<String, Box<dyn std::error::Error>> {
    env::var("GITHUB_CLIENT_SECRET")
        .map_err(|_| "GITHUB_CLIENT_SECRET environment variable must be set".into())
}

pub fn get_author_github_username() -> Result<String, Box<dyn std::error::Error>> {
    env::var("AUTHOR_GITHUB_USERNAME")
        .map_err(|_| "AUTHOR_GITHUB_USERNAME environment variable must be set".into())
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

