use sqlx::{migrate::MigrateDatabase, sqlite::{Sqlite, SqlitePoolOptions}, SqlitePool};

pub type DbPool = SqlitePool;

pub async fn init_db(database_url: &str) -> Result<DbPool, sqlx::Error> {
    if !Sqlite::database_exists(database_url).await.unwrap_or(false) {
        Sqlite::create_database(database_url).await?;
    }

    let pool = SqlitePoolOptions::new()
        .max_connections(5)
        .connect(database_url)
        .await?;

    sqlx::migrate!("./migrations").run(&pool).await?;

    Ok(pool)
}
