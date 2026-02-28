use std::collections::HashMap;
use std::sync::{Arc, Mutex};

use serde_json::{json, Value};
use tokio::sync::oneshot;

// ---------------------------------------------------------------------------
// Approval queue types
// ---------------------------------------------------------------------------

/// User decision for a pending websearch tool call.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ApprovalDecision {
    Fail,
    Mock,
    Accept,
}

/// Information about an intercepted tool call shown in the dashboard UI.
#[derive(Debug, Clone)]
pub struct PendingToolInfo {
    pub name: String,
    pub input_summary: String,
}

/// A pending approval waiting for user action.
pub struct PendingApproval {
    pub session_id: String,
    pub tools: Vec<PendingToolInfo>,
    pub sender: oneshot::Sender<ApprovalDecision>,
}

/// Shared approval queue: maps approval_id → PendingApproval.
pub type ApprovalQueue = Arc<Mutex<HashMap<String, PendingApproval>>>;

/// Create a new empty approval queue.
pub fn new_approval_queue() -> ApprovalQueue {
    Arc::new(Mutex::new(HashMap::new()))
}

/// List pending approvals for a given session.
pub fn list_pending(
    queue: &ApprovalQueue,
    session_id: &str,
) -> Vec<(String, Vec<PendingToolInfo>)> {
    let map = queue.lock().unwrap();
    map.iter()
        .filter(|(_, v)| v.session_id == session_id)
        .map(|(id, v)| (id.clone(), v.tools.clone()))
        .collect()
}

/// Resolve a pending approval by sending the decision through the oneshot channel.
/// Returns `true` if the approval was found and resolved.
pub fn resolve_pending(
    queue: &ApprovalQueue,
    approval_id: &str,
    decision: ApprovalDecision,
) -> bool {
    let pending = {
        let mut map = queue.lock().unwrap();
        map.remove(approval_id)
    };
    if let Some(pending) = pending {
        let _ = pending.sender.send(decision);
        true
    } else {
        false
    }
}

// ---------------------------------------------------------------------------
// Internal types
// ---------------------------------------------------------------------------

/// A tool_use block extracted from SSE events.
#[derive(Debug, Clone)]
struct ToolUse {
    id: String,
    name: String,
    input: Value,
}

/// What kind of web search activity was detected in the SSE stream.
#[derive(Debug)]
enum InterceptKind {
    /// Custom `tool_use` block (stop_reason: "tool_use") — proxy must supply mock results and
    /// send a follow-up request upstream.
    CustomTool {
        content_blocks: Vec<Value>,
        tool_uses: Vec<ToolUse>,
    },
    /// Anthropic-native `server_tool_use` block (stop_reason: "end_turn") — Anthropic already
    /// ran the search and returned results inline; the proxy can only annotate the log.
    NativeSearch { tool_names: Vec<String> },
}

/// Check if a host matches a whitelisted domain (exact match or subdomain).
/// E.g. whitelist entry "github.com" matches "github.com" and "api.github.com".
fn host_matches_whitelist(host: &str, whitelist: &[String]) -> bool {
    whitelist
        .iter()
        .any(|domain| host == domain || host.ends_with(&format!(".{}", domain)))
}

/// Check if ALL tool calls are whitelisted WebFetch calls.
/// Returns true only if every tool is a WebFetch with a URL whose host matches the whitelist.
fn is_all_whitelisted(
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
        host_matches_whitelist(host, whitelist)
    })
}

/// Parse SSE events and detect web search tool usage, covering both:
/// - `tool_use` blocks with stop_reason "tool_use" (custom tools needing a follow-up)
/// - `server_tool_use` blocks with stop_reason "end_turn" (Anthropic native search, results inline)
fn extract_websearch_from_sse(
    events: &[Value],
    websearch_names: &[String],
    webfetch_names: &[String],
) -> Option<InterceptKind> {
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
                                serde_json::from_str(&json_accum).unwrap_or(json!({}));
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
                    if !websearch_names.iter().any(|n| n == &name)
                        && !webfetch_names.iter().any(|n| n == &name)
                    {
                        return None;
                    }
                    let id = block.get("id").and_then(|v| v.as_str())?.to_string();
                    let input = block.get("input").cloned().unwrap_or(json!({}));
                    Some(ToolUse { id, name, input })
                })
                .collect();

            if tool_uses.is_empty() {
                return None;
            }

            Some(InterceptKind::CustomTool {
                content_blocks,
                tool_uses,
            })
        }
        Some("end_turn") => {
            // Anthropic native search: server_tool_use blocks with results already inline.
            // The search already ran; we can only observe and annotate.
            let tool_names: Vec<String> = content_blocks
                .iter()
                .filter_map(|block| {
                    if block.get("type").and_then(|v| v.as_str()) != Some("server_tool_use") {
                        return None;
                    }
                    let name = block.get("name").and_then(|v| v.as_str())?.to_string();
                    if !websearch_names.iter().any(|n| n == &name)
                        && !webfetch_names.iter().any(|n| n == &name)
                    {
                        return None;
                    }
                    Some(name)
                })
                .collect();

            if tool_names.is_empty() {
                return None;
            }

            Some(InterceptKind::NativeSearch { tool_names })
        }
        _ => None,
    }
}

