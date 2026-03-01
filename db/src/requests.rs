use common::models::ProxyRequest;
use sqlx::sqlite::SqlitePool;

/// All columns for the `requests` table, used in SELECT queries.
const REQUEST_COLUMNS: &str = "\
    id, session_id, method, path, timestamp, headers_json, body_json, \
    truncated_json, model, tools_json, messages_json, system_json, params_json, \
    note, created_at, response_status, response_headers_json, response_body, \
    response_events_json, webfetch_first_response_body, webfetch_first_response_events_json, \
    webfetch_followup_body_json, webfetch_rounds_json";

pub struct CreateRequestParams<'a> {
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

pub async fn list_requests(
    pool: &SqlitePool,
    session_id: &str,
) -> anyhow::Result<Vec<ProxyRequest>> {
    Ok(sqlx::query_as::<_, ProxyRequest>(&format!(
        "SELECT {} FROM requests WHERE session_id = ? ORDER BY created_at DESC",
        REQUEST_COLUMNS
    ))
    .bind(session_id)
    .fetch_all(pool)
    .await?)
}

pub async fn count_requests(pool: &SqlitePool, session_id: &str) -> anyhow::Result<i64> {
    let row: (i64,) = sqlx::query_as("SELECT COUNT(*) FROM requests WHERE session_id = ?")
        .bind(session_id)
        .fetch_one(pool)
        .await?;
    Ok(row.0)
}

pub async fn list_requests_paginated(
    pool: &SqlitePool,
    session_id: &str,
    limit: i64,
    offset: i64,
) -> anyhow::Result<Vec<ProxyRequest>> {
    Ok(sqlx::query_as::<_, ProxyRequest>(&format!(
        "SELECT {} FROM requests WHERE session_id = ? ORDER BY created_at DESC LIMIT ? OFFSET ?",
        REQUEST_COLUMNS
    ))
    .bind(session_id)
    .bind(limit)
    .bind(offset)
    .fetch_all(pool)
    .await?)
}

pub async fn get_request(pool: &SqlitePool, req_id: &str) -> anyhow::Result<Option<ProxyRequest>> {
    Ok(sqlx::query_as::<_, ProxyRequest>(&format!(
        "SELECT {} FROM requests WHERE id = ?",
        REQUEST_COLUMNS
    ))
    .bind(req_id)
    .fetch_all(pool)
    .await?
    .pop())
}

pub async fn create_request(
    pool: &SqlitePool,
    params: &CreateRequestParams<'_>,
) -> anyhow::Result<String> {
    let id = uuid::Uuid::new_v4().to_string();
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

pub async fn set_request_response(
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

pub async fn set_request_note(
    pool: &SqlitePool,
    request_id: &str,
    note: &str,
) -> anyhow::Result<()> {
    sqlx::query("UPDATE requests SET note = ? WHERE id = ?")
        .bind(note)
        .bind(request_id)
        .execute(pool)
        .await?;
    Ok(())
}

pub async fn set_request_webfetch_data(
    pool: &SqlitePool,
    request_id: &str,
    webfetch_first_response_body: Option<&str>,
    webfetch_first_response_events_json: Option<&str>,
    webfetch_followup_body_json: Option<&str>,
    webfetch_rounds_json: Option<&str>,
) -> anyhow::Result<()> {
    sqlx::query(
        "UPDATE requests SET webfetch_first_response_body = ?, webfetch_first_response_events_json = ?, \
         webfetch_followup_body_json = ?, webfetch_rounds_json = ? WHERE id = ?",
    )
    .bind(webfetch_first_response_body)
    .bind(webfetch_first_response_events_json)
    .bind(webfetch_followup_body_json)
    .bind(webfetch_rounds_json)
    .bind(request_id)
    .execute(pool)
    .await?;
    Ok(())
}
