use common::models::ProxyRequest;
use leptos::prelude::*;
use std::collections::HashMap;

use crate::collapsible_block;

/// Accumulate SSE block state from a single event.
fn accumulate_sse_block_state(
    event_type: &str,
    data: &serde_json::Value,
    block_text: &mut HashMap<i64, String>,
    block_json: &mut HashMap<i64, String>,
    block_names: &mut HashMap<i64, String>,
    block_types: &mut HashMap<i64, String>,
) {
    match event_type {
        "content_block_start" => {
            let index = data.get("index").and_then(|v| v.as_i64()).unwrap_or(0);
            let btype = data
                .pointer("/content_block/type")
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_string();
            let name = data
                .pointer("/content_block/name")
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_string();
            block_types.insert(index, btype);
            if !name.is_empty() {
                block_names.insert(index, name);
            }
            block_text.remove(&index);
            block_json.remove(&index);
        }
        "content_block_delta" => {
            let index = data.get("index").and_then(|v| v.as_i64()).unwrap_or(0);
            let delta = &data["delta"];
            let dtype = delta.get("type").and_then(|v| v.as_str()).unwrap_or("");
            match dtype {
                "text_delta" => {
                    let text = delta.get("text").and_then(|v| v.as_str()).unwrap_or("");
                    block_text.entry(index).or_default().push_str(text);
                }
                "thinking_delta" => {
                    let text = delta.get("thinking").and_then(|v| v.as_str()).unwrap_or("");
                    block_text.entry(index).or_default().push_str(text);
                }
                "input_json_delta" => {
                    let json = delta
                        .get("partial_json")
                        .and_then(|v| v.as_str())
                        .unwrap_or("");
                    block_json.entry(index).or_default().push_str(json);
                }
                _ => {}
            }
        }
        _ => {}
    }
}

/// Render the summary row for a completed content block.
fn render_sse_block_summary(
    block_types: &HashMap<i64, String>,
    block_names: &HashMap<i64, String>,
    block_json: &HashMap<i64, String>,
    block_text: &HashMap<i64, String>,
    index: i64,
) -> AnyView {
    let btype = block_types.get(&index).map(|s| s.as_str()).unwrap_or("");
    let name = block_names.get(&index).map(|s| s.as_str()).unwrap_or("");

    let label = if !name.is_empty() {
        format!("{} — {}", btype, name)
    } else {
        btype.to_string()
    };

    let content: AnyView = if let Some(json_str) = block_json.get(&index) {
        let formatted = serde_json::from_str::<serde_json::Value>(json_str)
            .and_then(|v| serde_json::to_string_pretty(&v))
            .unwrap_or_else(|_| json_str.clone());
        collapsible_block(&formatted, "")
    } else if let Some(text) = block_text.get(&index) {
        collapsible_block(text, "")
    } else {
        ().into_any()
    };

    view! {
        <tr>
            <td></td>
            <td><strong>{label}</strong></td>
            <td colspan="2">{content}</td>
        </tr>
    }
    .into_any()
}