/// Generate a mock tool_result for a given tool_use.
fn build_mock_result(tool_use: &ToolUse, webfetch_names: &[String]) -> Value {
    let content = if webfetch_names.iter().any(|n| n == &tool_use.name) {
        let url = tool_use
            .input
            .get("url")
            .and_then(|v| v.as_str())
            .unwrap_or("<unknown>");
        format!(
            "[Proxy mock] Web fetch intercepted. URL: '{}'. No real fetch was performed.",
            url
        )
    } else {
        // WebSearch or web_search_*
        let query = tool_use
            .input
            .get("query")
            .and_then(|v| v.as_str())
            .unwrap_or("<unknown>");
        format!(
            "[Proxy mock] Web search intercepted. Query: '{}'. No real search was performed.",
            query
        )
    };

    json!({
        "type": "tool_result",
        "tool_use_id": tool_use.id,
        "content": content,
    })
}

/// Generate a fail tool_result (is_error: true) for a rejected tool call.
fn build_fail_result(tool_use: &ToolUse) -> Value {
    json!({
        "type": "tool_result",
        "tool_use_id": tool_use.id,
        "is_error": true,
        "content": "The user doesn't want to proceed with this tool use. The tool use was rejected. Web search and fetch tools are not available through this proxy.",
    })
}

/// Maximum size (in bytes) of fetched content to include in a tool_result.
const MAX_ACCEPT_CONTENT_BYTES: usize = 100 * 1024;

/// Actually fetch the URL for a WebFetch tool call and return the content as a tool_result.
/// For non-WebFetch tools (e.g. WebSearch) this returns an error result since we can't
/// perform a real web search — only WebFetch (URL fetching) is supported for Accept.
async fn build_accept_result(
    tool_use: &ToolUse,
    client: &reqwest::Client,
    webfetch_names: &[String],
) -> Value {
    if !webfetch_names.iter().any(|n| n == &tool_use.name) {
        return json!({
            "type": "tool_result",
            "tool_use_id": tool_use.id,
            "is_error": true,
            "content": format!(
                "Accept is only supported for WebFetch tool calls. '{}' cannot be executed by the proxy.",
                tool_use.name
            ),
        });
    }

    let url_str = match tool_use.input.get("url").and_then(|v| v.as_str()) {
        Some(u) => u,
        None => {
            return json!({
                "type": "tool_result",
                "tool_use_id": tool_use.id,
                "is_error": true,
                "content": "WebFetch tool call is missing the 'url' input field.",
            });
        }
    };

    let original_url = match url::Url::parse(url_str) {
        Ok(u) => u,
        Err(e) => {
            return json!({
                "type": "tool_result",
                "tool_use_id": tool_use.id,
                "is_error": true,
                "content": format!("Invalid URL '{}': {}", url_str, e),
            });
        }
    };

    let original_host = original_url.host_str().unwrap_or("").to_string();

    // Fetch with Accept header preferring markdown/html
    let resp = match client
        .get(url_str)
        .header("Accept", "text/markdown, text/html, */*")
        .send()
        .await
    {
        Ok(r) => r,
        Err(e) => {
            return json!({
                "type": "tool_result",
                "tool_use_id": tool_use.id,
                "is_error": true,
                "content": format!("Failed to fetch URL '{}': {}", url_str, e),
            });
        }
    };

    let status = resp.status();

    // Handle redirects (client has redirect::Policy::none())
    if status.is_redirection() {
        if let Some(location) = resp.headers().get("location").and_then(|v| v.to_str().ok()) {
            // Resolve relative redirects against the original URL
            let redirect_url = match original_url.join(location) {
                Ok(u) => u,
                Err(_) => {
                    return json!({
                        "type": "tool_result",
                        "tool_use_id": tool_use.id,
                        "is_error": true,
                        "content": format!("Redirect to invalid URL: {}", location),
                    });
                }
            };
            let redirect_host = redirect_url.host_str().unwrap_or("").to_string();

            if redirect_host != original_host {
                // Cross-host redirect: inform the LLM so it can re-call with the new URL
                let content = format!(
                    "REDIRECT DETECTED: The URL {} redirected to a different host. \
                     New URL: {}. Please re-call WebFetch with the new URL if you want to follow it.",
                    url_str,
                    redirect_url.as_str()
                );
                return json!({
                    "type": "tool_result",
                    "tool_use_id": tool_use.id,
                    "content": content,
                });
            }

            // Same-host redirect: follow it manually
            let follow_resp = match client
                .get(redirect_url.as_str())
                .header("Accept", "text/markdown, text/html, */*")
                .send()
                .await
            {
                Ok(r) => r,
                Err(e) => {
                    return json!({
                        "type": "tool_result",
                        "tool_use_id": tool_use.id,
                        "is_error": true,
                        "content": format!("Failed to follow redirect to '{}': {}", redirect_url, e),
                    });
                }
            };

            if !follow_resp.status().is_success() {
                return json!({
                    "type": "tool_result",
                    "tool_use_id": tool_use.id,
                    "is_error": true,
                    "content": format!("HTTP error {} when fetching '{}'", follow_resp.status().as_u16(), redirect_url),
                });
            }

            return match follow_resp.bytes().await {
                Ok(bytes) => body_to_tool_result(&tool_use.id, &bytes),
                Err(e) => json!({
                    "type": "tool_result",
                    "tool_use_id": tool_use.id,
                    "is_error": true,
                    "content": format!("Failed to read response body from '{}': {}", redirect_url, e),
                }),
            };
        }

        // 3xx without Location header
        return json!({
            "type": "tool_result",
            "tool_use_id": tool_use.id,
            "is_error": true,
            "content": format!("HTTP {} redirect without Location header for '{}'", status.as_u16(), url_str),
        });
    }

    if !status.is_success() {
        return json!({
            "type": "tool_result",
            "tool_use_id": tool_use.id,
            "is_error": true,
            "content": format!("HTTP error {} when fetching '{}'", status.as_u16(), url_str),
        });
    }

    match resp.bytes().await {
        Ok(bytes) => body_to_tool_result(&tool_use.id, &bytes),
        Err(e) => json!({
            "type": "tool_result",
            "tool_use_id": tool_use.id,
            "is_error": true,
            "content": format!("Failed to read response body from '{}': {}", url_str, e),
        }),
    }
}

