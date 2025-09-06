use crate::handlers::error::LoadError;
use crate::models::article::{Article, Metadata};
use gray_matter::engine::YAML;
use gray_matter::Matter;
use serde_yaml::from_value;
use std::collections::{HashMap, HashSet};
use std::fs;
use std::path::Path;
use std::time::SystemTime;
use walkdir::WalkDir;

pub struct ArticleStore {
    articles: Vec<Article>,
    slug_map: HashMap<String, usize>,
    pub tags: HashSet<String>,
    pub categories: HashSet<String>,
    // 智能缓存相关字段
    file_cache: HashMap<String, SystemTime>, // 文件路径 -> 最后修改时间
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

        // 初始化文件缓存
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
        })
    }

    // 智能缓存：检测文件变化
    pub fn detect_file_changes(&self, content_dir: &str, article_extension: &str, enable_nested_categories: bool) -> Result<Vec<FileChangeInfo>, LoadError> {
        let mut changes = Vec::new();
        // 收集当前目录中的所有文件
        let current_files = self.collect_all_files(content_dir, article_extension, enable_nested_categories)?;

        // 检查新增或修改的文件
        for file_path in &current_files {
            match fs::metadata(file_path) {
                Ok(metadata) => {
                    let modified_time = metadata.modified().unwrap_or(SystemTime::UNIX_EPOCH);

                    if let Some(&cached_time) = self.file_cache.get(file_path) {
                        // 文件存在于缓存中，检查是否被修改
                        if modified_time > cached_time {
                            changes.push(FileChangeInfo {
                                path: file_path.clone(),
                                change_type: FileChange::Modified,
                            });
                        }
                    } else {
                        // 新文件
                        changes.push(FileChangeInfo {
                            path: file_path.clone(),
                            change_type: FileChange::Added,
                        });
                    }
                }
                Err(_) => continue,
            }
        }

        // 检查被删除的文件
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

    // 增量更新：只处理变更的文件
    pub fn incremental_update(&mut self, content_dir: &str, article_extension: &str, enable_nested_categories: bool) -> Result<bool, LoadError> {
        let changes = self.detect_file_changes(content_dir, article_extension, enable_nested_categories)?;

        if changes.is_empty() {
            return Ok(false); // 没有变化
        }

        tracing::info!("Detected {} file changes", changes.len());

        let mut articles_changed = false;

        for change in changes {
            match change.change_type {
                FileChange::Added | FileChange::Modified => {
                    if let Err(e) = self.update_single_article(&change.path, article_extension, enable_nested_categories) {
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
            // 重新构建索引和排序
            self.rebuild_indexes();
            // 更新文件缓存
            self.update_file_cache(content_dir, article_extension, enable_nested_categories)?;
        }

        Ok(articles_changed)
    }

    // 收集所有文件路径
    fn collect_all_files(&self, content_dir: &str, article_extension: &str, enable_nested_categories: bool) -> Result<HashSet<String>, LoadError> {
        let mut file_set = HashSet::new();
        if enable_nested_categories {
            self.collect_files_recursive(content_dir, article_extension, &mut file_set)?;
        } else {
            self.collect_files_flat(content_dir, article_extension, &mut file_set)?;
        }
        Ok(file_set)
    }

    // 收集文件路径（平面结构）
    fn collect_files_flat(&self, content_dir: &str, article_extension: &str, file_set: &mut HashSet<String>) -> Result<(), LoadError> {
        let entries = fs::read_dir(Path::new(content_dir))?;

        for entry in entries {
            let path = entry?.path();
            if path.is_file() && path.extension().is_some_and(|s| s == article_extension)
                && let Some(path_str) = path.to_str() {
                file_set.insert(path_str.to_string());
            }
        }
        Ok(())
    }

    // 收集文件路径（递归结构）
    fn collect_files_recursive(&self, content_dir: &str, article_extension: &str, file_set: &mut HashSet<String>) -> Result<(), LoadError> {
        for entry in WalkDir::new(content_dir).into_iter().filter_map(|e| e.ok()) {
            let path = entry.path();
            if path.is_file() && path.extension().is_some_and(|s| s == article_extension)
                && let Some(path_str) = path.to_str() {
                file_set.insert(path_str.to_string());
            }
        }
        Ok(())
    }

    // 更新单篇文章
    fn update_single_article(&mut self, file_path: &str, _article_extension: &str, enable_nested_categories: bool) -> Result<(), LoadError> {
        let path = Path::new(file_path);

        // 计算分类（如果启用嵌套分类）
        let category = if enable_nested_categories {
            // 获取文件所在的根目录
            let content_dir = file_path.split('/').next().unwrap_or("")
                .split('\\').next().unwrap_or(""); // 处理Windows路径
            self.calculate_category_from_path(path, content_dir)
        } else {
            None
        };

        // 解析文章
        let mut temp_articles = Vec::new();
        let mut temp_tags = HashSet::new();
        Self::process_article_file(path, category.as_deref(), &mut temp_articles, &mut temp_tags)?;

        if let Some(new_article) = temp_articles.into_iter().next() {
            // 查找是否已存在相同slug的文章
            if let Some(existing_index) = self.articles.iter().position(|a| a.slug == new_article.slug) {
                // 更新现有文章
                self.articles[existing_index] = new_article;
            } else {
                // 添加新文章
                self.articles.push(new_article);
            }

            // 更新标签和分类集合
            for tag in &temp_tags {
                self.tags.insert(tag.clone());
            }
            if let Some(ref cat) = category {
                self.categories.insert(cat.clone());
            }
        }

        Ok(())
    }

    // 根据文件路径移除文章
    fn remove_article_by_path(&mut self, file_path: &str) -> bool {
        // 找到要删除的文章索引
        if let Some(index) = self.articles.iter().position(|a| a.file_path == file_path) {
            let removed_article = self.articles.remove(index);
            tracing::info!("Removed article: {}", removed_article.slug);

            // 清理可能的孤立标签和分类（简化实现，不做深度清理）
            // 在实际应用中，可以选择重新扫描所有文章来清理孤立的标签和分类

            return true;
        }
        false
    }

    // 重新构建所有索引
    fn rebuild_indexes(&mut self) {
        // 重新排序文章
        self.articles.sort_by(|a, b| b.metadata.date.cmp(&a.metadata.date));

        // 重建slug映射
        self.slug_map = self.articles
            .iter()
            .enumerate()
            .map(|(idx, article)| (article.slug.clone(), idx))
            .collect();
    }

    // 更新文件缓存
    fn update_file_cache(&mut self, content_dir: &str, article_extension: &str, enable_nested_categories: bool) -> Result<(), LoadError> {
        self.file_cache.clear();

        let current_files = self.collect_all_files(content_dir, article_extension, enable_nested_categories)?;

        for file_path in current_files {
            if let Ok(metadata) = fs::metadata(&file_path)
                && let Ok(modified_time) = metadata.modified() {
                self.file_cache.insert(file_path, modified_time);
            }
        }

        Ok(())
    }

    // 计算分类的静态辅助方法
    fn calculate_category(path: &Path, base_path: &Path) -> Option<String> {
        if let Some(parent) = path.parent() {
            if parent != base_path {
                parent.strip_prefix(base_path)
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

    // 从路径计算分类（实例方法）
    fn calculate_category_from_path(&self, path: &Path, base_path: &str) -> Option<String> {
        let base = Path::new(base_path);
        Self::calculate_category(path, base)
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

            if path.is_file() && path.extension().is_some_and(|s| s == article_extension) {
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

            if path.is_file() && path.extension().is_some_and(|s| s == article_extension) {
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
            .ok_or_else(|| LoadError::InvalidFileName(
                path.to_string_lossy().to_string()
            ))?;

        let file_content = fs::read_to_string(path)?;

        let matter = Matter::<YAML>::new();
        let parsed_content = matter
            .parse(&file_content)
            .map_err(|e| LoadError::MatterParse(
                format!("Failed to parse front matter in {}: {}", path.to_string_lossy(), e)
            ))?;

        let data = parsed_content
            .data
            .ok_or_else(|| LoadError::MissingFrontMatter(
                path.to_string_lossy().to_string()
            ))?;
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

        // 获取文件的修改时间
        let last_modified = fs::metadata(path)
            .and_then(|m| m.modified())
            .unwrap_or(SystemTime::UNIX_EPOCH);

        let content = parsed_content.content;
        articles.push(Article {
            slug,
            metadata,
            content,
            file_path: path.to_string_lossy().to_string(),
            last_modified,
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
        self.slug_map.get(slug).and_then(|&idx| self.articles.get(idx))
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