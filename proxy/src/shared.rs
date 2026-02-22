use actix_web::error::{ErrorBadGateway, ErrorInternalServerError, ErrorNotFound};
use actix_web::http::StatusCode;
use actix_web::{HttpRequest, HttpResponseBuilder};
use common::truncate::truncate_strings;
use serde_json::Value;
use sqlx::SqlitePool;
use std::sync::LazyLock;

use crate::sse;

/// Loaded filter state for a profile.
pub struct ActiveFilters {
    pub system_filters: Vec<String>,
    pub tool_filters: Vec<String>,
    pub keep_tool_pairs: i64,
}

/// Load filters for the given profile. Returns None if profile_id is empty/None.
pub async fn load_filters_for_profile(
    pool: &SqlitePool,
    profile_id: Option<&str>,
) -> Option<ActiveFilters> {
    let profile_id = profile_id.filter(|s| !s.is_empty())?;
    let system_filters: Vec<String> = db::list_system_filters(pool, profile_id)
        .await
        .unwrap_or_default()
        .into_iter()
        .map(|f| f.pattern)
        .collect();
    let tool_filters: Vec<String> = db::list_tool_filters(pool, profile_id)
        .await
        .unwrap_or_default()
        .into_iter()
        .map(|f| f.name)
        .collect();
    let keep_tool_pairs = db::get_keep_tool_pairs(pool, profile_id).await.unwrap_or(0);
    Some(ActiveFilters {
        system_filters,
        tool_filters,
        keep_tool_pairs,
    })
}

/// Look up a session by ID, returning an actix error on failure or not-found.
pub async fn get_session_or_error(
    pool: &SqlitePool,
    session_id: &str,
) -> Result<common::models::Session, actix_web::Error> {
    match db::get_session(pool, session_id).await {
        Ok(Some(s)) => Ok(s),
        Ok(None) => Err(ErrorNotFound(format!("Session '{}' not found", session_id))),
        Err(e) => Err(ErrorInternalServerError(format!("DB error: {}", e))),
    }
}

/// Serialize an iterator of (name, value) header pairs to a pretty-printed JSON string.
pub fn headers_to_json(headers: impl Iterator<Item = (String, String)>) -> anyhow::Result<String> {
    let map: std::collections::HashMap<String, String> = headers.collect();
    Ok(serde_json::to_string_pretty(&map)?)
}

/// Fields extracted from a JSON request body.
#[derive(Default)]
pub struct ParsedRequestBody {
    pub body_json: Option<String>,
    pub truncated_json: Option<String>,
    pub model: Option<String>,
    pub tools_json: Option<String>,
    pub messages_json: Option<String>,
    pub system_json: Option<String>,
    pub params_json: Option<String>,
}

/// Extract common fields (model, tools, messages, system, params, truncated body)
/// from a parsed JSON value. If `model_override` is provided, it is used only when
/// the body does not already contain a "model" field.
pub fn extract_request_fields(
    data: &Value,
    model_override: Option<String>,
) -> anyhow::Result<ParsedRequestBody> {
    let truncated = truncate_strings(data, 100);

    let model = data
        .get("model")
        .and_then(|v| v.as_str())
        .map(|s| s.to_string())
        .or(model_override);

    let tools_json = data
        .get("tools")
        .filter(|v| v.is_array())
        .map(serde_json::to_string)
        .transpose()?;

    let messages_json = data
        .get("messages")
        .filter(|v| v.is_array())
        .map(serde_json::to_string)
        .transpose()?;

    let system_json = data
        .get("system")
        .map(serde_json::to_string_pretty)
        .transpose()?;

    let other: serde_json::Map<String, Value> = data
        .as_object()
        .map(|obj| {
            obj.iter()
                .filter(|(k, _)| !matches!(k.as_str(), "tools" | "messages" | "system"))
                .map(|(k, v)| (k.clone(), v.clone()))
                .collect()
        })
        .unwrap_or_default();
    let params_json = if other.is_empty() {
        None
    } else {
        Some(serde_json::to_string_pretty(&Value::Object(other))?)
    };

    Ok(ParsedRequestBody {
        body_json: Some(serde_json::to_string_pretty(data)?),
        truncated_json: Some(serde_json::to_string_pretty(&truncated)?),
        model,
        tools_json,
        messages_json,
        system_json,
        params_json,
    })
}