/// Convert fetched HTML bytes into a text tool_result, truncating if needed.
fn body_to_tool_result(tool_use_id: &str, bytes: &[u8]) -> Value {
    let text = match html2text::from_read(bytes, 120) {
        Ok(t) => t,
        Err(_) => String::from_utf8_lossy(bytes).to_string(),
    };
    let content = if text.len() > MAX_ACCEPT_CONTENT_BYTES {
        let mut truncated = text[..MAX_ACCEPT_CONTENT_BYTES].to_string();
        truncated.push_str("\n\n[Content truncated at 100KB]");
        truncated
    } else {
        text
    };
    json!({
        "type": "tool_result",
        "tool_use_id": tool_use_id,
        "content": content,
    })
}

/// Build an input summary string for display in the dashboard UI.
fn build_input_summary(tool_use: &ToolUse, webfetch_names: &[String]) -> String {
    if webfetch_names.iter().any(|n| n == &tool_use.name) {
        let url = tool_use
            .input
            .get("url")
            .and_then(|v| v.as_str())
            .unwrap_or("<unknown>");
        format!("URL: {}", url)
    } else {
        let query = tool_use
            .input
            .get("query")
            .and_then(|v| v.as_str())
            .unwrap_or("<unknown>");
        format!("Query: {}", query)
    }
}

/// Construct the follow-up request body.
/// Takes the original (filtered) request body, the assistant's content blocks,
/// and the mock tool_result blocks.
fn build_followup_body(
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
    messages.push(json!({
        "role": "assistant",
        "content": assistant_content,
    }));

    // Add user message with tool_results
    messages.push(json!({
        "role": "user",
        "content": tool_results,
    }));

    body["messages"] = Value::Array(messages);
    body["stream"] = Value::Bool(true);

    body
}

/// Maximum number of intercept rounds to prevent infinite loops.
const MAX_INTERCEPT_ROUNDS: usize = 10;

/// Data collected for each round of interception.
struct RoundData {
    decision: String,
    tool_names: Vec<String>,
    request_id: Option<String>,
    followup_body: Value,
    response_body: String,
    response_events: Vec<Value>,
}

/// Result of websearch interception.
#[derive(Debug)]
pub enum InterceptResult {
    /// Custom tool_use was intercepted: contains the follow-up response.
    Intercepted {
        status: u16,
        headers: reqwest::header::HeaderMap,
        body: bytes::Bytes,
        note: String,
        followup_body_json: String,
        rounds_json: String,
    },
    /// Anthropic native server_tool_use detected; original response is unchanged.
    /// Only a log annotation is needed.
    Annotated { note: String },
}

/// Parameters for websearch interception.
pub struct InterceptParams<'a> {
    pub response_body: &'a str,
    pub original_body: &'a [u8],
    pub target_url: &'a str,
    pub forward_headers: &'a reqwest::header::HeaderMap,
    pub client: &'a reqwest::Client,
    pub approval_queue: &'a ApprovalQueue,
    pub session_id: &'a str,
    pub whitelist: &'a [String],
    pub pool: &'a sqlx::SqlitePool,
    pub stored_path: &'a str,
    pub websearch_names: &'a [String],
    pub webfetch_names: &'a [String],
}

