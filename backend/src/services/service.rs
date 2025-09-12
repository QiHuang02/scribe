use crate::handlers::error::LoadError;
use crate::models::article::{Article, ArticleContent, Metadata};
use chrono::{DateTime, Utc};
use gray_matter::Matter;
use gray_matter::engine::YAML;
use serde_yaml::from_value;
use std::collections::{HashMap, HashSet};
use std::fs;
use std::path::Path;
use std::sync::Mutex;
use std::time::SystemTime;
use walkdir::WalkDir;

pub struct ArticleStore {
    articles: Vec<Article>,
    slug_map: HashMap<String, usize>,
    pub tags: HashSet<String>,
    pub categories: HashSet<String>,
    file_cache: HashMap<String, SystemTime>,
    content_cache: Mutex<HashMap<String, String>>,
}

#[derive(Debug)]
pub enum FileChange {
    Added,
    Modified,
    Removed,
}

#[derive(Debug)]
pub struct FileChangeInfo {
    pub path: String,
    pub change_type: FileChange,
}

impl ArticleStore {
    pub fn new(content_dir: &str, enable_nested_categories: bool) -> Result<Self, LoadError> {
        let mut articles = Vec::new();
        let mut all_tags = HashSet::new();
        let mut all_categories = HashSet::new();

        if enable_nested_categories {
            Self::load_articles_recursive(
                content_dir,
                &mut articles,
                &mut all_tags,
                &mut all_categories,
            )?;
        } else {
            Self::load_articles_flat(content_dir, &mut articles, &mut all_tags)?;
        }

        articles.sort_by(|a, b| b.metadata.date.cmp(&a.metadata.date));

        let slug_map = articles
            .iter()
            .enumerate()
            .map(|(idx, article)| (article.slug.clone(), idx))
            .collect();

        let mut file_cache = HashMap::new();
        for article in &articles {
            file_cache.insert(article.file_path.clone(), article.last_modified);
        }

        Ok(Self {
            articles,
            slug_map,
            tags: all_tags,
            categories: all_categories,
            file_cache,
            content_cache: Mutex::new(HashMap::new()),
        })
    }

    pub fn detect_file_changes(
        &self,
        content_dir: &str,
        enable_nested_categories: bool,
    ) -> Result<Vec<FileChangeInfo>, LoadError> {
        let mut changes = Vec::new();
        let current_files = self.collect_all_files(content_dir, enable_nested_categories)?;

        for file_path in &current_files {
            match fs::metadata(file_path) {
                Ok(metadata) => {
                    let modified_time = metadata.modified().unwrap_or(SystemTime::UNIX_EPOCH);

                    if let Some(&cached_time) = self.file_cache.get(file_path) {
                        if modified_time > cached_time {
                            changes.push(FileChangeInfo {
                                path: file_path.clone(),
                                change_type: FileChange::Modified,
                            });
                        }
                    } else {
                        changes.push(FileChangeInfo {
                            path: file_path.clone(),
                            change_type: FileChange::Added,
                        });
                    }
                }
                Err(_) => continue,
            }
        }

        for cached_path in self.file_cache.keys() {
            if !current_files.contains(cached_path) {
                changes.push(FileChangeInfo {
                    path: cached_path.clone(),
                    change_type: FileChange::Removed,
                });
            }
        }

        Ok(changes)
    }

    pub fn incremental_update(
        &mut self,
        content_dir: &str,
        enable_nested_categories: bool,
    ) -> Result<bool, LoadError> {
        let changes = self.detect_file_changes(content_dir, enable_nested_categories)?;

        if changes.is_empty() {
            return Ok(false);
        }

        tracing::info!("Detected {} file changes", changes.len());

        let mut articles_changed = false;

        for change in changes {
            match change.change_type {
                FileChange::Added | FileChange::Modified => {
                    if let Err(e) =
                        self.update_single_article(&change.path, enable_nested_categories)
                    {
                        tracing::warn!("Failed to update article {}: {:?}", change.path, e);
                        continue;
                    }
                    articles_changed = true;
                }
                FileChange::Removed => {
                    if self.remove_article_by_path(&change.path) {
                        articles_changed = true;
                    }
                }
            }
        }

        if articles_changed {
            self.rebuild_indexes();
            self.update_file_cache(content_dir, enable_nested_categories)?;
        }

        Ok(articles_changed)
    }

