use common::models::{FilterProfile, SystemFilter, ToolFilter};
use sqlx::sqlite::SqlitePool;

const PROFILE_COLUMNS: &str = "id, name, is_default, created_at";
const SYSTEM_FILTER_COLUMNS: &str = "id, profile_id, pattern, created_at";
const TOOL_FILTER_COLUMNS: &str = "id, profile_id, name, created_at";

// -- Filter Profiles --

pub async fn count_filter_profiles(pool: &SqlitePool) -> anyhow::Result<i64> {
    let row: (i64,) = sqlx::query_as("SELECT COUNT(*) FROM filter_profiles")
        .fetch_one(pool)
        .await?;
    Ok(row.0)
}

pub async fn list_filter_profiles(pool: &SqlitePool) -> anyhow::Result<Vec<FilterProfile>> {
    Ok(sqlx::query_as::<_, FilterProfile>(&format!(
        "SELECT {} FROM filter_profiles ORDER BY created_at ASC",
        PROFILE_COLUMNS
    ))
    .fetch_all(pool)
    .await?)
}

pub async fn create_filter_profile(pool: &SqlitePool, name: &str) -> anyhow::Result<uuid::Uuid> {
    let id = uuid::Uuid::new_v4();
    sqlx::query("INSERT INTO filter_profiles (id, name) VALUES (?, ?)")
        .bind(id.to_string())
        .bind(name)
        .execute(pool)
        .await?;
    Ok(id)
}

pub async fn delete_filter_profile(pool: &SqlitePool, id: &str) -> anyhow::Result<()> {
    sqlx::query("DELETE FROM filter_profiles WHERE id = ?")
        .bind(id)
        .execute(pool)
        .await?;
    Ok(())
}

pub async fn set_filter_profile_name(
    pool: &SqlitePool,
    id: &str,
    name: &str,
) -> anyhow::Result<()> {
    sqlx::query("UPDATE filter_profiles SET name = ? WHERE id = ?")
        .bind(name)
        .bind(id)
        .execute(pool)
        .await?;
    Ok(())
}

pub async fn get_filter_profile(
    pool: &SqlitePool,
    id: &str,
) -> anyhow::Result<Option<FilterProfile>> {
    Ok(sqlx::query_as::<_, FilterProfile>(&format!(
        "SELECT {} FROM filter_profiles WHERE id = ?",
        PROFILE_COLUMNS
    ))
    .bind(id)
    .fetch_optional(pool)
    .await?)
}

pub async fn get_filter_profile_by_name(
    pool: &SqlitePool,
    name: &str,
) -> anyhow::Result<Option<FilterProfile>> {
    Ok(sqlx::query_as::<_, FilterProfile>(&format!(
        "SELECT {} FROM filter_profiles WHERE name = ?",
        PROFILE_COLUMNS
    ))
    .bind(name)
    .fetch_optional(pool)
    .await?)
}

pub async fn get_default_filter_profile_id(pool: &SqlitePool) -> anyhow::Result<String> {
    let row: Option<(String,)> =
        sqlx::query_as("SELECT id FROM filter_profiles WHERE is_default = 1 LIMIT 1")
            .fetch_optional(pool)
            .await?;
    Ok(row.map(|r| r.0).unwrap_or_default())
}

pub async fn count_system_filters(pool: &SqlitePool, profile_id: &str) -> anyhow::Result<i64> {
    let row: (i64,) = sqlx::query_as("SELECT COUNT(*) FROM system_filters WHERE profile_id = ?")
        .bind(profile_id)
        .fetch_one(pool)
        .await?;
    Ok(row.0)
}

pub async fn count_tool_filters(pool: &SqlitePool, profile_id: &str) -> anyhow::Result<i64> {
    let row: (i64,) = sqlx::query_as("SELECT COUNT(*) FROM tool_filters WHERE profile_id = ?")
        .bind(profile_id)
        .fetch_one(pool)
        .await?;
    Ok(row.0)
}

// -- Settings --

pub async fn get_setting(pool: &SqlitePool, key: &str) -> anyhow::Result<Option<String>> {
    let row: Option<(String,)> = sqlx::query_as("SELECT value FROM settings WHERE key = ?")
        .bind(key)
        .fetch_optional(pool)
        .await?;
    Ok(row.map(|r| r.0))
}

pub async fn set_setting(pool: &SqlitePool, key: &str, value: &str) -> anyhow::Result<()> {
    sqlx::query(
        "INSERT INTO settings (key, value) VALUES (?, ?) ON CONFLICT(key) DO UPDATE SET value = excluded.value",
    )
    .bind(key)
    .bind(value)
    .execute(pool)
    .await?;
    Ok(())
}

