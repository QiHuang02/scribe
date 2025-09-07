use crate::models::article::Article;
use std::fs;
use std::io::Result;
use std::path::Path;

pub fn save_version(article: &Article) -> Result<()> {
    let version_dir = format!("data/articles/{}/versions", article.slug);
    fs::create_dir_all(&version_dir)?;
    let next_version = fs::read_dir(&version_dir)
        .map(|rd| rd.count() as u32 + 1)
        .unwrap_or(1);
    let content = fs::read_to_string(&article.file_path)?;
    let version_file = Path::new(&version_dir).join(format!("{}.md", next_version));
    fs::write(version_file, content)
}
