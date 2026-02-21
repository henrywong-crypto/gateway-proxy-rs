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

    sqlx::query(include_str!("../../migrations/001_init.sql"))
        .execute(&pool)
        .await?;

    // Run migration 002: add response columns (ignore errors if columns already exist)
    for stmt in include_str!("../../migrations/002_add_response.sql").split(';') {
        let stmt = stmt.trim();
        if !stmt.is_empty() {
            let _ = sqlx::query(stmt).execute(&pool).await;
        }
    }

    // Run migration 003: add tls_verify_disabled column
    for stmt in include_str!("../../migrations/003_add_tls_verify_disabled.sql").split(';') {
        let stmt = stmt.trim();
        if !stmt.is_empty() {
            let _ = sqlx::query(stmt).execute(&pool).await;
        }
    }

    // Run migration 004: add system_filters table
    for stmt in include_str!("../../migrations/004_add_system_filters.sql").split(';') {
        let stmt = stmt.trim();
        if !stmt.is_empty() {
            let _ = sqlx::query(stmt).execute(&pool).await;
        }
    }

    // Run migration 005: add auth_header column
    for stmt in include_str!("../../migrations/005_add_auth_header.sql").split(';') {
        let stmt = stmt.trim();
        if !stmt.is_empty() {
            let _ = sqlx::query(stmt).execute(&pool).await;
        }
    }

    Ok(pool)
}
