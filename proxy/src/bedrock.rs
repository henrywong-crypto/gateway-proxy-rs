use actix_web::{
    error::{ErrorBadGateway, ErrorBadRequest, ErrorInternalServerError},
    web, HttpRequest, HttpResponse,
};
use base64::{engine::general_purpose::STANDARD as BASE64, Engine as _};
use bytes::Bytes;
use futures::StreamExt;
use sqlx::SqlitePool;

use crate::{
    shared::{
        actix_headers_iter, effective_client, extract_request_fields, get_session_or_error,
        headers_to_json, load_filters_for_profile, log_request, to_actix_status, RequestMeta,
    },
    sse::parse_sse_events,
};

// --- AWS Event Stream binary protocol encoding ---

/// Encode an AWS Event Stream binary message with the given headers and payload.
fn encode_event_stream_message(headers: &[(&str, &str)], payload: &[u8]) -> Vec<u8> {
    // Encode headers
    let mut headers_buf = Vec::new();
    for &(name, value) in headers {
        headers_buf.push(name.len() as u8);
        headers_buf.extend_from_slice(name.as_bytes());
        headers_buf.push(7); // String type
        headers_buf.extend_from_slice(&(value.len() as u16).to_be_bytes());
        headers_buf.extend_from_slice(value.as_bytes());
    }

    let headers_length = headers_buf.len();
    let total_length = 16 + headers_length + payload.len();

    let mut message = Vec::with_capacity(total_length);

    // Prelude: total_length (4 bytes) + headers_length (4 bytes)
    message.extend_from_slice(&(total_length as u32).to_be_bytes());
    message.extend_from_slice(&(headers_length as u32).to_be_bytes());

    // Prelude CRC (CRC32 of the 8-byte prelude)
    let prelude_crc = crc32fast::hash(&message[..8]);
    message.extend_from_slice(&prelude_crc.to_be_bytes());

    // Headers
    message.extend_from_slice(&headers_buf);

    // Payload
    message.extend_from_slice(payload);

    // Message CRC (CRC32 of entire message so far)
    let message_crc = crc32fast::hash(&message);
    message.extend_from_slice(&message_crc.to_be_bytes());

    message
}

