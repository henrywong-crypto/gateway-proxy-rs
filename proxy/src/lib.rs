pub mod bedrock;
pub(crate) mod shared;
pub(crate) mod sse;

use actix_web::error::{ErrorBadGateway, ErrorBadRequest};
use actix_web::{web, HttpRequest, HttpResponse};
use sqlx::SqlitePool;

use shared::{
    actix_headers_iter, effective_client, extract_request_fields, forward_response_headers,
    get_session_or_error, headers_to_json, is_sse_content_type, log_request, store_response,
    to_actix_status, RequestMeta,
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

    // Build target URL
    let target_path = if full_path.is_empty() {
        String::new()
    } else {
        format!("/{}", full_path)
    };
    let mut target_url = format!(
        "{}{}",
        session.target_url.trim_end_matches('/'),
        target_path
    );
    if let Some(qs) = req.uri().query() {
        target_url.push('?');
        target_url.push_str(qs);
    }

    // Log the request
    let timestamp = chrono::Local::now().format("%H:%M:%S").to_string();
    let method = req.method().to_string();
    let stored_path = {
        let p = format!("/{}", full_path);
        if let Some(qs) = req.uri().query() {
            format!("{}?{}", p, qs)
        } else {
            p
        }
    };
    log::info!(
        "{} {} -> {} {}",
        session.name,
        method,
        stored_path,
        target_url
    );

    let req_headers_json = headers_to_json(actix_headers_iter(&req));

    // Parse body and extract fields
    let (fields, note) = if body.is_empty() {
        (shared::ParsedRequestBody::default(), Some("no body".to_string()))
    } else if let Ok(data) = serde_json::from_slice::<serde_json::Value>(&body) {
        // Fallback: extract model ID from URL path (e.g. /model/{id}/invoke-with-response-stream)
        let url_model = {
            let parts: Vec<&str> = full_path.split('/').collect();
            parts
                .iter()
                .position(|&p| p == "model")
                .and_then(|pos| parts.get(pos + 1).map(|s| s.to_string()))
        };
        (extract_request_fields(&data, url_model), None)
    } else {
        (
            shared::ParsedRequestBody::default(),
            Some(format!("non-JSON body, {} bytes", body.len())),
        )
    };

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

    // Forward the request — copy all headers except Host, inject auth
    let mut forward_headers = reqwest::header::HeaderMap::new();
    for (key, value) in req.headers() {
        if key.as_str().eq_ignore_ascii_case("host") {
            continue;
        }
        if let Ok(name) = reqwest::header::HeaderName::from_bytes(key.as_ref()) {
            if let Ok(val) = reqwest::header::HeaderValue::from_bytes(value.as_bytes()) {
                forward_headers.insert(name, val);
            }
        }
    }
    if let Some(ref auth_value) = session.auth_header {
        if let Ok(val) = reqwest::header::HeaderValue::from_str(auth_value) {
            forward_headers.insert(reqwest::header::AUTHORIZATION, val);
        }
    }

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

    // Store response in DB
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
