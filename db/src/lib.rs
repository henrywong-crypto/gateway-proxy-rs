use sqlx::sqlite::{SqlitePool, SqlitePoolOptions};
use uuid::Uuid;

mod filters;
mod requests;
mod sessions;

pub use filters::*;
pub use requests::*;
pub use sessions::*;

pub fn generate_id() -> String {
    Uuid::new_v4().to_string()
}

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

    Ok(pool)
}