/// Convert an Anthropic SSE event data JSON string into a Bedrock Event Stream chunk frame.
fn encode_bedrock_chunk(data_json: &str) -> Vec<u8> {
    let b64 = BASE64.encode(data_json.as_bytes());
    let payload = format!(r#"{{"bytes":"{}"}}"#, b64);

    encode_event_stream_message(
        &[
            (":message-type", "event"),
            (":event-type", "chunk"),
            (":content-type", "application/json"),
        ],
        payload.as_bytes(),
    )
}

// --- Incremental SSE parser ---

struct SseParser {
    buffer: String,
    current_data: Vec<String>,
}

impl SseParser {
    fn new() -> Self {
        SseParser {
            buffer: String::new(),
            current_data: Vec::new(),
        }
    }

    /// Feed a chunk of text and return completed event data strings.
    fn feed(&mut self, chunk: &str) -> Vec<String> {
        self.buffer.push_str(chunk);
        let mut events = Vec::new();

        while let Some(pos) = self.buffer.find('\n') {
            let line = self.buffer[..pos].trim_end_matches('\r').to_string();
            self.buffer = self.buffer[pos + 1..].to_string();

            if line.is_empty() {
                // Empty line = event boundary
                if !self.current_data.is_empty() {
                    events.push(self.current_data.join("\n"));
                    self.current_data.clear();
                }
            } else if let Some(rest) = line.strip_prefix("data:") {
                let data = rest.strip_prefix(' ').unwrap_or(rest);
                self.current_data.push(data.to_string());
            }
            // Ignore event type, comment lines (starting with ':'), and unknown fields
        }

        events
    }

    /// Flush any remaining buffered event at end of stream.
    fn flush(&mut self) -> Option<String> {
        if self.current_data.is_empty() {
            None
        } else {
            let data = self.current_data.join("\n");
            self.current_data.clear();
            Some(data)
        }
    }
}

/// Translate a Bedrock-style request body into an Anthropic API request.
/// Extracts `anthropic_version` and `anthropic_beta` from the body, adds
/// `model` and `stream: true`, and returns the serialized body + headers.
fn translate_bedrock_request(
    req: &HttpRequest,
    mut data: serde_json::Value,
    model_id: &str,
    auth_header: Option<&str>,
    x_api_key: Option<&str>,
) -> Result<(Vec<u8>, reqwest::header::HeaderMap), actix_web::Error> {
    let obj = data
        .as_object_mut()
        .ok_or_else(|| ErrorBadRequest("Request body must be a JSON object"))?;

    // Extract anthropic_version and anthropic_beta from body
    let anthropic_version = obj
        .remove("anthropic_version")
        .and_then(|v| v.as_str().map(|s| s.to_string()));
    let body_beta = obj.remove("anthropic_beta").map(|v| match v {
        serde_json::Value::Array(arr) => arr
            .iter()
            .filter_map(|item| item.as_str().map(|s| s.to_string()))
            .collect::<Vec<_>>()
            .join(","),
        serde_json::Value::String(s) => s,
        other => other.to_string(),
    });

    // Combine anthropic-beta from request header and body
    let header_beta = req
        .headers()
        .get("anthropic-beta")
        .and_then(|v| v.to_str().ok())
        .map(|s| s.to_string());
    let anthropic_beta = match (header_beta, body_beta) {
        (Some(h), Some(b)) => Some(format!("{},{}", h, b)),
        (Some(h), None) => Some(h),
        (None, Some(b)) => Some(b),
        (None, None) => None,
    };

    // Add model and stream to body
    obj.insert(
        "model".to_string(),
        serde_json::Value::String(model_id.to_string()),
    );
    obj.insert("stream".to_string(), serde_json::Value::Bool(true));

    let body = serde_json::to_vec(&data).map_err(|e| {
        ErrorInternalServerError(format!("Failed to serialize translated body: {}", e))
    })?;

    // Build headers
    let mut headers = reqwest::header::HeaderMap::new();
    headers.insert(
        reqwest::header::CONTENT_TYPE,
        reqwest::header::HeaderValue::from_static("application/json"),
    );
    if let Some(ref ver) = anthropic_version {
        if let Ok(val) = reqwest::header::HeaderValue::from_str(ver) {
            headers.insert(
                reqwest::header::HeaderName::from_static("anthropic-version"),
                val,
            );
        }
    }
    if let Some(ref beta) = anthropic_beta {
        if let Ok(val) = reqwest::header::HeaderValue::from_str(beta) {
            headers.insert(
                reqwest::header::HeaderName::from_static("anthropic-beta"),
                val,
            );
        }
    }
    if let Some(auth_value) = auth_header {
        if let Ok(val) = reqwest::header::HeaderValue::from_str(auth_value) {
            headers.insert(reqwest::header::AUTHORIZATION, val);
        }
    }
    if let Some(key_value) = x_api_key {
        if let Ok(val) = reqwest::header::HeaderValue::from_str(key_value) {
            headers.insert(reqwest::header::HeaderName::from_static("x-api-key"), val);
        }
    }

    Ok((body, headers))
}

pub async fn bedrock_streaming_handler(
    req: HttpRequest,
    body: web::Bytes,
    pool: web::Data<SqlitePool>,
    client: web::Data<reqwest::Client>,
) -> Result<HttpResponse, actix_web::Error> {
    let session_id = req
        .match_info()
        .get("session_id")
        .ok_or_else(|| ErrorBadRequest("Missing session_id"))?;
    let model_id = req
        .match_info()
        .get("model_id")
        .ok_or_else(|| ErrorBadRequest("Missing model_id"))?;

    let session = get_session_or_error(pool.get_ref(), session_id).await?;

    // Return injected error if error injection is active for this session.
    // Return the error JSON with the correct HTTP status code so clients recognize it.
    if let Some(ref error_type) = session.error_inject {
        if !error_type.is_empty() {
            if let Some(e) = common::error_inject::find_by_key(error_type) {
                let actix_status = actix_web::http::StatusCode::from_u16(e.status)
                    .unwrap_or(actix_web::http::StatusCode::INTERNAL_SERVER_ERROR);
                return Ok(HttpResponse::build(actix_status)
                    .insert_header((actix_web::http::header::CONTENT_TYPE, "application/json"))
                    .body(e.data_json));
            }
        }
    }

    // Parse and log the original request
    let original_data: serde_json::Value = serde_json::from_slice(&body)
        .map_err(|e| ErrorBadRequest(format!("Invalid JSON body: {}", e)))?;
    if !original_data.is_object() {
        return Err(ErrorBadRequest("Request body must be a JSON object"));
    }

    let timestamp = chrono::Local::now().format("%H:%M:%S").to_string();
    let stored_path = format!("/model/{}/invoke-with-response-stream", model_id);
    let req_headers_json =
        headers_to_json(actix_headers_iter(&req)).map_err(ErrorInternalServerError)?;
    let fields = extract_request_fields(&original_data, Some(model_id.to_string()))
        .map_err(ErrorInternalServerError)?;

    let request_id = log_request(
        &RequestMeta {
            pool: pool.get_ref(),
            session_id,
            method: "POST",
            path: &stored_path,
            timestamp: &timestamp,
            headers_json: Some(&req_headers_json),
            note: None,
        },
        &fields,
    )
    .await
    .map_err(ErrorInternalServerError)?;

    // Apply filters to the data before forwarding
    let mut filtered_data = original_data.clone();
    if let Some(filters) =
        load_filters_for_profile(pool.get_ref(), session.profile_id.as_deref()).await
    {
        crate::filter::apply_filters(
            &mut filtered_data,
            &filters.system_filters,
            &filters.tool_filters,
            filters.keep_tool_pairs,
        );
    }

    // Translate request and send upstream
    let (translated_body, forward_headers) = translate_bedrock_request(
        &req,
        filtered_data,
        model_id,
        session.auth_header.as_deref(),
        session.x_api_key.as_deref(),
    )?;

    let target_url = format!("{}/v1/messages", session.target_url.trim_end_matches('/'));
    let effective_client = effective_client(&session, client.get_ref());

    log::info!("{} POST {} -> {}", session.name, stored_path, target_url);

    let upstream = effective_client
        .post(&target_url)
        .headers(forward_headers)
        .body(translated_body)
        .send()
        .await
        .map_err(|e| ErrorBadGateway(format!("Upstream error: {}", e)))?;

    let status = upstream.status().as_u16();
    let resp_headers_json = headers_to_json(
        upstream
            .headers()
            .iter()
            .filter_map(|(k, v)| v.to_str().ok().map(|s| (k.to_string(), s.to_string()))),
    )
    .map_err(ErrorInternalServerError)?;
    let actix_status = to_actix_status(status)?;

    // For non-200 responses, return the error body directly instead of
    // trying to parse it as SSE (upstream returns plain JSON for errors).
    if status != 200 {
        let error_body = upstream
            .bytes()
            .await
            .map_err(|e| ErrorBadGateway(format!("Failed to read error body: {}", e)))?;

        let body_str = String::from_utf8_lossy(&error_body);
        let events = parse_sse_events(&body_str);
        let events_json = serde_json::to_string(&events).unwrap_or_else(|_| "[]".to_string());

        if let Err(e) = db::set_request_response(
            pool.get_ref(),
            &request_id,
            status as i64,
            Some(&resp_headers_json),
            Some(&body_str),
            Some(&events_json),
        )
        .await
        {
            log::warn!("bedrock: failed to store error response: {}", e);
        }

        return Ok(HttpResponse::build(actix_status)
            .insert_header((actix_web::http::header::CONTENT_TYPE, "application/json"))
            .body(error_body));
    }

    // Streaming SSE response — convert to Bedrock Event Stream format
    let mut builder = HttpResponse::build(actix_status);
    builder.insert_header((
        actix_web::http::header::CONTENT_TYPE,
        "application/vnd.amazon.eventstream",
    ));

    let pool_bg = pool.clone();
    let request_id_bg = request_id.clone();
    let resp_headers_json_bg = resp_headers_json.clone();

    let (tx, rx) = futures::channel::mpsc::unbounded::<Result<Bytes, actix_web::Error>>();
    let mut byte_stream = upstream.bytes_stream();

    actix_web::rt::spawn(async move {
        let mut accumulated = Vec::new();
        let mut parser = SseParser::new();

        while let Some(chunk_result) = byte_stream.next().await {
            match chunk_result {
                Ok(chunk) => {
                    accumulated.extend_from_slice(&chunk);

                    let chunk_str = String::from_utf8_lossy(&chunk);
                    for data in parser.feed(&chunk_str) {
                        let frame = encode_bedrock_chunk(&data);
                        if tx.unbounded_send(Ok(Bytes::from(frame))).is_err() {
                            return; // Client disconnected
                        }
                    }
                }
                Err(e) => {
                    let _ = tx.unbounded_send(Err(actix_web::error::ErrorBadGateway(format!(
                        "Upstream stream error: {}",
                        e
                    ))));
                    return;
                }
            }
        }

        if let Some(data) = parser.flush() {
            let frame = encode_bedrock_chunk(&data);
            let _ = tx.unbounded_send(Ok(Bytes::from(frame)));
        }

        // Store accumulated response to DB
        let store: anyhow::Result<()> = async {
            let body_str = String::from_utf8_lossy(&accumulated);
            let events = parse_sse_events(&body_str);
            let events_json = serde_json::to_string(&events)?;
            db::set_request_response(
                pool_bg.get_ref(),
                &request_id_bg,
                status as i64,
                Some(&resp_headers_json_bg),
                Some(&body_str),
                Some(&events_json),
            )
            .await?;
            Ok(())
        }
        .await;
        if let Err(e) = store {
            log::error!("Failed to store response: {}", e);
        }
    });

    Ok(builder.streaming(rx))
}

#[cfg(test)]
mod tests {
    use super::*;

    // --- SseParser tests ---

    #[test]
    fn sse_parser_single_event() {
        let mut parser = SseParser::new();
        let events = parser.feed("event: message_start\ndata: {\"type\":\"message_start\"}\n\n");
        assert_eq!(events, vec!["{\"type\":\"message_start\"}"]);
    }

    #[test]
    fn sse_parser_split_chunks() {
        let mut parser = SseParser::new();
        let events1 = parser.feed("event: content\nda");
        assert!(events1.is_empty());
        let events2 = parser.feed("ta: hello world\n\n");
        assert_eq!(events2, vec!["hello world"]);
    }

    #[test]
    fn sse_parser_multiple_events() {
        let mut parser = SseParser::new();
        let events = parser.feed("event: a\ndata: one\n\nevent: b\ndata: two\n\n");
        assert_eq!(events, vec!["one", "two"]);
    }

    #[test]
    fn sse_parser_multiline_data() {
        let mut parser = SseParser::new();
        let events = parser.feed("event: msg\ndata: line1\ndata: line2\n\n");
        assert_eq!(events, vec!["line1\nline2"]);
    }

    #[test]
    fn sse_parser_flush() {
        let mut parser = SseParser::new();
        let events = parser.feed("event: last\ndata: final\n");
        assert!(events.is_empty());
        assert_eq!(parser.flush(), Some("final".to_string()));
    }

    #[test]
    fn sse_parser_comments_ignored() {
        let mut parser = SseParser::new();
        let events = parser.feed(": this is a comment\nevent: ping\ndata: pong\n\n");
        assert_eq!(events, vec!["pong"]);
    }

    #[test]
    fn sse_parser_no_event_field() {
        let mut parser = SseParser::new();
        let events = parser.feed("data: just data\n\n");
        assert_eq!(events, vec!["just data"]);
    }

    #[test]
    fn sse_parser_carriage_returns() {
        let mut parser = SseParser::new();
        let events = parser.feed("event: cr\r\ndata: value\r\n\r\n");
        assert_eq!(events, vec!["value"]);
    }

    // --- Event Stream encoding tests ---

    #[test]
    fn event_stream_message_structure() {
        let msg = encode_event_stream_message(&[(":message-type", "event")], b"payload");
        // Total: 16 (framing) + header_len + payload_len
        // Header: 1 (name_len) + 13 (":message-type") + 1 (type=7) + 2 (value_len) + 5 ("event") = 22
        let expected_headers_len = 22u32;
        let expected_total = 16 + 22 + 7;

        let total = u32::from_be_bytes([msg[0], msg[1], msg[2], msg[3]]);
        let headers_len = u32::from_be_bytes([msg[4], msg[5], msg[6], msg[7]]);
        assert_eq!(total, expected_total as u32);
        assert_eq!(headers_len, expected_headers_len);
        assert_eq!(msg.len(), expected_total);
    }

    #[test]
    fn event_stream_message_crc_verification() {
        let msg = encode_event_stream_message(&[(":event-type", "chunk")], b"test");
        // Verify prelude CRC
        let prelude_crc = crc32fast::hash(&msg[..8]);
        let stored_prelude_crc = u32::from_be_bytes([msg[8], msg[9], msg[10], msg[11]]);
        assert_eq!(prelude_crc, stored_prelude_crc);

        // Verify message CRC
        let message_crc = crc32fast::hash(&msg[..msg.len() - 4]);
        let stored_message_crc = u32::from_be_bytes([
            msg[msg.len() - 4],
            msg[msg.len() - 3],
            msg[msg.len() - 2],
            msg[msg.len() - 1],
        ]);
        assert_eq!(message_crc, stored_message_crc);
    }

    #[test]
    fn bedrock_chunk_base64_roundtrip() {
        let data = r#"{"type":"content_block_delta","delta":{"text":"hi"}}"#;
        let frame = encode_bedrock_chunk(data);

        // Extract payload from the frame (skip 12 bytes prelude, then headers, then payload before final 4-byte CRC)
        let total = u32::from_be_bytes([frame[0], frame[1], frame[2], frame[3]]) as usize;
        let headers_len = u32::from_be_bytes([frame[4], frame[5], frame[6], frame[7]]) as usize;
        let payload_start = 12 + headers_len;
        let payload_end = total - 4;
        let payload_str = std::str::from_utf8(&frame[payload_start..payload_end]).unwrap();

        let payload_val: serde_json::Value = serde_json::from_str(payload_str).unwrap();
        let b64 = payload_val["bytes"].as_str().unwrap();
        let decoded = BASE64.decode(b64).unwrap();
        assert_eq!(std::str::from_utf8(&decoded).unwrap(), data);
    }
}