/// Ensure a "default" profile with is_default=1 exists.
pub async fn ensure_default_filter_profile(pool: &SqlitePool) -> anyhow::Result<()> {
    let profiles = list_filter_profiles(pool).await?;

    let has_default = profiles.iter().any(|p| p.is_default);

    if profiles.is_empty() {
        // Create default profile and mark it as default
        let id = uuid::Uuid::new_v4();
        sqlx::query("INSERT INTO filter_profiles (id, name, is_default) VALUES (?, ?, 1)")
            .bind(id.to_string())
            .bind("default")
            .execute(pool)
            .await?;
    } else if !has_default {
        // Mark the first profile as default
        sqlx::query("UPDATE filter_profiles SET is_default = 1 WHERE id = ?")
            .bind(profiles[0].id.to_string())
            .execute(pool)
            .await?;
    }

    Ok(())
}

// -- System Filters --

pub async fn list_system_filters(
    pool: &SqlitePool,
    profile_id: &str,
) -> anyhow::Result<Vec<SystemFilter>> {
    Ok(sqlx::query_as::<_, SystemFilter>(&format!(
        "SELECT {} FROM system_filters WHERE profile_id = ? ORDER BY created_at DESC",
        SYSTEM_FILTER_COLUMNS
    ))
    .bind(profile_id)
    .fetch_all(pool)
    .await?)
}

pub async fn create_system_filter(
    pool: &SqlitePool,
    profile_id: &str,
    pattern: &str,
) -> anyhow::Result<()> {
    let id = uuid::Uuid::new_v4().to_string();
    sqlx::query("INSERT INTO system_filters (id, profile_id, pattern) VALUES (?, ?, ?)")
        .bind(&id)
        .bind(profile_id)
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

pub use common::models::DEFAULT_SYSTEM_FILTER_SUGGESTIONS as DEFAULT_FILTER_SUGGESTIONS;

pub async fn get_system_filter(
    pool: &SqlitePool,
    id: &str,
) -> anyhow::Result<Option<SystemFilter>> {
    Ok(sqlx::query_as::<_, SystemFilter>(&format!(
        "SELECT {} FROM system_filters WHERE id = ?",
        SYSTEM_FILTER_COLUMNS
    ))
    .bind(id)
    .fetch_optional(pool)
    .await?)
}

// -- Tool Filters --

pub async fn list_tool_filters(
    pool: &SqlitePool,
    profile_id: &str,
) -> anyhow::Result<Vec<ToolFilter>> {
    Ok(sqlx::query_as::<_, ToolFilter>(&format!(
        "SELECT {} FROM tool_filters WHERE profile_id = ? ORDER BY created_at DESC",
        TOOL_FILTER_COLUMNS
    ))
    .bind(profile_id)
    .fetch_all(pool)
    .await?)
}

pub async fn get_tool_filter(pool: &SqlitePool, id: &str) -> anyhow::Result<Option<ToolFilter>> {
    Ok(sqlx::query_as::<_, ToolFilter>(&format!(
        "SELECT {} FROM tool_filters WHERE id = ?",
        TOOL_FILTER_COLUMNS
    ))
    .bind(id)
    .fetch_optional(pool)
    .await?)
}

pub async fn create_tool_filter(
    pool: &SqlitePool,
    profile_id: &str,
    name: &str,
) -> anyhow::Result<()> {
    let id = uuid::Uuid::new_v4().to_string();
    sqlx::query("INSERT INTO tool_filters (id, profile_id, name) VALUES (?, ?, ?)")
        .bind(&id)
        .bind(profile_id)
        .bind(name)
        .execute(pool)
        .await?;
    Ok(())
}

pub async fn delete_tool_filter(pool: &SqlitePool, id: &str) -> anyhow::Result<()> {
    sqlx::query("DELETE FROM tool_filters WHERE id = ?")
        .bind(id)
        .execute(pool)
        .await?;
    Ok(())
}

pub async fn update_tool_filter(pool: &SqlitePool, id: &str, name: &str) -> anyhow::Result<()> {
    sqlx::query("UPDATE tool_filters SET name = ? WHERE id = ?")
        .bind(name)
        .bind(id)
        .execute(pool)
        .await?;
    Ok(())
}

pub use common::models::DEFAULT_TOOL_FILTER_SUGGESTIONS;

// -- Message Filters --

pub async fn get_filter_profile_keep_tool_pairs(
    pool: &SqlitePool,
    profile_id: &str,
) -> anyhow::Result<i64> {
    let row: Option<(i64,)> =
        sqlx::query_as("SELECT keep_tool_pairs FROM message_filters WHERE profile_id = ?")
            .bind(profile_id)
            .fetch_optional(pool)
            .await?;
    Ok(row.map(|r| r.0).unwrap_or(0))
}

pub async fn set_filter_profile_message_filter(
    pool: &SqlitePool,
    profile_id: &str,
    keep_tool_pairs: i64,
) -> anyhow::Result<()> {
    sqlx::query(
        "INSERT INTO message_filters (profile_id, keep_tool_pairs) VALUES (?, ?) ON CONFLICT(profile_id) DO UPDATE SET keep_tool_pairs = excluded.keep_tool_pairs",
    )
    .bind(profile_id)
    .bind(keep_tool_pairs)
    .execute(pool)
    .await?;
    Ok(())
}
