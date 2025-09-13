use crate::models::article::ArticleContent;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::Path;
use tantivy::collector::TopDocs;
use tantivy::query::QueryParser;
use tantivy::schema::*;
use tantivy::{Index, ReloadPolicy, TantivyDocument, Term, doc};
use thiserror::Error;
use tokio::sync::RwLock;

#[derive(Error, Debug)]
pub enum SearchError {
    #[error("Tantivy error: {0}")]
    TantivyError(#[from] tantivy::TantivyError),
    #[error("Query parsing error: {0}")]
    QueryParseError(#[from] tantivy::query::QueryParserError),
    #[error("IO error: {0}")]
    IoError(#[from] std::io::Error),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchResult {
    pub slug: String,
    pub title: String,
    pub description: String,
    pub score: f32,
    pub highlights: Option<Vec<String>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchStats {
    pub query: String,
    pub count: usize,
    pub timestamp: chrono::DateTime<chrono::Utc>,
}

pub struct SearchService {
    index: Index,
    reader: tantivy::IndexReader,
    query_parser: QueryParser,
    slug_field: Field,
    title_field: Field,
    content_field: Field,
    description_field: Field,
    tags_field: Field,
    category_field: Field,
    search_stats: RwLock<HashMap<String, usize>>,
    recent_searches: RwLock<Vec<SearchStats>>,
}

impl SearchService {
    pub fn new(index_dir: &str) -> Result<Self, SearchError> {
        let schema = Self::build_schema();

        let index_path = Path::new(index_dir);
        let index = if index_path.exists() {
            Index::open_in_dir(index_path)?
        } else {
            std::fs::create_dir_all(index_path)?;
            Index::create_in_dir(index_path, schema.clone())?
        };

        let slug_field = schema.get_field("slug")?;
        let title_field = schema.get_field("title")?;
        let content_field = schema.get_field("content")?;
        let description_field = schema.get_field("description")?;
        let tags_field = schema.get_field("tags")?;
        let category_field = schema.get_field("category")?;

        let reader = index
            .reader_builder()
            .reload_policy(ReloadPolicy::Manual)
            .try_into()?;

        let query_parser = QueryParser::for_index(
            &index,
            vec![title_field, content_field, description_field, tags_field],
        );

        Ok(SearchService {
            index,
            reader,
            query_parser,
            slug_field,
            title_field,
            content_field,
            description_field,
            tags_field,
            category_field,
            search_stats: RwLock::new(HashMap::new()),
            recent_searches: RwLock::new(Vec::new()),
        })
    }

    fn build_schema() -> Schema {
        let mut schema_builder = Schema::builder();

        schema_builder.add_text_field("slug", STRING | STORED);
        schema_builder.add_text_field("title", TEXT | STORED);
        schema_builder.add_text_field("content", TEXT);
        schema_builder.add_text_field("description", TEXT | STORED);
        schema_builder.add_text_field("tags", TEXT);
        schema_builder.add_text_field("category", TEXT | STORED);

        schema_builder.build()
    }

    pub fn index_articles(
        &self,
        articles: &[ArticleContent],
        heap_size: usize,
    ) -> Result<(), SearchError> {
        let mut index_writer = self.index.writer(heap_size)?;

        index_writer.delete_all_documents()?;

        for article in articles {
            if !article.metadata.draft {
                let tags_text = article.metadata.tags.join(" ");
                let category_text = article.metadata.category.as_deref().unwrap_or("");

                let doc = doc!(
                    self.slug_field => article.slug.clone(),
                    self.title_field => article.metadata.title.clone(),
                    self.content_field => article.content.clone(),
                    self.description_field => article.metadata.description.clone(),
                    self.tags_field => tags_text,
                    self.category_field => category_text,
                );

                index_writer.add_document(doc)?;
            }
        }

        index_writer.commit()?;
        self.reader.reload()?;
        Ok(())
    }

    pub fn index_article(
        &self,
        article: &ArticleContent,
        heap_size: usize,
    ) -> Result<(), SearchError> {
        let mut index_writer = self.index.writer(heap_size)?;
        let term = Term::from_field_text(self.slug_field, &article.slug);
        index_writer.delete_term(term);

        if !article.metadata.draft {
            let tags_text = article.metadata.tags.join(" ");
            let category_text = article.metadata.category.as_deref().unwrap_or("");

            let doc = doc!(
                self.slug_field => article.slug.clone(),
                self.title_field => article.metadata.title.clone(),
                self.content_field => article.content.clone(),
                self.description_field => article.metadata.description.clone(),
                self.tags_field => tags_text,
                self.category_field => category_text,
            );

            index_writer.add_document(doc)?;
        }

        index_writer.commit()?;
        self.reader.reload()?;
        Ok(())
    }

    pub fn remove_article(&self, slug: &str, heap_size: usize) -> Result<(), SearchError> {
        let mut index_writer = self.index.writer::<TantivyDocument>(heap_size)?;
        let term = Term::from_field_text(self.slug_field, slug);
        index_writer.delete_term(term);
        index_writer.commit()?;
        self.reader.reload()?;
        Ok(())
    }

    pub fn apply_batch(
        &self,
        to_index: &[ArticleContent],
        to_remove: &[String],
        heap_size: usize,
    ) -> Result<(), SearchError> {
        let mut index_writer = self.index.writer(heap_size)?;

        for slug in to_remove {
            let term = Term::from_field_text(self.slug_field, slug);
            index_writer.delete_term(term);
        }

        for article in to_index {
            if !article.metadata.draft {
                let tags_text = article.metadata.tags.join(" ");
                let category_text = article.metadata.category.as_deref().unwrap_or("");

                let doc = doc!(
                    self.slug_field => article.slug.clone(),
                    self.title_field => article.metadata.title.clone(),
                    self.content_field => article.content.clone(),
                    self.description_field => article.metadata.description.clone(),
                    self.tags_field => tags_text,
                    self.category_field => category_text,
                );

                index_writer.add_document(doc)?;
            }
        }

        index_writer.commit()?;
        self.reader.reload()?;
        Ok(())
    }

    pub async fn search(
        &self,
        query_text: &str,
        limit: usize,
        with_highlights: bool,
    ) -> Result<Vec<SearchResult>, SearchError> {
        let searcher = self.reader.searcher();

        self.record_search(query_text).await;

        let query = self.query_parser.parse_query(query_text)?;
        let top_docs = searcher.search(&query, &TopDocs::with_limit(limit))?;

        let mut results = Vec::new();

        for (_score, doc_address) in top_docs {
            let retrieved_doc: TantivyDocument = searcher.doc(doc_address)?;

            let slug = retrieved_doc
                .get_first(self.slug_field)
                .and_then(|f| f.as_str())
                .unwrap_or("")
                .to_string();

            let title = retrieved_doc
                .get_first(self.title_field)
                .and_then(|f| f.as_str())
                .unwrap_or("")
                .to_string();

            let description = retrieved_doc
                .get_first(self.description_field)
                .and_then(|f| f.as_str())
                .unwrap_or("")
                .to_string();

            let highlights = if with_highlights {
                Some(self.create_simple_highlights(query_text, &title, &description))
            } else {
                None
            };

            results.push(SearchResult {
                slug,
                title,
                description,
                score: _score,
                highlights,
            });
        }

        Ok(results)
    }

    fn create_simple_highlights(&self, query: &str, title: &str, description: &str) -> Vec<String> {
        let mut highlights = Vec::new();
        let query_lower = query.to_lowercase();
        let title_lower = title.to_lowercase();
        let description_lower = description.to_lowercase();

        if title_lower.contains(&query_lower) {
            highlights.push(format!("Title: {}", title));
        }

        if description_lower.contains(&query_lower) {
            if let Some(pos) = description_lower.find(&query_lower) {
                let start = pos.saturating_sub(50);
                let end = std::cmp::min(pos + query.len() + 50, description.len());
                let snippet = &description[start..end];
                highlights.push(format!("...{snippet}..."));
            }
        }

        highlights
    }

    async fn record_search(&self, query: &str) {
        let mut stats = self.search_stats.write().await;
        *stats.entry(query.to_string()).or_insert(0) += 1;

        let mut recent = self.recent_searches.write().await;
        let search_stat = SearchStats {
            query: query.to_string(),
            count: 1,
            timestamp: chrono::Utc::now(),
        };
        recent.push(search_stat);

        if recent.len() > 1000 {
            let len = recent.len();
            recent.drain(0..len - 1000);
        }
    }

    pub async fn get_popular_searches(&self, limit: usize) -> Vec<(String, usize)> {
        let stats = self.search_stats.read().await;
        let mut popular: Vec<(String, usize)> =
            stats.iter().map(|(k, v)| (k.clone(), *v)).collect();

        popular.sort_by(|a, b| b.1.cmp(&a.1));
        popular.into_iter().take(limit).collect()
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchQuery {
    pub q: String,
    pub limit: Option<usize>,
    pub highlights: Option<bool>,
    pub fields: Option<Vec<String>>,
}

impl Default for SearchQuery {
    fn default() -> Self {
        Self {
            q: String::new(),
            limit: Some(20),
            highlights: Some(true),
            fields: None,
        }
    }
}