pub fn render_response_sse(req: &ProxyRequest) -> AnyView {
    // SSE events
    if let Some(ref events_json) = req.response_events_json {
        if let Ok(sse_events) = serde_json::from_str::<Vec<serde_json::Value>>(events_json) {
            let count = sse_events.len().to_string();

            // Track accumulated text/json per content block index
            let mut block_text: HashMap<i64, String> = HashMap::new();
            let mut block_json: HashMap<i64, String> = HashMap::new();
            let mut block_names: HashMap<i64, String> = HashMap::new();
            let mut block_types: HashMap<i64, String> = HashMap::new();

            let mut rows: Vec<AnyView> = Vec::new();

            for (i, event) in sse_events.iter().enumerate() {
                let event_type = event.get("event").and_then(|e| e.as_str()).unwrap_or("");
                let data = &event["data"];

                accumulate_sse_block_state(
                    event_type,
                    data,
                    &mut block_text,
                    &mut block_json,
                    &mut block_names,
                    &mut block_types,
                );

                let summary = summarize_sse_event(event_type, data);
                let raw = serde_json::to_string_pretty(data).unwrap_or_default();
                let idx = (i + 1).to_string();
                let event_type_str = event_type.to_string();
                rows.push(
                    view! {
                        <tr>
                            <td>{idx}</td>
                            <td>{event_type_str}</td>
                            <td>{summary}</td>
                            <td>
                                <details class="collapsible">
                                    <summary><span class="show-more">"show raw"</span></summary>
                                    <pre class="collapsible-full">{raw}</pre>
                                </details>
                            </td>
                        </tr>
                    }
                    .into_any(),
                );

                // Insert summary row after content_block_stop
                if event_type == "content_block_stop" {
                    let index = data.get("index").and_then(|v| v.as_i64()).unwrap_or(0);
                    rows.push(render_sse_block_summary(
                        &block_types,
                        &block_names,
                        &block_json,
                        &block_text,
                        index,
                    ));
                }
            }

            return view! {
                {count}" SSE events"
                <table>
                    <tr><th>"#"</th><th>"Event"</th><th>"Data"</th><th>"Raw"</th></tr>
                    {rows}
                </table>
            }
            .into_any();
        }
    }

    if let Some(ref body) = req.response_body {
        let body = body.clone();
        return view! { <pre>{body}</pre> }.into_any();
    }

    ().into_any()
}

fn summarize_message_start(data: &serde_json::Value) -> String {
    let model = data
        .pointer("/message/model")
        .and_then(|v| v.as_str())
        .unwrap_or("");
    let role = data
        .pointer("/message/role")
        .and_then(|v| v.as_str())
        .unwrap_or("");
    let id = data
        .pointer("/message/id")
        .and_then(|v| v.as_str())
        .unwrap_or("");
    let mut parts = vec![format!("{} {} {}", model, role, id)];
    for key in &[
        "input_tokens",
        "output_tokens",
        "cache_creation_input_tokens",
        "cache_read_input_tokens",
    ] {
        if let Some(tokens) = data
            .pointer(&format!("/message/usage/{}", key))
            .and_then(|v| v.as_i64())
        {
            parts.push(format!("{}: {}", key, tokens));
        }
    }
    parts.join(" | ")
}

fn summarize_content_block_delta(data: &serde_json::Value) -> String {
    let delta = &data["delta"];
    let dtype = delta.get("type").and_then(|v| v.as_str()).unwrap_or("");
    match dtype {
        "text_delta" => {
            let text = delta.get("text").and_then(|v| v.as_str()).unwrap_or("");
            if text.len() > 80 {
                format!("{}...", &text[..80])
            } else {
                text.to_string()
            }
        }
        "thinking_delta" => {
            let text = delta.get("thinking").and_then(|v| v.as_str()).unwrap_or("");
            if text.len() > 80 {
                format!("{}...", &text[..80])
            } else {
                text.to_string()
            }
        }
        "input_json_delta" => {
            let json = delta
                .get("partial_json")
                .and_then(|v| v.as_str())
                .unwrap_or("");
            if json.len() > 80 {
                format!("{}...", &json[..80])
            } else {
                json.to_string()
            }
        }
        _ => serde_json::to_string(delta).unwrap_or_default(),
    }
}

fn summarize_message_delta(data: &serde_json::Value) -> String {
    let stop_reason = data
        .pointer("/delta/stop_reason")
        .and_then(|v| v.as_str())
        .unwrap_or("");
    let mut parts = Vec::new();
    if !stop_reason.is_empty() {
        parts.push(format!("stop: {}", stop_reason));
    }
    for key in &[
        "input_tokens",
        "output_tokens",
        "cache_creation_input_tokens",
        "cache_read_input_tokens",
    ] {
        if let Some(tokens) = data
            .pointer(&format!("/usage/{}", key))
            .and_then(|v| v.as_i64())
        {
            parts.push(format!("{}: {}", key, tokens));
        }
    }
    parts.join(" | ")
}

