pub mod bedrock;
pub mod filter;
pub(crate) mod shared;
pub(crate) mod sse;
pub mod webfetch;

use actix_web::{
    error::{ErrorBadGateway, ErrorBadRequest, ErrorInternalServerError},
    web, HttpRequest, HttpResponse,
};
use common::config::AppConfig;
use shared::{
    actix_headers_iter, build_forward_headers, build_injected_sse_error, build_stored_path,
    build_target_url, effective_client, forward_response_headers, get_session_or_error,
    headers_to_json, load_filters_for_profile, log_request, parse_body_fields, store_response,
    to_actix_status, RequestMeta,
};
use sqlx::SqlitePool;

async fn apply_request_filters(
    pool: &SqlitePool,
    profile_id: Option<&str>,
    body: &web::Bytes,
) -> Vec<u8> {
    if let Some(filters) = load_filters_for_profile(pool, profile_id).await {
        if let Ok(mut json_body) = serde_json::from_slice::<serde_json::Value>(body) {
            filter::apply_filters(
                &mut json_body,
                &filters.system_filters,
                &filters.tool_filters,
                filters.keep_tool_pairs,
            );
            return serde_json::to_vec(&json_body).unwrap_or_else(|_| body.to_vec());
        }
    }
    body.to_vec()
}

fn collect_webfetch_names(session: &common::models::Session) -> Vec<String> {
    if session.webfetch_intercept {
        session
            .webfetch_tool_names
            .lines()
            .map(|l| l.trim().to_string())
            .filter(|l| !l.is_empty())
            .collect()
    } else {
        vec![]
    }
}

