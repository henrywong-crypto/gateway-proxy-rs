pub mod bedrock;
pub(crate) mod shared;
pub(crate) mod sse;

use actix_web::error::{ErrorBadGateway, ErrorBadRequest};
use actix_web::{web, HttpRequest, HttpResponse};
use sqlx::SqlitePool;

use shared::{
    build_forward_headers, build_stored_path, build_target_url, effective_client,
    forward_response_headers, get_session_or_error, headers_to_json, is_sse_content_type,
    log_request, parse_body_fields, store_response, to_actix_status, actix_headers_iter,
    RequestMeta,
};

pub async fn proxy_handler(
    req: HttpRequest,
    body: web::Bytes,
    pool: web::Data<SqlitePool>,
    client: web::Data<reqwest::Client>,
) -> Result<HttpResponse, actix_web::Error> {
    let full_path = req.match_info().get("tail").unwrap_or("");
    let session_id = req
        .match_info()
        .get("session_id")
        .ok_or_else(|| ErrorBadRequest("Missing session_id"))?;

    let session = get_session_or_error(pool.get_ref(), session_id).await?;

    let query = req.uri().query();
    let target_url = build_target_url(&session.target_url, full_path, query);
    let stored_path = build_stored_path(full_path, query);
    let method = req.method().to_string();
    let timestamp = chrono::Local::now().format("%H:%M:%S").to_string();

    log::info!("{} {} -> {} {}", session.name, method, stored_path, target_url);

    // Log request to DB
    let req_headers_json = headers_to_json(actix_headers_iter(&req));
    let url_model = {
        let parts: Vec<&str> = full_path.split('/').collect();
        parts
            .iter()
            .position(|&p| p == "model")
            .and_then(|pos| parts.get(pos + 1).map(|s| s.to_string()))
    };
    let (fields, note) = parse_body_fields(&body, url_model);
    let request_id = log_request(
        &RequestMeta {
            pool: pool.get_ref(),
            session_id,
            method: &method,
            path: &stored_path,
            timestamp: &timestamp,
            headers_json: req_headers_json.as_deref(),
            note: note.as_deref(),
        },
        &fields,
    )
    .await;

    // Forward the request upstream
    let forward_headers = build_forward_headers(
        &req,
        session.auth_header.as_deref(),
        "authorization",
    );
    let effective_client = effective_client(&session, client.get_ref());
    let parsed_method = reqwest::Method::from_bytes(method.as_bytes())
        .map_err(|e| ErrorBadRequest(format!("Invalid HTTP method: {}", e)))?;

    let upstream = effective_client
        .request(parsed_method, &target_url)
        .headers(forward_headers)
        .body(body.to_vec())
        .send()
        .await
        .map_err(|e| ErrorBadGateway(format!("Upstream error: {}", e)))?;

    // Build response
    let status = upstream.status().as_u16();
    let resp_headers_json = headers_to_json(upstream.headers().iter().filter_map(|(k, v)| {
        v.to_str().ok().map(|s| (k.to_string(), s.to_string()))
    }));
    let is_sse = is_sse_content_type(upstream.headers());
    let actix_status = to_actix_status(status)?;

    let mut builder = HttpResponse::build(actix_status);
    forward_response_headers(&mut builder, upstream.headers());

    let response_body = upstream
        .bytes()
        .await
        .map_err(|e| ErrorBadGateway(format!("Failed to read upstream response body: {}", e)))?;

    if let Some(ref req_id) = request_id {
        let body_str = String::from_utf8_lossy(&response_body);
        store_response(
            pool.get_ref(),
            req_id,
            status,
            resp_headers_json.as_deref(),
            &body_str,
            is_sse,
        )
        .await;
    }

    Ok(builder.body(response_body.to_vec()))
}
