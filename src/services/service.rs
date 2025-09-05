use crate::handlers::error::LoadError;
use crate::models::article::{Article, Metadata};
use gray_matter::engine::YAML;
use gray_matter::Matter;
use serde_yaml::from_value;
use std::collections::{HashMap, HashSet};
use std::fs;
use std::path::Path;
use walkdir::WalkDir;

pub struct ArticleStore {
    articles: Vec<Article>,
    slug_map: HashMap<String, usize>,
    pub tags: HashSet<String>,
    pub categories: HashSet<String>,
}

impl ArticleStore {
    pub fn new(content_dir: &str, article_extension: &str, enable_nested_categories: bool) -> Result<Self, LoadError> {
        let mut articles = Vec::new();
        let mut all_tags = HashSet::new();
        let mut all_categories = HashSet::new();
        
        if enable_nested_categories {
            Self::load_articles_recursive(
                content_dir,
                article_extension,
                &mut articles,
                &mut all_tags,
                &mut all_categories,
            )?;
        } else {
            Self::load_articles_flat(
                content_dir,
                article_extension,
                &mut articles,
                &mut all_tags,
            )?;
        }

        articles.sort_by(|a, b| b.metadata.date.cmp(&a.metadata.date));

        let slug_map = articles
            .iter()
            .enumerate()
            .map(|(idx, article)| (article.slug.clone(), idx))
            .collect();

        Ok(Self { 
            articles, 
            slug_map, 
            tags: all_tags,
            categories: all_categories,
        })
    }

    fn load_articles_flat(
        content_dir: &str,
        article_extension: &str,
        articles: &mut Vec<Article>,
        all_tags: &mut HashSet<String>,
    ) -> Result<(), LoadError> {
        let entries = fs::read_dir(Path::new(content_dir))?;

        for entry in entries {
            let path = entry?.path();

            if path.is_file() && path.extension().map_or(false, |s| s == article_extension) {
                Self::process_article_file(&path, None, articles, all_tags)?;
            }
        }
        Ok(())
    }

    fn load_articles_recursive(
        content_dir: &str,
        article_extension: &str,
        articles: &mut Vec<Article>,
        all_tags: &mut HashSet<String>,
        all_categories: &mut HashSet<String>,
    ) -> Result<(), LoadError> {
        let base_path = Path::new(content_dir);
        
        for entry in WalkDir::new(content_dir).into_iter().filter_map(|e| e.ok()) {
            let path = entry.path();
            
            if path.is_file() && path.extension().map_or(false, |s| s == article_extension) {
                // Calculate category from relative path
                let category = if let Some(parent) = path.parent() {
                    if parent != base_path {
                        parent.strip_prefix(base_path)
                            .ok()
                            .and_then(|p| p.to_str())
                            .map(|s| s.replace('\\', "/"))
                    } else {
                        None
                    }
                } else {
                    None
                };

                if let Some(ref cat) = category {
                    all_categories.insert(cat.clone());
                }

                Self::process_article_file(path, category.as_deref(), articles, all_tags)?;
            }
        }
        Ok(())
    }

    fn process_article_file(
        path: &Path,
        category: Option<&str>,
        articles: &mut Vec<Article>,
        all_tags: &mut HashSet<String>,
    ) -> Result<(), LoadError> {
        let slug = path
            .file_stem()
            .and_then(|s| s.to_str())
            .map(String::from)
            .ok_or_else(|| LoadError::InvalidFileName(()))?;

        let file_content = fs::read_to_string(path)?;

        let matter = Matter::<YAML>::new();
        let parsed_content = matter
            .parse(&file_content)
            .map_err(|_e| LoadError::MatterParse(()))?;

        let data = parsed_content
            .data
            .ok_or_else(|| LoadError::MissingFrontMatter(()))?;
        let mut metadata: Metadata = from_value(data)?;

        // Set category from directory structure if nested categories are enabled
        if let Some(cat) = category {
            metadata.category = Some(cat.to_string());
        }

        if !metadata.draft {
            for tag in &metadata.tags {
                all_tags.insert(tag.clone());
            }
        }

        let content = parsed_content.content;
        articles.push(Article { slug, metadata, content });
        
        Ok(())
    }

    pub fn get_all_tags(&self) -> Vec<String> {
        let mut tags: Vec<String> = self.tags.iter().cloned().collect();
        tags.sort();
        tags
    }

    pub fn get_all_categories(&self) -> Vec<String> {
        let mut categories: Vec<String> = self.categories.iter().cloned().collect();
        categories.sort();
        categories
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


