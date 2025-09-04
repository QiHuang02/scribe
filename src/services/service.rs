use crate::models::article::{Article, Metadata};
use gray_matter::engine::YAML;
use gray_matter::Matter;
use serde_yaml::{from_value, Error as SerdeYAMLError};
use std::collections::HashMap;
use std::fs;
use std::io::Error as IoError;
use std::path::Path;

#[derive(Debug)]
pub enum LoadError {
    Io(()),
    YamlParse(()),
    MatterParse(()),
    InvalidFileName(()),
    MissingFrontMatter(()),
}

impl From<IoError> for LoadError {
    fn from(_err: IoError) -> Self {
        LoadError::Io(())
    }
}

impl From<SerdeYAMLError> for LoadError {
    fn from(_err: SerdeYAMLError) -> Self {
        LoadError::YamlParse(())
    }
}

pub struct ArticleStore {
    articles: Vec<Article>,
    slug_map: HashMap<String, usize>,
}

impl ArticleStore {
    pub fn new(content_dir: &str, article_extension: &str) -> Result<Self, LoadError> {
        let mut articles = Vec::new();
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

        Ok(Self { articles, slug_map })
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