    fn collect_all_files(
        &self,
        content_dir: &str,
        enable_nested_categories: bool,
    ) -> Result<HashSet<String>, LoadError> {
        let mut file_set = HashSet::new();
        if enable_nested_categories {
            self.collect_files_recursive(content_dir, &mut file_set)?;
        } else {
            self.collect_files_flat(content_dir, &mut file_set)?;
        }
        Ok(file_set)
    }

    fn collect_files_flat(
        &self,
        content_dir: &str,
        file_set: &mut HashSet<String>,
    ) -> Result<(), LoadError> {
        let entries = fs::read_dir(Path::new(content_dir))?;

        for entry in entries {
            let path = entry?.path();
            if path.is_file()
                && path.extension().is_some_and(|s| s == "md")
                && let Some(path_str) = path.to_str()
            {
                file_set.insert(path_str.to_string());
            }
        }
        Ok(())
    }

    fn collect_files_recursive(
        &self,
        content_dir: &str,
        file_set: &mut HashSet<String>,
    ) -> Result<(), LoadError> {
        for entry in WalkDir::new(content_dir).into_iter().filter_map(|e| e.ok()) {
            let path = entry.path();
            if path.is_file()
                && path.extension().is_some_and(|s| s == "md")
                && let Some(path_str) = path.to_str()
            {
                file_set.insert(path_str.to_string());
            }
        }
        Ok(())
    }

    fn update_single_article(
        &mut self,
        file_path: &str,
        enable_nested_categories: bool,
    ) -> Result<(), LoadError> {
        self.content_cache.lock().unwrap().remove(file_path);
        let path = Path::new(file_path);

        let category = if enable_nested_categories {
            let content_dir = file_path
                .split('/')
                .next()
                .unwrap_or("")
                .split('\\')
                .next()
                .unwrap_or("");
            self.calculate_category_from_path(path, content_dir)
        } else {
            None
        };

        let mut temp_articles = Vec::new();
        let mut temp_tags = HashSet::new();
        Self::process_article_file(
            path,
            category.as_deref(),
            &mut temp_articles,
            &mut temp_tags,
        )?;

        if let Some(new_article) = temp_articles.into_iter().next() {
            if let Some(existing_index) = self
                .articles
                .iter()
                .position(|a| a.slug == new_article.slug)
            {
                self.articles[existing_index] = new_article;
            } else {
                self.articles.push(new_article);
            }

            for tag in &temp_tags {
                self.tags.insert(tag.clone());
            }
            if let Some(ref cat) = category {
                self.categories.insert(cat.clone());
            }
        }

        Ok(())
    }

    fn remove_article_by_path(&mut self, file_path: &str) -> bool {
        self.content_cache.lock().unwrap().remove(file_path);
        if let Some(article) = self.articles.iter_mut().find(|a| a.file_path == file_path) {
            article.deleted = true;
            self.slug_map.remove(&article.slug);
            tracing::info!("Soft deleted article: {}", article.slug);
            return true;
        }
        false
    }

    fn rebuild_indexes(&mut self) {
        self.articles
            .sort_by(|a, b| b.metadata.date.cmp(&a.metadata.date));

        self.slug_map = self
            .articles
            .iter()
            .enumerate()
            .filter(|(_, a)| !a.deleted)
            .map(|(idx, article)| (article.slug.clone(), idx))
            .collect();
    }

    fn update_file_cache(
        &mut self,
        content_dir: &str,
        enable_nested_categories: bool,
    ) -> Result<(), LoadError> {
        self.file_cache.clear();

        let current_files = self.collect_all_files(content_dir, enable_nested_categories)?;

        for file_path in current_files {
            if let Ok(metadata) = fs::metadata(&file_path)
                && let Ok(modified_time) = metadata.modified()
            {
                self.file_cache.insert(file_path, modified_time);
            }
        }

        self.content_cache
            .lock()
            .unwrap()
            .retain(|path, _| self.file_cache.contains_key(path));

        Ok(())
    }

    fn calculate_category(path: &Path, base_path: &Path) -> Option<String> {
        if let Some(parent) = path.parent() {
            if parent != base_path {
                parent
                    .strip_prefix(base_path)
                    .ok()
                    .and_then(|p| p.to_str())
                    .map(|s| s.replace(std::path::MAIN_SEPARATOR, "/"))
            } else {
                None
            }
        } else {
            None
        }
    }