/// Main entry point for websearch interception.
///
/// - Custom `tool_use` (stop_reason "tool_use"): pauses and waits for the user's approval
///   decision (Fail or Mock) via the dashboard UI, then builds the appropriate tool_results,
///   sends a follow-up request upstream, and returns the follow-up response to the client.
/// - Anthropic native `server_tool_use` (stop_reason "end_turn"): the search already ran
///   and results are inline; passes the original response through unchanged but returns a
///   note so the request log can be annotated.
///
/// Returns `Some(InterceptResult)` if any web search activity was detected, `None` otherwise.
pub async fn maybe_intercept(params: &InterceptParams<'_>) -> Option<InterceptResult> {
    let response_body = params.response_body;
    let original_body = params.original_body;
    let target_url = params.target_url;
    let forward_headers = params.forward_headers;
    let client = params.client;
    let approval_queue = params.approval_queue;
    let session_id = params.session_id;
    let whitelist = params.whitelist;
    let pool = params.pool;
    let stored_path = params.stored_path;
    let websearch_names = params.websearch_names;
    let webfetch_names = params.webfetch_names;

    let events = crate::sse::parse_sse_events(response_body);

    match extract_websearch_from_sse(&events, websearch_names, webfetch_names)? {
        InterceptKind::NativeSearch { tool_names } => {
            // Native search is always search-type; only annotate if websearch names are configured
            if websearch_names.is_empty() {
                return None;
            }
            let note = format!("native web search detected: {}", tool_names.join(", "));
            log::info!("WebSearch interception: {}", note);
            Some(InterceptResult::Annotated { note })
        }
        InterceptKind::CustomTool {
            mut content_blocks,
            tool_uses,
        } => {
            // Filter tool_uses by enabled toggles
            let tool_uses: Vec<ToolUse> = tool_uses
                .into_iter()
                .filter(|t| {
                    websearch_names.iter().any(|n| n == &t.name)
                        || webfetch_names.iter().any(|n| n == &t.name)
                })
                .collect();
            if tool_uses.is_empty() {
                return None;
            }

            // Remove tool_use content blocks that were filtered out, so the
            // follow-up body stays consistent with the tool_results we provide.
            let kept_ids: std::collections::HashSet<&str> =
                tool_uses.iter().map(|t| t.id.as_str()).collect();
            content_blocks.retain(|block| {
                let block_type = block.get("type").and_then(|v| v.as_str()).unwrap_or("");
                if block_type != "tool_use" {
                    return true;
                }
                let id = block.get("id").and_then(|v| v.as_str()).unwrap_or("");
                kept_ids.contains(id)
            });
            let original_json: Value = match serde_json::from_slice(original_body) {
                Ok(v) => v,
                Err(e) => {
                    log::warn!(
                        "WebSearch interception: failed to parse original body: {}",
                        e
                    );
                    return None;
                }
            };

            // Strip Content-Length once — reuse for all rounds
            let mut headers = forward_headers.clone();
            headers.remove(reqwest::header::CONTENT_LENGTH);

            // State carried across rounds
            let mut current_body = original_json;
            let mut current_content_blocks = content_blocks;
            let mut current_tool_uses = tool_uses;
            let mut rounds: Vec<RoundData> = Vec::new();
            let mut all_tool_names: Vec<String> = Vec::new();

            // Final response state (updated each round)
            let mut final_status: u16 = 0;
            let mut final_headers = reqwest::header::HeaderMap::new();
            let mut final_body = bytes::Bytes::new();

            for round_idx in 0..MAX_INTERCEPT_ROUNDS {
                let intercepted_tools: Vec<&str> =
                    current_tool_uses.iter().map(|t| t.name.as_str()).collect();
                all_tool_names.extend(intercepted_tools.iter().map(|s| s.to_string()));

                log::info!(
                    "WebSearch interception round {}: {} — waiting for user approval",
                    round_idx + 1,
                    intercepted_tools.join(", ")
                );

                // Build tool info for the UI
                let tools_info: Vec<PendingToolInfo> = current_tool_uses
                    .iter()
                    .map(|t| PendingToolInfo {
                        name: t.name.clone(),
                        input_summary: build_input_summary(t, webfetch_names),
                    })
                    .collect();

                // Auto-accept if all tools are whitelisted WebFetch calls
                let (decision, decision_label) =
                    if is_all_whitelisted(&current_tool_uses, whitelist, webfetch_names) {
                        log::info!(
                        "WebSearch interception round {}: all tools whitelisted, auto-accepting",
                        round_idx + 1,
                    );
                        (ApprovalDecision::Accept, "Auto-Accept (whitelisted)")
                    } else {
                        // Create oneshot channel and insert into the approval queue
                        let (tx, rx) = oneshot::channel();
                        let approval_id = uuid::Uuid::new_v4().to_string();
                        {
                            let mut map = approval_queue.lock().unwrap();
                            map.insert(
                                approval_id.clone(),
                                PendingApproval {
                                    session_id: session_id.to_string(),
                                    tools: tools_info,
                                    sender: tx,
                                },
                            );
                        }

                        // Wait for user decision with 120s timeout
                        match tokio::time::timeout(std::time::Duration::from_secs(120), rx).await {
                            Ok(Ok(d)) => {
                                let label = match d {
                                    ApprovalDecision::Accept => "Accept",
                                    ApprovalDecision::Fail => "Fail",
                                    ApprovalDecision::Mock => "Mock",
                                };
                                (d, label)
                            }
                            _ => {
                                let mut map = approval_queue.lock().unwrap();
                                map.remove(&approval_id);
                                log::info!(
                                    "WebSearch interception: approval timed out, auto-failing"
                                );
                                (ApprovalDecision::Fail, "Timeout (auto-fail)")
                            }
                        }
                    };

                log::info!(
                    "WebSearch interception round {}: user decided {:?}",
                    round_idx + 1,
                    decision
                );

                let tool_results: Vec<Value> = match decision {
                    ApprovalDecision::Fail => {
                        current_tool_uses.iter().map(build_fail_result).collect()
                    }
                    ApprovalDecision::Mock => current_tool_uses
                        .iter()
                        .map(|tu| build_mock_result(tu, webfetch_names))
                        .collect(),
                    ApprovalDecision::Accept => {
                        let mut results = Vec::with_capacity(current_tool_uses.len());
                        for tu in &current_tool_uses {
                            results.push(build_accept_result(tu, client, webfetch_names).await);
                        }
                        results
                    }
                };

                let followup_body =
                    build_followup_body(&current_body, current_content_blocks, tool_results);

                let followup_bytes = match serde_json::to_vec(&followup_body) {
                    Ok(v) => v,
                    Err(e) => {
                        log::warn!(
                            "WebSearch interception: failed to serialize follow-up body: {}",
                            e
                        );
                        return None;
                    }
                };

                let resp = match client
                    .post(target_url)
                    .headers(headers.clone())
                    .body(followup_bytes)
                    .send()
                    .await
                {
                    Ok(r) => r,
                    Err(e) => {
                        log::warn!("WebSearch interception: follow-up request failed: {}", e);
                        return None;
                    }
                };

                final_status = resp.status().as_u16();
                final_headers = resp.headers().clone();

                final_body = match resp.bytes().await {
                    Ok(b) => b,
                    Err(e) => {
                        log::warn!(
                            "WebSearch interception: failed to read follow-up response: {}",
                            e
                        );
                        return None;
                    }
                };

                let response_body_str = String::from_utf8_lossy(&final_body).to_string();
                let response_events = crate::sse::parse_sse_events(&response_body_str);

                // Log the follow-up as a separate request entry
                let round_request_id = {
                    let timestamp = chrono::Local::now().format("%H:%M:%S").to_string();
                    let note = format!("websearch follow-up (round {})", round_idx + 1);
                    let fields = crate::shared::extract_request_fields(&followup_body, None)
                        .unwrap_or_default();
                    let headers_json =
                        crate::shared::headers_to_json(headers.iter().filter_map(|(k, v)| {
                            v.to_str().ok().map(|s| (k.to_string(), s.to_string()))
                        }))
                        .ok();
                    match crate::shared::log_request(
                        &crate::shared::RequestMeta {
                            pool,
                            session_id,
                            method: "POST",
                            path: stored_path,
                            timestamp: &timestamp,
                            headers_json: headers_json.as_deref(),
                            note: Some(&note),
                        },
                        &fields,
                    )
                    .await
                    {
                        Ok(id) => {
                            let resp_headers_json = crate::shared::headers_to_json(
                                final_headers.iter().filter_map(|(k, v)| {
                                    v.to_str().ok().map(|s| (k.to_string(), s.to_string()))
                                }),
                            )
                            .ok();
                            let _ = crate::shared::store_response(
                                pool,
                                &id,
                                final_status,
                                resp_headers_json.as_deref(),
                                &response_body_str,
                            )
                            .await;
                            Some(id)
                        }
                        Err(e) => {
                            log::warn!(
                                "WebSearch interception: failed to log follow-up request: {}",
                                e
                            );
                            None
                        }
                    }
                };

                rounds.push(RoundData {
                    decision: decision_label.to_string(),
                    tool_names: current_tool_uses.iter().map(|t| t.name.clone()).collect(),
                    request_id: round_request_id,
                    followup_body: followup_body.clone(),
                    response_body: response_body_str,
                    response_events: response_events.clone(),
                });

                // Check if the follow-up response contains more websearch tool calls
                match extract_websearch_from_sse(&response_events, websearch_names, webfetch_names)
                {
                    Some(InterceptKind::CustomTool {
                        content_blocks: mut next_blocks,
                        tool_uses: next_uses,
                    }) => {
                        // Filter next round's tool_uses by configured names
                        let next_uses: Vec<ToolUse> = next_uses
                            .into_iter()
                            .filter(|t| {
                                websearch_names.iter().any(|n| n == &t.name)
                                    || webfetch_names.iter().any(|n| n == &t.name)
                            })
                            .collect();
                        if next_uses.is_empty() {
                            break;
                        }
                        let kept_ids: std::collections::HashSet<&str> =
                            next_uses.iter().map(|t| t.id.as_str()).collect();
                        next_blocks.retain(|block| {
                            let block_type =
                                block.get("type").and_then(|v| v.as_str()).unwrap_or("");
                            if block_type != "tool_use" {
                                return true;
                            }
                            let id = block.get("id").and_then(|v| v.as_str()).unwrap_or("");
                            kept_ids.contains(id)
                        });
                        // More tool calls — loop again with updated state
                        current_body = followup_body;
                        current_content_blocks = next_blocks;
                        current_tool_uses = next_uses;
                        continue;
                    }
                    _ => {
                        // No more interceptable tool calls — done
                        break;
                    }
                }
            }

            if rounds.is_empty() {
                return None;
            }

            if rounds.len() >= MAX_INTERCEPT_ROUNDS {
                log::warn!(
                    "WebSearch interception: reached max rounds ({}), returning last response as-is",
                    MAX_INTERCEPT_ROUNDS
                );
            }

            // Build note
            let note = if rounds.len() == 1 {
                format!("websearch intercepted: {}", all_tool_names.join(", "))
            } else {
                format!(
                    "websearch intercepted ({} rounds): {}",
                    rounds.len(),
                    all_tool_names.join(", ")
                )
            };

            // First round's followup body for backward compatibility
            let followup_body_json_str =
                match serde_json::to_string_pretty(&rounds[0].followup_body) {
                    Ok(v) => v,
                    Err(e) => {
                        log::warn!(
                            "WebSearch interception: failed to serialize follow-up body: {}",
                            e
                        );
                        return None;
                    }
                };

            // Serialize all rounds to JSON
            let rounds_value: Vec<Value> = rounds
                .iter()
                .map(|r| {
                    json!({
                        "decision": r.decision,
                        "tool_names": r.tool_names,
                        "request_id": r.request_id,
                        "followup_body": r.followup_body,
                        "response_body": r.response_body,
                        "response_events": r.response_events,
                    })
                })
                .collect();
            let rounds_json = serde_json::to_string(&rounds_value).unwrap_or_default();

            Some(InterceptResult::Intercepted {
                status: final_status,
                headers: final_headers,
                body: final_body,
                note,
                followup_body_json: followup_body_json_str,
                rounds_json,
            })
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn default_ws_names() -> Vec<String> {
        vec!["WebSearch".to_string()]
    }

    fn default_wf_names() -> Vec<String> {
        vec!["WebFetch".to_string()]
    }

    #[test]
    fn test_build_mock_result_websearch() {
        let tool_use = ToolUse {
            id: "toolu_123".to_string(),
            name: "WebSearch".to_string(),
            input: json!({"query": "rust programming"}),
        };
        let result = build_mock_result(&tool_use, &default_wf_names());
        assert_eq!(result["type"], "tool_result");
        assert_eq!(result["tool_use_id"], "toolu_123");
        let content = result["content"].as_str().unwrap();
        assert!(content.contains("Web search intercepted"));
        assert!(content.contains("rust programming"));
    }

    #[test]
    fn test_build_mock_result_webfetch() {
        let tool_use = ToolUse {
            id: "toolu_456".to_string(),
            name: "WebFetch".to_string(),
            input: json!({"url": "https://example.com"}),
        };
        let result = build_mock_result(&tool_use, &default_wf_names());
        assert_eq!(result["type"], "tool_result");
        assert_eq!(result["tool_use_id"], "toolu_456");
        let content = result["content"].as_str().unwrap();
        assert!(content.contains("Web fetch intercepted"));
        assert!(content.contains("https://example.com"));
    }

    #[test]
    fn test_build_mock_result_custom_search_name() {
        let tool_use = ToolUse {
            id: "toolu_789".to_string(),
            name: "CustomSearch".to_string(),
            input: json!({"query": "latest news"}),
        };
        // CustomSearch is not in webfetch_names, so it gets the search branch
        let result = build_mock_result(&tool_use, &default_wf_names());
        let content = result["content"].as_str().unwrap();
        assert!(content.contains("Web search intercepted"));
        assert!(content.contains("latest news"));
    }

    #[test]
    fn test_build_fail_result_websearch() {
        let tool_use = ToolUse {
            id: "toolu_fail1".to_string(),
            name: "WebSearch".to_string(),
            input: json!({"query": "test"}),
        };
        let result = build_fail_result(&tool_use);
        assert_eq!(result["type"], "tool_result");
        assert_eq!(result["tool_use_id"], "toolu_fail1");
        assert_eq!(result["is_error"], true);
        let content = result["content"].as_str().unwrap();
        assert!(content.contains("rejected"));
    }

    #[test]
    fn test_build_fail_result_webfetch() {
        let tool_use = ToolUse {
            id: "toolu_fail2".to_string(),
            name: "WebFetch".to_string(),
            input: json!({"url": "https://example.com"}),
        };
        let result = build_fail_result(&tool_use);
        assert_eq!(result["type"], "tool_result");
        assert_eq!(result["tool_use_id"], "toolu_fail2");
        assert_eq!(result["is_error"], true);
    }

    #[test]
    fn test_build_input_summary() {
        let wf_names = default_wf_names();
        let fetch = ToolUse {
            id: "t1".to_string(),
            name: "WebFetch".to_string(),
            input: json!({"url": "https://example.com"}),
        };
        assert_eq!(
            build_input_summary(&fetch, &wf_names),
            "URL: https://example.com"
        );

        let search = ToolUse {
            id: "t2".to_string(),
            name: "WebSearch".to_string(),
            input: json!({"query": "rust lang"}),
        };
        assert_eq!(build_input_summary(&search, &wf_names), "Query: rust lang");
    }

    #[test]
    fn test_list_pending_and_resolve() {
        let queue = new_approval_queue();
        let (tx, rx) = oneshot::channel();
        {
            let mut map = queue.lock().unwrap();
            map.insert(
                "approval_1".to_string(),
                PendingApproval {
                    session_id: "sess_a".to_string(),
                    tools: vec![PendingToolInfo {
                        name: "WebSearch".to_string(),
                        input_summary: "Query: test".to_string(),
                    }],
                    sender: tx,
                },
            );
        }

        // list_pending filters by session_id
        let pending = list_pending(&queue, "sess_a");
        assert_eq!(pending.len(), 1);
        assert_eq!(pending[0].0, "approval_1");

        let empty = list_pending(&queue, "sess_b");
        assert!(empty.is_empty());

        // resolve_pending sends the decision
        assert!(resolve_pending(
            &queue,
            "approval_1",
            ApprovalDecision::Mock
        ));
        assert_eq!(rx.blocking_recv().unwrap(), ApprovalDecision::Mock);

        // Already removed
        assert!(!resolve_pending(
            &queue,
            "approval_1",
            ApprovalDecision::Fail
        ));
    }

    #[test]
    fn test_extract_no_websearch_end_turn() {
        // end_turn with no server_tool_use blocks → None
        let events = vec![
            json!({"event": "message_start", "data": {"type": "message_start"}}),
            json!({"event": "message_delta", "data": {"delta": {"stop_reason": "end_turn"}}}),
        ];
        assert!(
            extract_websearch_from_sse(&events, &default_ws_names(), &default_wf_names()).is_none()
        );
    }

    #[test]
    fn test_extract_custom_tool_websearch() {
        let events = vec![
            json!({
                "event": "message_start",
                "data": {"type": "message_start", "message": {"role": "assistant"}}
            }),
            json!({
                "event": "content_block_start",
                "data": {"index": 0, "content_block": {"type": "text", "text": ""}}
            }),
            json!({
                "event": "content_block_delta",
                "data": {"index": 0, "delta": {"type": "text_delta", "text": "Let me search"}}
            }),
            json!({
                "event": "content_block_stop",
                "data": {"index": 0}
            }),
            json!({
                "event": "content_block_start",
                "data": {"index": 1, "content_block": {"type": "tool_use", "id": "toolu_abc", "name": "WebSearch", "input": {}}}
            }),
            json!({
                "event": "content_block_delta",
                "data": {"index": 1, "delta": {"type": "input_json_delta", "partial_json": "{\"query\":"}}
            }),
            json!({
                "event": "content_block_delta",
                "data": {"index": 1, "delta": {"type": "input_json_delta", "partial_json": " \"test\"}"}}
            }),
            json!({
                "event": "content_block_stop",
                "data": {"index": 1}
            }),
            json!({
                "event": "message_delta",
                "data": {"delta": {"stop_reason": "tool_use"}}
            }),
        ];

        let result = extract_websearch_from_sse(&events, &default_ws_names(), &default_wf_names());
        assert!(result.is_some());

        match result.unwrap() {
            InterceptKind::CustomTool {
                content_blocks,
                tool_uses,
            } => {
                assert_eq!(content_blocks.len(), 2);
                assert_eq!(content_blocks[0]["type"], "text");
                assert_eq!(content_blocks[0]["text"], "Let me search");
                assert_eq!(content_blocks[1]["type"], "tool_use");
                assert_eq!(tool_uses.len(), 1);
                assert_eq!(tool_uses[0].name, "WebSearch");
                assert_eq!(tool_uses[0].id, "toolu_abc");
                assert_eq!(tool_uses[0].input["query"], "test");
            }
            InterceptKind::NativeSearch { .. } => panic!("expected CustomTool"),
        }
    }

    #[test]
    fn test_extract_native_server_tool_use() {
        // Anthropic native web search: server_tool_use block, stop_reason end_turn
        // The native tool name is "web_search", so we need it in the configured names
        let ws_names = vec!["WebSearch".to_string(), "web_search".to_string()];
        let events = vec![
            json!({
                "event": "content_block_start",
                "data": {"index": 0, "content_block": {"type": "text", "text": ""}}
            }),
            json!({
                "event": "content_block_delta",
                "data": {"index": 0, "delta": {"type": "text_delta", "text": "I'll check the web."}},
            }),
            json!({
                "event": "content_block_stop",
                "data": {"index": 0}
            }),
            json!({
                "event": "content_block_start",
                "data": {"index": 1, "content_block": {"type": "server_tool_use", "id": "srvtoolu_xyz", "name": "web_search", "input": {}}}
            }),
            json!({
                "event": "content_block_delta",
                "data": {"index": 1, "delta": {"type": "input_json_delta", "partial_json": "{\"query\": \"rust lang\"}"}}
            }),
            json!({
                "event": "content_block_stop",
                "data": {"index": 1}
            }),
            json!({
                "event": "content_block_start",
                "data": {"index": 2, "content_block": {"type": "web_search_tool_result", "tool_use_id": "srvtoolu_xyz", "content": []}}
            }),
            json!({
                "event": "content_block_stop",
                "data": {"index": 2}
            }),
            json!({
                "event": "message_delta",
                "data": {"delta": {"stop_reason": "end_turn"}}
            }),
        ];

        let result = extract_websearch_from_sse(&events, &ws_names, &default_wf_names());
        assert!(result.is_some());

        match result.unwrap() {
            InterceptKind::NativeSearch { tool_names } => {
                assert_eq!(tool_names, vec!["web_search"]);
            }
            InterceptKind::CustomTool { .. } => panic!("expected NativeSearch"),
        }
    }

    #[test]
    fn test_extract_ignores_non_websearch_tools() {
        let events = vec![
            json!({
                "event": "content_block_start",
                "data": {"index": 0, "content_block": {"type": "tool_use", "id": "toolu_xyz", "name": "Calculator", "input": {}}}
            }),
            json!({
                "event": "content_block_delta",
                "data": {"index": 0, "delta": {"type": "input_json_delta", "partial_json": "{}"}}
            }),
            json!({
                "event": "content_block_stop",
                "data": {"index": 0}
            }),
            json!({
                "event": "message_delta",
                "data": {"delta": {"stop_reason": "tool_use"}}
            }),
        ];

        assert!(
            extract_websearch_from_sse(&events, &default_ws_names(), &default_wf_names()).is_none()
        );
    }

    #[test]
    fn test_extract_custom_tool_with_thinking() {
        // When extended thinking is enabled, the response has thinking blocks
        // before tool_use blocks. These must be reconstructed properly.
        let events = vec![
            json!({
                "event": "message_start",
                "data": {"type": "message_start", "message": {"role": "assistant"}}
            }),
            json!({
                "event": "content_block_start",
                "data": {"index": 0, "content_block": {"type": "thinking", "thinking": ""}}
            }),
            json!({
                "event": "content_block_delta",
                "data": {"index": 0, "delta": {"type": "thinking_delta", "thinking": "I need to search "}}
            }),
            json!({
                "event": "content_block_delta",
                "data": {"index": 0, "delta": {"type": "thinking_delta", "thinking": "for this query."}}
            }),
            json!({
                "event": "content_block_delta",
                "data": {"index": 0, "delta": {"type": "signature_delta", "signature": "sig_abc123"}}
            }),
            json!({
                "event": "content_block_stop",
                "data": {"index": 0}
            }),
            json!({
                "event": "content_block_start",
                "data": {"index": 1, "content_block": {"type": "text", "text": ""}}
            }),
            json!({
                "event": "content_block_delta",
                "data": {"index": 1, "delta": {"type": "text_delta", "text": "Let me search"}}
            }),
            json!({
                "event": "content_block_stop",
                "data": {"index": 1}
            }),
            json!({
                "event": "content_block_start",
                "data": {"index": 2, "content_block": {"type": "tool_use", "id": "toolu_abc", "name": "WebSearch", "input": {}}}
            }),
            json!({
                "event": "content_block_delta",
                "data": {"index": 2, "delta": {"type": "input_json_delta", "partial_json": "{\"query\": \"test\"}"}}
            }),
            json!({
                "event": "content_block_stop",
                "data": {"index": 2}
            }),
            json!({
                "event": "message_delta",
                "data": {"delta": {"stop_reason": "tool_use"}}
            }),
        ];

        let result = extract_websearch_from_sse(&events, &default_ws_names(), &default_wf_names());
        assert!(result.is_some());

        match result.unwrap() {
            InterceptKind::CustomTool {
                content_blocks,
                tool_uses,
            } => {
                assert_eq!(content_blocks.len(), 3);
                // Thinking block must have its content properly reconstructed
                assert_eq!(content_blocks[0]["type"], "thinking");
                assert_eq!(
                    content_blocks[0]["thinking"],
                    "I need to search for this query."
                );
                assert_eq!(content_blocks[0]["signature"], "sig_abc123");
                // Text block
                assert_eq!(content_blocks[1]["type"], "text");
                assert_eq!(content_blocks[1]["text"], "Let me search");
                // Tool use
                assert_eq!(content_blocks[2]["type"], "tool_use");
                assert_eq!(tool_uses.len(), 1);
                assert_eq!(tool_uses[0].name, "WebSearch");
            }
            InterceptKind::NativeSearch { .. } => panic!("expected CustomTool"),
        }
    }

    #[test]
    fn test_build_followup_body() {
        let original = json!({
            "model": "claude-3-5-sonnet-20241022",
            "max_tokens": 1024,
            "system": "You are helpful.",
            "tools": [{"name": "WebSearch"}],
            "messages": [{"role": "user", "content": "Search for Rust"}],
            "stream": true,
        });

        let assistant_content = vec![
            json!({"type": "text", "text": "Let me search"}),
            json!({"type": "tool_use", "id": "toolu_1", "name": "WebSearch", "input": {"query": "Rust"}}),
        ];

        let tool_results = vec![json!({
            "type": "tool_result",
            "tool_use_id": "toolu_1",
            "content": "[Proxy mock] Web search intercepted. Query: 'Rust'. No real search was performed.",
        })];

        let followup = build_followup_body(&original, assistant_content, tool_results);

        // Should preserve model, max_tokens, system, tools
        assert_eq!(followup["model"], "claude-3-5-sonnet-20241022");
        assert_eq!(followup["max_tokens"], 1024);
        assert_eq!(followup["stream"], true);

        // Messages: original + assistant + user with tool_results
        let msgs = followup["messages"].as_array().unwrap();
        assert_eq!(msgs.len(), 3);
        assert_eq!(msgs[0]["role"], "user");
        assert_eq!(msgs[1]["role"], "assistant");
        assert_eq!(msgs[1]["content"].as_array().unwrap().len(), 2);
        assert_eq!(msgs[2]["role"], "user");
        assert_eq!(msgs[2]["content"][0]["type"], "tool_result");
    }

    #[tokio::test]
    async fn test_build_accept_result_missing_url() {
        let client = reqwest::Client::builder()
            .redirect(reqwest::redirect::Policy::none())
            .build()
            .unwrap();
        let tool_use = ToolUse {
            id: "toolu_accept1".to_string(),
            name: "WebFetch".to_string(),
            input: json!({}), // no url field
        };
        let result = build_accept_result(&tool_use, &client, &default_wf_names()).await;
        assert_eq!(result["type"], "tool_result");
        assert_eq!(result["tool_use_id"], "toolu_accept1");
        assert_eq!(result["is_error"], true);
        let content = result["content"].as_str().unwrap();
        assert!(content.contains("missing"));
    }

    #[tokio::test]
    async fn test_build_accept_result_non_webfetch() {
        let client = reqwest::Client::builder()
            .redirect(reqwest::redirect::Policy::none())
            .build()
            .unwrap();
        let tool_use = ToolUse {
            id: "toolu_accept2".to_string(),
            name: "WebSearch".to_string(),
            input: json!({"query": "test"}),
        };
        let result = build_accept_result(&tool_use, &client, &default_wf_names()).await;
        assert_eq!(result["type"], "tool_result");
        assert_eq!(result["tool_use_id"], "toolu_accept2");
        assert_eq!(result["is_error"], true);
        let content = result["content"].as_str().unwrap();
        assert!(content.contains("only supported for WebFetch"));
    }
}
