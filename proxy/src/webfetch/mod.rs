mod approval;
mod extract;
mod fetch;
mod mock;

pub use approval::{
    list_pending, new_approval_queue, resolve_pending, ApprovalDecision, ApprovalQueue,
    PendingApproval,
};
pub use common::models::PendingToolInfo;
pub use fetch::WEBFETCH_AGENT_SYSTEM_PROMPT;

use common::config::AppConfig;
use serde_json::Value;

use self::extract::{
    build_followup_body, build_input_summary, extract_webfetch_from_sse, is_all_whitelisted,
    retain_matched_tool_blocks, InterceptedTools, ToolUse,
};
use self::fetch::{build_accept_result, FetchContext};
use self::mock::{build_fail_result, build_mock_result};
use crate::shared::{
    extract_request_fields, headers_to_json, log_request, store_response, RequestMeta,
};
use crate::sse::parse_sse_events;

/// Maximum number of intercept rounds to prevent infinite loops.
const MAX_INTERCEPT_ROUNDS: usize = 10;

/// Timeout in seconds for waiting for user approval.
const APPROVAL_TIMEOUT_SECS: u64 = 120;

/// Data collected for each round of interception.
struct RoundData {
    decision: String,
    tool_names: Vec<String>,
    request_id: Option<String>,
    agent_request_ids: Vec<Option<String>>,
    followup_body: Value,
    response_body: String,
    response_events: Vec<Value>,
}

/// Result of webfetch interception.
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
}

/// Parameters for webfetch interception.
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
    pub webfetch_names: &'a [String],
    pub config: &'a AppConfig,
}

/// Wait for user approval via the dashboard UI, or auto-accept if all tools are whitelisted.
/// Returns the decision and a human-readable label for logging/display.
async fn wait_for_approval(
    tool_uses: &[extract::ToolUse],
    tools_info: Vec<PendingToolInfo>,
    whitelist: &[String],
    webfetch_names: &[String],
    approval_queue: &ApprovalQueue,
    session_id: &str,
    round_idx: usize,
) -> (ApprovalDecision, &'static str) {
    if is_all_whitelisted(tool_uses, whitelist, webfetch_names) {
        log::info!(
            "WebFetch interception round {}: all tools whitelisted, auto-accepting",
            round_idx + 1,
        );
        return (ApprovalDecision::Accept, "Auto-Accept (whitelisted)");
    }

    let (tx, rx) = tokio::sync::oneshot::channel();
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

    match tokio::time::timeout(std::time::Duration::from_secs(APPROVAL_TIMEOUT_SECS), rx).await {
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
            log::info!("WebFetch interception: approval timed out, auto-failing");
            (ApprovalDecision::Fail, "Timeout (auto-fail)")
        }
    }
}

/// Context for logging a follow-up round to the database.
struct FollowupRoundContext<'a> {
    pool: &'a sqlx::SqlitePool,
    session_id: &'a str,
    stored_path: &'a str,
    headers: &'a reqwest::header::HeaderMap,
    followup_body: &'a Value,
    final_status: u16,
    final_headers: &'a reqwest::header::HeaderMap,
    response_body_str: &'a str,
    round_idx: usize,
}

/// Log a follow-up request/response round to the database.
/// Returns the request ID if logging succeeded.
async fn log_followup_round(ctx: &FollowupRoundContext<'_>) -> Option<String> {
    let timestamp = chrono::Local::now().format("%H:%M:%S").to_string();
    let note = format!("webfetch follow-up (round {})", ctx.round_idx + 1);
    let fields = extract_request_fields(ctx.followup_body, None).unwrap_or_default();
    let headers_json = headers_to_json(
        ctx.headers
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
        Ok(id) => {
            let resp_headers_json = headers_to_json(
                ctx.final_headers
                    .iter()
                    .filter_map(|(k, v)| v.to_str().ok().map(|s| (k.to_string(), s.to_string()))),
            )
            .ok();
            if let Err(e) = store_response(
                ctx.pool,
                &id,
                ctx.final_status,
                resp_headers_json.as_deref(),
                ctx.response_body_str,
            )
            .await
            {
                log::warn!("webfetch: failed to store response: {}", e);
            }
            Some(id)
        }
        Err(e) => {
            log::warn!(
                "WebFetch interception: failed to log follow-up request: {}",
                e
            );
            None
        }
    }
}