    fn calculate_category_from_path(&self, path: &Path, base_path: &str) -> Option<String> {
        let base = Path::new(base_path);
        Self::calculate_category(path, base)
    }

    fn load_articles_flat(
        content_dir: &str,
        articles: &mut Vec<Article>,
        all_tags: &mut HashSet<String>,
    ) -> Result<(), LoadError> {
        let entries = fs::read_dir(Path::new(content_dir))?;

        for entry in entries {
            let path = entry?.path();

            if path.is_file() && path.extension().is_some_and(|s| s == "md") {
                Self::process_article_file(&path, None, articles, all_tags)?;
            }
        }
        Ok(())
    }

    fn load_articles_recursive(
        content_dir: &str,
        articles: &mut Vec<Article>,
        all_tags: &mut HashSet<String>,
        all_categories: &mut HashSet<String>,
    ) -> Result<(), LoadError> {
        let base_path = Path::new(content_dir);

        for entry in WalkDir::new(content_dir).into_iter().filter_map(|e| e.ok()) {
            let path = entry.path();

            if path.is_file() && path.extension().is_some_and(|s| s == "md") {
                // Calculate category from relative path
                let category = Self::calculate_category(path, base_path);

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
            .ok_or_else(|| LoadError::InvalidFileName(path.to_string_lossy().to_string()))?;

        if slug.eq_ignore_ascii_case("readme") {
            return Ok(());
        }

        let file_content = fs::read_to_string(path)?;

        let matter = Matter::<YAML>::new();
        let parsed_content = matter
            .parse::<serde_yaml::Value>(&file_content)
            .map_err(|e| {
                LoadError::MatterParse(format!(
                    "Failed to parse front matter in {}: {}",
                    path.to_string_lossy(),
                    e
                ))
            })?;

        let data = parsed_content
            .data
            .ok_or_else(|| LoadError::MissingFrontMatter(path.to_string_lossy().to_string()))?;
        let mut metadata: Metadata = from_value(data)?;

        if let Some(cat) = category {
            metadata.category = Some(cat.to_string());
        }

        if !metadata.draft {
            for tag in &metadata.tags {
                all_tags.insert(tag.clone());
            }
        }

        let last_modified = fs::metadata(path)
            .and_then(|m| m.modified())
            .unwrap_or(SystemTime::UNIX_EPOCH);
        let updated_at: DateTime<Utc> = last_modified.into();
        let version_dir = format!("data/articles/{}/versions", slug);
        let version = fs::read_dir(&version_dir)
            .map(|rd| rd.count() as u32 + 1)
            .unwrap_or(1);

        articles.push(Article {
            slug,
            metadata,
            version,
            updated_at,
            file_path: path.to_string_lossy().to_string(),
            last_modified,
            deleted: false,
        });

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
        self.slug_map
            .get(slug)
            .and_then(|&idx| self.articles.get(idx))
    }

    pub fn query<F>(&self, filter: F) -> Vec<&Article>
    where
        F: Fn(&Article) -> bool,
    {
        self.articles
            .iter()
            .filter(|a| !a.deleted)
            .filter(|a| filter(a))
            .collect()
    }

    pub fn load_content_for(&self, article: &Article) -> Result<String, LoadError> {
        {
            let cache = self.content_cache.lock().unwrap();
            if let Some(content) = cache.get(&article.file_path) {
                return Ok(content.clone());
            }
        }

        let file_content = fs::read_to_string(&article.file_path)?;
        let matter = Matter::<YAML>::new();
        let parsed_content = matter
            .parse::<serde_yaml::Value>(&file_content)
            .map_err(|e| {
                LoadError::MatterParse(format!(
                    "Failed to parse front matter in {}: {}",
                    article.file_path, e
                ))
            })?;
        let content = parsed_content.content;
        self.content_cache
            .lock()
            .unwrap()
            .insert(article.file_path.clone(), content.clone());
        Ok(content)
    }

    pub fn load_full_articles(&self) -> Vec<ArticleContent> {
        let mut loaded = Vec::new();

        for article in self.articles.iter().filter(|a| !a.deleted) {
            match self.load_content_for(article) {
                Ok(content) => loaded.push(ArticleContent {
                    slug: article.slug.clone(),
                    metadata: article.metadata.clone(),
                    content,
                }),
                Err(e) => {
                    tracing::warn!(
                        "Failed to load content for article {}: {:?}",
                        article.slug,
                        e
                    );
                }
            }
        }

        loaded
    }
}
