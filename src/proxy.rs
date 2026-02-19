use actix_web::{web, HttpRequest, HttpResponse};
use sqlx::SqlitePool;

use crate::db;
use crate::truncate::truncate_strings;

pub async fn proxy_handler(
    req: HttpRequest,
    body: web::Bytes,
    pool: web::Data<SqlitePool>,
    client: web::Data<reqwest::Client>,
) -> HttpResponse {
    let full_path = req.match_info().get("tail").unwrap_or("");
    let session_id = req.match_info().get("session_id").unwrap_or("");

    // Look up session
    let session = match db::get_session(pool.get_ref(), session_id).await {
        Ok(Some(s)) => s,
        Ok(None) => {
            return HttpResponse::NotFound().body(format!("Session '{}' not found", session_id))
        }
        Err(e) => return HttpResponse::InternalServerError().body(format!("DB error: {}", e)),
    };

    let target_path = if full_path.is_empty() {
        String::new()
    } else {
        format!("/{}", full_path)
    };

    let mut target_url = format!("{}{}", session.target_url.trim_end_matches('/'), target_path);
    if let Some(qs) = req.uri().query() {
        target_url.push('?');
        target_url.push_str(qs);
    }

    // Log the request
    let timestamp = chrono::Local::now().format("%H:%M:%S").to_string();
    let method = req.method().to_string();
    let headers_map: std::collections::HashMap<String, String> = req
        .headers()
        .iter()
        .map(|(k, v)| (k.to_string(), v.to_str().unwrap_or("").to_string()))
        .collect();
    let headers_json = serde_json::to_string_pretty(&headers_map).ok();

    let mut body_json: Option<String> = None;
    let mut truncated_json: Option<String> = None;
    let mut model: Option<String> = None;
    let mut tools_json: Option<String> = None;
    let mut messages_json: Option<String> = None;
    let mut system_json: Option<String> = None;
    let mut params_json: Option<String> = None;
    let mut note: Option<String> = None;

    if body.is_empty() {
        note = Some("no body".to_string());
    } else if let Ok(data) = serde_json::from_slice::<serde_json::Value>(&body) {
        body_json = serde_json::to_string_pretty(&data).ok();
        let truncated = truncate_strings(&data, 100);
        truncated_json = serde_json::to_string_pretty(&truncated).ok();
        model = data.get("model").and_then(|v| v.as_str()).map(|s| s.to_string());

        if let Some(tools) = data.get("tools").filter(|v| v.is_array()) {
            tools_json = serde_json::to_string(tools).ok();
        }
        if let Some(messages) = data.get("messages").filter(|v| v.is_array()) {
            messages_json = serde_json::to_string(messages).ok();
        }
        if let Some(system) = data.get("system") {
            system_json = serde_json::to_string_pretty(system).ok();
        }

        let other: serde_json::Map<String, serde_json::Value> = data
            .as_object()
            .map(|obj| {
                obj.iter()
                    .filter(|(k, _)| !matches!(k.as_str(), "tools" | "messages" | "system"))
                    .map(|(k, v)| (k.clone(), v.clone()))
                    .collect()
            })
            .unwrap_or_default();
        if !other.is_empty() {
            params_json =
                serde_json::to_string_pretty(&serde_json::Value::Object(other)).ok();
        }
    } else {
        note = Some(format!("non-JSON body, {} bytes", body.len()));
    }

    let request_id = db::insert_request(
        pool.get_ref(),
        session_id,
        &method,
        &format!("/{}", full_path),
        &timestamp,
        headers_json.as_deref(),
        body_json.as_deref(),
        truncated_json.as_deref(),
        model.as_deref(),
        tools_json.as_deref(),
        messages_json.as_deref(),
        system_json.as_deref(),
        params_json.as_deref(),
        note.as_deref(),
    )
    .await
    .ok();

    // Forward the request
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

    // Build a separate client if TLS verification is disabled for this session
    let insecure_client;
    let effective_client: &reqwest::Client = if session.tls_verify_disabled {
        insecure_client = reqwest::Client::builder()
            .redirect(reqwest::redirect::Policy::none())
            .danger_accept_invalid_certs(true)
            .build()
            .unwrap_or_else(|_| client.get_ref().clone());
        &insecure_client
    } else {
        client.get_ref()
    };

    let resp = effective_client
        .request(
            reqwest::Method::from_bytes(method.as_bytes()).unwrap_or(reqwest::Method::GET),
            &target_url,
        )
        .headers(forward_headers)
        .body(body.to_vec())
        .send()
        .await;

    match resp {
        Ok(upstream) => {
            let status = upstream.status().as_u16();

            // Capture response headers
            let resp_headers_map: std::collections::HashMap<String, String> = upstream
                .headers()
                .iter()
                .map(|(k, v)| (k.to_string(), v.to_str().unwrap_or("").to_string()))
                .collect();
            let resp_headers_json = serde_json::to_string_pretty(&resp_headers_map).ok();

            // Check if SSE
            let content_type = upstream
                .headers()
                .get("content-type")
                .and_then(|v| v.to_str().ok())
                .unwrap_or("")
                .to_string();
            let is_sse = content_type.contains("text/event-stream");

            let mut builder = HttpResponse::build(
                actix_web::http::StatusCode::from_u16(status)
                    .unwrap_or(actix_web::http::StatusCode::INTERNAL_SERVER_ERROR),
            );
            for (key, value) in upstream.headers() {
                let k = key.as_str().to_lowercase();
                if k == "transfer-encoding" || k == "content-encoding" {
                    continue;
                }
                if let Ok(name) = actix_web::http::header::HeaderName::from_bytes(key.as_ref()) {
                    if let Ok(val) =
                        actix_web::http::header::HeaderValue::from_bytes(value.as_bytes())
                    {
                        builder.insert_header((name, val));
                    }
                }
            }
            let response_body = upstream.bytes().await.unwrap_or_default();

            // Store response data
            if let Some(req_id) = request_id {
                let body_str = String::from_utf8_lossy(&response_body);
                let (resp_body, resp_events) = if is_sse {
                    let events = parse_sse_events(&body_str);
                    let events_json = serde_json::to_string(&events).ok();
                    (Some(body_str.to_string()), events_json)
                } else {
                    (Some(body_str.to_string()), None)
                };
                let _ = db::update_request_response(
                    pool.get_ref(),
                    req_id,
                    status as i64,
                    resp_headers_json.as_deref(),
                    resp_body.as_deref(),
                    resp_events.as_deref(),
                )
                .await;
            }

            builder.body(response_body.to_vec())
        }
        Err(e) => HttpResponse::BadGateway().body(format!("Upstream error: {}", e)),
    }
}

