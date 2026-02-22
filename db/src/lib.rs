use sqlx::sqlite::{SqliteConnectOptions, SqlitePool, SqlitePoolOptions};
use std::str::FromStr;

mod filters;
mod requests;
mod sessions;

pub use filters::*;
pub use requests::*;
pub use sessions::*;

pub async fn init_pool(db_path: &str) -> anyhow::Result<SqlitePool> {
    let opts = SqliteConnectOptions::from_str(&format!("sqlite:{}?mode=rwc", db_path))?
        .pragma("foreign_keys", "ON");
    let pool = SqlitePoolOptions::new()
        .max_connections(5)
        .connect_with(opts)
        .await?;

    sqlx::migrate!("../migrations").run(&pool).await?;

    ensure_default_profile(&pool).await?;

    Ok(pool)
}
