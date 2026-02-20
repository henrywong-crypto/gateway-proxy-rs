mod sse;

use actix_web::{web, HttpRequest, HttpResponse};
use sqlx::SqlitePool;

use common::truncate::truncate_strings;

pub async fn proxy_handler(
    req: HttpRequest,
    body: web::Bytes,
    pool: web::Data<SqlitePool>,
    client: web::Data<reqwest::Client>,
) -> HttpResponse {
    let full_path = req.match_info().get("tail").unwrap_or("");
    let session_id = match req.match_info().get("session_id") {
        Some(id) => id,
        None => return HttpResponse::BadRequest().body("Missing session_id"),
    };

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
        .filter_map(|(k, v)| {
            match v.to_str() {
                Ok(s) => Some((k.to_string(), s.to_string())),
                Err(_) => None,
            }
        })
        .collect();
    let headers_json = match serde_json::to_string_pretty(&headers_map) {
        Ok(s) => Some(s),
        Err(e) => {
            eprintln!("Failed to serialize request headers: {}", e);
            None
        }
    };

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
        body_json = match serde_json::to_string_pretty(&data) {
            Ok(s) => Some(s),
            Err(e) => {
                eprintln!("Failed to serialize body JSON: {}", e);
                None
            }
        };
        let truncated = truncate_strings(&data, 100);
        truncated_json = match serde_json::to_string_pretty(&truncated) {
            Ok(s) => Some(s),
            Err(e) => {
                eprintln!("Failed to serialize truncated JSON: {}", e);
                None
            }
        };
        model = data.get("model").and_then(|v| v.as_str()).map(|s| s.to_string());

        if let Some(tools) = data.get("tools").filter(|v| v.is_array()) {
            tools_json = match serde_json::to_string(tools) {
                Ok(s) => Some(s),
                Err(e) => {
                    eprintln!("Failed to serialize tools: {}", e);
                    None
                }
            };
        }
        if let Some(messages) = data.get("messages").filter(|v| v.is_array()) {
            messages_json = match serde_json::to_string(messages) {
                Ok(s) => Some(s),
                Err(e) => {
                    eprintln!("Failed to serialize messages: {}", e);
                    None
                }
            };
        }
        if let Some(system) = data.get("system") {
            system_json = match serde_json::to_string_pretty(system) {
                Ok(s) => Some(s),
                Err(e) => {
                    eprintln!("Failed to serialize system: {}", e);
                    None
                }
            };
        }

        let other: serde_json::Map<String, serde_json::Value> = match data.as_object() {
            Some(obj) => obj
                .iter()
                .filter(|(k, _)| !matches!(k.as_str(), "tools" | "messages" | "system"))
                .map(|(k, v)| (k.clone(), v.clone()))
                .collect(),
            None => serde_json::Map::new(),
        };
        if !other.is_empty() {
            params_json = match serde_json::to_string_pretty(&serde_json::Value::Object(other)) {
                Ok(s) => Some(s),
                Err(e) => {
                    eprintln!("Failed to serialize params: {}", e);
                    None
                }
            };
        }
    } else {
        note = Some(format!("non-JSON body, {} bytes", body.len()));
    }

    let request_id = match db::insert_request(
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
    {
        Ok(id) => Some(id),
        Err(e) => {
            eprintln!("Failed to insert request: {}", e);
            None
        }
    };

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
        match reqwest::Client::builder()
            .redirect(reqwest::redirect::Policy::none())
            .danger_accept_invalid_certs(true)
            .build()
        {
            Ok(c) => {
                insecure_client = c;
                &insecure_client
            }
            Err(e) => {
                return HttpResponse::InternalServerError()
                    .body(format!("Failed to build TLS-insecure client: {}", e));
            }
        }
    } else {
        client.get_ref()
    };

    let parsed_method = match reqwest::Method::from_bytes(method.as_bytes()) {
        Ok(m) => m,
        Err(e) => {
            return HttpResponse::BadRequest().body(format!("Invalid HTTP method: {}", e));
        }
    };

    let resp = effective_client
        .request(parsed_method, &target_url)
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
                .filter_map(|(k, v)| match v.to_str() {
                    Ok(s) => Some((k.to_string(), s.to_string())),
                    Err(_) => None,
                })
                .collect();
            let resp_headers_json = match serde_json::to_string_pretty(&resp_headers_map) {
                Ok(s) => Some(s),
                Err(e) => {
                    eprintln!("Failed to serialize response headers: {}", e);
                    None
                }
            };

            // Check if SSE
            let content_type = upstream
                .headers()
                .get("content-type")
                .and_then(|v| v.to_str().ok());
            let is_sse = match content_type {
                Some(ct) => ct.contains("text/event-stream"),
                None => false,
            };

            let mut builder = HttpResponse::build(
                match actix_web::http::StatusCode::from_u16(status) {
                    Ok(s) => s,
                    Err(_) => {
                        return HttpResponse::BadGateway()
                            .body(format!("Invalid status code from upstream: {}", status));
                    }
                },
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
            let response_body = match upstream.bytes().await {
                Ok(b) => b,
                Err(e) => {
                    return HttpResponse::BadGateway()
                        .body(format!("Failed to read upstream response body: {}", e));
                }
            };

            // Store response data
            if let Some(req_id) = request_id {
                let body_str = String::from_utf8_lossy(&response_body);
                let (resp_body, resp_events) = if is_sse {
                    let events = sse::parse_sse_events(&body_str);
                    let events_json = match serde_json::to_string(&events) {
                        Ok(s) => Some(s),
                        Err(e) => {
                            eprintln!("Failed to serialize SSE events: {}", e);
                            None
                        }
                    };
                    (Some(body_str.to_string()), events_json)
                } else {
                    (Some(body_str.to_string()), None)
                };
                if let Err(e) = db::update_request_response(
                    pool.get_ref(),
                    req_id,
                    status as i64,
                    resp_headers_json.as_deref(),
                    resp_body.as_deref(),
                    resp_events.as_deref(),
                )
                .await
                {
                    eprintln!("Failed to update request response: {}", e);
                }
            }

            builder.body(response_body.to_vec())
        }
        Err(e) => HttpResponse::BadGateway().body(format!("Upstream error: {}", e)),
    }
}
