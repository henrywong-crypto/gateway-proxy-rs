use serde_json::Value;
use std::collections::HashSet;

/// A tool_use block extracted from SSE events.
#[derive(Debug, Clone)]
pub(super) struct ToolUse {
    pub id: String,
    pub name: String,
    pub input: Value,
}

/// Intercepted tool_use blocks extracted from the SSE stream.
#[derive(Debug)]
pub(super) struct InterceptedTools {
    pub content_blocks: Vec<Value>,
    pub tool_uses: Vec<ToolUse>,
}

/// Check if a host matches a whitelisted domain (exact match or subdomain).
/// E.g. whitelist entry "github.com" matches "github.com" and "api.github.com".
pub(super) fn matches_whitelist_host(host: &str, whitelist: &[String]) -> bool {
    whitelist
        .iter()
        .any(|domain| host == domain || host.ends_with(&format!(".{}", domain)))
}

/// Check if ALL tool calls are whitelisted WebFetch calls.
/// Returns true only if every tool is a WebFetch with a URL whose host matches the whitelist.
pub(super) fn is_all_whitelisted(
    tool_uses: &[ToolUse],
    whitelist: &[String],
    webfetch_names: &[String],
) -> bool {
    if whitelist.is_empty() || tool_uses.is_empty() {
        return false;
    }
    tool_uses.iter().all(|tu| {
        if !webfetch_names.iter().any(|n| n == &tu.name) {
            return false;
        }
        let url_str = match tu.input.get("url").and_then(|v| v.as_str()) {
            Some(u) => u,
            None => return false,
        };
        let parsed = match url::Url::parse(url_str) {
            Ok(u) => u,
            Err(_) => return false,
        };
        let host = match parsed.host_str() {
            Some(h) => h,
            None => return false,
        };
        matches_whitelist_host(host, whitelist)
    })
}

/// Parse SSE events and detect webfetch tool usage — `tool_use` blocks with
/// stop_reason "tool_use" (custom tools needing a follow-up request).
pub(super) fn extract_webfetch_from_sse(
    events: &[Value],
    webfetch_names: &[String],
) -> Option<InterceptedTools> {
    let stop_reason = events.iter().find_map(|e| {
        if e.get("event").and_then(|v| v.as_str()) != Some("message_delta") {
            return None;
        }
        e.get("data")
            .and_then(|d| d.get("delta"))
            .and_then(|d| d.get("stop_reason"))
            .and_then(|v| v.as_str())
            .map(|s| s.to_string())
    });

    // Reconstruct all content blocks from SSE events
    let mut content_blocks: Vec<Value> = Vec::new();
    let mut current_block: Option<Value> = None;
    let mut text_accum = String::new();
    let mut json_accum = String::new();
    let mut thinking_accum = String::new();
    let mut signature_accum = String::new();

    for event in events {
        let event_type = match event.get("event").and_then(|v| v.as_str()) {
            Some(t) => t,
            None => continue,
        };
        let data = match event.get("data") {
            Some(d) => d,
            None => continue,
        };

        match event_type {
            "content_block_start" => {
                current_block = data.get("content_block").cloned();
                text_accum.clear();
                json_accum.clear();
                thinking_accum.clear();
                signature_accum.clear();
            }
            "content_block_delta" => {
                if let Some(delta) = data.get("delta") {
                    let delta_type = delta.get("type").and_then(|v| v.as_str()).unwrap_or("");
                    match delta_type {
                        "text_delta" => {
                            if let Some(text) = delta.get("text").and_then(|v| v.as_str()) {
                                text_accum.push_str(text);
                            }
                        }
                        "input_json_delta" => {
                            if let Some(json_part) =
                                delta.get("partial_json").and_then(|v| v.as_str())
                            {
                                json_accum.push_str(json_part);
                            }
                        }
                        "thinking_delta" => {
                            if let Some(text) = delta.get("thinking").and_then(|v| v.as_str()) {
                                thinking_accum.push_str(text);
                            }
                        }
                        "signature_delta" => {
                            if let Some(sig) = delta.get("signature").and_then(|v| v.as_str()) {
                                signature_accum.push_str(sig);
                            }
                        }
                        _ => {}
                    }
                }
            }
            "content_block_stop" => {
                if let Some(mut block) = current_block.take() {
                    let block_type = block
                        .get("type")
                        .and_then(|v| v.as_str())
                        .unwrap_or("")
                        .to_string();
                    match block_type.as_str() {
                        "text" => {
                            block["text"] = Value::String(text_accum.clone());
                        }
                        "thinking" => {
                            block["thinking"] = Value::String(thinking_accum.clone());
                            if !signature_accum.is_empty() {
                                block["signature"] = Value::String(signature_accum.clone());
                            }
                        }
                        // Both custom tool_use and native server_tool_use carry input JSON
                        "tool_use" | "server_tool_use" => {
                            let input: Value =
                                serde_json::from_str(&json_accum).unwrap_or(serde_json::json!({}));
                            block["input"] = input;
                        }
                        _ => {}
                    }
                    content_blocks.push(block);
                }
            }
            _ => {}
        }
    }

    match stop_reason.as_deref() {
        Some("tool_use") => {
            // Custom tool: the model is waiting for the proxy to execute and return results.
            let tool_uses: Vec<ToolUse> = content_blocks
                .iter()
                .filter_map(|block| {
                    if block.get("type").and_then(|v| v.as_str()) != Some("tool_use") {
                        return None;
                    }
                    let name = block.get("name").and_then(|v| v.as_str())?.to_string();
                    if !webfetch_names.iter().any(|n| n == &name) {
                        return None;
                    }
                    let id = block.get("id").and_then(|v| v.as_str())?.to_string();
                    let input = block.get("input").cloned().unwrap_or(serde_json::json!({}));
                    Some(ToolUse { id, name, input })
                })
                .collect();

            if tool_uses.is_empty() {
                return None;
            }

            Some(InterceptedTools {
                content_blocks,
                tool_uses,
            })
        }
        _ => None,
    }
}

