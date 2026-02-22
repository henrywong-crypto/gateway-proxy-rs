use sqlx::sqlite::{SqlitePool, SqlitePoolOptions};

mod filters;
mod requests;
mod sessions;

pub use filters::*;
pub use requests::*;
pub use sessions::*;


pub async fn init_pool(db_path: &str) -> anyhow::Result<SqlitePool> {
    let pool = SqlitePoolOptions::new()
        .max_connections(5)
        .connect(&format!("sqlite:{}?mode=rwc", db_path))
        .await?;

    for stmt in include_str!("../../migrations/init.sql").split(';') {
        let stmt = stmt.trim();
        if !stmt.is_empty() {
            sqlx::query(stmt).execute(&pool).await?;
        }
    }

    ensure_default_profile(&pool).await?;

    Ok(pool)
}