fn parse_sse_events(body: &str) -> Vec<serde_json::Value> {
    let mut events = Vec::new();
    let mut current_event_type = String::new();
    let mut current_data = String::new();

    for line in body.lines() {
        if line.starts_with("event:") {
            current_event_type = line["event:".len()..].trim().to_string();
        } else if line.starts_with("data:") {
            if !current_data.is_empty() {
                current_data.push('\n');
            }
            current_data.push_str(line["data:".len()..].trim());
        } else if line.trim().is_empty() && !current_data.is_empty() {
            // Empty line = end of event
            let data_value = serde_json::from_str::<serde_json::Value>(&current_data)
                .unwrap_or_else(|_| serde_json::Value::String(current_data.clone()));
            let mut event = serde_json::Map::new();
            if !current_event_type.is_empty() {
                event.insert(
                    "event".to_string(),
                    serde_json::Value::String(current_event_type.clone()),
                );
            }
            event.insert("data".to_string(), data_value);
            events.push(serde_json::Value::Object(event));
            current_data.clear();
            current_event_type.clear();
        }
    }

    // Handle trailing event without final blank line
    if !current_data.is_empty() {
        let data_value = serde_json::from_str::<serde_json::Value>(&current_data)
            .unwrap_or_else(|_| serde_json::Value::String(current_data.clone()));
        let mut event = serde_json::Map::new();
        if !current_event_type.is_empty() {
            event.insert(
                "event".to_string(),
                serde_json::Value::String(current_event_type),
            );
        }
        event.insert("data".to_string(), data_value);
        events.push(serde_json::Value::Object(event));
    }

    events
}
