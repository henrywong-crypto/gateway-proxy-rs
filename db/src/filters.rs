use sqlx::sqlite::SqlitePool;

use common::models::{FilterProfile, SystemFilter, ToolFilter};

// -- Filter Profiles --

pub async fn count_profiles(pool: &SqlitePool) -> anyhow::Result<i64> {
    let row: (i64,) = sqlx::query_as("SELECT COUNT(*) FROM filter_profiles")
        .fetch_one(pool)
        .await?;
    Ok(row.0)
}

pub async fn list_profiles(pool: &SqlitePool) -> anyhow::Result<Vec<FilterProfile>> {
    Ok(sqlx::query_as::<_, FilterProfile>(
        "SELECT id, name, created_at FROM filter_profiles ORDER BY created_at ASC",
    )
    .fetch_all(pool)
    .await?)
}

pub async fn create_profile(pool: &SqlitePool, name: &str) -> anyhow::Result<uuid::Uuid> {
    let id = uuid::Uuid::new_v4();
    sqlx::query("INSERT INTO filter_profiles (id, name) VALUES (?, ?)")
        .bind(id.to_string())
        .bind(name)
        .execute(pool)
        .await?;
    Ok(id)
}

pub async fn delete_profile(pool: &SqlitePool, id: &str) -> anyhow::Result<()> {
    sqlx::query("DELETE FROM system_filters WHERE profile_id = ?")
        .bind(id)
        .execute(pool)
        .await?;
    sqlx::query("DELETE FROM tool_filters WHERE profile_id = ?")
        .bind(id)
        .execute(pool)
        .await?;
    sqlx::query("DELETE FROM filter_profiles WHERE id = ?")
        .bind(id)
        .execute(pool)
        .await?;
    Ok(())
}

pub async fn rename_profile(pool: &SqlitePool, id: &str, name: &str) -> anyhow::Result<()> {
    sqlx::query("UPDATE filter_profiles SET name = ? WHERE id = ?")
        .bind(name)
        .bind(id)
        .execute(pool)
        .await?;
    Ok(())
}

pub async fn get_profile(pool: &SqlitePool, id: &str) -> anyhow::Result<Option<FilterProfile>> {
    Ok(sqlx::query_as::<_, FilterProfile>(
        "SELECT id, name, created_at FROM filter_profiles WHERE id = ?",
    )
    .bind(id)
    .fetch_optional(pool)
    .await?)
}

pub async fn get_profile_by_name(pool: &SqlitePool, name: &str) -> anyhow::Result<Option<FilterProfile>> {
    Ok(sqlx::query_as::<_, FilterProfile>(
        "SELECT id, name, created_at FROM filter_profiles WHERE name = ?",
    )
    .bind(name)
    .fetch_optional(pool)
    .await?)
}

pub async fn count_system_filters(pool: &SqlitePool, profile_id: &str) -> anyhow::Result<i64> {
    let row: (i64,) = sqlx::query_as(
        "SELECT COUNT(*) FROM system_filters WHERE profile_id = ?",
    )
    .bind(profile_id)
    .fetch_one(pool)
    .await?;
    Ok(row.0)
}

pub async fn count_tool_filters(pool: &SqlitePool, profile_id: &str) -> anyhow::Result<i64> {
    let row: (i64,) = sqlx::query_as(
        "SELECT COUNT(*) FROM tool_filters WHERE profile_id = ?",
    )
    .bind(profile_id)
    .fetch_one(pool)
    .await?;
    Ok(row.0)
}

// -- Settings --

pub async fn get_setting(pool: &SqlitePool, key: &str) -> anyhow::Result<Option<String>> {
    let row: Option<(String,)> =
        sqlx::query_as("SELECT value FROM settings WHERE key = ?")
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

pub async fn get_active_profile_id(pool: &SqlitePool) -> anyhow::Result<String> {
    get_setting(pool, "active_profile_id")
        .await
        .map(|v| v.unwrap_or_default())
}

pub async fn set_active_profile_id(pool: &SqlitePool, profile_id: &str) -> anyhow::Result<()> {
    set_setting(pool, "active_profile_id", profile_id).await
}

/// Ensure a "default" profile exists and the active_profile_id setting points to a valid profile.
pub async fn ensure_default_profile(pool: &SqlitePool) -> anyhow::Result<()> {
    let profiles = list_profiles(pool).await?;
    let active_id = get_setting(pool, "active_profile_id").await?;

    // Check if active_profile_id points to an existing profile
    let active_valid = active_id
        .as_ref()
        .map(|aid| profiles.iter().any(|p| p.id.to_string() == *aid))
        .unwrap_or(false);

    if profiles.is_empty() {
        // Create default profile and set it active
        let id = create_profile(pool, "default").await?;
        set_active_profile_id(pool, &id.to_string()).await?;
    } else if !active_valid {
        // Point to first existing profile
        set_active_profile_id(pool, &profiles[0].id.to_string()).await?;
    }

    Ok(())
}

// -- System Filters --

pub async fn list_system_filters(
    pool: &SqlitePool,
    profile_id: &str,
) -> anyhow::Result<Vec<SystemFilter>> {
    Ok(sqlx::query_as::<_, SystemFilter>(
        "SELECT id, profile_id, pattern, created_at FROM system_filters WHERE profile_id = ? ORDER BY created_at DESC",
    )
    .bind(profile_id)
    .fetch_all(pool)
    .await?)
}

pub async fn add_system_filter(
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

pub const DEFAULT_FILTER_SUGGESTIONS: &[&str] = &[
    "^x-anthropic-billing-header: cc_version=",
    "^You are Claude Code, Anthropic's official CLI for Claude.$",
];

pub async fn get_system_filter(pool: &SqlitePool, id: &str) -> anyhow::Result<Option<SystemFilter>> {
    Ok(sqlx::query_as::<_, SystemFilter>(
        "SELECT id, profile_id, pattern, created_at FROM system_filters WHERE id = ?",
    )
    .bind(id)
    .fetch_optional(pool)
    .await?)
}

// -- Tool Filters --

pub async fn list_tool_filters(
    pool: &SqlitePool,
    profile_id: &str,
) -> anyhow::Result<Vec<ToolFilter>> {
    Ok(sqlx::query_as::<_, ToolFilter>(
        "SELECT id, profile_id, name, created_at FROM tool_filters WHERE profile_id = ? ORDER BY created_at DESC",
    )
    .bind(profile_id)
    .fetch_all(pool)
    .await?)
}

pub async fn get_tool_filter(pool: &SqlitePool, id: &str) -> anyhow::Result<Option<ToolFilter>> {
    Ok(sqlx::query_as::<_, ToolFilter>(
        "SELECT id, profile_id, name, created_at FROM tool_filters WHERE id = ?",
    )
    .bind(id)
    .fetch_optional(pool)
    .await?)
}

pub async fn add_tool_filter(
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

pub async fn delete_tool_filter(pool: &SqlitePool, id: uuid::Uuid) -> anyhow::Result<()> {
    sqlx::query("DELETE FROM tool_filters WHERE id = ?")
        .bind(id.to_string())
        .execute(pool)
        .await?;
    Ok(())
}

pub async fn update_tool_filter(
    pool: &SqlitePool,
    id: uuid::Uuid,
    name: &str,
) -> anyhow::Result<()> {
    sqlx::query("UPDATE tool_filters SET name = ? WHERE id = ?")
        .bind(name)
        .bind(id.to_string())
        .execute(pool)
        .await?;
    Ok(())
}

pub const DEFAULT_TOOL_FILTER_SUGGESTIONS: &[&str] = &["WebSearch"];
