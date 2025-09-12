use crate::models::article::Article;
use chrono::Utc;
use std::fs::{self, OpenOptions};
use std::io::{ErrorKind, Result, Write};
use std::path::Path;

pub fn save_version(article: &Article) -> Result<()> {
    let version_dir = format!("data/articles/{}/versions", article.slug);
    fs::create_dir_all(&version_dir)?;
    let version = Utc::now().timestamp_millis() as u64;
    let content = fs::read_to_string(&article.file_path)?;
    let mut candidate = version;
    loop {
        let version_file = Path::new(&version_dir).join(format!("{}.md", candidate));
        match OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&version_file)
        {
            Ok(mut file) => {
                file.write_all(content.as_bytes())?;
                break;
            }
            Err(e) if e.kind() == ErrorKind::AlreadyExists => {
                candidate += 1;
            }
            Err(e) => return Err(e),
        }
    }
    Ok(())
}