pub fn summarize_sse_event(event_type: &str, data: &serde_json::Value) -> String {
    match event_type {
        "message_start" => summarize_message_start(data),
        "content_block_start" => {
            let btype = data
                .pointer("/content_block/type")
                .and_then(|v| v.as_str())
                .unwrap_or("");
            let index = data.get("index").and_then(|v| v.as_i64()).unwrap_or(0);
            let name = data
                .pointer("/content_block/name")
                .and_then(|v| v.as_str())
                .unwrap_or("");
            if name.is_empty() {
                format!("[{}] {}", index, btype)
            } else {
                format!("[{}] {} {}", index, btype, name)
            }
        }
        "content_block_delta" => summarize_content_block_delta(data),
        "content_block_stop" => {
            let index = data.get("index").and_then(|v| v.as_i64()).unwrap_or(0);
            format!("[{}]", index)
        }
        "message_delta" => summarize_message_delta(data),
        "message_stop" => String::new(),
        _ => {
            let s = serde_json::to_string(data).unwrap_or_default();
            if s.len() > 120 {
                format!("{}...", &s[..120])
            } else {
                s
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;

    // --- accumulate_sse_block_state tests ---

    #[test]
    fn accumulate_block_start_populates_types_and_names() {
        let mut block_text = HashMap::new();
        let mut block_json = HashMap::new();
        let mut block_names = HashMap::new();
        let mut block_types = HashMap::new();

        let data = serde_json::json!({
            "index": 0,
            "content_block": {"type": "tool_use", "name": "WebFetch"}
        });
        accumulate_sse_block_state(
            "content_block_start",
            &data,
            &mut block_text,
            &mut block_json,
            &mut block_names,
            &mut block_types,
        );

        assert_eq!(block_types.get(&0).unwrap(), "tool_use");
        assert_eq!(block_names.get(&0).unwrap(), "WebFetch");
        assert!(block_text.is_empty());
        assert!(block_json.is_empty());
    }

    #[test]
    fn accumulate_block_start_no_name() {
        let mut block_text = HashMap::new();
        let mut block_json = HashMap::new();
        let mut block_names = HashMap::new();
        let mut block_types = HashMap::new();

        let data = serde_json::json!({
            "index": 1,
            "content_block": {"type": "text"}
        });
        accumulate_sse_block_state(
            "content_block_start",
            &data,
            &mut block_text,
            &mut block_json,
            &mut block_names,
            &mut block_types,
        );

        assert_eq!(block_types.get(&1).unwrap(), "text");
        assert!(!block_names.contains_key(&1));
    }

    #[test]
    fn accumulate_text_delta() {
        let mut block_text = HashMap::new();
        let mut block_json = HashMap::new();
        let mut block_names = HashMap::new();
        let mut block_types = HashMap::new();

        let data = serde_json::json!({
            "index": 0,
            "delta": {"type": "text_delta", "text": "Hello "}
        });
        accumulate_sse_block_state(
            "content_block_delta",
            &data,
            &mut block_text,
            &mut block_json,
            &mut block_names,
            &mut block_types,
        );

        let data2 = serde_json::json!({
            "index": 0,
            "delta": {"type": "text_delta", "text": "World"}
        });
        accumulate_sse_block_state(
            "content_block_delta",
            &data2,
            &mut block_text,
            &mut block_json,
            &mut block_names,
            &mut block_types,
        );

        assert_eq!(block_text.get(&0).unwrap(), "Hello World");
    }

    #[test]
    fn accumulate_thinking_delta() {
        let mut block_text = HashMap::new();
        let mut block_json = HashMap::new();
        let mut block_names = HashMap::new();
        let mut block_types = HashMap::new();

        let data = serde_json::json!({
            "index": 0,
            "delta": {"type": "thinking_delta", "thinking": "I think..."}
        });
        accumulate_sse_block_state(
            "content_block_delta",
            &data,
            &mut block_text,
            &mut block_json,
            &mut block_names,
            &mut block_types,
        );

        assert_eq!(block_text.get(&0).unwrap(), "I think...");
    }

    #[test]
    fn accumulate_input_json_delta() {
        let mut block_text = HashMap::new();
        let mut block_json = HashMap::new();
        let mut block_names = HashMap::new();
        let mut block_types = HashMap::new();

        let data = serde_json::json!({
            "index": 0,
            "delta": {"type": "input_json_delta", "partial_json": "{\"url\":"}
        });
        accumulate_sse_block_state(
            "content_block_delta",
            &data,
            &mut block_text,
            &mut block_json,
            &mut block_names,
            &mut block_types,
        );

        let data2 = serde_json::json!({
            "index": 0,
            "delta": {"type": "input_json_delta", "partial_json": " \"https://example.com\"}"}
        });
        accumulate_sse_block_state(
            "content_block_delta",
            &data2,
            &mut block_text,
            &mut block_json,
            &mut block_names,
            &mut block_types,
        );

        assert_eq!(
            block_json.get(&0).unwrap(),
            "{\"url\": \"https://example.com\"}"
        );
        assert!(!block_text.contains_key(&0));
    }

    #[test]
    fn accumulate_block_start_clears_previous() {
        let mut block_text = HashMap::new();
        let mut block_json = HashMap::new();
        let mut block_names = HashMap::new();
        let mut block_types = HashMap::new();

        // First populate some data
        block_text.insert(0, "old text".to_string());
        block_json.insert(0, "old json".to_string());

        let data = serde_json::json!({
            "index": 0,
            "content_block": {"type": "text"}
        });
        accumulate_sse_block_state(
            "content_block_start",
            &data,
            &mut block_text,
            &mut block_json,
            &mut block_names,
            &mut block_types,
        );

        // Previous data should be cleared
        assert!(!block_text.contains_key(&0));
        assert!(!block_json.contains_key(&0));
    }

    #[test]
    fn accumulate_unknown_event_ignored() {
        let mut block_text = HashMap::new();
        let mut block_json = HashMap::new();
        let mut block_names = HashMap::new();
        let mut block_types = HashMap::new();

        let data = serde_json::json!({"index": 0});
        accumulate_sse_block_state(
            "message_start",
            &data,
            &mut block_text,
            &mut block_json,
            &mut block_names,
            &mut block_types,
        );

        assert!(block_types.is_empty());
        assert!(block_text.is_empty());
    }

    // --- summarize_message_start tests ---

    #[test]
    fn summarize_message_start_full() {
        let data = serde_json::json!({
            "message": {
                "model": "claude-3-5-sonnet",
                "role": "assistant",
                "id": "msg_123",
                "usage": {
                    "input_tokens": 100,
                    "output_tokens": 50,
                }
            }
        });
        let result = summarize_message_start(&data);
        assert!(result.contains("claude-3-5-sonnet"));
        assert!(result.contains("assistant"));
        assert!(result.contains("msg_123"));
        assert!(result.contains("input_tokens: 100"));
        assert!(result.contains("output_tokens: 50"));
    }

    #[test]
    fn summarize_message_start_no_usage() {
        let data = serde_json::json!({
            "message": {
                "model": "claude-3-haiku",
                "role": "assistant",
                "id": "msg_456",
            }
        });
        let result = summarize_message_start(&data);
        assert_eq!(result, "claude-3-haiku assistant msg_456");
    }

    #[test]
    fn summarize_message_start_with_cache_tokens() {
        let data = serde_json::json!({
            "message": {
                "model": "claude",
                "role": "assistant",
                "id": "m1",
                "usage": {
                    "input_tokens": 10,
                    "cache_creation_input_tokens": 5,
                    "cache_read_input_tokens": 3,
                }
            }
        });
        let result = summarize_message_start(&data);
        assert!(result.contains("cache_creation_input_tokens: 5"));
        assert!(result.contains("cache_read_input_tokens: 3"));
    }

    #[test]
    fn summarize_message_start_empty() {
        let data = serde_json::json!({});
        let result = summarize_message_start(&data);
        assert_eq!(result, "  ");
    }

    // --- summarize_content_block_delta tests ---

    #[test]
    fn summarize_content_block_delta_text() {
        let data = serde_json::json!({
            "delta": {"type": "text_delta", "text": "Hello world"}
        });
        assert_eq!(summarize_content_block_delta(&data), "Hello world");
    }

    #[test]
    fn summarize_content_block_delta_text_truncated() {
        let long_text = "a".repeat(120);
        let data = serde_json::json!({
            "delta": {"type": "text_delta", "text": long_text}
        });
        let result = summarize_content_block_delta(&data);
        assert!(result.ends_with("..."));
        assert!(result.len() < 90);
    }

    #[test]
    fn summarize_content_block_delta_thinking() {
        let data = serde_json::json!({
            "delta": {"type": "thinking_delta", "thinking": "Let me consider"}
        });
        assert_eq!(summarize_content_block_delta(&data), "Let me consider");
    }

    #[test]
    fn summarize_content_block_delta_input_json() {
        let data = serde_json::json!({
            "delta": {"type": "input_json_delta", "partial_json": "{\"url\": \"test\"}"}
        });
        assert_eq!(summarize_content_block_delta(&data), "{\"url\": \"test\"}");
    }

    #[test]
    fn summarize_content_block_delta_unknown_type() {
        let data = serde_json::json!({
            "delta": {"type": "unknown_delta", "value": 42}
        });
        let result = summarize_content_block_delta(&data);
        // Should serialize the delta as JSON
        assert!(result.contains("unknown_delta"));
    }

    // --- summarize_message_delta tests ---

    #[test]
    fn summarize_message_delta_end_turn() {
        let data = serde_json::json!({
            "delta": {"stop_reason": "end_turn"},
            "usage": {"output_tokens": 150}
        });
        let result = summarize_message_delta(&data);
        assert!(result.contains("stop: end_turn"));
        assert!(result.contains("output_tokens: 150"));
    }

    #[test]
    fn summarize_message_delta_tool_use() {
        let data = serde_json::json!({
            "delta": {"stop_reason": "tool_use"},
            "usage": {"output_tokens": 200}
        });
        let result = summarize_message_delta(&data);
        assert!(result.contains("stop: tool_use"));
    }

    #[test]
    fn summarize_message_delta_no_stop_reason() {
        let data = serde_json::json!({
            "delta": {},
            "usage": {"output_tokens": 50}
        });
        let result = summarize_message_delta(&data);
        assert_eq!(result, "output_tokens: 50");
    }

    #[test]
    fn summarize_message_delta_empty() {
        let data = serde_json::json!({
            "delta": {}
        });
        let result = summarize_message_delta(&data);
        assert!(result.is_empty());
    }

    #[test]
    fn summarize_message_delta_all_token_types() {
        let data = serde_json::json!({
            "delta": {"stop_reason": "end_turn"},
            "usage": {
                "input_tokens": 10,
                "output_tokens": 20,
                "cache_creation_input_tokens": 5,
                "cache_read_input_tokens": 3,
            }
        });
        let result = summarize_message_delta(&data);
        assert!(result.contains("stop: end_turn"));
        assert!(result.contains("input_tokens: 10"));
        assert!(result.contains("output_tokens: 20"));
        assert!(result.contains("cache_creation_input_tokens: 5"));
        assert!(result.contains("cache_read_input_tokens: 3"));
    }
}
