use serde_json::Value;

use super::extract::ToolUse;
use super::mock::render_template;
use crate::shared::{
    extract_request_fields, headers_to_json, log_request, store_response, RequestMeta,
};
use crate::sse::{extract_text_from_events, parse_sse_events};

/// Maximum size (in bytes) of fetched content to include in a tool_result.
const MAX_ACCEPT_CONTENT_BYTES: usize = 100 * 1024;

pub const WEBFETCH_AGENT_SYSTEM_PROMPT: &str =
    "You are Claude Code, Anthropic's official CLI for Claude.";

/// Result of building an accept tool_result, optionally with an agent request.
pub(super) struct AcceptResult {
    pub tool_result: Value,
    pub agent_request_id: Option<String>,
}

/// Shared context for fetching and agent requests.
pub(super) struct FetchContext<'a> {
    pub client: &'a reqwest::Client,
    pub webfetch_names: &'a [String],
    pub accept_prompt: &'a str,
    pub redirect_prompt: &'a str,
    pub agent_model: &'a str,
    pub target_url: &'a str,
    pub forward_headers: &'a reqwest::header::HeaderMap,
    pub pool: &'a sqlx::SqlitePool,
    pub session_id: &'a str,
    pub stored_path: &'a str,
}

/// Actually fetch the URL for a WebFetch tool call and return the content as a tool_result.
/// For non-WebFetch tools this returns an error result since we can only
/// perform URL fetching.
///
/// When `agent_model` is non-empty, sends the rendered page content to an agentic API
/// request instead of stuffing it directly into the tool_result.
pub(super) async fn build_accept_result(
    tool_use: &ToolUse,
    ctx: &FetchContext<'_>,
) -> AcceptResult {
    if !ctx.webfetch_names.iter().any(|n| n == &tool_use.name) {
        return AcceptResult {
            tool_result: serde_json::json!({
                "type": "tool_result",
                "tool_use_id": tool_use.id,
                "is_error": true,
                "content": format!(
                    "Accept is only supported for WebFetch tool calls. '{}' cannot be executed by the proxy.",
                    tool_use.name
                ),
            }),
            agent_request_id: None,
        };
    }

    let url_str = match tool_use.input.get("url").and_then(|v| v.as_str()) {
        Some(u) => u,
        None => {
            return AcceptResult {
                tool_result: serde_json::json!({
                    "type": "tool_result",
                    "tool_use_id": tool_use.id,
                    "is_error": true,
                    "content": "WebFetch tool call is missing the 'url' input field.",
                }),
                agent_request_id: None,
            };
        }
    };

    let user_prompt = tool_use
        .input
        .get("prompt")
        .and_then(|v| v.as_str())
        .unwrap_or("");

    let original_url = match url::Url::parse(url_str) {
        Ok(u) => u,
        Err(e) => {
            return AcceptResult {
                tool_result: serde_json::json!({
                    "type": "tool_result",
                    "tool_use_id": tool_use.id,
                    "is_error": true,
                    "content": format!("Invalid URL '{}': {}", url_str, e),
                }),
                agent_request_id: None,
            };
        }
    };

    let original_host = original_url.host_str().unwrap_or("").to_string();

    // Fetch with Accept header preferring markdown/html
    let fetch_response = match ctx
        .client
        .get(url_str)
        .header("Accept", "text/markdown, text/html, */*")
        .send()
        .await
    {
        Ok(r) => r,
        Err(e) => {
            return AcceptResult {
                tool_result: serde_json::json!({
                    "type": "tool_result",
                    "tool_use_id": tool_use.id,
                    "is_error": true,
                    "content": format!("Failed to fetch URL '{}': {}", url_str, e),
                }),
                agent_request_id: None,
            };
        }
    };

    let status = fetch_response.status();

    // Handle redirects (client has redirect::Policy::none())
    if status.is_redirection() {
        if let Some(location) = fetch_response
            .headers()
            .get("location")
            .and_then(|v| v.to_str().ok())
        {
            // Resolve relative redirects against the original URL
            let redirect_url = match original_url.join(location) {
                Ok(u) => u,
                Err(_) => {
                    return AcceptResult {
                        tool_result: serde_json::json!({
                            "type": "tool_result",
                            "tool_use_id": tool_use.id,
                            "is_error": true,
                            "content": format!("Redirect to invalid URL: {}", location),
                        }),
                        agent_request_id: None,
                    };
                }
            };
            let redirect_host = redirect_url.host_str().unwrap_or("").to_string();

            if redirect_host != original_host {
                // Cross-host redirect: inform the LLM so it can re-call with the new URL
                let content = render_template(
                    ctx.redirect_prompt,
                    &serde_json::json!({
                        "original_url": url_str,
                        "redirect_url": redirect_url.as_str(),
                        "status": status.as_u16().to_string(),
                        "prompt": user_prompt,
                    }),
                );
                return AcceptResult {
                    tool_result: serde_json::json!({
                        "type": "tool_result",
                        "tool_use_id": tool_use.id,
                        "content": content,
                    }),
                    agent_request_id: None,
                };
            }

            // Same-host redirect: follow it manually
            let follow_response = match ctx
                .client
                .get(redirect_url.as_str())
                .header("Accept", "text/markdown, text/html, */*")
                .send()
                .await
            {
                Ok(r) => r,
                Err(e) => {
                    return AcceptResult {
                        tool_result: serde_json::json!({
                            "type": "tool_result",
                            "tool_use_id": tool_use.id,
                            "is_error": true,
                            "content": format!("Failed to follow redirect to '{}': {}", redirect_url, e),
                        }),
                        agent_request_id: None,
                    };
                }
            };

            if !follow_response.status().is_success() {
                return AcceptResult {
                    tool_result: serde_json::json!({
                        "type": "tool_result",
                        "tool_use_id": tool_use.id,
                        "is_error": true,
                        "content": format!("HTTP error {} when fetching '{}'", follow_response.status().as_u16(), redirect_url),
                    }),
                    agent_request_id: None,
                };
            }

            return match follow_response.bytes().await {
                Ok(bytes) => {
                    parse_bytes_to_accept_result(
                        &tool_use.id,
                        &bytes,
                        user_prompt,
                        &original_host,
                        ctx,
                    )
                    .await
                }
                Err(e) => AcceptResult {
                    tool_result: serde_json::json!({
                        "type": "tool_result",
                        "tool_use_id": tool_use.id,
                        "is_error": true,
                        "content": format!("Failed to read response body from '{}': {}", redirect_url, e),
                    }),
                    agent_request_id: None,
                },
            };
        }

        // 3xx without Location header
        return AcceptResult {
            tool_result: serde_json::json!({
                "type": "tool_result",
                "tool_use_id": tool_use.id,
                "is_error": true,
                "content": format!("HTTP {} redirect without Location header for '{}'", status.as_u16(), url_str),
            }),
            agent_request_id: None,
        };
    }

    if !status.is_success() {
        return AcceptResult {
            tool_result: serde_json::json!({
                "type": "tool_result",
                "tool_use_id": tool_use.id,
                "is_error": true,
                "content": format!("HTTP error {} when fetching '{}'", status.as_u16(), url_str),
            }),
            agent_request_id: None,
        };
    }

    match fetch_response.bytes().await {
        Ok(bytes) => {
            parse_bytes_to_accept_result(&tool_use.id, &bytes, user_prompt, &original_host, ctx)
                .await
        }
        Err(e) => AcceptResult {
            tool_result: serde_json::json!({
                "type": "tool_result",
                "tool_use_id": tool_use.id,
                "is_error": true,
                "content": format!("Failed to read response body from '{}': {}", url_str, e),
            }),
            agent_request_id: None,
        },
    }
}

