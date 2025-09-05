use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Metadata {
    pub title: String,
    pub author: String,
    pub date: DateTime<Utc>,
    #[serde(default)]
    pub tags: Vec<String>,
    pub description: String,
    #[serde(default)]
    pub draft: bool,
    pub last_updated: Option<String>,
}

#[derive(Serialize, Debug, Clone)]
pub struct Article {
    pub slug: String,
    pub metadata: Metadata,
    pub content: String,
}

#[derive(Serialize, Debug, Clone)]
pub struct ArticleTeaser {
    pub slug: String,
    pub metadata: Metadata,
}

#[derive(Serialize, Debug, Clone)]
#[serde(untagged)]
pub enum ArticleRepresentation {
    Full(Article),
    Teaser(ArticleTeaser),
}

#[derive(Serialize, Debug)]
pub struct PaginatedArticles<T> {
    pub articles: Vec<T>,
    pub total_pages: usize,
    pub current_page: usize,
}
