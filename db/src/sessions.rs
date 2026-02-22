use sqlx::sqlite::SqlitePool;

use common::models::Session;

pub async fn count_sessions(pool: &SqlitePool) -> anyhow::Result<i64> {
    let row: (i64,) = sqlx::query_as("SELECT COUNT(*) FROM sessions")
        .fetch_one(pool)
        .await?;
    Ok(row.0)
}

pub async fn list_sessions(pool: &SqlitePool) -> anyhow::Result<Vec<Session>> {
    Ok(sqlx::query_as::<_, Session>(
        "SELECT s.id, s.name, s.target_url, s.tls_verify_disabled, s.auth_header, s.x_api_key, s.created_at, \
         COALESCE((SELECT COUNT(*) FROM requests r WHERE r.session_id = s.id), 0) as request_count \
         FROM sessions s ORDER BY s.created_at DESC",
    )
    .fetch_all(pool)
    .await?)
}

pub async fn get_session(pool: &SqlitePool, id: &str) -> anyhow::Result<Option<Session>> {
    Ok(sqlx::query_as::<_, Session>(
        "SELECT s.id, s.name, s.target_url, s.tls_verify_disabled, s.auth_header, s.x_api_key, s.created_at, \
         COALESCE((SELECT COUNT(*) FROM requests r WHERE r.session_id = s.id), 0) as request_count \
         FROM sessions s WHERE s.id = ?",
    )
    .bind(id)
    .fetch_optional(pool)
    .await?)
}

pub async fn create_session(
    pool: &SqlitePool,
    id: &str,
    name: &str,
    target_url: &str,
    tls_verify_disabled: bool,
    auth_header: Option<&str>,
    x_api_key: Option<&str>,
) -> anyhow::Result<()> {
    sqlx::query(
        "INSERT INTO sessions (id, name, target_url, tls_verify_disabled, auth_header, x_api_key) VALUES (?, ?, ?, ?, ?, ?)",
    )
    .bind(id)
    .bind(name)
    .bind(target_url)
    .bind(tls_verify_disabled)
    .bind(auth_header)
    .bind(x_api_key)
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

pub async fn update_session(
    pool: &SqlitePool,
    id: &str,
    name: &str,
    target_url: &str,
    tls_verify_disabled: bool,
    auth_header: Option<&str>,
    x_api_key: Option<&str>,
) -> anyhow::Result<()> {
    sqlx::query(
        "UPDATE sessions SET name = ?, target_url = ?, tls_verify_disabled = ?, auth_header = ?, x_api_key = ? WHERE id = ?",
    )
    .bind(name)
    .bind(target_url)
    .bind(tls_verify_disabled)
    .bind(auth_header)
    .bind(x_api_key)
    .bind(id)
    .execute(pool)
    .await?;
    Ok(())
}