/// Build an input summary string for display in the dashboard UI.
pub(super) fn build_input_summary(tool_use: &ToolUse) -> String {
    let url = tool_use
        .input
        .get("url")
        .and_then(|v| v.as_str())
        .unwrap_or("<unknown>");
    format!("URL: {}", url)
}

/// Construct the follow-up request body.
/// Takes the original (filtered) request body, the assistant's content blocks,
/// and the mock tool_result blocks.
pub(super) fn build_followup_body(
    original_body: &Value,
    assistant_content: Vec<Value>,
    tool_results: Vec<Value>,
) -> Value {
    let mut body = original_body.clone();

    // Build updated messages array: original messages + assistant message + user tool_results
    let mut messages: Vec<Value> = original_body
        .get("messages")
        .and_then(|v| v.as_array())
        .cloned()
        .unwrap_or_default();

    // Add assistant message with the content blocks from the response
    messages.push(serde_json::json!({
        "role": "assistant",
        "content": assistant_content,
    }));

    // Add user message with tool_results
    messages.push(serde_json::json!({
        "role": "user",
        "content": tool_results,
    }));

    body["messages"] = Value::Array(messages);
    body["stream"] = Value::Bool(true);

    body
}

/// Remove unmatched tool_use blocks from content, keeping only blocks whose
/// IDs appear in the given tool_uses list.
pub(super) fn retain_matched_tool_blocks(content_blocks: &mut Vec<Value>, tool_uses: &[ToolUse]) {
    let kept_ids: HashSet<&str> = tool_uses.iter().map(|t| t.id.as_str()).collect();
    content_blocks.retain(|block| {
        let block_type = block.get("type").and_then(|v| v.as_str()).unwrap_or("");
        if block_type != "tool_use" {
            return true;
        }
        let id = block.get("id").and_then(|v| v.as_str()).unwrap_or("");
        kept_ids.contains(id)
    });
}
