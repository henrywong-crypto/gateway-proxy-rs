use sqlx::sqlite::{SqlitePool, SqlitePoolOptions};
use uuid::Uuid;

use common::models::{ProxyRequest, Session, SessionWithCount, SystemFilter};

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

pub async fn list_sessions(pool: &SqlitePool) -> anyhow::Result<Vec<SessionWithCount>> {
    Ok(sqlx::query_as::<_, SessionWithCount>(
        "SELECT s.id, s.name, s.target_url, s.tls_verify_disabled, s.auth_header, s.created_at, \
         COALESCE((SELECT COUNT(*) FROM requests r WHERE r.session_id = s.id), 0) as request_count \
         FROM sessions s ORDER BY s.created_at DESC",
    )
    .fetch_all(pool)
    .await?)
}

pub async fn get_session(pool: &SqlitePool, id: &str) -> anyhow::Result<Option<Session>> {
    Ok(sqlx::query_as::<_, Session>(
        "SELECT id, name, target_url, tls_verify_disabled, auth_header, created_at FROM sessions WHERE id = ?",
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
) -> anyhow::Result<()> {
    sqlx::query(
        "INSERT INTO sessions (id, name, target_url, tls_verify_disabled, auth_header) VALUES (?, ?, ?, ?, ?)",
    )
    .bind(id)
    .bind(name)
    .bind(target_url)
    .bind(tls_verify_disabled)
    .bind(auth_header)
    .execute(pool)
    .await?;
    Ok(())
}

pub async fn list_requests(
    pool: &SqlitePool,
    session_id: &str,
) -> anyhow::Result<Vec<ProxyRequest>> {
    Ok(sqlx::query_as::<_, ProxyRequest>(
        "SELECT id, session_id, method, path, timestamp, headers_json, body_json, \
         truncated_json, model, tools_json, messages_json, system_json, params_json, \
         note, created_at, response_status, response_headers_json, response_body, \
         response_events_json FROM requests WHERE session_id = ? ORDER BY created_at DESC",
    )
    .bind(session_id)
    .fetch_all(pool)
    .await?)
}

pub async fn get_request(pool: &SqlitePool, req_id: &str) -> anyhow::Result<Option<ProxyRequest>> {
    Ok(sqlx::query_as::<_, ProxyRequest>(
        "SELECT id, session_id, method, path, timestamp, headers_json, body_json, \
         truncated_json, model, tools_json, messages_json, system_json, params_json, \
         note, created_at, response_status, response_headers_json, response_body, \
         response_events_json FROM requests WHERE id = ?",
    )
    .bind(req_id)
    .fetch_all(pool)
    .await?
    .pop())
}

pub struct InsertRequestParams<'a> {
    pub session_id: &'a str,
    pub method: &'a str,
    pub path: &'a str,
    pub timestamp: &'a str,
    pub headers_json: Option<&'a str>,
    pub body_json: Option<&'a str>,
    pub truncated_json: Option<&'a str>,
    pub model: Option<&'a str>,
    pub tools_json: Option<&'a str>,
    pub messages_json: Option<&'a str>,
    pub system_json: Option<&'a str>,
    pub params_json: Option<&'a str>,
    pub note: Option<&'a str>,
}

pub async fn insert_request(
    pool: &SqlitePool,
    params: &InsertRequestParams<'_>,
) -> anyhow::Result<String> {
    let id = generate_id();
    sqlx::query(
        "INSERT INTO requests (id, session_id, method, path, timestamp, headers_json, body_json, \
         truncated_json, model, tools_json, messages_json, system_json, params_json, note) \
         VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)",
    )
    .bind(&id)
    .bind(params.session_id)
    .bind(params.method)
    .bind(params.path)
    .bind(params.timestamp)
    .bind(params.headers_json)
    .bind(params.body_json)
    .bind(params.truncated_json)
    .bind(params.model)
    .bind(params.tools_json)
    .bind(params.messages_json)
    .bind(params.system_json)
    .bind(params.params_json)
    .bind(params.note)
    .execute(pool)
    .await?;
    Ok(id)
}

pub async fn update_request_response(
    pool: &SqlitePool,
    request_id: &str,
    response_status: i64,
    response_headers_json: Option<&str>,
    response_body: Option<&str>,
    response_events_json: Option<&str>,
) -> anyhow::Result<()> {
    sqlx::query(
        "UPDATE requests SET response_status = ?, response_headers_json = ?, \
         response_body = ?, response_events_json = ? WHERE id = ?",
    )
    .bind(response_status)
    .bind(response_headers_json)
    .bind(response_body)
    .bind(response_events_json)
    .bind(request_id)
    .execute(pool)
    .await?;
    Ok(())
}

pub async fn clear_requests(pool: &SqlitePool, session_id: &str) -> anyhow::Result<()> {
    sqlx::query("DELETE FROM requests WHERE session_id = ?")
        .bind(session_id)
        .execute(pool)
        .await?;
    Ok(())
}

pub async fn delete_session(pool: &SqlitePool, session_id: &str) -> anyhow::Result<()> {
    sqlx::query("DELETE FROM requests WHERE session_id = ?")
        .bind(session_id)
        .execute(pool)
        .await?;
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
) -> anyhow::Result<()> {
    sqlx::query(
        "UPDATE sessions SET name = ?, target_url = ?, tls_verify_disabled = ?, auth_header = ? WHERE id = ?",
    )
    .bind(name)
    .bind(target_url)
    .bind(tls_verify_disabled)
    .bind(auth_header)
    .bind(id)
    .execute(pool)
    .await?;
    Ok(())
}

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
