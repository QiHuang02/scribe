use serde::Deserialize;
use std::fs;
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
}

fn default_search_index_dir() -> String {
    "search_index".to_string()
}

pub fn load_config() -> Result<Config, Box<dyn std::error::Error>> {
    let config_content = fs::read_to_string("config.toml")?;
    let config: Config = toml::from_str(&config_content)?;
    Ok(config)
}

pub fn initialize_config() -> Arc<Config> {
    match load_config() {
        Ok(cfg) => Arc::new(cfg),
        Err(e) => {
            eprintln!("Failed to load config: {}", e);
            std::process::exit(1);
        }
    }
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