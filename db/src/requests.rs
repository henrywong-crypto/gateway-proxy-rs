use common::models::ProxyRequest;
use sqlx::sqlite::SqlitePool;

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

pub async fn list_requests(
    pool: &SqlitePool,
    session_id: &str,
) -> anyhow::Result<Vec<ProxyRequest>> {
    Ok(sqlx::query_as::<_, ProxyRequest>(
        "SELECT id, session_id, method, path, timestamp, headers_json, body_json, \
         truncated_json, model, tools_json, messages_json, system_json, params_json, \
         note, created_at, response_status, response_headers_json, response_body, \
         response_events_json, ws_first_response_body, ws_first_response_events_json, \
         ws_followup_body_json, ws_rounds_json FROM requests WHERE session_id = ? ORDER BY created_at DESC",
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
         response_events_json, ws_first_response_body, ws_first_response_events_json, \
         ws_followup_body_json, ws_rounds_json FROM requests WHERE id = ?",
    )
    .bind(req_id)
    .fetch_all(pool)
    .await?
    .pop())
}

pub async fn insert_request(
    pool: &SqlitePool,
    params: &InsertRequestParams<'_>,
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

pub async fn update_request_note(
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

pub async fn update_websearch_data(
    pool: &SqlitePool,
    request_id: &str,
    ws_first_response_body: Option<&str>,
    ws_first_response_events_json: Option<&str>,
    ws_followup_body_json: Option<&str>,
    ws_rounds_json: Option<&str>,
) -> anyhow::Result<()> {
    sqlx::query(
        "UPDATE requests SET ws_first_response_body = ?, ws_first_response_events_json = ?, \
         ws_followup_body_json = ?, ws_rounds_json = ? WHERE id = ?",
    )
    .bind(ws_first_response_body)
    .bind(ws_first_response_events_json)
    .bind(ws_followup_body_json)
    .bind(ws_rounds_json)
    .bind(request_id)
    .execute(pool)
    .await?;
    Ok(())
}