/// Metadata for a request log entry (everything except the parsed body fields).
pub struct RequestMeta<'a> {
    pub pool: &'a SqlitePool,
    pub session_id: &'a str,
    pub method: &'a str,
    pub path: &'a str,
    pub timestamp: &'a str,
    pub headers_json: Option<&'a str>,
    pub note: Option<&'a str>,
}

/// Insert a request record into the DB. Returns the request ID on success, None on failure.
pub async fn log_request(
    meta: &RequestMeta<'_>,
    fields: &ParsedRequestBody,
) -> anyhow::Result<String> {
    db::insert_request(
        meta.pool,
        &db::InsertRequestParams {
            session_id: meta.session_id,
            method: meta.method,
            path: meta.path,
            timestamp: meta.timestamp,
            headers_json: meta.headers_json,
            body_json: fields.body_json.as_deref(),
            truncated_json: fields.truncated_json.as_deref(),
            model: fields.model.as_deref(),
            tools_json: fields.tools_json.as_deref(),
            messages_json: fields.messages_json.as_deref(),
            system_json: fields.system_json.as_deref(),
            params_json: fields.params_json.as_deref(),
            note: meta.note,
        },
    )
    .await
}

/// Store a buffered response (with optional SSE event parsing) into the DB.
pub async fn store_response(
    pool: &SqlitePool,
    request_id: &str,
    status: u16,
    resp_headers_json: Option<&str>,
    response_body: &str,
) -> anyhow::Result<()> {
    let events = sse::parse_sse_events(response_body);
    let events_json = serde_json::to_string(&events)?;

    db::update_request_response(
        pool,
        request_id,
        status as i64,
        resp_headers_json,
        Some(response_body),
        Some(&events_json),
    )
    .await?;
    Ok(())
}

/// Convert a u16 status code to an actix StatusCode.
pub fn to_actix_status(status: u16) -> Result<StatusCode, actix_web::Error> {
    StatusCode::from_u16(status)
        .map_err(|_| ErrorBadGateway(format!("Invalid status code from upstream: {}", status)))
}

/// Copy upstream response headers into an actix HttpResponseBuilder,
/// skipping transfer-encoding and content-encoding.
pub fn forward_response_headers(
    builder: &mut HttpResponseBuilder,
    upstream_headers: &reqwest::header::HeaderMap,
) {
    for (key, value) in upstream_headers {
        let k = key.as_str().to_lowercase();
        if k == "transfer-encoding" || k == "content-encoding" {
            continue;
        }
        if let Ok(name) = actix_web::http::header::HeaderName::from_bytes(key.as_ref()) {
            if let Ok(val) = actix_web::http::header::HeaderValue::from_bytes(value.as_bytes()) {
                builder.insert_header((name, val));
            }
        }
    }
}

/// Cached insecure reqwest::Client for sessions with TLS verification disabled.
static INSECURE_CLIENT: LazyLock<reqwest::Client> = LazyLock::new(|| {
    reqwest::Client::builder()
        .redirect(reqwest::redirect::Policy::none())
        .danger_accept_invalid_certs(true)
        .build()
        .expect("Failed to build TLS-insecure client")
});

/// Return the insecure client if the session has TLS verification disabled,
/// otherwise return the default client reference.
pub fn effective_client<'a>(
    session: &common::models::Session,
    default_client: &'a reqwest::Client,
) -> &'a reqwest::Client {
    if session.tls_verify_disabled {
        &INSECURE_CLIENT
    } else {
        default_client
    }
}

/// Extract header (name, value) pairs from an actix HttpRequest.
pub fn actix_headers_iter(
    req: &actix_web::HttpRequest,
) -> impl Iterator<Item = (String, String)> + '_ {
    req.headers()
        .iter()
        .filter_map(|(k, v)| v.to_str().ok().map(|s| (k.to_string(), s.to_string())))
}

/// Build the full target URL from a session's base URL, the request path, and
/// an optional query string.
pub fn build_target_url(base_url: &str, path: &str, query: Option<&str>) -> String {
    let target_path = if path.is_empty() {
        String::new()
    } else {
        format!("/{}", path)
    };
    let mut url = format!("{}{}", base_url.trim_end_matches('/'), target_path);
    if let Some(qs) = query {
        url.push('?');
        url.push_str(qs);
    }
    url
}

