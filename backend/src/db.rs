use sqlx::{sqlite::SqlitePoolOptions, SqlitePool};

pub type DbPool = SqlitePool;

pub async fn init_db(database_url: &str) -> Result<DbPool, sqlx::Error> {
    let pool = SqlitePoolOptions::new()
        .max_connections(5)
        .connect(database_url)
        .await?;

    sqlx::query(
        "CREATE TABLE IF NOT EXISTS comments (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            article_slug TEXT NOT NULL,
            author_github_id INTEGER NOT NULL,
            content TEXT NOT NULL,
            created_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP
        )",
    )
    .execute(&pool)
    .await?;

    sqlx::query(
        "CREATE INDEX IF NOT EXISTS idx_comments_article_slug ON comments(article_slug)",
    )
    .execute(&pool)
    .await?;

    sqlx::query(
        "CREATE INDEX IF NOT EXISTS idx_comments_author_github_id ON comments(author_github_id)",
    )
    .execute(&pool)
    .await?;

    Ok(pool)
}