/// Helper: send fetched bytes to agent model for summarization.
async fn parse_bytes_to_accept_result(
    tool_use_id: &str,
    bytes: &[u8],
    user_prompt: &str,
    url_host: &str,
    ctx: &FetchContext<'_>,
) -> AcceptResult {
    let rendered = render_accept_content(bytes, ctx.accept_prompt, user_prompt);
    send_agent_request(tool_use_id, &rendered, url_host, ctx).await
}

/// Convert fetched HTML bytes into rendered text content using the accept prompt template.
/// Returns the rendered string (HTML-to-text + truncation + Handlebars template).
fn render_accept_content(bytes: &[u8], accept_prompt: &str, user_prompt: &str) -> String {
    let text = match html2text::from_read(bytes, 120) {
        Ok(t) => t,
        Err(_) => String::from_utf8_lossy(bytes).to_string(),
    };
    let raw_content = if text.len() > MAX_ACCEPT_CONTENT_BYTES {
        let mut truncated = text[..MAX_ACCEPT_CONTENT_BYTES].to_string();
        truncated.push_str("\n\n[Content truncated at 100KB]");
        truncated
    } else {
        text
    };
    render_template(
        accept_prompt,
        &serde_json::json!({"content": raw_content, "prompt": user_prompt}),
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn render_accept_content_basic_html() {
        let html = b"<html><body><h1>Hello World</h1><p>Some content</p></body></html>";
        let result = render_accept_content(html, "Content: {{content}}", "summarize this");
        assert!(result.contains("Hello World"));
        assert!(result.contains("Some content"));
        assert!(result.starts_with("Content: "));
    }

    #[test]
    fn render_accept_content_plain_text() {
        let text = b"Just plain text content";
        let result = render_accept_content(text, "{{content}}", "");
        assert!(result.contains("Just plain text content"));
    }

    #[test]
    fn render_accept_content_includes_prompt() {
        let html = b"<p>Page</p>";
        let result =
            render_accept_content(html, "Content: {{content}} Prompt: {{prompt}}", "my prompt");
        assert!(result.contains("my prompt"));
    }

    #[test]
    fn render_accept_content_truncation() {
        // Create content larger than 100KB
        let large_html = vec![b'a'; 200 * 1024];
        let result = render_accept_content(&large_html, "{{content}}", "");
        assert!(result.contains("[Content truncated at 100KB]"));
        // The output should be bounded in size (template wrapping + truncated content)
        assert!(result.len() < 150 * 1024);
    }

    #[test]
    fn render_accept_content_empty_template() {
        let html = b"<p>test</p>";
        let result = render_accept_content(html, "", "");
        // Empty template renders to empty string
        assert!(result.is_empty());
    }

    #[test]
    fn render_accept_content_no_template_vars() {
        let html = b"<p>test</p>";
        let result = render_accept_content(html, "static prompt", "");
        assert_eq!(result, "static prompt");
    }
}

/// Log an agent request to the database. Returns the request ID on success.
async fn log_agent_request(
    ctx: &FetchContext<'_>,
    agent_body: &Value,
    url_host: &str,
) -> Result<String, ()> {
    let timestamp = chrono::Local::now().format("%H:%M:%S").to_string();
    let note = format!("webfetch agent ({})", url_host);
    let fields = extract_request_fields(agent_body, None).unwrap_or_default();
    let headers_json = headers_to_json(
        ctx.forward_headers
            .iter()
            .filter_map(|(k, v)| v.to_str().ok().map(|s| (k.to_string(), s.to_string()))),
    )
    .ok();
    match log_request(
        &RequestMeta {
            pool: ctx.pool,
            session_id: ctx.session_id,
            method: "POST",
            path: ctx.stored_path,
            timestamp: &timestamp,
            headers_json: headers_json.as_deref(),
            note: Some(&note),
        },
        &fields,
    )
    .await
    {
        Ok(id) => Ok(id),
        Err(e) => {
            log::warn!("webfetch agent: failed to log request: {}", e);
            Err(())
        }
    }
}

/// Send the agent request upstream and return `(status, headers, body)`.
async fn send_upstream_agent_request(
    ctx: &FetchContext<'_>,
    agent_body: &Value,
) -> Result<(u16, reqwest::header::HeaderMap, bytes::Bytes), ()> {
    let agent_bytes = match serde_json::to_vec(agent_body) {
        Ok(b) => b,
        Err(_) => return Err(()),
    };

    let mut agent_headers = ctx.forward_headers.clone();
    agent_headers.remove(reqwest::header::CONTENT_LENGTH);

    let agent_response = match ctx
        .client
        .post(ctx.target_url)
        .headers(agent_headers)
        .body(agent_bytes)
        .send()
        .await
    {
        Ok(r) => r,
        Err(e) => {
            log::warn!("webfetch agent: upstream request failed: {}", e);
            return Err(());
        }
    };

    let resp_status = agent_response.status().as_u16();
    let resp_headers = agent_response.headers().clone();
    let resp_body = match agent_response.bytes().await {
        Ok(b) => b,
        Err(e) => {
            log::warn!("webfetch agent: failed to read response: {}", e);
            return Err(());
        }
    };

    Ok((resp_status, resp_headers, resp_body))
}

/// Store the agent response in the database.
async fn store_agent_response(
    ctx: &FetchContext<'_>,
    agent_request_id: &str,
    resp_status: u16,
    resp_headers: &reqwest::header::HeaderMap,
    resp_body_str: &str,
) {
    let resp_headers_json = headers_to_json(
        resp_headers
            .iter()
            .filter_map(|(k, v)| v.to_str().ok().map(|s| (k.to_string(), s.to_string()))),
    )
    .ok();
    if let Err(e) = store_response(
        ctx.pool,
        agent_request_id,
        resp_status,
        resp_headers_json.as_deref(),
        resp_body_str,
    )
    .await
    {
        log::warn!("webfetch agent: failed to store response: {}", e);
    }
}

/// Send an agentic API request with the rendered page content and return the
/// agent's response text as a tool_result. Logs the request and response in the DB.
/// On failure, falls back to raw content tool_result.
async fn send_agent_request(
    tool_use_id: &str,
    rendered_content: &str,
    url_host: &str,
    ctx: &FetchContext<'_>,
) -> AcceptResult {
    let agent_model = std::env::var("ANTHROPIC_DEFAULT_HAIKU_MODEL")
        .unwrap_or_else(|_| ctx.agent_model.to_string());
    let agent_body = serde_json::json!({
        "model": agent_model,
        "messages": [{
            "role": "user",
            "content": [{"type": "text", "text": rendered_content}]
        }],
        "system": [{"type": "text", "text": WEBFETCH_AGENT_SYSTEM_PROMPT}],
        "max_tokens": 16384,
        "stream": true,
    });

    // Log the agent request
    let agent_request_id = match log_agent_request(ctx, &agent_body, url_host).await {
        Ok(id) => id,
        Err(()) => {
            return AcceptResult {
                tool_result: serde_json::json!({
                    "type": "tool_result",
                    "tool_use_id": tool_use_id,
                    "content": rendered_content,
                }),
                agent_request_id: None,
            };
        }
    };

    // Send the agent request upstream
    let (resp_status, resp_headers, resp_body) =
        match send_upstream_agent_request(ctx, &agent_body).await {
            Ok(result) => result,
            Err(()) => {
                return AcceptResult {
                    tool_result: serde_json::json!({
                        "type": "tool_result",
                        "tool_use_id": tool_use_id,
                        "content": rendered_content,
                    }),
                    agent_request_id: Some(agent_request_id),
                };
            }
        };

    let resp_body_str = String::from_utf8_lossy(&resp_body).to_string();

    // Store the response
    store_agent_response(
        ctx,
        &agent_request_id,
        resp_status,
        &resp_headers,
        &resp_body_str,
    )
    .await;

    // Extract text from the SSE events
    let sse_events = parse_sse_events(&resp_body_str);
    let agent_text = extract_text_from_events(&sse_events);

    if agent_text.is_empty() {
        log::warn!("webfetch agent: no text extracted from response, falling back to raw content");
        return AcceptResult {
            tool_result: serde_json::json!({
                "type": "tool_result",
                "tool_use_id": tool_use_id,
                "content": rendered_content,
            }),
            agent_request_id: Some(agent_request_id),
        };
    }

    AcceptResult {
        tool_result: serde_json::json!({
            "type": "tool_result",
            "tool_use_id": tool_use_id,
            "content": agent_text,
        }),
        agent_request_id: Some(agent_request_id),
    }
}
