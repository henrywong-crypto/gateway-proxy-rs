use sqlx::sqlite::{SqlitePool, SqlitePoolOptions};

use crate::models::{ProxyRequest, Session, SessionWithCount};

pub async fn init_pool(db_path: &str) -> Result<SqlitePool, sqlx::Error> {
    let pool = SqlitePoolOptions::new()
        .max_connections(5)
        .connect(&format!("sqlite:{}?mode=rwc", db_path))
        .await?;

    sqlx::query(include_str!("../migrations/001_init.sql"))
        .execute(&pool)
        .await?;

    // Run migration 002: add response columns (ignore errors if columns already exist)
    for stmt in include_str!("../migrations/002_add_response.sql").split(';') {
        let stmt = stmt.trim();
        if !stmt.is_empty() {
            let _ = sqlx::query(stmt).execute(&pool).await;
        }
    }

    // Run migration 003: add tls_verify_disabled column
    for stmt in include_str!("../migrations/003_add_tls_verify_disabled.sql").split(';') {
        let stmt = stmt.trim();
        if !stmt.is_empty() {
            let _ = sqlx::query(stmt).execute(&pool).await;
        }
    }

    Ok(pool)
}

pub async fn list_sessions(pool: &SqlitePool) -> Result<Vec<SessionWithCount>, sqlx::Error> {
    sqlx::query_as::<_, SessionWithCount>(
        "SELECT s.id, s.name, s.target_url, s.tls_verify_disabled, s.created_at, \
         COALESCE((SELECT COUNT(*) FROM requests r WHERE r.session_id = s.id), 0) as request_count \
         FROM sessions s ORDER BY s.created_at DESC",
    )
    .fetch_all(pool)
    .await
}

pub async fn get_session(pool: &SqlitePool, id: &str) -> Result<Option<Session>, sqlx::Error> {
    sqlx::query_as::<_, Session>("SELECT id, name, target_url, tls_verify_disabled, created_at FROM sessions WHERE id = ?")
        .bind(id)
        .fetch_optional(pool)
        .await
}

pub async fn create_session(
    pool: &SqlitePool,
    id: &str,
    name: &str,
    target_url: &str,
    tls_verify_disabled: bool,
) -> Result<(), sqlx::Error> {
    sqlx::query("INSERT INTO sessions (id, name, target_url, tls_verify_disabled) VALUES (?, ?, ?, ?)")
        .bind(id)
        .bind(name)
        .bind(target_url)
        .bind(tls_verify_disabled)
        .execute(pool)
        .await?;
    Ok(())
}

pub async fn list_requests(
    pool: &SqlitePool,
    session_id: &str,
) -> Result<Vec<ProxyRequest>, sqlx::Error> {
    sqlx::query_as::<_, ProxyRequest>(
        "SELECT id, session_id, method, path, timestamp, headers_json, body_json, \
         truncated_json, model, tools_json, messages_json, system_json, params_json, \
         note, created_at, response_status, response_headers_json, response_body, \
         response_events_json FROM requests WHERE session_id = ? ORDER BY id DESC",
    )
    .bind(session_id)
    .fetch_all(pool)
    .await
}

pub async fn get_request(
    pool: &SqlitePool,
    req_id: i64,
) -> Result<Option<ProxyRequest>, sqlx::Error> {
    sqlx::query_as::<_, ProxyRequest>(
        "SELECT id, session_id, method, path, timestamp, headers_json, body_json, \
         truncated_json, model, tools_json, messages_json, system_json, params_json, \
         note, created_at, response_status, response_headers_json, response_body, \
         response_events_json FROM requests WHERE id = ?",
    )
    .bind(req_id)
    .fetch_all(pool)
    .await
    .map(|mut v| v.pop())
}

pub async fn insert_request(
    pool: &SqlitePool,
    session_id: &str,
    method: &str,
    path: &str,
    timestamp: &str,
    headers_json: Option<&str>,
    body_json: Option<&str>,
    truncated_json: Option<&str>,
    model: Option<&str>,
    tools_json: Option<&str>,
    messages_json: Option<&str>,
    system_json: Option<&str>,
    params_json: Option<&str>,
    note: Option<&str>,
) -> Result<i64, sqlx::Error> {
    let result = sqlx::query(
        "INSERT INTO requests (session_id, method, path, timestamp, headers_json, body_json, \
         truncated_json, model, tools_json, messages_json, system_json, params_json, note) \
         VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)",
    )
    .bind(session_id)
    .bind(method)
    .bind(path)
    .bind(timestamp)
    .bind(headers_json)
    .bind(body_json)
    .bind(truncated_json)
    .bind(model)
    .bind(tools_json)
    .bind(messages_json)
    .bind(system_json)
    .bind(params_json)
    .bind(note)
    .execute(pool)
    .await?;
    Ok(result.last_insert_rowid())
}

pub async fn update_request_response(
    pool: &SqlitePool,
    request_id: i64,
    response_status: i64,
    response_headers_json: Option<&str>,
    response_body: Option<&str>,
    response_events_json: Option<&str>,
) -> Result<(), sqlx::Error> {
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

pub async fn clear_requests(pool: &SqlitePool, session_id: &str) -> Result<(), sqlx::Error> {
    sqlx::query("DELETE FROM requests WHERE session_id = ?")
        .bind(session_id)
        .execute(pool)
        .await?;
    Ok(())
}

pub async fn delete_session(pool: &SqlitePool, session_id: &str) -> Result<(), sqlx::Error> {
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
) -> Result<(), sqlx::Error> {
    sqlx::query("UPDATE sessions SET name = ?, target_url = ?, tls_verify_disabled = ? WHERE id = ?")
        .bind(name)
        .bind(target_url)
        .bind(tls_verify_disabled)
        .bind(id)
        .execute(pool)
        .await?;
    Ok(())
}
