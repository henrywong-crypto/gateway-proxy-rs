use sqlx::sqlite::SqlitePool;

use common::models::SystemFilter;

use crate::generate_id;

pub async fn list_system_filters(
    pool: &SqlitePool,
) -> anyhow::Result<Vec<SystemFilter>> {
    Ok(sqlx::query_as::<_, SystemFilter>(
        "SELECT id, pattern, created_at FROM system_filters ORDER BY created_at DESC",
    )
    .fetch_all(pool)
    .await?)
}

pub async fn add_system_filter(
    pool: &SqlitePool,
    pattern: &str,
) -> anyhow::Result<()> {
    let id = generate_id();
    sqlx::query("INSERT INTO system_filters (id, pattern) VALUES (?, ?)")
        .bind(&id)
        .bind(pattern)
        .execute(pool)
        .await?;
    Ok(())
}

pub async fn delete_system_filter(pool: &SqlitePool, id: &str) -> anyhow::Result<()> {
    sqlx::query("DELETE FROM system_filters WHERE id = ?")
        .bind(id)
        .execute(pool)
        .await?;
    Ok(())
}

pub async fn update_system_filter(
    pool: &SqlitePool,
    id: &str,
    pattern: &str,
) -> anyhow::Result<()> {
    sqlx::query("UPDATE system_filters SET pattern = ? WHERE id = ?")
        .bind(pattern)
        .bind(id)
        .execute(pool)
        .await?;
    Ok(())
}

pub const DEFAULT_FILTER_SUGGESTIONS: &[&str] = &[
    "^x-anthropic-billing-header: cc_version=",
    "^You are Claude Code, Anthropic's official CLI for Claude.$",
];
