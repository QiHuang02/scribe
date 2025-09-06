use serde::Deserialize;
use std::fs;
use std::net::SocketAddr;
use std::path::Path;
use std::sync::Arc;
use tracing_subscriber::layer::SubscriberExt;
use tracing_subscriber::util::SubscriberInitExt;

#[derive(Deserialize, Debug)]
pub struct Config {
    pub article_dir: String,
    pub log_level: String,
    pub server_addr: String,
    pub latest_articles_count: usize,
    pub article_extension: String,
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
        // 验证文章目录存在
        if !Path::new(&self.article_dir).exists() {
            return Err(format!("Article directory does not exist: {}", self.article_dir));
        }

        // 验证服务器地址格式
        if self.server_addr.parse::<SocketAddr>().is_err() {
            return Err(format!("Invalid server address: {}", self.server_addr));
        }

        // 验证日志级别
        match self.log_level.to_lowercase().as_str() {
            "trace" | "debug" | "info" | "warn" | "error" => {},
            _ => return Err(format!("Invalid log level: {}", self.log_level)),
        }

        // 验证文章扩展名格式
        if !self.article_extension.starts_with('.') {
            return Err(format!("Article extension must start with '.': {}", self.article_extension));
        }

        // 验证最新文章数量
        if self.latest_articles_count == 0 {
            return Err("Latest articles count must be greater than 0".to_string());
        }

        // 验证搜索索引堆大小
        if self.search_index_heap_size < 1_000_000 { // 最少 1MB
            return Err(format!("Search index heap size too small: {} bytes (minimum: 1MB)", self.search_index_heap_size));
        }

        Ok(())
    }
}

fn default_search_index_dir() -> String {
    "search_index".to_string()
}

fn default_search_index_heap_size() -> usize {
    50_000_000 // 50MB
}

fn default_content_search_limit() -> usize {
    10_000 // 限制搜索内容长度为10KB
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