/// Build tool results for a single round based on the approval decision.
async fn build_tool_results(
    decision: &ApprovalDecision,
    tool_uses: &[ToolUse],
    config: &AppConfig,
    ctx: &FetchContext<'_>,
) -> (Vec<Value>, Vec<Option<String>>) {
    match decision {
        ApprovalDecision::Fail => {
            let results: Vec<Value> = tool_uses.iter().map(build_fail_result).collect();
            let ids = vec![None; results.len()];
            (results, ids)
        }
        ApprovalDecision::Mock => {
            let results: Vec<Value> = tool_uses
                .iter()
                .map(|tu| build_mock_result(tu, &config.webfetch_mock_prompt))
                .collect();
            let ids = vec![None; results.len()];
            (results, ids)
        }
        ApprovalDecision::Accept => {
            let mut results = Vec::with_capacity(tool_uses.len());
            let mut ids = Vec::with_capacity(tool_uses.len());
            for tu in tool_uses {
                let accept = build_accept_result(tu, ctx).await;
                results.push(accept.tool_result);
                ids.push(accept.agent_request_id);
            }
            (results, ids)
        }
    }
}

/// Send a follow-up request to the upstream API and return the response.
async fn send_followup_request(
    client: &reqwest::Client,
    target_url: &str,
    headers: &reqwest::header::HeaderMap,
    followup_body: &Value,
) -> Option<(u16, reqwest::header::HeaderMap, bytes::Bytes)> {
    let followup_bytes = match serde_json::to_vec(followup_body) {
        Ok(v) => v,
        Err(e) => {
            log::warn!(
                "WebFetch interception: failed to serialize follow-up body: {}",
                e
            );
            return None;
        }
    };

    let followup_response = match client
        .post(target_url)
        .headers(headers.clone())
        .body(followup_bytes)
        .send()
        .await
    {
        Ok(r) => r,
        Err(e) => {
            log::warn!("WebFetch interception: follow-up request failed: {}", e);
            return None;
        }
    };

    let status = followup_response.status().as_u16();
    let response_headers = followup_response.headers().clone();

    let body = match followup_response.bytes().await {
        Ok(b) => b,
        Err(e) => {
            log::warn!(
                "WebFetch interception: failed to read follow-up response: {}",
                e
            );
            return None;
        }
    };

    Some((status, response_headers, body))
}

/// Build the note string summarizing the interception.
fn build_intercept_note(all_tool_names: &[String], round_count: usize) -> String {
    if round_count == 1 {
        format!("webfetch intercepted: {}", all_tool_names.join(", "))
    } else {
        format!(
            "webfetch intercepted ({} rounds): {}",
            round_count,
            all_tool_names.join(", ")
        )
    }
}

/// Serialize rounds data into `(followup_body_json, rounds_json)`.
fn serialize_rounds(rounds: &[RoundData]) -> Option<(String, String)> {
    // First round's followup body for backward compatibility
    let followup_body_json_str = match serde_json::to_string_pretty(&rounds[0].followup_body) {
        Ok(v) => v,
        Err(e) => {
            log::warn!(
                "WebFetch interception: failed to serialize follow-up body: {}",
                e
            );
            return None;
        }
    };

    // Serialize all rounds to JSON
    let rounds_value: Vec<Value> = rounds
        .iter()
        .map(|r| {
            serde_json::json!({
                "decision": r.decision,
                "tool_names": r.tool_names,
                "request_id": r.request_id,
                "agent_request_ids": r.agent_request_ids,
                "followup_body": r.followup_body,
                "response_body": r.response_body,
                "response_events": r.response_events,
            })
        })
        .collect();
    let rounds_json = serde_json::to_string(&rounds_value).unwrap_or_default();

    Some((followup_body_json_str, rounds_json))
}