async fn store_webfetch_interception(
    pool: &SqlitePool,
    request_id: &str,
    body_str: &str,
    followup_body_json: &str,
    rounds_json: &str,
    note: Option<&str>,
    webfetch_note: &str,
) {
    let first_events = sse::parse_sse_events(body_str);
    let first_events_json = serde_json::to_string(&first_events).unwrap_or_default();
    if let Err(e) = db::set_request_webfetch_data(
        pool,
        request_id,
        Some(body_str),
        Some(&first_events_json),
        Some(followup_body_json),
        Some(rounds_json),
    )
    .await
    {
        log::warn!("webfetch: failed to store interception data: {}", e);
    }

    let combined_note = match note {
        Some(n) => format!("{}; {}", n, webfetch_note),
        None => webfetch_note.to_string(),
    };
    if let Err(e) = db::set_request_note(pool, request_id, &combined_note).await {
        log::warn!("webfetch: failed to store request note: {}", e);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_session(intercept: bool, tool_names: &str) -> common::models::Session {
        common::models::Session {
            id: uuid::Uuid::nil(),
            name: "test".to_string(),
            target_url: "https://api.example.com".to_string(),
            tls_verify_disabled: false,
            auth_header: None,
            x_api_key: None,
            profile_id: None,
            webfetch_intercept: intercept,
            webfetch_tool_names: tool_names.to_string(),
            webfetch_whitelist: None,
            error_inject: None,
            created_at: None,
            request_count: 0,
        }
    }

    #[test]
    fn collect_webfetch_names_intercept_off() {
        let session = make_session(false, "WebFetch\nWebSearch");
        assert!(collect_webfetch_names(&session).is_empty());
    }

    #[test]
    fn collect_webfetch_names_intercept_on() {
        let session = make_session(true, "WebFetch\nWebSearch");
        assert_eq!(
            collect_webfetch_names(&session),
            vec!["WebFetch", "WebSearch"]
        );
    }

    #[test]
    fn collect_webfetch_names_empty_lines_and_whitespace() {
        let session = make_session(true, "  WebFetch  \n\n  WebSearch  \n\n");
        assert_eq!(
            collect_webfetch_names(&session),
            vec!["WebFetch", "WebSearch"]
        );
    }

    #[test]
    fn collect_webfetch_names_empty_string() {
        let session = make_session(true, "");
        assert!(collect_webfetch_names(&session).is_empty());
    }

    #[test]
    fn collect_webfetch_names_single_name() {
        let session = make_session(true, "WebFetch");
        assert_eq!(collect_webfetch_names(&session), vec!["WebFetch"]);
    }
}

pub async fn proxy_handler(
    req: HttpRequest,
    body: web::Bytes,
    pool: web::Data<SqlitePool>,
    client: web::Data<reqwest::Client>,
    approval_queue: web::Data<webfetch::ApprovalQueue>,
    config: web::Data<AppConfig>,
) -> Result<HttpResponse, actix_web::Error> {
    let full_path = req.match_info().get("tail").unwrap_or("");
    let session_id = req
        .match_info()
        .get("session_id")
        .ok_or_else(|| ErrorBadRequest("Missing session_id"))?;

    let session = get_session_or_error(pool.get_ref(), session_id).await?;

    // Return injected SSE error if error injection is active for this session.
    if let Some(ref error_type) = session.error_inject {
        if !error_type.is_empty() {
            if let Some(resp) = build_injected_sse_error(error_type) {
                return Ok(resp);
            }
        }
    }

    let query = req.uri().query();
    let target_url = build_target_url(&session.target_url, full_path, query);
    let stored_path = build_stored_path(full_path, query);
    let method = req.method().to_string();
    let timestamp = chrono::Local::now().format("%H:%M:%S").to_string();

    log::info!(
        "{} {} -> {} {}",
        session.name,
        method,
        stored_path,
        target_url
    );

    // Log request to DB
    let req_headers_json =
        headers_to_json(actix_headers_iter(&req)).map_err(ErrorInternalServerError)?;
    let url_model = {
        let parts: Vec<&str> = full_path.split('/').collect();
        parts
            .iter()
            .position(|&p| p == "model")
            .and_then(|pos| parts.get(pos + 1).map(|s| s.to_string()))
    };
    let (fields, note) = parse_body_fields(&body, url_model).map_err(ErrorInternalServerError)?;
    let request_id = log_request(
        &RequestMeta {
            pool: pool.get_ref(),
            session_id,
            method: &method,
            path: &stored_path,
            timestamp: &timestamp,
            headers_json: Some(&req_headers_json),
            note: note.as_deref(),
        },
        &fields,
    )
    .await
    .map_err(ErrorInternalServerError)?;

    // Apply filters to the body before forwarding
    let forward_body =
        apply_request_filters(pool.get_ref(), session.profile_id.as_deref(), &body).await;

    // Forward the request upstream
    let forward_headers = build_forward_headers(
        &req,
        session.auth_header.as_deref(),
        session.x_api_key.as_deref(),
    );
    let effective_client = effective_client(&session, client.get_ref());
    let parsed_method = reqwest::Method::from_bytes(method.as_bytes())
        .map_err(|e| ErrorBadRequest(format!("Invalid HTTP method: {}", e)))?;

    // Save copies for potential webfetch follow-up before the upstream call consumes them
    let webfetch_names = collect_webfetch_names(&session);
    let webfetch_context = if !webfetch_names.is_empty() {
        Some((forward_body.clone(), forward_headers.clone()))
    } else {
        None
    };

    let upstream = effective_client
        .request(parsed_method, &target_url)
        .headers(forward_headers)
        .body(forward_body)
        .send()
        .await
        .map_err(|e| ErrorBadGateway(format!("Upstream error: {}", e)))?;

    // Build response
    let status = upstream.status().as_u16();
    let resp_headers_json = headers_to_json(
        upstream
            .headers()
            .iter()
            .filter_map(|(k, v)| v.to_str().ok().map(|s| (k.to_string(), s.to_string()))),
    )
    .map_err(ErrorInternalServerError)?;
    let actix_status = to_actix_status(status)?;

    let mut builder = HttpResponse::build(actix_status);
    forward_response_headers(&mut builder, upstream.headers());

    let response_body = upstream
        .bytes()
        .await
        .map_err(|e| ErrorBadGateway(format!("Failed to read upstream response body: {}", e)))?;

    let body_str = String::from_utf8_lossy(&response_body);

    // WebFetch interception: if enabled, check for tool_use and send follow-up request
    if let Some((saved_body, saved_headers)) = webfetch_context {
        let whitelist: Vec<String> = session
            .webfetch_whitelist
            .as_deref()
            .unwrap_or("")
            .lines()
            .map(|l| l.trim().to_string())
            .filter(|l| !l.is_empty())
            .collect();

        if let Some(result) = webfetch::maybe_intercept(&webfetch::InterceptParams {
            response_body: &body_str,
            original_body: &saved_body,
            target_url: &target_url,
            forward_headers: &saved_headers,
            client: effective_client,
            approval_queue: approval_queue.get_ref(),
            session_id,
            whitelist: &whitelist,
            pool: pool.get_ref(),
            stored_path: &stored_path,
            webfetch_names: &webfetch_names,
            config: config.get_ref(),
        })
        .await
        {
            let webfetch::InterceptResult::Intercepted {
                status: followup_status,
                headers: followup_headers,
                body: followup_body,
                note: webfetch_note,
                followup_body_json,
                rounds_json,
            } = result;

            // Use follow-up response's status, headers, and body
            let followup_actix_status = to_actix_status(followup_status)?;
            let followup_resp_headers_json = headers_to_json(
                followup_headers
                    .iter()
                    .filter_map(|(k, v)| v.to_str().ok().map(|s| (k.to_string(), s.to_string()))),
            )
            .map_err(ErrorInternalServerError)?;

            let mut followup_builder = HttpResponse::build(followup_actix_status);
            forward_response_headers(&mut followup_builder, &followup_headers);

            let followup_body_str = String::from_utf8_lossy(&followup_body);
            store_response(
                pool.get_ref(),
                &request_id,
                followup_status,
                Some(&followup_resp_headers_json),
                &followup_body_str,
            )
            .await
            .map_err(ErrorInternalServerError)?;

            // Store webfetch interception data: intercepted response + follow-up body
            store_webfetch_interception(
                pool.get_ref(),
                &request_id,
                &body_str,
                &followup_body_json,
                &rounds_json,
                note.as_deref(),
                &webfetch_note,
            )
            .await;

            return Ok(followup_builder.body(followup_body.to_vec()));
        }
    }

    // Default path: no interception, store and return original response
    store_response(
        pool.get_ref(),
        &request_id,
        status,
        Some(&resp_headers_json),
        &body_str,
    )
    .await
    .map_err(ErrorInternalServerError)?;

    Ok(builder.body(response_body.to_vec()))
}
