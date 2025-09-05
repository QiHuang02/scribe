use crate::handlers::error::LoadError;
use crate::models::article::{Article, Metadata};
use gray_matter::engine::YAML;
use gray_matter::Matter;
use serde_yaml::from_value;
use std::collections::{HashMap, HashSet};
use std::fs;
use std::path::Path;

pub struct ArticleStore {
    articles: Vec<Article>,
    slug_map: HashMap<String, usize>,
    pub tags: HashSet<String>,
}

impl ArticleStore {
    pub fn new(content_dir: &str, article_extension: &str) -> Result<Self, LoadError> {
        let mut articles = Vec::new();
        let mut all_tags = HashSet::new();
        let entries = fs::read_dir(Path::new(content_dir))?;

        for entry in entries {
            let path = entry?.path();

            if path.is_file() && path.extension().map_or(false, |s| s == article_extension) {
                let slug = path
                    .file_stem()
                    .and_then(|s| s.to_str())
                    .map(String::from)
                    .ok_or_else(|| LoadError::InvalidFileName(()))?;

                let file_content = fs::read_to_string(&path)?;

                let matter = Matter::<YAML>::new();
                let parsed_content = matter
                    .parse(&file_content)
                    .map_err(|_e| LoadError::MatterParse(()))?;

                let data = parsed_content
                    .data
                    .ok_or_else(|| LoadError::MissingFrontMatter(()))?;
                let metadata: Metadata = from_value(data)?;

                if !metadata.draft {
                    for tag in &metadata.tags {
                        all_tags.insert(tag.clone());
                    }
                }

                let content = parsed_content.content;

                articles.push(Article { slug, metadata, content });
            }
        }

        articles.sort_by(|a, b| b.metadata.date.cmp(&a.metadata.date));

        let slug_map = articles
            .iter()
            .enumerate()
            .map(|(idx, article)| (article.slug.clone(), idx))
            .collect();

        Ok(Self { articles, slug_map, tags: all_tags })
    }

    pub fn get_all_tags(&self) -> Vec<String> {
        let mut tags: Vec<String> = self.tags.iter().cloned().collect();
        tags.sort();
        tags
    }

    pub fn get_by_slug(&self, slug: &str) -> Option<&Article> {
        self.slug_map.get(slug).and_then(|&idx| self.articles.get(idx))
    }

    pub fn get_latest(&self, count: usize) -> Vec<&Article> {
        self.articles.iter().take(count).collect()
    }

    pub fn query<F>(&self, filter: F) -> Vec<&Article>
    where
        F: Fn(&Article) -> bool,
    {
        self.articles
            .iter()
            .filter(|&a| filter(a))
            .collect()
    }
}