/// Main entry point for webfetch interception.
///
/// Custom `tool_use` (stop_reason "tool_use"): pauses and waits for the user's approval
/// decision (Fail, Mock, or Accept) via the dashboard UI, then builds the appropriate
/// tool_results, sends a follow-up request upstream, and returns the follow-up response
/// to the client.
///
/// Returns `Some(InterceptResult)` if any webfetch tool calls were detected, `None` otherwise.
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
    let webfetch_names = params.webfetch_names;
    let config = params.config;

    let sse_events = parse_sse_events(response_body);

    let InterceptedTools {
        mut content_blocks,
        tool_uses,
    } = extract_webfetch_from_sse(&sse_events, webfetch_names)?;

    // Remove tool_use content blocks that were filtered out, so the
    // follow-up body stays consistent with the tool_results we provide.
    retain_matched_tool_blocks(&mut content_blocks, &tool_uses);

    let original_json: Value = match serde_json::from_slice(original_body) {
        Ok(v) => v,
        Err(e) => {
            log::warn!(
                "WebFetch interception: failed to parse original body: {}",
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

    let fetch_ctx = FetchContext {
        client,
        webfetch_names,
        accept_prompt: &config.webfetch_accept_prompt,
        redirect_prompt: &config.webfetch_redirect_prompt,
        agent_model: &config.webfetch_agent_model,
        target_url,
        forward_headers: &headers,
        pool,
        session_id,
        stored_path,
    };

    for round_idx in 0..MAX_INTERCEPT_ROUNDS {
        let intercepted_tools: Vec<&str> =
            current_tool_uses.iter().map(|t| t.name.as_str()).collect();
        all_tool_names.extend(intercepted_tools.iter().map(|s| s.to_string()));

        log::info!(
            "WebFetch interception round {}: {} — waiting for user approval",
            round_idx + 1,
            intercepted_tools.join(", ")
        );

        // Build tool info for the UI
        let tools_info: Vec<PendingToolInfo> = current_tool_uses
            .iter()
            .map(|t| PendingToolInfo {
                name: t.name.clone(),
                input_summary: build_input_summary(t),
            })
            .collect();

        // Auto-accept if all tools are whitelisted WebFetch calls
        let (decision, decision_label) = wait_for_approval(
            &current_tool_uses,
            tools_info,
            whitelist,
            webfetch_names,
            approval_queue,
            session_id,
            round_idx,
        )
        .await;

        log::info!(
            "WebFetch interception round {}: user decided {:?}",
            round_idx + 1,
            decision
        );

        let (tool_results, agent_request_ids) =
            build_tool_results(&decision, &current_tool_uses, config, &fetch_ctx).await;

        let followup_body =
            build_followup_body(&current_body, current_content_blocks, tool_results);

        let (followup_status, followup_headers, followup_body_bytes) =
            send_followup_request(client, target_url, &headers, &followup_body).await?;

        final_status = followup_status;
        final_headers = followup_headers;
        final_body = followup_body_bytes;

        let response_body_str = String::from_utf8_lossy(&final_body).to_string();
        let response_events = parse_sse_events(&response_body_str);

        // Log the follow-up as a separate request entry
        let round_request_id = log_followup_round(&FollowupRoundContext {
            pool,
            session_id,
            stored_path,
            headers: &headers,
            followup_body: &followup_body,
            final_status,
            final_headers: &final_headers,
            response_body_str: &response_body_str,
            round_idx,
        })
        .await;

        rounds.push(RoundData {
            decision: decision_label.to_string(),
            tool_names: current_tool_uses.iter().map(|t| t.name.clone()).collect(),
            request_id: round_request_id,
            agent_request_ids,
            followup_body: followup_body.clone(),
            response_body: response_body_str,
            response_events: response_events.clone(),
        });

        // Check if the follow-up response contains more webfetch tool calls
        match extract_webfetch_from_sse(&response_events, webfetch_names) {
            Some(InterceptedTools {
                content_blocks: mut next_blocks,
                tool_uses: next_uses,
            }) => {
                if next_uses.is_empty() {
                    break;
                }
                retain_matched_tool_blocks(&mut next_blocks, &next_uses);
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
            "WebFetch interception: reached max rounds ({}), returning last response as-is",
            MAX_INTERCEPT_ROUNDS
        );
    }

    let note = build_intercept_note(&all_tool_names, rounds.len());
    let (followup_body_json, rounds_json) = serialize_rounds(&rounds)?;

    Some(InterceptResult::Intercepted {
        status: final_status,
        headers: final_headers,
        body: final_body,
        note,
        followup_body_json,
        rounds_json,
    })
}

#[cfg(test)]
mod tests {
    use super::extract::*;
    use super::mock::*;
    use super::*;
    use common::config::AppConfig;
    use tokio::sync::oneshot;

    fn default_config() -> AppConfig {
        AppConfig::default()
    }

    fn default_wf_names() -> Vec<String> {
        vec!["WebFetch".to_string()]
    }

    // --- build_intercept_note tests ---

    #[test]
    fn test_build_intercept_note_single_round() {
        let names = vec!["WebFetch".to_string()];
        assert_eq!(
            build_intercept_note(&names, 1),
            "webfetch intercepted: WebFetch"
        );
    }

    #[test]
    fn test_build_intercept_note_multiple_rounds() {
        let names = vec!["WebFetch".to_string(), "WebSearch".to_string()];
        assert_eq!(
            build_intercept_note(&names, 3),
            "webfetch intercepted (3 rounds): WebFetch, WebSearch"
        );
    }

    #[test]
    fn test_build_intercept_note_single_round_multiple_tools() {
        let names = vec![
            "WebFetch".to_string(),
            "WebSearch".to_string(),
            "WebFetch".to_string(),
        ];
        assert_eq!(
            build_intercept_note(&names, 1),
            "webfetch intercepted: WebFetch, WebSearch, WebFetch"
        );
    }

    // --- serialize_rounds tests ---

    #[test]
    fn test_serialize_rounds_single() {
        let rounds = vec![RoundData {
            decision: "Accept".to_string(),
            tool_names: vec!["WebFetch".to_string()],
            request_id: Some("req_1".to_string()),
            agent_request_ids: vec![Some("agent_1".to_string())],
            followup_body: serde_json::json!({"model": "test", "messages": []}),
            response_body: "response data".to_string(),
            response_events: vec![serde_json::json!({"event": "message_start"})],
        }];
        let (followup_json, rounds_json) = serialize_rounds(&rounds).unwrap();
        // followup_json should be the pretty-printed first round's followup body
        let parsed: serde_json::Value = serde_json::from_str(&followup_json).unwrap();
        assert_eq!(parsed["model"], "test");
        // rounds_json should be a valid JSON array
        let parsed_rounds: Vec<serde_json::Value> = serde_json::from_str(&rounds_json).unwrap();
        assert_eq!(parsed_rounds.len(), 1);
        assert_eq!(parsed_rounds[0]["decision"], "Accept");
        assert_eq!(parsed_rounds[0]["tool_names"][0], "WebFetch");
    }

    #[test]
    fn test_serialize_rounds_multiple() {
        let rounds = vec![
            RoundData {
                decision: "Accept".to_string(),
                tool_names: vec!["WebFetch".to_string()],
                request_id: Some("req_1".to_string()),
                agent_request_ids: vec![None],
                followup_body: serde_json::json!({"round": 1}),
                response_body: "resp1".to_string(),
                response_events: vec![],
            },
            RoundData {
                decision: "Mock".to_string(),
                tool_names: vec!["WebSearch".to_string()],
                request_id: None,
                agent_request_ids: vec![],
                followup_body: serde_json::json!({"round": 2}),
                response_body: "resp2".to_string(),
                response_events: vec![],
            },
        ];
        let (followup_json, rounds_json) = serialize_rounds(&rounds).unwrap();
        // followup_json is always from the first round
        let parsed: serde_json::Value = serde_json::from_str(&followup_json).unwrap();
        assert_eq!(parsed["round"], 1);
        let parsed_rounds: Vec<serde_json::Value> = serde_json::from_str(&rounds_json).unwrap();
        assert_eq!(parsed_rounds.len(), 2);
        assert_eq!(parsed_rounds[1]["decision"], "Mock");
    }

    #[test]
    fn test_build_mock_result_webfetch() {
        let tool_use = ToolUse {
            id: "toolu_456".to_string(),
            name: "WebFetch".to_string(),
            input: serde_json::json!({"url": "https://example.com"}),
        };
        let result = build_mock_result(&tool_use, &default_config().webfetch_mock_prompt);
        assert_eq!(result["type"], "tool_result");
        assert_eq!(result["tool_use_id"], "toolu_456");
        let content = result["content"].as_str().unwrap();
        assert!(content.contains("Web fetch intercepted"));
        assert!(content.contains("https://example.com"));
    }

    #[test]
    fn test_build_fail_result_webfetch() {
        let tool_use = ToolUse {
            id: "toolu_fail2".to_string(),
            name: "WebFetch".to_string(),
            input: serde_json::json!({"url": "https://example.com"}),
        };
        let result = build_fail_result(&tool_use);
        assert_eq!(result["type"], "tool_result");
        assert_eq!(result["tool_use_id"], "toolu_fail2");
        assert_eq!(result["is_error"], true);
    }

    #[test]
    fn test_build_input_summary() {
        let fetch = ToolUse {
            id: "t1".to_string(),
            name: "WebFetch".to_string(),
            input: serde_json::json!({"url": "https://example.com"}),
        };
        assert_eq!(build_input_summary(&fetch), "URL: https://example.com");
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
    fn test_extract_no_webfetch_end_turn() {
        // end_turn with no tool_use blocks → None
        let events = vec![
            serde_json::json!({"event": "message_start", "data": {"type": "message_start"}}),
            serde_json::json!({"event": "message_delta", "data": {"delta": {"stop_reason": "end_turn"}}}),
        ];
        assert!(extract_webfetch_from_sse(&events, &default_wf_names()).is_none());
    }

    #[test]
    fn test_extract_custom_tool_webfetch() {
        let events = vec![
            serde_json::json!({
                "event": "message_start",
                "data": {"type": "message_start", "message": {"role": "assistant"}}
            }),
            serde_json::json!({
                "event": "content_block_start",
                "data": {"index": 0, "content_block": {"type": "text", "text": ""}}
            }),
            serde_json::json!({
                "event": "content_block_delta",
                "data": {"index": 0, "delta": {"type": "text_delta", "text": "Let me fetch"}}
            }),
            serde_json::json!({
                "event": "content_block_stop",
                "data": {"index": 0}
            }),
            serde_json::json!({
                "event": "content_block_start",
                "data": {"index": 1, "content_block": {"type": "tool_use", "id": "toolu_abc", "name": "WebFetch", "input": {}}}
            }),
            serde_json::json!({
                "event": "content_block_delta",
                "data": {"index": 1, "delta": {"type": "input_json_delta", "partial_json": "{\"url\":"}}
            }),
            serde_json::json!({
                "event": "content_block_delta",
                "data": {"index": 1, "delta": {"type": "input_json_delta", "partial_json": " \"https://example.com\"}"}}
            }),
            serde_json::json!({
                "event": "content_block_stop",
                "data": {"index": 1}
            }),
            serde_json::json!({
                "event": "message_delta",
                "data": {"delta": {"stop_reason": "tool_use"}}
            }),
        ];

        let result = extract_webfetch_from_sse(&events, &default_wf_names());
        assert!(result.is_some());

        let InterceptedTools {
            content_blocks,
            tool_uses,
        } = result.unwrap();
        assert_eq!(content_blocks.len(), 2);
        assert_eq!(content_blocks[0]["type"], "text");
        assert_eq!(content_blocks[0]["text"], "Let me fetch");
        assert_eq!(content_blocks[1]["type"], "tool_use");
        assert_eq!(tool_uses.len(), 1);
        assert_eq!(tool_uses[0].name, "WebFetch");
        assert_eq!(tool_uses[0].id, "toolu_abc");
        assert_eq!(tool_uses[0].input["url"], "https://example.com");
    }

    #[test]
    fn test_extract_ignores_non_webfetch_tools() {
        let events = vec![
            serde_json::json!({
                "event": "content_block_start",
                "data": {"index": 0, "content_block": {"type": "tool_use", "id": "toolu_xyz", "name": "Calculator", "input": {}}}
            }),
            serde_json::json!({
                "event": "content_block_delta",
                "data": {"index": 0, "delta": {"type": "input_json_delta", "partial_json": "{}"}}
            }),
            serde_json::json!({
                "event": "content_block_stop",
                "data": {"index": 0}
            }),
            serde_json::json!({
                "event": "message_delta",
                "data": {"delta": {"stop_reason": "tool_use"}}
            }),
        ];

        assert!(extract_webfetch_from_sse(&events, &default_wf_names()).is_none());
    }

    #[test]
    fn test_extract_custom_tool_with_thinking() {
        let events = vec![
            serde_json::json!({
                "event": "message_start",
                "data": {"type": "message_start", "message": {"role": "assistant"}}
            }),
            serde_json::json!({
                "event": "content_block_start",
                "data": {"index": 0, "content_block": {"type": "thinking", "thinking": ""}}
            }),
            serde_json::json!({
                "event": "content_block_delta",
                "data": {"index": 0, "delta": {"type": "thinking_delta", "thinking": "I need to search "}}
            }),
            serde_json::json!({
                "event": "content_block_delta",
                "data": {"index": 0, "delta": {"type": "thinking_delta", "thinking": "for this query."}}
            }),
            serde_json::json!({
                "event": "content_block_delta",
                "data": {"index": 0, "delta": {"type": "signature_delta", "signature": "sig_abc123"}}
            }),
            serde_json::json!({
                "event": "content_block_stop",
                "data": {"index": 0}
            }),
            serde_json::json!({
                "event": "content_block_start",
                "data": {"index": 1, "content_block": {"type": "text", "text": ""}}
            }),
            serde_json::json!({
                "event": "content_block_delta",
                "data": {"index": 1, "delta": {"type": "text_delta", "text": "Let me search"}}
            }),
            serde_json::json!({
                "event": "content_block_stop",
                "data": {"index": 1}
            }),
            serde_json::json!({
                "event": "content_block_start",
                "data": {"index": 2, "content_block": {"type": "tool_use", "id": "toolu_abc", "name": "WebFetch", "input": {}}}
            }),
            serde_json::json!({
                "event": "content_block_delta",
                "data": {"index": 2, "delta": {"type": "input_json_delta", "partial_json": "{\"url\": \"https://example.com\"}"}}
            }),
            serde_json::json!({
                "event": "content_block_stop",
                "data": {"index": 2}
            }),
            serde_json::json!({
                "event": "message_delta",
                "data": {"delta": {"stop_reason": "tool_use"}}
            }),
        ];

        let result = extract_webfetch_from_sse(&events, &default_wf_names());
        assert!(result.is_some());

        let InterceptedTools {
            content_blocks,
            tool_uses,
        } = result.unwrap();
        assert_eq!(content_blocks.len(), 3);
        assert_eq!(content_blocks[0]["type"], "thinking");
        assert_eq!(
            content_blocks[0]["thinking"],
            "I need to search for this query."
        );
        assert_eq!(content_blocks[0]["signature"], "sig_abc123");
        assert_eq!(content_blocks[1]["type"], "text");
        assert_eq!(content_blocks[1]["text"], "Let me search");
        assert_eq!(content_blocks[2]["type"], "tool_use");
        assert_eq!(tool_uses.len(), 1);
        assert_eq!(tool_uses[0].name, "WebFetch");
    }

    #[test]
    fn test_build_followup_body() {
        let original = serde_json::json!({
            "model": "claude-3-5-sonnet-20241022",
            "max_tokens": 1024,
            "system": "You are helpful.",
            "tools": [{"name": "WebSearch"}],
            "messages": [{"role": "user", "content": "Search for Rust"}],
            "stream": true,
        });

        let assistant_content = vec![
            serde_json::json!({"type": "text", "text": "Let me search"}),
            serde_json::json!({"type": "tool_use", "id": "toolu_1", "name": "WebSearch", "input": {"query": "Rust"}}),
        ];

        let tool_results = vec![serde_json::json!({
            "type": "tool_result",
            "tool_use_id": "toolu_1",
            "content": "[Proxy mock] Web search intercepted. Query: 'Rust'. No real search was performed.",
        })];

        let followup = build_followup_body(&original, assistant_content, tool_results);

        assert_eq!(followup["model"], "claude-3-5-sonnet-20241022");
        assert_eq!(followup["max_tokens"], 1024);
        assert_eq!(followup["stream"], true);

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
            input: serde_json::json!({}),
        };
        let headers = reqwest::header::HeaderMap::new();
        let pool = sqlx::SqlitePool::connect("sqlite::memory:").await.unwrap();
        let wf_names = default_wf_names();
        let ctx = FetchContext {
            client: &client,
            webfetch_names: &wf_names,
            accept_prompt: "",
            redirect_prompt: "",
            agent_model: "",
            target_url: "",
            forward_headers: &headers,
            pool: &pool,
            session_id: "test-session",
            stored_path: "/test",
        };
        let result = build_accept_result(&tool_use, &ctx).await;
        assert_eq!(result.tool_result["type"], "tool_result");
        assert_eq!(result.tool_result["tool_use_id"], "toolu_accept1");
        assert_eq!(result.tool_result["is_error"], true);
        let content = result.tool_result["content"].as_str().unwrap();
        assert!(content.contains("missing"));
        assert!(result.agent_request_id.is_none());
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
            input: serde_json::json!({"query": "test"}),
        };
        let headers = reqwest::header::HeaderMap::new();
        let pool = sqlx::SqlitePool::connect("sqlite::memory:").await.unwrap();
        let wf_names = default_wf_names();
        let ctx = FetchContext {
            client: &client,
            webfetch_names: &wf_names,
            accept_prompt: "",
            redirect_prompt: "",
            agent_model: "",
            target_url: "",
            forward_headers: &headers,
            pool: &pool,
            session_id: "test-session",
            stored_path: "/test",
        };
        let result = build_accept_result(&tool_use, &ctx).await;
        assert_eq!(result.tool_result["type"], "tool_result");
        assert_eq!(result.tool_result["tool_use_id"], "toolu_accept2");
        assert_eq!(result.tool_result["is_error"], true);
        let content = result.tool_result["content"].as_str().unwrap();
        assert!(content.contains("only supported for WebFetch"));
        assert!(result.agent_request_id.is_none());
    }
}
