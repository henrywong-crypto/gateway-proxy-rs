use common::models::Session;
use sqlx::sqlite::SqlitePool;

pub async fn count_sessions(pool: &SqlitePool) -> anyhow::Result<i64> {
    let row: (i64,) = sqlx::query_as("SELECT COUNT(*) FROM sessions")
        .fetch_one(pool)
        .await?;
    Ok(row.0)
}

pub async fn list_sessions(pool: &SqlitePool) -> anyhow::Result<Vec<Session>> {
    Ok(sqlx::query_as::<_, Session>(
        "SELECT s.id, s.name, s.target_url, s.tls_verify_disabled, s.auth_header, s.x_api_key, s.profile_id, s.error_inject, s.websearch_intercept, s.webfetch_intercept, s.websearch_whitelist, s.websearch_tool_names, s.webfetch_tool_names, s.created_at, \
         COALESCE((SELECT COUNT(*) FROM requests r WHERE r.session_id = s.id), 0) as request_count \
         FROM sessions s ORDER BY s.created_at DESC",
    )
    .fetch_all(pool)
    .await?)
}

pub async fn get_session(pool: &SqlitePool, id: &str) -> anyhow::Result<Option<Session>> {
    Ok(sqlx::query_as::<_, Session>(
        "SELECT s.id, s.name, s.target_url, s.tls_verify_disabled, s.auth_header, s.x_api_key, s.profile_id, s.error_inject, s.websearch_intercept, s.webfetch_intercept, s.websearch_whitelist, s.websearch_tool_names, s.webfetch_tool_names, s.created_at, \
         COALESCE((SELECT COUNT(*) FROM requests r WHERE r.session_id = s.id), 0) as request_count \
         FROM sessions s WHERE s.id = ?",
    )
    .bind(id)
    .fetch_optional(pool)
    .await?)
}

pub struct SessionParams<'a> {
    pub id: &'a str,
    pub name: &'a str,
    pub target_url: &'a str,
    pub tls_verify_disabled: bool,
    pub auth_header: Option<&'a str>,
    pub x_api_key: Option<&'a str>,
    pub profile_id: Option<&'a str>,
}

pub async fn create_session(pool: &SqlitePool, params: &SessionParams<'_>) -> anyhow::Result<()> {
    sqlx::query(
        "INSERT INTO sessions (id, name, target_url, tls_verify_disabled, auth_header, x_api_key, profile_id) VALUES (?, ?, ?, ?, ?, ?, ?)",
    )
    .bind(params.id)
    .bind(params.name)
    .bind(params.target_url)
    .bind(params.tls_verify_disabled)
    .bind(params.auth_header)
    .bind(params.x_api_key)
    .bind(params.profile_id)
    .execute(pool)
    .await?;
    Ok(())
}

pub async fn set_error_inject(
    pool: &SqlitePool,
    session_id: &str,
    error_inject: Option<&str>,
) -> anyhow::Result<()> {
    sqlx::query("UPDATE sessions SET error_inject = ? WHERE id = ?")
        .bind(error_inject)
        .bind(session_id)
        .execute(pool)
        .await?;
    Ok(())
}

pub async fn set_websearch_intercept(
    pool: &SqlitePool,
    session_id: &str,
    enabled: bool,
) -> anyhow::Result<()> {
    sqlx::query("UPDATE sessions SET websearch_intercept = ? WHERE id = ?")
        .bind(enabled)
        .bind(session_id)
        .execute(pool)
        .await?;
    Ok(())
}

pub async fn set_webfetch_intercept(
    pool: &SqlitePool,
    session_id: &str,
    enabled: bool,
) -> anyhow::Result<()> {
    sqlx::query("UPDATE sessions SET webfetch_intercept = ? WHERE id = ?")
        .bind(enabled)
        .bind(session_id)
        .execute(pool)
        .await?;
    Ok(())
}

pub async fn delete_session(pool: &SqlitePool, session_id: &str) -> anyhow::Result<()> {
    sqlx::query("DELETE FROM sessions WHERE id = ?")
        .bind(session_id)
        .execute(pool)
        .await?;
    Ok(())
}

pub async fn set_websearch_whitelist(
    pool: &SqlitePool,
    session_id: &str,
    whitelist: Option<&str>,
) -> anyhow::Result<()> {
    sqlx::query("UPDATE sessions SET websearch_whitelist = ? WHERE id = ?")
        .bind(whitelist)
        .bind(session_id)
        .execute(pool)
        .await?;
    Ok(())
}

pub async fn set_websearch_tool_names(
    pool: &SqlitePool,
    session_id: &str,
    tool_names: &str,
) -> anyhow::Result<()> {
    sqlx::query("UPDATE sessions SET websearch_tool_names = ? WHERE id = ?")
        .bind(tool_names)
        .bind(session_id)
        .execute(pool)
        .await?;
    Ok(())
}

pub async fn set_webfetch_tool_names(
    pool: &SqlitePool,
    session_id: &str,
    tool_names: &str,
) -> anyhow::Result<()> {
    sqlx::query("UPDATE sessions SET webfetch_tool_names = ? WHERE id = ?")
        .bind(tool_names)
        .bind(session_id)
        .execute(pool)
        .await?;
    Ok(())
}

pub async fn update_session(pool: &SqlitePool, params: &SessionParams<'_>) -> anyhow::Result<()> {
    sqlx::query(
        "UPDATE sessions SET name = ?, target_url = ?, tls_verify_disabled = ?, auth_header = ?, x_api_key = ?, profile_id = ? WHERE id = ?",
    )
    .bind(params.name)
    .bind(params.target_url)
    .bind(params.tls_verify_disabled)
    .bind(params.auth_header)
    .bind(params.x_api_key)
    .bind(params.profile_id)
    .bind(params.id)
    .execute(pool)
    .await?;
    Ok(())
}