/// Build the stored path shown in the UI (always prefixed with `/`).
pub fn build_stored_path(path: &str, query: Option<&str>) -> String {
    let p = format!("/{}", path);
    if let Some(qs) = query {
        format!("{}?{}", p, qs)
    } else {
        p
    }
}

/// Copy headers from an actix HttpRequest into a reqwest HeaderMap, skipping
/// the `Host` header. If `auth_header` is provided, it is injected as the
/// `Authorization` header. If `x_api_key` is provided, it is injected as
/// the `x-api-key` header.
pub fn build_forward_headers(
    req: &HttpRequest,
    auth_header: Option<&str>,
    x_api_key: Option<&str>,
) -> reqwest::header::HeaderMap {
    let mut map = reqwest::header::HeaderMap::new();
    for (key, value) in req.headers() {
        if key.as_str().eq_ignore_ascii_case("host") {
            continue;
        }
        if let Ok(name) = reqwest::header::HeaderName::from_bytes(key.as_ref()) {
            if let Ok(val) = reqwest::header::HeaderValue::from_bytes(value.as_bytes()) {
                map.insert(name, val);
            }
        }
    }
    if let Some(auth_value) = auth_header {
        if let Ok(val) = reqwest::header::HeaderValue::from_str(auth_value) {
            map.insert(reqwest::header::AUTHORIZATION, val);
        }
    }
    if let Some(key_value) = x_api_key {
        if let Ok(val) = reqwest::header::HeaderValue::from_str(key_value) {
            map.insert(reqwest::header::HeaderName::from_static("x-api-key"), val);
        }
    }
    map
}

/// Parse the request body and extract fields for DB logging.
/// Returns `(ParsedRequestBody, optional_note)`.
pub fn parse_body_fields(
    body: &[u8],
    url_model: Option<String>,
) -> anyhow::Result<(ParsedRequestBody, Option<String>)> {
    if body.is_empty() {
        Ok((ParsedRequestBody::default(), Some("no body".to_string())))
    } else if let Ok(data) = serde_json::from_slice::<Value>(body) {
        Ok((extract_request_fields(&data, url_model)?, None))
    } else {
        Ok((
            ParsedRequestBody::default(),
            Some(format!("non-JSON body, {} bytes", body.len())),
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn extract_basic_fields() {
        let data: Value = serde_json::json!({
            "model": "claude-3-opus",
            "messages": [{"role": "user", "content": "hi"}],
            "tools": [{"name": "search"}],
            "system": "You are helpful.",
            "max_tokens": 1024
        });
        let fields = extract_request_fields(&data, None).unwrap();
        assert_eq!(fields.model.as_deref(), Some("claude-3-opus"));
        assert!(fields.messages_json.is_some());
        assert!(fields.tools_json.is_some());
        assert!(fields.system_json.is_some());
        assert!(fields.params_json.is_some());
        // params should contain max_tokens and model but not messages/tools/system
        let params: Value = serde_json::from_str(fields.params_json.as_ref().unwrap()).unwrap();
        assert!(params.get("max_tokens").is_some());
        assert!(params.get("messages").is_none());
    }

    #[test]
    fn extract_model_override_used_as_fallback() {
        let data: Value = serde_json::json!({"messages": []});
        let fields = extract_request_fields(&data, Some("fallback-model".to_string())).unwrap();
        assert_eq!(fields.model.as_deref(), Some("fallback-model"));
    }

    #[test]
    fn extract_body_model_takes_precedence_over_override() {
        let data: Value = serde_json::json!({"model": "body-model", "messages": []});
        let fields = extract_request_fields(&data, Some("override-model".to_string())).unwrap();
        assert_eq!(fields.model.as_deref(), Some("body-model"));
    }

    #[test]
    fn headers_to_json_basic() {
        let headers = vec![
            ("content-type".to_string(), "application/json".to_string()),
            ("x-custom".to_string(), "value".to_string()),
        ];
        let json = headers_to_json(headers.into_iter()).unwrap();
        let parsed: std::collections::HashMap<String, String> =
            serde_json::from_str(&json).unwrap();
        assert_eq!(parsed.get("content-type").unwrap(), "application/json");
        assert_eq!(parsed.get("x-custom").unwrap(), "value");
    }

    #[test]
    fn headers_to_json_empty() {
        let json = headers_to_json(std::iter::empty()).unwrap();
        let parsed: std::collections::HashMap<String, String> =
            serde_json::from_str(&json).unwrap();
        assert!(parsed.is_empty());
    }
}
